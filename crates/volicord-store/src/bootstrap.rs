use std::{
    collections::BTreeSet,
    fmt, fs, io,
    path::{Path, PathBuf},
    sync::Arc,
};

use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;
use sha2::{Digest, Sha256};
use tempfile::{Builder as TempDirBuilder, TempDir};
use volicord_platform_fs::{
    DirectoryEntryDurability, DirectoryTreeRemovalEffect, DirectoryTreeRemovalError,
    DirectoryTreeTargetState,
};
use volicord_types::canonical::canonical_json_string;
use volicord_types::ids::RuntimeHomePublicationId;
use volicord_types::schema::BASELINE_PROJECT_ENFORCEMENT_PROFILE_JSON;
use volicord_types::storage_contract::{
    GeneratedRelationKind, StorageDatabaseKind, StorageManifest,
};
use volicord_types::values::UtcTimestamp;

use crate::{
    runtime_home::{
        normalize_lexical_path, paths_equal_for_boundary, validate_project_home_boundary,
        validate_project_home_boundary_admitted, validate_runtime_home_product_repository,
        validate_runtime_home_product_repository_admitted, RuntimePathBoundaryError,
    },
    schema::{
        current_schema_facts, current_storage_manifest, current_storage_manifest_json,
        extract_schema_facts, GeneratedSchemaFacts,
    },
    sqlite::{
        begin_immediate_transaction, create_project_state_database_for_mutation,
        create_registry_database_for_setup, open_read_only_database,
        open_registry_database_for_mutation, open_registry_database_read_only, project_home_path,
        registry_db_path, validate_project_state_schema, validate_registry_schema,
        with_immediate_transaction, PROJECT_STATE_DB_FILE,
    },
    CanonicalRuntimeHomePath, RuntimeHomeMutationContext, StoreError, StoreResult,
};

/// Baseline-valid project registration status.
pub const ACTIVE_PROJECT_STATUS: &str = "active";

/// Runtime Home metadata stored in `registry.sqlite`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeHomeRecord {
    pub runtime_home: PathBuf,
    pub registry_db_path: PathBuf,
    pub runtime_home_id: String,
    pub publication_id: RuntimeHomePublicationId,
    pub storage_profile: String,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Installation profile registration input stored in the Runtime Home registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallationProfileRegistration {
    pub installation_id: String,
    pub volicord_command: String,
    pub volicord_mcp_command: String,
    pub bin_dir: PathBuf,
    pub default_connection_mode: String,
    pub metadata_json: String,
}

/// Installation profile record stored in `registry.sqlite`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallationProfileRecord {
    pub installation_id: String,
    pub runtime_home_id: String,
    pub volicord_command: String,
    pub volicord_mcp_command: String,
    pub bin_dir: PathBuf,
    pub default_connection_mode: String,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Local project registration input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectRegistration {
    pub project_id: String,
    pub repo_root: PathBuf,
    pub project_home: Option<PathBuf>,
    pub status: String,
    pub metadata_json: String,
}

/// Repository-root project ensure input that does not require caller-supplied IDs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoProjectRegistration {
    pub project_name: Option<String>,
    pub project_alias: Option<String>,
    pub repo_root: PathBuf,
    pub project_home: Option<PathBuf>,
    pub status: String,
    pub metadata_json: String,
}

/// Project registration record stored in `registry.sqlite`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectRecord {
    pub project_internal_id: String,
    pub project_id: String,
    pub project_name: String,
    pub project_alias: String,
    pub runtime_home_id: String,
    pub repo_root: PathBuf,
    pub project_home: PathBuf,
    pub state_db_path: PathBuf,
    pub status: String,
    pub metadata_json: String,
}

/// Exact categories that differ between an existing Registry and the current manifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RuntimeHomeSchemaCategory {
    StorageProfile,
    Relation,
    Trigger,
    Column,
    Index,
    Constraint,
}

impl RuntimeHomeSchemaCategory {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::StorageProfile => "storage_profile",
            Self::Relation => "relation",
            Self::Trigger => "trigger",
            Self::Column => "column",
            Self::Index => "index",
            Self::Constraint => "constraint",
        }
    }
}

impl fmt::Display for RuntimeHomeSchemaCategory {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Bounded read-only mismatch facts for an existing Runtime Home.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeHomeSchemaMismatch {
    pub runtime_home: PathBuf,
    pub expected_manifest_digest: String,
    pub observed_manifest_digest: Option<String>,
    pub missing_relations: Vec<String>,
    pub unexpected_relations: Vec<String>,
    pub changed_relation_categories: Vec<RuntimeHomeSchemaCategory>,
    pub storage_profile_mismatch: bool,
    pub facts_truncated: bool,
    pub existing_state_preserved: bool,
}

impl fmt::Display for RuntimeHomeSchemaMismatch {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let observed = self
            .observed_manifest_digest
            .as_deref()
            .unwrap_or("unavailable");
        write!(
            formatter,
            "Runtime Home schema mismatch at {}: expected manifest digest {}, observed manifest digest {}; missing relations [{}]; unexpected relations [{}]; changed categories [{}]; storage profile mismatch: {}; existing state preserved. Preserve this home, choose a fresh explicit --home, or use an owner-defined importer only if one exists",
            self.runtime_home.display(),
            self.expected_manifest_digest,
            observed,
            self.missing_relations.join(", "),
            self.unexpected_relations.join(", "),
            self.changed_relation_categories
                .iter()
                .map(|category| category.as_str())
                .collect::<Vec<_>>()
                .join(", "),
            self.storage_profile_mismatch,
        )
    }
}

/// Closed corruption classes produced by read-only bootstrap inspection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeHomeCorruptionKind {
    RegistryMissing,
    RuntimeHomeNotDirectory,
    RegistryNotFile,
    SqliteInvalid,
    ManifestCarrierInvalid,
    RuntimeHomeRecordInvalid,
    IntegrityViolation,
}

impl RuntimeHomeCorruptionKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RegistryMissing => "registry_missing",
            Self::RuntimeHomeNotDirectory => "runtime_home_not_directory",
            Self::RegistryNotFile => "registry_not_file",
            Self::SqliteInvalid => "sqlite_invalid",
            Self::ManifestCarrierInvalid => "manifest_carrier_invalid",
            Self::RuntimeHomeRecordInvalid => "runtime_home_record_invalid",
            Self::IntegrityViolation => "integrity_violation",
        }
    }
}

/// Bounded corruption report for an existing Runtime Home.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeHomeCorruption {
    pub runtime_home: PathBuf,
    pub kind: RuntimeHomeCorruptionKind,
    pub existing_state_preserved: bool,
}

impl fmt::Display for RuntimeHomeCorruption {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Runtime Home at {} is corrupt ({}); existing state preserved. Preserve this home, choose a fresh explicit --home, or use an owner-defined importer only if one exists",
            self.runtime_home.display(),
            self.kind.as_str(),
        )
    }
}

/// Read-only state used to decide whether Runtime Home creation may begin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeHomeBootstrapState {
    Absent,
    Ready(RuntimeHomeRecord),
    Incompatible(RuntimeHomeSchemaMismatch),
    Corrupt(RuntimeHomeCorruption),
}

/// Fully built but unpublished Runtime Home state.
#[derive(Debug)]
pub struct PreparedRuntimeHome {
    final_runtime_home: CanonicalRuntimeHomePath,
    staging_directory: TempDir,
    expected_runtime_home_id: String,
    expected_publication_id: RuntimeHomePublicationId,
    expected_manifest_digest: String,
    expected_installation_id: Option<String>,
}

impl PreparedRuntimeHome {
    /// Final Runtime Home path proposed by this preparation.
    pub fn final_path(&self) -> &Path {
        self.final_runtime_home.as_path()
    }

    /// Same-parent unpublished staging path owned by this preparation.
    pub fn staging_path(&self) -> &Path {
        self.staging_directory.path()
    }

    /// Invocation-specific publication provenance persisted in staging.
    pub fn publication_id(&self) -> &RuntimeHomePublicationId {
        &self.expected_publication_id
    }

    /// Expected canonical manifest digest retained across publication.
    pub fn manifest_digest(&self) -> &str {
        &self.expected_manifest_digest
    }
}

/// Explicit result of an atomic no-replace Runtime Home publication attempt.
#[derive(Debug)]
pub enum RuntimeHomePublicationOutcome {
    /// This invocation performed the successful no-replace rename and owns the
    /// guard required for confirmation or rollback.
    PublishedByThisInvocation {
        publication: RuntimeHomePublicationGuard,
    },
    /// Another invocation already published the current final Runtime Home.
    /// This branch carries no removal authority.
    ObservedConcurrentWinner { record: RuntimeHomeRecord },
}

/// Non-cloneable proof that this invocation performed the successful
/// no-replace rename for one prepared Runtime Home.
#[derive(Debug)]
pub struct RuntimeHomePublicationGuard {
    final_path: CanonicalRuntimeHomePath,
    runtime_home_id: String,
    publication_id: RuntimeHomePublicationId,
    manifest_digest: String,
    installation_id: Option<String>,
    state: RuntimeHomePublicationGuardState,
}

#[derive(Debug)]
enum RuntimeHomePublicationGuardState {
    Published,
    Confirmed,
    Preserved(RuntimeHomePublicationPreservationReason),
    OwnershipLost(RuntimeHomePublicationOwnershipLoss),
    RemovalIncomplete(Arc<DirectoryTreeRemovalError>),
    RolledBack {
        durability: DirectoryEntryDurability,
    },
}

/// Closed reason why an owned publication remains at its final path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeHomePublicationPreservationReason {
    /// The setup owner determined that current transaction policy forbids
    /// removal, including possible external visibility.
    SetupPolicy,
    /// A managed-host runtime has consumed the published Runtime Home.
    ManagedHostConsumption,
}

impl RuntimeHomePublicationPreservationReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SetupPolicy => "setup_policy",
            Self::ManagedHostConsumption => "managed_host_consumption",
        }
    }
}

/// Closed exact mismatch that prevents an owned publication rollback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeHomePublicationOwnershipLoss {
    FinalPathMissing,
    FinalPathMismatch,
    RegistryPathMismatch,
    RuntimeHomeIdMismatch,
    PublicationIdMismatch,
    ManifestDigestMismatch,
    InstallationIdentityMismatch,
    SchemaOrRecordInvalid,
}

impl RuntimeHomePublicationOwnershipLoss {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FinalPathMissing => "final_path_missing",
            Self::FinalPathMismatch => "final_path_mismatch",
            Self::RegistryPathMismatch => "registry_path_mismatch",
            Self::RuntimeHomeIdMismatch => "runtime_home_id_mismatch",
            Self::PublicationIdMismatch => "publication_id_mismatch",
            Self::ManifestDigestMismatch => "manifest_digest_mismatch",
            Self::InstallationIdentityMismatch => "installation_identity_mismatch",
            Self::SchemaOrRecordInvalid => "schema_or_record_invalid",
        }
    }
}

/// Result of an explicit token-backed publication rollback attempt.
#[derive(Debug)]
pub enum RuntimeHomePublicationRollbackOutcome {
    RolledBack {
        durability: DirectoryEntryDurability,
        failure: Option<Arc<DirectoryTreeRemovalError>>,
    },
    AlreadyRolledBack {
        durability: DirectoryEntryDurability,
    },
    RemovalIncomplete {
        failure: Arc<DirectoryTreeRemovalError>,
    },
    Preserved {
        reason: RuntimeHomePublicationPreservationReason,
    },
    OwnershipLost {
        reason: RuntimeHomePublicationOwnershipLoss,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuntimeHomeBootstrapPhase {
    SchemaCreation,
    SingletonInsert,
    ManifestValidation,
    AtomicRename,
}

/// Confirmation operation that failed after an owned Runtime Home publication.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeHomePublicationConfirmationPhase {
    ParentDirectorySync,
    PublicationReadBack,
    PublicationManifestValidation,
}

impl RuntimeHomePublicationConfirmationPhase {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ParentDirectorySync => "parent_directory_sync",
            Self::PublicationReadBack => "publication_read_back",
            Self::PublicationManifestValidation => "publication_manifest_validation",
        }
    }
}

/// Typed primary failure from confirmation of an owned publication.
#[derive(Debug)]
pub struct RuntimeHomePublicationConfirmationError {
    pub phase: RuntimeHomePublicationConfirmationPhase,
    pub parent_durability: DirectoryEntryDurability,
    source: Box<StoreError>,
}

impl RuntimeHomePublicationConfirmationError {
    fn new(
        phase: RuntimeHomePublicationConfirmationPhase,
        parent_durability: DirectoryEntryDurability,
        source: StoreError,
    ) -> Self {
        Self {
            phase,
            parent_durability,
            source: Box::new(source),
        }
    }

    pub fn store_error(&self) -> &StoreError {
        &self.source
    }
}

impl fmt::Display for RuntimeHomePublicationConfirmationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Runtime Home publication confirmation failed during {} (parent durability: {}): {}",
            self.phase.as_str(),
            self.parent_durability.as_str(),
            self.source
        )
    }
}

impl std::error::Error for RuntimeHomePublicationConfirmationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.source.as_ref())
    }
}

/// Final-path observation retained with a publication-confirmation failure.
///
/// This is evidence at rollback completion, not a promise that another process
/// cannot recreate the path later.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeHomeFinalPathState {
    Present,
    Absent,
    Uncertain,
}

impl RuntimeHomeFinalPathState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Present => "present",
            Self::Absent => "absent",
            Self::Uncertain => "uncertain",
        }
    }
}

/// Rollback attempt retained by a publication-confirmation failure.
#[derive(Debug)]
pub enum RuntimeHomePublicationRollbackAttempt {
    Completed(RuntimeHomePublicationRollbackOutcome),
    Failed(StoreError),
}

/// Composite failure for a published Runtime Home that could not be confirmed.
#[derive(Debug)]
pub struct RuntimeHomePublicationConfirmationFailure {
    pub primary: RuntimeHomePublicationConfirmationError,
    pub publication_occurred: bool,
    pub rollback: RuntimeHomePublicationRollbackAttempt,
    pub final_path_state: RuntimeHomeFinalPathState,
    pub rollback_parent_durability: DirectoryEntryDurability,
}

impl fmt::Display for RuntimeHomePublicationConfirmationFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}; publication occurred: {}; rollback final path: {}; rollback parent durability: {}",
            self.primary,
            self.publication_occurred,
            self.final_path_state.as_str(),
            self.rollback_parent_durability.as_str()
        )
    }
}

impl std::error::Error for RuntimeHomePublicationConfirmationFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.primary)
    }
}

const BOOTSTRAP_MISMATCH_FACT_LIMIT: usize = 32;

/// Inspects an existing Runtime Home read-only, or reports that its final path is absent.
pub fn inspect_runtime_home_bootstrap(
    runtime_home: impl AsRef<Path>,
) -> StoreResult<RuntimeHomeBootstrapState> {
    let runtime_home = runtime_home.as_ref().to_path_buf();
    let runtime_metadata = match fs::symlink_metadata(&runtime_home) {
        Ok(metadata) => metadata,
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
            ) =>
        {
            return Ok(RuntimeHomeBootstrapState::Absent);
        }
        Err(error) => return Err(StoreError::Io(error)),
    };
    if !runtime_metadata.file_type().is_dir() {
        return Ok(corrupt_runtime_home(
            runtime_home,
            RuntimeHomeCorruptionKind::RuntimeHomeNotDirectory,
        ));
    }

    let registry_path = registry_db_path(&runtime_home);
    let registry_metadata = match fs::symlink_metadata(&registry_path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(corrupt_runtime_home(
                runtime_home,
                RuntimeHomeCorruptionKind::RegistryMissing,
            ));
        }
        Err(error) => return Err(StoreError::Io(error)),
    };
    if !registry_metadata.file_type().is_file() {
        return Ok(corrupt_runtime_home(
            runtime_home,
            RuntimeHomeCorruptionKind::RegistryNotFile,
        ));
    }

    let conn = match open_read_only_database(&registry_path) {
        Ok(conn) => conn,
        Err(StoreError::Sqlite(error)) if sqlite_is_corrupt(&error) => {
            return Ok(corrupt_runtime_home(
                runtime_home,
                RuntimeHomeCorruptionKind::SqliteInvalid,
            ));
        }
        Err(error) => return Err(error),
    };
    let expected = current_schema_facts(StorageDatabaseKind::Registry)?;
    let actual = match extract_schema_facts(&conn, StorageDatabaseKind::Registry) {
        Ok(actual) => actual,
        Err(_) => {
            return Ok(corrupt_runtime_home(
                runtime_home,
                RuntimeHomeCorruptionKind::SqliteInvalid,
            ));
        }
    };
    let observed_profile = observed_registry_manifest(&conn);
    let expected_manifest_json = current_storage_manifest_json()?;

    if actual != expected {
        return Ok(RuntimeHomeBootstrapState::Incompatible(
            runtime_home_schema_mismatch(
                runtime_home,
                &expected,
                &actual,
                expected_manifest_json,
                observed_profile.as_deref(),
            ),
        ));
    }

    let Some(observed_profile) = observed_profile else {
        return Ok(corrupt_runtime_home(
            runtime_home,
            RuntimeHomeCorruptionKind::ManifestCarrierInvalid,
        ));
    };
    let observed_manifest = match serde_json::from_str::<StorageManifest>(&observed_profile) {
        Ok(manifest) => manifest,
        Err(_) => {
            return Ok(corrupt_runtime_home(
                runtime_home,
                RuntimeHomeCorruptionKind::ManifestCarrierInvalid,
            ));
        }
    };
    let observed_canonical = canonical_json_string(&observed_manifest).map_err(|error| {
        StoreError::schema_invariant(
            "registry",
            format!("observed manifest canonical encoding failed: {error}"),
        )
    })?;
    if &observed_manifest != current_storage_manifest()? {
        return Ok(RuntimeHomeBootstrapState::Incompatible(
            runtime_home_schema_mismatch(
                runtime_home,
                &expected,
                &actual,
                expected_manifest_json,
                Some(&observed_profile),
            ),
        ));
    }
    if observed_canonical != observed_profile {
        return Ok(corrupt_runtime_home(
            runtime_home,
            RuntimeHomeCorruptionKind::ManifestCarrierInvalid,
        ));
    }

    let foreign_key_violation = conn
        .prepare("PRAGMA foreign_key_check")?
        .query([])?
        .next()?
        .is_some();
    if foreign_key_violation {
        return Ok(corrupt_runtime_home(
            runtime_home,
            RuntimeHomeCorruptionKind::IntegrityViolation,
        ));
    }

    let record_is_current =
        current_runtime_home_record_matches_path(&conn, &runtime_home, &registry_path)?;
    if !record_is_current {
        return Ok(corrupt_runtime_home(
            runtime_home,
            RuntimeHomeCorruptionKind::RuntimeHomeRecordInvalid,
        ));
    }
    let Some(record) = runtime_home_record_from_conn(&conn, runtime_home.clone(), registry_path)?
    else {
        return Ok(corrupt_runtime_home(
            runtime_home,
            RuntimeHomeCorruptionKind::RuntimeHomeRecordInvalid,
        ));
    };
    if validate_identifier("runtime_home_id", &record.runtime_home_id).is_err()
        || validate_json_object("runtime_home.metadata_json", &record.metadata_json).is_err()
        || record.created_at.trim().is_empty()
        || record.updated_at.trim().is_empty()
    {
        return Ok(corrupt_runtime_home(
            runtime_home,
            RuntimeHomeCorruptionKind::RuntimeHomeRecordInvalid,
        ));
    }

    Ok(RuntimeHomeBootstrapState::Ready(record))
}

/// Creates and validates a Runtime Home in an unpublished same-parent staging directory.
pub fn prepare_runtime_home(
    context: &RuntimeHomeMutationContext<'_>,
    runtime_home_id: &str,
    metadata_json: &str,
) -> StoreResult<PreparedRuntimeHome> {
    let mut hook = |_| Ok(());
    prepare_runtime_home_inner(context, runtime_home_id, metadata_json, None, &mut hook)
}

/// Creates a staged Runtime Home with its installation profile in the same Registry transaction.
pub fn prepare_runtime_home_with_installation(
    context: &RuntimeHomeMutationContext<'_>,
    runtime_home_id: &str,
    metadata_json: &str,
    installation: InstallationProfileRegistration,
) -> StoreResult<PreparedRuntimeHome> {
    let mut hook = |_| Ok(());
    prepare_runtime_home_inner(
        context,
        runtime_home_id,
        metadata_json,
        Some(installation),
        &mut hook,
    )
}

/// Atomically publishes a prepared Runtime Home without replacing an existing
/// path and preserves whether this invocation performed the rename.
pub fn commit_runtime_home(
    context: &RuntimeHomeMutationContext<'_>,
    prepared: PreparedRuntimeHome,
) -> StoreResult<RuntimeHomePublicationOutcome> {
    context.require_exclusive_setup()?;
    context.ensure_runtime_home_identity(&prepared.final_runtime_home)?;
    let mut hook = |_| Ok(());
    commit_runtime_home_inner(prepared, &mut hook)
}

fn commit_runtime_home_inner(
    prepared: PreparedRuntimeHome,
    hook: &mut impl FnMut(RuntimeHomeBootstrapPhase) -> StoreResult<()>,
) -> StoreResult<RuntimeHomePublicationOutcome> {
    let PreparedRuntimeHome {
        final_runtime_home,
        staging_directory,
        expected_runtime_home_id,
        expected_publication_id,
        expected_manifest_digest,
        expected_installation_id,
    } = prepared;
    let staging_path = staging_directory.path().to_path_buf();

    hook(RuntimeHomeBootstrapPhase::AtomicRename)?;
    match rename_directory_no_replace(&staging_path, final_runtime_home.as_path()) {
        Ok(()) => {
            drop(staging_directory);
            Ok(RuntimeHomePublicationOutcome::PublishedByThisInvocation {
                publication: RuntimeHomePublicationGuard {
                    final_path: final_runtime_home,
                    runtime_home_id: expected_runtime_home_id,
                    publication_id: expected_publication_id,
                    manifest_digest: expected_manifest_digest,
                    installation_id: expected_installation_id,
                    state: RuntimeHomePublicationGuardState::Published,
                },
            })
        }
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            drop(staging_directory);
            ready_runtime_home_after_publication(final_runtime_home.as_path())
                .map(|record| RuntimeHomePublicationOutcome::ObservedConcurrentWinner { record })
        }
        Err(error) => Err(StoreError::Io(error)),
    }
}

impl RuntimeHomePublicationGuard {
    /// Final path on which this invocation performed its no-replace rename.
    pub fn final_path(&self) -> &Path {
        self.final_path.as_path()
    }

    /// Invocation-specific persisted publication provenance.
    pub fn publication_id(&self) -> &RuntimeHomePublicationId {
        &self.publication_id
    }

    /// Expected digest of the complete canonical manifest carrier.
    pub fn manifest_digest(&self) -> &str {
        &self.manifest_digest
    }

    /// Synchronizes the final path's parent directory after publication.
    ///
    /// The guard remains owned by the caller when this operation fails.
    pub fn synchronize_parent_directory(
        &mut self,
        context: &RuntimeHomeMutationContext<'_>,
    ) -> StoreResult<DirectoryEntryDurability> {
        context.require_exclusive_setup()?;
        context.ensure_runtime_home_identity(&self.final_path)?;
        self.require_live("synchronize")?;
        let parent =
            self.final_path
                .as_path()
                .parent()
                .ok_or_else(|| StoreError::InvalidInput {
                    detail: "runtime_home must have a parent directory".to_owned(),
                })?;
        sync_directory(parent)?;
        Ok(runtime_home_parent_durability())
    }

    /// Reads back and validates the exact publication identity without yet
    /// accepting the complete canonical schema.
    ///
    /// The guard remains owned by the caller when this operation fails.
    pub fn read_back(
        &mut self,
        context: &RuntimeHomeMutationContext<'_>,
    ) -> StoreResult<RuntimeHomeRecord> {
        context.require_exclusive_setup()?;
        context.ensure_runtime_home_identity(&self.final_path)?;
        self.require_live("read back")?;
        let candidate = runtime_home_publication_candidate(self.final_path.as_path())?;
        if let Some(reason) = self.ownership_loss(&candidate) {
            return Err(self.ownership_error(reason));
        }
        self.validate_installation_identity(&candidate.conn)?;
        Ok(candidate.record)
    }

    /// Validates the complete current canonical manifest, schema, exact final
    /// paths, publication identity, and prepared installation identity.
    ///
    /// The guard remains owned by the caller when this operation fails.
    pub fn validate_manifest_and_confirm(
        &mut self,
        context: &RuntimeHomeMutationContext<'_>,
    ) -> StoreResult<RuntimeHomeRecord> {
        context.require_exclusive_setup()?;
        context.ensure_runtime_home_identity(&self.final_path)?;
        self.require_live("confirm")?;
        let record = ready_runtime_home_after_publication(self.final_path.as_path())?;
        let candidate = runtime_home_publication_candidate(self.final_path.as_path())?;
        if let Some(reason) = self.ownership_loss(&candidate) {
            return Err(self.ownership_error(reason));
        }
        self.validate_installation_identity(&candidate.conn)?;
        if record != candidate.record {
            return Err(
                self.ownership_error(RuntimeHomePublicationOwnershipLoss::SchemaOrRecordInvalid)
            );
        }
        self.state = RuntimeHomePublicationGuardState::Confirmed;
        Ok(record)
    }

    /// Completes parent synchronization, read-back, and exact manifest
    /// validation while keeping the guard in the caller's ownership on error.
    pub fn confirm(
        &mut self,
        context: &RuntimeHomeMutationContext<'_>,
    ) -> Result<RuntimeHomeRecord, RuntimeHomePublicationConfirmationError> {
        let mut hook = |_| Ok(());
        confirm_runtime_home_inner(context, self, &mut hook)
    }

    /// Permanently disables rollback authority for this guard under the
    /// caller's setup policy.
    pub fn preserve(&mut self) {
        if matches!(
            self.state,
            RuntimeHomePublicationGuardState::Published
                | RuntimeHomePublicationGuardState::Confirmed
        ) {
            self.state = RuntimeHomePublicationGuardState::Preserved(
                RuntimeHomePublicationPreservationReason::SetupPolicy,
            );
        }
    }

    /// Removes the final Runtime Home only after immediately revalidating the
    /// exact publication identity, manifest, paths, schema, and consumption
    /// state.
    pub fn rollback_if_owned(
        &mut self,
        context: &RuntimeHomeMutationContext<'_>,
    ) -> StoreResult<RuntimeHomePublicationRollbackOutcome> {
        context.require_exclusive_setup()?;
        context.ensure_runtime_home_identity(&self.final_path)?;
        match &self.state {
            RuntimeHomePublicationGuardState::RolledBack { durability } => {
                return Ok(RuntimeHomePublicationRollbackOutcome::AlreadyRolledBack {
                    durability: *durability,
                });
            }
            RuntimeHomePublicationGuardState::Preserved(reason) => {
                return Ok(RuntimeHomePublicationRollbackOutcome::Preserved { reason: *reason });
            }
            RuntimeHomePublicationGuardState::OwnershipLost(reason) => {
                return Ok(RuntimeHomePublicationRollbackOutcome::OwnershipLost {
                    reason: *reason,
                });
            }
            RuntimeHomePublicationGuardState::RemovalIncomplete(failure) => {
                return Ok(RuntimeHomePublicationRollbackOutcome::RemovalIncomplete {
                    failure: Arc::clone(failure),
                });
            }
            RuntimeHomePublicationGuardState::Published
            | RuntimeHomePublicationGuardState::Confirmed => {}
        }

        let candidate = match runtime_home_publication_candidate(self.final_path.as_path()) {
            Ok(candidate) => candidate,
            Err(StoreError::Io(error))
                if matches!(
                    error.kind(),
                    io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
                ) =>
            {
                return Ok(
                    self.lose_ownership(RuntimeHomePublicationOwnershipLoss::FinalPathMissing)
                );
            }
            Err(StoreError::NotFound { .. }) => {
                return Ok(
                    self.lose_ownership(RuntimeHomePublicationOwnershipLoss::FinalPathMissing)
                );
            }
            Err(
                StoreError::RuntimeHomeSchemaMismatch(_)
                | StoreError::RuntimeHomeCorruption(_)
                | StoreError::CorruptStoredValue { .. }
                | StoreError::CorruptStoredJson { .. }
                | StoreError::SchemaInvariant { .. }
                | StoreError::Sqlite(_),
            ) => {
                return Ok(
                    self.lose_ownership(RuntimeHomePublicationOwnershipLoss::SchemaOrRecordInvalid)
                );
            }
            Err(error) => return Err(error),
        };
        if let Some(reason) = self.ownership_loss(&candidate) {
            return Ok(self.lose_ownership(reason));
        }
        if self
            .validate_installation_identity(&candidate.conn)
            .is_err()
        {
            return Ok(self.lose_ownership(
                RuntimeHomePublicationOwnershipLoss::InstallationIdentityMismatch,
            ));
        }
        match inspect_runtime_home_bootstrap(&self.final_path)? {
            RuntimeHomeBootstrapState::Ready(record) if record == candidate.record => {}
            RuntimeHomeBootstrapState::Absent => {
                return Ok(
                    self.lose_ownership(RuntimeHomePublicationOwnershipLoss::FinalPathMissing)
                );
            }
            RuntimeHomeBootstrapState::Ready(_)
            | RuntimeHomeBootstrapState::Incompatible(_)
            | RuntimeHomeBootstrapState::Corrupt(_) => {
                return Ok(
                    self.lose_ownership(RuntimeHomePublicationOwnershipLoss::SchemaOrRecordInvalid)
                );
            }
        }
        let managed_host_consumed = candidate.conn.query_row(
            "SELECT EXISTS(
                SELECT 1
                  FROM mcp_runtime_sessions
                 WHERE session_source = 'managed_host'
            )",
            [],
            |row| row.get::<_, bool>(0),
        )?;
        drop(candidate);
        if managed_host_consumed {
            let reason = RuntimeHomePublicationPreservationReason::ManagedHostConsumption;
            self.state = RuntimeHomePublicationGuardState::Preserved(reason);
            return Ok(RuntimeHomePublicationRollbackOutcome::Preserved { reason });
        }

        match volicord_platform_fs::remove_owned_directory_tree(self.final_path.as_path()) {
            Ok(outcome) => {
                self.state = RuntimeHomePublicationGuardState::RolledBack {
                    durability: outcome.durability,
                };
                Ok(RuntimeHomePublicationRollbackOutcome::RolledBack {
                    durability: outcome.durability,
                    failure: None,
                })
            }
            Err(error)
                if error.phase
                    == volicord_platform_fs::DirectoryTreeRemovalPhase::TargetInspection
                    && error.effect == DirectoryTreeRemovalEffect::NotRemoved
                    && error.target_state == DirectoryTreeTargetState::Absent =>
            {
                Ok(self.lose_ownership(RuntimeHomePublicationOwnershipLoss::FinalPathMissing))
            }
            Err(error) if error.effect == DirectoryTreeRemovalEffect::Removed => {
                let durability = error.durability;
                let failure = Arc::new(error);
                self.state = RuntimeHomePublicationGuardState::RolledBack { durability };
                Ok(RuntimeHomePublicationRollbackOutcome::RolledBack {
                    durability,
                    failure: Some(failure),
                })
            }
            Err(error) => {
                let failure = Arc::new(error);
                self.state =
                    RuntimeHomePublicationGuardState::RemovalIncomplete(Arc::clone(&failure));
                Ok(RuntimeHomePublicationRollbackOutcome::RemovalIncomplete { failure })
            }
        }
    }

    fn require_live(&self, operation: &'static str) -> StoreResult<()> {
        if matches!(
            self.state,
            RuntimeHomePublicationGuardState::RolledBack { .. }
        ) {
            return Err(StoreError::Conflict {
                entity: "runtime_home_publication",
                id: self.publication_id.to_string(),
                detail: format!("cannot {operation} a rolled-back Runtime Home publication"),
            });
        }
        if matches!(self.state, RuntimeHomePublicationGuardState::Preserved(_)) {
            return Err(StoreError::Conflict {
                entity: "runtime_home_publication",
                id: self.publication_id.to_string(),
                detail: format!("cannot {operation} a preserved Runtime Home publication"),
            });
        }
        if matches!(
            self.state,
            RuntimeHomePublicationGuardState::OwnershipLost(_)
                | RuntimeHomePublicationGuardState::RemovalIncomplete(_)
        ) {
            return Err(StoreError::Conflict {
                entity: "runtime_home_publication",
                id: self.publication_id.to_string(),
                detail: format!(
                    "cannot {operation} a Runtime Home publication without live rollback ownership"
                ),
            });
        }
        Ok(())
    }

    fn lose_ownership(
        &mut self,
        reason: RuntimeHomePublicationOwnershipLoss,
    ) -> RuntimeHomePublicationRollbackOutcome {
        self.state = RuntimeHomePublicationGuardState::OwnershipLost(reason);
        RuntimeHomePublicationRollbackOutcome::OwnershipLost { reason }
    }

    fn ownership_loss(
        &self,
        candidate: &RuntimeHomePublicationCandidate,
    ) -> Option<RuntimeHomePublicationOwnershipLoss> {
        if candidate.stored_runtime_home != self.final_path.as_path() {
            return Some(RuntimeHomePublicationOwnershipLoss::FinalPathMismatch);
        }
        if candidate.stored_registry_path != registry_db_path(&self.final_path) {
            return Some(RuntimeHomePublicationOwnershipLoss::RegistryPathMismatch);
        }
        if candidate.record.runtime_home_id != self.runtime_home_id {
            return Some(RuntimeHomePublicationOwnershipLoss::RuntimeHomeIdMismatch);
        }
        if candidate.record.publication_id != self.publication_id {
            return Some(RuntimeHomePublicationOwnershipLoss::PublicationIdMismatch);
        }
        if sha256_digest(candidate.record.storage_profile.as_bytes()) != self.manifest_digest {
            return Some(RuntimeHomePublicationOwnershipLoss::ManifestDigestMismatch);
        }
        None
    }

    fn validate_installation_identity(&self, conn: &Connection) -> StoreResult<()> {
        let Some(expected) = self.installation_id.as_deref() else {
            return Ok(());
        };
        let observed = conn
            .query_row(
                "SELECT installation_id
                   FROM installation_profile
                  WHERE runtime_home_id = ?1",
                params![self.runtime_home_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        if observed.as_deref() == Some(expected) {
            Ok(())
        } else {
            Err(self
                .ownership_error(RuntimeHomePublicationOwnershipLoss::InstallationIdentityMismatch))
        }
    }

    fn ownership_error(&self, reason: RuntimeHomePublicationOwnershipLoss) -> StoreError {
        StoreError::Conflict {
            entity: "runtime_home_publication",
            id: self.publication_id.to_string(),
            detail: format!(
                "Runtime Home publication ownership could not be confirmed: {}",
                reason.as_str()
            ),
        }
    }
}

fn confirm_runtime_home_inner(
    context: &RuntimeHomeMutationContext<'_>,
    publication: &mut RuntimeHomePublicationGuard,
    hook: &mut impl FnMut(RuntimeHomePublicationConfirmationPhase) -> StoreResult<()>,
) -> Result<RuntimeHomeRecord, RuntimeHomePublicationConfirmationError> {
    hook(RuntimeHomePublicationConfirmationPhase::ParentDirectorySync).map_err(|source| {
        RuntimeHomePublicationConfirmationError::new(
            RuntimeHomePublicationConfirmationPhase::ParentDirectorySync,
            failed_runtime_home_parent_durability(),
            source,
        )
    })?;
    let parent_durability =
        publication
            .synchronize_parent_directory(context)
            .map_err(|source| {
                RuntimeHomePublicationConfirmationError::new(
                    RuntimeHomePublicationConfirmationPhase::ParentDirectorySync,
                    failed_runtime_home_parent_durability(),
                    source,
                )
            })?;
    hook(RuntimeHomePublicationConfirmationPhase::PublicationReadBack).map_err(|source| {
        RuntimeHomePublicationConfirmationError::new(
            RuntimeHomePublicationConfirmationPhase::PublicationReadBack,
            parent_durability,
            source,
        )
    })?;
    let _ = publication.read_back(context).map_err(|source| {
        RuntimeHomePublicationConfirmationError::new(
            RuntimeHomePublicationConfirmationPhase::PublicationReadBack,
            parent_durability,
            source,
        )
    })?;
    hook(RuntimeHomePublicationConfirmationPhase::PublicationManifestValidation).map_err(
        |source| {
            RuntimeHomePublicationConfirmationError::new(
                RuntimeHomePublicationConfirmationPhase::PublicationManifestValidation,
                parent_durability,
                source,
            )
        },
    )?;
    publication
        .validate_manifest_and_confirm(context)
        .map_err(|source| {
            RuntimeHomePublicationConfirmationError::new(
                RuntimeHomePublicationConfirmationPhase::PublicationManifestValidation,
                parent_durability,
                source,
            )
        })
}

/// Creates a fresh Runtime Home atomically or validates an existing one read-only.
pub fn initialize_runtime_home(
    context: &RuntimeHomeMutationContext<'_>,
    runtime_home_id: &str,
    metadata_json: &str,
) -> StoreResult<RuntimeHomeRecord> {
    validate_identifier("runtime_home_id", runtime_home_id)?;
    validate_json_object("runtime_home.metadata_json", metadata_json)?;

    let runtime_home = context.runtime_home().as_path();
    match inspect_runtime_home_bootstrap(runtime_home)? {
        RuntimeHomeBootstrapState::Absent => publish_and_confirm_runtime_home(
            context,
            prepare_runtime_home(context, runtime_home_id, metadata_json)?,
        ),
        RuntimeHomeBootstrapState::Ready(record) => Ok(record),
        RuntimeHomeBootstrapState::Incompatible(mismatch) => {
            Err(StoreError::RuntimeHomeSchemaMismatch(Box::new(mismatch)))
        }
        RuntimeHomeBootstrapState::Corrupt(corruption) => {
            Err(StoreError::RuntimeHomeCorruption(corruption))
        }
    }
}

/// Atomically creates a fresh Runtime Home with installation metadata, or updates
/// installation metadata only after an existing home validates as Ready.
pub fn initialize_runtime_home_with_installation(
    context: &RuntimeHomeMutationContext<'_>,
    runtime_home_id: &str,
    metadata_json: &str,
    installation: InstallationProfileRegistration,
) -> StoreResult<(RuntimeHomeRecord, InstallationProfileRecord)> {
    validate_identifier("runtime_home_id", runtime_home_id)?;
    validate_json_object("runtime_home.metadata_json", metadata_json)?;
    validate_installation_profile_registration(&installation)?;

    let runtime_home = context.runtime_home().as_path();
    let (runtime_home_record, fresh_profile) = match inspect_runtime_home_bootstrap(runtime_home)? {
        RuntimeHomeBootstrapState::Absent => {
            let record = publish_and_confirm_runtime_home(
                context,
                prepare_runtime_home_with_installation(
                    context,
                    runtime_home_id,
                    metadata_json,
                    installation.clone(),
                )?,
            )?;
            (record, installation_profile_read_only(runtime_home)?)
        }
        RuntimeHomeBootstrapState::Ready(record) => (record, None),
        RuntimeHomeBootstrapState::Incompatible(mismatch) => {
            return Err(StoreError::RuntimeHomeSchemaMismatch(Box::new(mismatch)));
        }
        RuntimeHomeBootstrapState::Corrupt(corruption) => {
            return Err(StoreError::RuntimeHomeCorruption(corruption));
        }
    };
    let profile = match fresh_profile {
        Some(profile) => profile,
        None => write_installation_profile(context, installation)?,
    };
    Ok((runtime_home_record, profile))
}

fn publish_and_confirm_runtime_home(
    context: &RuntimeHomeMutationContext<'_>,
    prepared: PreparedRuntimeHome,
) -> StoreResult<RuntimeHomeRecord> {
    let mut confirmation_hook = |_| Ok(());
    let mut before_rollback = |_: &mut RuntimeHomePublicationGuard| Ok(());
    publish_and_confirm_runtime_home_inner(
        context,
        prepared,
        &mut confirmation_hook,
        &mut before_rollback,
    )
}

fn publish_and_confirm_runtime_home_inner(
    context: &RuntimeHomeMutationContext<'_>,
    prepared: PreparedRuntimeHome,
    confirmation_hook: &mut impl FnMut(RuntimeHomePublicationConfirmationPhase) -> StoreResult<()>,
    before_rollback: &mut impl FnMut(&mut RuntimeHomePublicationGuard) -> StoreResult<()>,
) -> StoreResult<RuntimeHomeRecord> {
    match commit_runtime_home(context, prepared)? {
        RuntimeHomePublicationOutcome::ObservedConcurrentWinner { record } => Ok(record),
        RuntimeHomePublicationOutcome::PublishedByThisInvocation { mut publication } => {
            match confirm_runtime_home_inner(context, &mut publication, confirmation_hook) {
                Ok(record) => Ok(record),
                Err(primary) => {
                    let rollback = match before_rollback(&mut publication) {
                        Ok(()) => publication
                            .rollback_if_owned(context)
                            .map(RuntimeHomePublicationRollbackAttempt::Completed)
                            .unwrap_or_else(RuntimeHomePublicationRollbackAttempt::Failed),
                        Err(error) => RuntimeHomePublicationRollbackAttempt::Failed(error),
                    };
                    let (final_path_state, rollback_parent_durability) =
                        rollback_observation(publication.final_path.as_path(), &rollback);
                    Err(StoreError::RuntimeHomePublicationConfirmation(Box::new(
                        RuntimeHomePublicationConfirmationFailure {
                            primary,
                            publication_occurred: true,
                            rollback,
                            final_path_state,
                            rollback_parent_durability,
                        },
                    )))
                }
            }
        }
    }
}

fn rollback_observation(
    final_path: &Path,
    rollback: &RuntimeHomePublicationRollbackAttempt,
) -> (RuntimeHomeFinalPathState, DirectoryEntryDurability) {
    match rollback {
        RuntimeHomePublicationRollbackAttempt::Completed(
            RuntimeHomePublicationRollbackOutcome::RolledBack { durability, .. }
            | RuntimeHomePublicationRollbackOutcome::AlreadyRolledBack { durability },
        ) => (RuntimeHomeFinalPathState::Absent, *durability),
        RuntimeHomePublicationRollbackAttempt::Completed(
            RuntimeHomePublicationRollbackOutcome::RemovalIncomplete { failure },
        ) => (
            final_path_state_from_removal(failure.target_state),
            failure.durability,
        ),
        RuntimeHomePublicationRollbackAttempt::Completed(
            RuntimeHomePublicationRollbackOutcome::Preserved { .. },
        ) => (
            RuntimeHomeFinalPathState::Present,
            DirectoryEntryDurability::NotApplicable,
        ),
        RuntimeHomePublicationRollbackAttempt::Completed(
            RuntimeHomePublicationRollbackOutcome::OwnershipLost { reason },
        ) => (
            if *reason == RuntimeHomePublicationOwnershipLoss::FinalPathMissing {
                RuntimeHomeFinalPathState::Absent
            } else {
                observe_final_path_state(final_path)
            },
            DirectoryEntryDurability::NotApplicable,
        ),
        RuntimeHomePublicationRollbackAttempt::Failed(_) => (
            observe_final_path_state(final_path),
            DirectoryEntryDurability::NotApplicable,
        ),
    }
}

fn final_path_state_from_removal(state: DirectoryTreeTargetState) -> RuntimeHomeFinalPathState {
    match state {
        DirectoryTreeTargetState::Present => RuntimeHomeFinalPathState::Present,
        DirectoryTreeTargetState::Absent => RuntimeHomeFinalPathState::Absent,
        DirectoryTreeTargetState::Unknown => RuntimeHomeFinalPathState::Uncertain,
    }
}

fn observe_final_path_state(path: &Path) -> RuntimeHomeFinalPathState {
    match fs::symlink_metadata(path) {
        Ok(_) => RuntimeHomeFinalPathState::Present,
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
            ) =>
        {
            RuntimeHomeFinalPathState::Absent
        }
        Err(_) => RuntimeHomeFinalPathState::Uncertain,
    }
}

fn prepare_runtime_home_inner(
    context: &RuntimeHomeMutationContext<'_>,
    runtime_home_id: &str,
    metadata_json: &str,
    installation: Option<InstallationProfileRegistration>,
    hook: &mut impl FnMut(RuntimeHomeBootstrapPhase) -> StoreResult<()>,
) -> StoreResult<PreparedRuntimeHome> {
    context.require_exclusive_setup()?;
    let runtime_home_identity = context.runtime_home().clone();
    let runtime_home = runtime_home_identity.as_path();
    validate_identifier("runtime_home_id", runtime_home_id)?;
    validate_json_object("runtime_home.metadata_json", metadata_json)?;
    if let Some(registration) = installation.as_ref() {
        validate_installation_profile_registration(registration)?;
    }
    let parent = validate_fresh_runtime_home_destination(runtime_home)?;
    match inspect_runtime_home_bootstrap(runtime_home)? {
        RuntimeHomeBootstrapState::Absent => {}
        RuntimeHomeBootstrapState::Ready(_) => {
            return Err(StoreError::Conflict {
                entity: "runtime_home",
                id: runtime_home.display().to_string(),
                detail: "the final Runtime Home already exists and is current".to_owned(),
            });
        }
        RuntimeHomeBootstrapState::Incompatible(mismatch) => {
            return Err(StoreError::RuntimeHomeSchemaMismatch(Box::new(mismatch)));
        }
        RuntimeHomeBootstrapState::Corrupt(corruption) => {
            return Err(StoreError::RuntimeHomeCorruption(corruption));
        }
    }

    let staging_directory = TempDirBuilder::new()
        .prefix(".volicord-runtime-staging-")
        .tempdir_in(parent)?;
    let staging_registry = registry_db_path(staging_directory.path());
    let final_registry = registry_db_path(runtime_home);
    let runtime_home_text = path_to_text("runtime_home.runtime_home_path", runtime_home)?;
    let registry_path_text = path_to_text("runtime_home.registry_db_path", &final_registry)?;
    let storage_manifest_json = current_storage_manifest_json()?;
    let expected_manifest_digest = sha256_digest(storage_manifest_json.as_bytes());
    let publication_id = RuntimeHomePublicationId::generate().map_err(|error| {
        StoreError::Io(io::Error::other(format!(
            "Runtime Home publication ID generation failed: {error}"
        )))
    })?;
    let expected_installation_id = installation
        .as_ref()
        .map(|registration| registration.installation_id.clone());
    hook(RuntimeHomeBootstrapPhase::SchemaCreation)?;
    let mut conn = create_registry_database_for_setup(context, &staging_registry)?;

    hook(RuntimeHomeBootstrapPhase::SingletonInsert)?;
    with_immediate_transaction(&mut conn, |tx| {
        tx.execute(
            "INSERT INTO runtime_home (
                singleton_id,
                runtime_home_id,
                publication_id,
                runtime_home_path,
                registry_db_path,
                storage_profile,
                metadata_json,
                created_at,
                updated_at
            )
            VALUES (
                1,
                ?1,
                ?2,
                ?3,
                ?4,
                ?5,
                ?6,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
            )",
            params![
                runtime_home_id,
                publication_id.as_str(),
                runtime_home_text,
                registry_path_text,
                storage_manifest_json,
                metadata_json
            ],
        )?;
        if let Some(registration) = installation.as_ref() {
            insert_installation_profile_tx(tx, runtime_home_id, registration)?;
        }
        Ok(())
    })?;
    hook(RuntimeHomeBootstrapPhase::ManifestValidation)?;
    validate_registry_schema(&conn)?;
    let _record = runtime_home_record_from_conn(&conn, runtime_home.to_path_buf(), final_registry)?
        .ok_or_else(|| StoreError::NotFound {
            entity: "runtime_home",
            id: runtime_home_id.to_owned(),
        })?;
    drop(conn);
    sync_staged_runtime_home(staging_directory.path())?;

    Ok(PreparedRuntimeHome {
        final_runtime_home: runtime_home_identity,
        staging_directory,
        expected_runtime_home_id: runtime_home_id.to_owned(),
        expected_publication_id: publication_id,
        expected_manifest_digest,
        expected_installation_id,
    })
}

fn validate_fresh_runtime_home_destination(runtime_home: &Path) -> StoreResult<&Path> {
    if !runtime_home.is_absolute() {
        return Err(StoreError::InvalidInput {
            detail: "runtime_home must be an absolute path".to_owned(),
        });
    }
    if runtime_home.file_name().is_none() {
        return Err(StoreError::InvalidInput {
            detail: "runtime_home must name a directory below an existing parent".to_owned(),
        });
    }
    let parent = runtime_home
        .parent()
        .ok_or_else(|| StoreError::InvalidInput {
            detail: "runtime_home must have a parent directory".to_owned(),
        })?;
    let parent_metadata = fs::metadata(parent)?;
    if !parent_metadata.is_dir() {
        return Err(StoreError::InvalidInput {
            detail: format!(
                "runtime_home parent must be an existing directory: {}",
                parent.display()
            ),
        });
    }
    Ok(parent)
}

fn ready_runtime_home_after_publication(runtime_home: &Path) -> StoreResult<RuntimeHomeRecord> {
    match inspect_runtime_home_bootstrap(runtime_home)? {
        RuntimeHomeBootstrapState::Ready(record) => Ok(record),
        RuntimeHomeBootstrapState::Absent => Err(StoreError::NotFound {
            entity: "runtime_home",
            id: runtime_home.display().to_string(),
        }),
        RuntimeHomeBootstrapState::Incompatible(mismatch) => {
            Err(StoreError::RuntimeHomeSchemaMismatch(Box::new(mismatch)))
        }
        RuntimeHomeBootstrapState::Corrupt(corruption) => {
            Err(StoreError::RuntimeHomeCorruption(corruption))
        }
    }
}

struct RuntimeHomePublicationCandidate {
    conn: Connection,
    record: RuntimeHomeRecord,
    stored_runtime_home: PathBuf,
    stored_registry_path: PathBuf,
}

fn runtime_home_publication_candidate(
    runtime_home: &Path,
) -> StoreResult<RuntimeHomePublicationCandidate> {
    let runtime_metadata = fs::symlink_metadata(runtime_home)?;
    if !runtime_metadata.file_type().is_dir() {
        return Err(StoreError::RuntimeHomeCorruption(RuntimeHomeCorruption {
            runtime_home: runtime_home.to_path_buf(),
            kind: RuntimeHomeCorruptionKind::RuntimeHomeNotDirectory,
            existing_state_preserved: true,
        }));
    }
    let registry_path = registry_db_path(runtime_home);
    let registry_metadata = fs::symlink_metadata(&registry_path)?;
    if !registry_metadata.file_type().is_file() {
        return Err(StoreError::RuntimeHomeCorruption(RuntimeHomeCorruption {
            runtime_home: runtime_home.to_path_buf(),
            kind: RuntimeHomeCorruptionKind::RegistryNotFile,
            existing_state_preserved: true,
        }));
    }
    let conn = open_read_only_database(&registry_path)?;
    let count = conn.query_row("SELECT COUNT(*) FROM runtime_home", [], |row| {
        row.get::<_, i64>(0)
    })?;
    if count != 1 {
        return Err(StoreError::RuntimeHomeCorruption(RuntimeHomeCorruption {
            runtime_home: runtime_home.to_path_buf(),
            kind: RuntimeHomeCorruptionKind::RuntimeHomeRecordInvalid,
            existing_state_preserved: true,
        }));
    }
    let values = conn
        .query_row(
            "SELECT
                runtime_home_id,
                publication_id,
                runtime_home_path,
                registry_db_path,
                storage_profile,
                metadata_json,
                created_at,
                updated_at
               FROM runtime_home
              WHERE singleton_id = 1",
            [],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    PathBuf::from(row.get::<_, String>(2)?),
                    PathBuf::from(row.get::<_, String>(3)?),
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| StoreError::NotFound {
            entity: "runtime_home",
            id: runtime_home.display().to_string(),
        })?;
    let publication_id = RuntimeHomePublicationId::parse(values.1)
        .map_err(|_| StoreError::corrupt_stored_value("registry", "runtime_home.publication_id"))?;
    let record = RuntimeHomeRecord {
        runtime_home: runtime_home.to_path_buf(),
        registry_db_path: registry_path,
        runtime_home_id: values.0,
        publication_id,
        storage_profile: values.4,
        metadata_json: values.5,
        created_at: values.6,
        updated_at: values.7,
    };
    Ok(RuntimeHomePublicationCandidate {
        conn,
        record,
        stored_runtime_home: values.2,
        stored_registry_path: values.3,
    })
}

fn current_runtime_home_record_matches_path(
    conn: &Connection,
    runtime_home: &Path,
    registry_path: &Path,
) -> StoreResult<bool> {
    let count = conn.query_row("SELECT COUNT(*) FROM runtime_home", [], |row| {
        row.get::<_, i64>(0)
    })?;
    if count != 1 {
        return Ok(false);
    }
    let stored = conn
        .query_row(
            "SELECT runtime_home_path, registry_db_path
               FROM runtime_home
              WHERE singleton_id = 1",
            [],
            |row| {
                Ok((
                    PathBuf::from(row.get::<_, String>(0)?),
                    PathBuf::from(row.get::<_, String>(1)?),
                ))
            },
        )
        .optional()?;
    Ok(
        stored.is_some_and(|(stored_runtime_home, stored_registry)| {
            stored_runtime_home == runtime_home && stored_registry == registry_path
        }),
    )
}

fn observed_registry_manifest(conn: &Connection) -> Option<String> {
    let mut statement = conn
        .prepare("SELECT storage_profile FROM runtime_home ORDER BY singleton_id")
        .ok()?;
    let profiles = statement
        .query_map([], |row| row.get::<_, String>(0))
        .ok()?
        .collect::<rusqlite::Result<Vec<_>>>()
        .ok()?;
    if profiles.len() == 1 {
        profiles.into_iter().next()
    } else {
        None
    }
}

fn runtime_home_schema_mismatch(
    runtime_home: PathBuf,
    expected: &GeneratedSchemaFacts,
    actual: &GeneratedSchemaFacts,
    expected_manifest_json: &str,
    observed_manifest: Option<&str>,
) -> RuntimeHomeSchemaMismatch {
    let expected_relations = expected
        .tables
        .iter()
        .map(|relation| (relation.relation_kind, relation.name.as_str()))
        .collect::<BTreeSet<_>>();
    let actual_relations = actual
        .tables
        .iter()
        .map(|relation| (relation.relation_kind, relation.name.as_str()))
        .collect::<BTreeSet<_>>();
    let all_missing = expected_relations
        .difference(&actual_relations)
        .map(|(_, name)| (*name).to_owned())
        .collect::<Vec<_>>();
    let all_unexpected = actual_relations
        .difference(&expected_relations)
        .map(|(_, name)| (*name).to_owned())
        .collect::<Vec<_>>();
    let missing_relations = all_missing
        .iter()
        .take(BOOTSTRAP_MISMATCH_FACT_LIMIT)
        .cloned()
        .collect();
    let unexpected_relations = all_unexpected
        .iter()
        .take(BOOTSTRAP_MISMATCH_FACT_LIMIT)
        .cloned()
        .collect();

    let storage_profile_mismatch =
        observed_manifest.is_none_or(|profile| profile != expected_manifest_json);
    let mut categories = BTreeSet::new();
    if storage_profile_mismatch {
        categories.insert(RuntimeHomeSchemaCategory::StorageProfile);
    }
    if actual.tables != expected.tables {
        categories.insert(RuntimeHomeSchemaCategory::Relation);
        let expected_triggers = expected
            .tables
            .iter()
            .filter(|relation| relation.relation_kind == GeneratedRelationKind::Trigger)
            .collect::<Vec<_>>();
        let actual_triggers = actual
            .tables
            .iter()
            .filter(|relation| relation.relation_kind == GeneratedRelationKind::Trigger)
            .collect::<Vec<_>>();
        if actual_triggers != expected_triggers {
            categories.insert(RuntimeHomeSchemaCategory::Trigger);
        }
    }
    if actual.columns != expected.columns {
        categories.insert(RuntimeHomeSchemaCategory::Column);
    }
    if actual.indexes != expected.indexes {
        categories.insert(RuntimeHomeSchemaCategory::Index);
    }
    if actual.constraints != expected.constraints {
        categories.insert(RuntimeHomeSchemaCategory::Constraint);
    }

    RuntimeHomeSchemaMismatch {
        runtime_home,
        expected_manifest_digest: sha256_digest(expected_manifest_json.as_bytes()),
        observed_manifest_digest: observed_manifest
            .map(|manifest| sha256_digest(manifest.as_bytes())),
        missing_relations,
        unexpected_relations,
        changed_relation_categories: categories.into_iter().collect(),
        storage_profile_mismatch,
        facts_truncated: all_missing.len() > BOOTSTRAP_MISMATCH_FACT_LIMIT
            || all_unexpected.len() > BOOTSTRAP_MISMATCH_FACT_LIMIT,
        existing_state_preserved: true,
    }
}

fn sha256_digest(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("sha256:{digest:x}")
}

fn corrupt_runtime_home(
    runtime_home: PathBuf,
    kind: RuntimeHomeCorruptionKind,
) -> RuntimeHomeBootstrapState {
    RuntimeHomeBootstrapState::Corrupt(RuntimeHomeCorruption {
        runtime_home,
        kind,
        existing_state_preserved: true,
    })
}

fn sqlite_is_corrupt(error: &rusqlite::Error) -> bool {
    matches!(
        error,
        rusqlite::Error::SqliteFailure(sqlite_error, _)
            if matches!(
                sqlite_error.code,
                rusqlite::ErrorCode::DatabaseCorrupt | rusqlite::ErrorCode::NotADatabase
            )
    )
}

fn sync_staged_runtime_home(staging_directory: &Path) -> StoreResult<()> {
    fs::File::open(registry_db_path(staging_directory))?.sync_all()?;
    sync_directory(staging_directory)
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> StoreResult<()> {
    fs::File::open(path)?.sync_all()?;
    Ok(())
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) -> StoreResult<()> {
    Ok(())
}

#[cfg(unix)]
const fn runtime_home_parent_durability() -> DirectoryEntryDurability {
    DirectoryEntryDurability::ParentSynchronized
}

#[cfg(not(unix))]
const fn runtime_home_parent_durability() -> DirectoryEntryDurability {
    DirectoryEntryDurability::NotApplicable
}

#[cfg(unix)]
const fn failed_runtime_home_parent_durability() -> DirectoryEntryDurability {
    DirectoryEntryDurability::ParentSynchronizationFailed
}

#[cfg(not(unix))]
const fn failed_runtime_home_parent_durability() -> DirectoryEntryDurability {
    DirectoryEntryDurability::NotApplicable
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn rename_directory_no_replace(source: &Path, destination: &Path) -> io::Result<()> {
    use rustix::fs::{renameat_with, RenameFlags, CWD};

    renameat_with(CWD, source, CWD, destination, RenameFlags::NOREPLACE).map_err(io::Error::from)
}

#[cfg(windows)]
fn rename_directory_no_replace(source: &Path, destination: &Path) -> io::Result<()> {
    volicord_platform_fs::move_file_no_replace(source, destination)
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn rename_directory_no_replace(_source: &Path, _destination: &Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "atomic no-replace Runtime Home publication is unsupported on this platform",
    ))
}

/// Reads Runtime Home metadata when the registry database already exists.
pub fn runtime_home_record(
    runtime_home: impl AsRef<Path>,
) -> StoreResult<Option<RuntimeHomeRecord>> {
    runtime_home_record_read_only(runtime_home)
}

/// Reads Runtime Home metadata without creating, migrating, or writing registry state.
pub fn runtime_home_record_read_only(
    runtime_home: impl AsRef<Path>,
) -> StoreResult<Option<RuntimeHomeRecord>> {
    match inspect_runtime_home_bootstrap(runtime_home)? {
        RuntimeHomeBootstrapState::Absent => Ok(None),
        RuntimeHomeBootstrapState::Ready(record) => Ok(Some(record)),
        RuntimeHomeBootstrapState::Incompatible(mismatch) => {
            Err(StoreError::RuntimeHomeSchemaMismatch(Box::new(mismatch)))
        }
        RuntimeHomeBootstrapState::Corrupt(corruption) => {
            Err(StoreError::RuntimeHomeCorruption(corruption))
        }
    }
}

/// Creates or updates the installation profile for the selected Runtime Home.
pub fn write_installation_profile(
    context: &RuntimeHomeMutationContext<'_>,
    registration: InstallationProfileRegistration,
) -> StoreResult<InstallationProfileRecord> {
    validate_installation_profile_registration(&registration)?;

    let runtime_home = context.runtime_home().as_path().to_path_buf();
    let registry_path = registry_db_path(&runtime_home);
    let mut conn = open_registry_database_for_mutation(context)?;
    let runtime_home_row =
        runtime_home_record_from_conn(&conn, runtime_home.clone(), registry_path.clone())?
            .ok_or_else(|| StoreError::NotFound {
                entity: "runtime_home",
                id: registry_path.display().to_string(),
            })?;

    with_immediate_transaction(&mut conn, |tx| {
        insert_installation_profile_tx(tx, &runtime_home_row.runtime_home_id, &registration)?;
        Ok(())
    })?;

    installation_profile_from_conn(&conn)
}

fn validate_installation_profile_registration(
    registration: &InstallationProfileRegistration,
) -> StoreResult<()> {
    validate_identifier("installation_id", &registration.installation_id)?;
    validate_command_text("volicord_command", &registration.volicord_command)?;
    validate_command_text("volicord_mcp_command", &registration.volicord_mcp_command)?;
    validate_connection_mode(&registration.default_connection_mode)?;
    validate_json_object(
        "installation_profile.metadata_json",
        &registration.metadata_json,
    )?;
    path_to_text("installation_profile.bin_dir", &registration.bin_dir)?;
    Ok(())
}

fn insert_installation_profile_tx(
    tx: &rusqlite::Transaction<'_>,
    runtime_home_id: &str,
    registration: &InstallationProfileRegistration,
) -> rusqlite::Result<()> {
    let bin_dir_text = registration
        .bin_dir
        .to_str()
        .expect("installation bin_dir was validated as UTF-8");
    tx.execute(
        "INSERT INTO installation_profile (
            installation_id,
            runtime_home_id,
            volicord_command,
            volicord_mcp_command,
            bin_dir,
            default_connection_mode,
            metadata_json,
            created_at,
            updated_at
        )
        VALUES (
            ?1,
            ?2,
            ?3,
            ?4,
            ?5,
            ?6,
            ?7,
            strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
            strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
        )
        ON CONFLICT(installation_id) DO UPDATE SET
            runtime_home_id = excluded.runtime_home_id,
            volicord_command = excluded.volicord_command,
            volicord_mcp_command = excluded.volicord_mcp_command,
            bin_dir = excluded.bin_dir,
            default_connection_mode = excluded.default_connection_mode,
            metadata_json = excluded.metadata_json,
            updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
        params![
            registration.installation_id,
            runtime_home_id,
            registration.volicord_command,
            registration.volicord_mcp_command,
            bin_dir_text,
            registration.default_connection_mode,
            registration.metadata_json,
        ],
    )?;
    Ok(())
}

/// Reads the installation profile when one has been written.
pub fn installation_profile(
    runtime_home: impl AsRef<Path>,
) -> StoreResult<Option<InstallationProfileRecord>> {
    installation_profile_read_only(runtime_home)
}

/// Reads the installation profile without creating, migrating, or writing registry state.
pub fn installation_profile_read_only(
    runtime_home: impl AsRef<Path>,
) -> StoreResult<Option<InstallationProfileRecord>> {
    let runtime_home = runtime_home.as_ref();
    match inspect_runtime_home_bootstrap(runtime_home)? {
        RuntimeHomeBootstrapState::Absent => Ok(None),
        RuntimeHomeBootstrapState::Ready(_) => {
            let conn = open_registry_database_read_only(registry_db_path(runtime_home))?;
            installation_profile_from_conn_optional(&conn)
        }
        RuntimeHomeBootstrapState::Incompatible(mismatch) => {
            Err(StoreError::RuntimeHomeSchemaMismatch(Box::new(mismatch)))
        }
        RuntimeHomeBootstrapState::Corrupt(corruption) => {
            Err(StoreError::RuntimeHomeCorruption(corruption))
        }
    }
}

/// Reads the installation profile and returns a storage error when setup is incomplete.
pub fn require_installation_profile(
    runtime_home: impl AsRef<Path>,
) -> StoreResult<InstallationProfileRecord> {
    installation_profile(runtime_home)?.ok_or_else(|| StoreError::NotFound {
        entity: "installation_profile",
        id: "singleton".to_owned(),
    })
}

/// Reads the installation profile read-only and errors when setup is incomplete.
pub fn require_installation_profile_read_only(
    runtime_home: impl AsRef<Path>,
) -> StoreResult<InstallationProfileRecord> {
    installation_profile_read_only(runtime_home)?.ok_or_else(|| StoreError::NotFound {
        entity: "installation_profile",
        id: "singleton".to_owned(),
    })
}

/// Registers a Product Repository project and creates its project `state.sqlite`.
pub fn register_project(
    context: &RuntimeHomeMutationContext<'_>,
    registration: ProjectRegistration,
) -> StoreResult<ProjectRecord> {
    validate_project_id(&registration.project_id)?;
    write_project_registration(
        context,
        ProjectWriteRegistration {
            project_internal_id: registration.project_id.clone(),
            project_name: registration.project_id.clone(),
            project_alias: registration.project_id.clone(),
            repo_root: registration.repo_root,
            project_home: registration.project_home,
            status: registration.status,
            metadata_json: registration.metadata_json,
        },
    )
}

/// Ensures a project from its repository root and derives the internal ID.
pub fn ensure_project_for_repo(
    context: &RuntimeHomeMutationContext<'_>,
    registration: RepoProjectRegistration,
) -> StoreResult<ProjectRecord> {
    validate_project_status(&registration.status)?;
    validate_json_object("projects.metadata_json", &registration.metadata_json)?;

    let path_validation = validate_runtime_home_product_repository(
        context.runtime_home().as_path(),
        &registration.repo_root,
    )
    .map_err(path_boundary_input)?;
    if let Some(existing) = project_record_by_repo_root_read_only(
        &path_validation.runtime_home,
        &path_validation.repo_root,
    )? {
        return Ok(existing);
    }

    let project_internal_id = project_internal_id_for_repo(&path_validation.repo_root)?;
    let project_name = registration
        .project_name
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| default_project_name(&path_validation.repo_root));
    let project_alias = registration
        .project_alias
        .filter(|alias| !alias.trim().is_empty())
        .unwrap_or_else(|| default_project_alias(&project_name, &project_internal_id));
    write_project_registration_from_validated_paths(
        context,
        path_validation.repo_root,
        ProjectWriteRegistration {
            project_internal_id,
            project_name,
            project_alias,
            repo_root: PathBuf::new(),
            project_home: registration.project_home,
            status: registration.status,
            metadata_json: registration.metadata_json,
        },
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProjectWriteRegistration {
    project_internal_id: String,
    project_name: String,
    project_alias: String,
    repo_root: PathBuf,
    project_home: Option<PathBuf>,
    status: String,
    metadata_json: String,
}

fn write_project_registration(
    context: &RuntimeHomeMutationContext<'_>,
    registration: ProjectWriteRegistration,
) -> StoreResult<ProjectRecord> {
    validate_project_id(&registration.project_internal_id)?;
    validate_project_name(&registration.project_name)?;
    validate_project_alias(&registration.project_alias)?;
    validate_project_status(&registration.status)?;
    validate_json_object("projects.metadata_json", &registration.metadata_json)?;

    let path_validation = validate_runtime_home_product_repository(
        context.runtime_home().as_path(),
        &registration.repo_root,
    )
    .map_err(path_boundary_input)?;
    write_project_registration_from_validated_paths(
        context,
        path_validation.repo_root,
        registration,
    )
}

fn write_project_registration_from_validated_paths(
    context: &RuntimeHomeMutationContext<'_>,
    repo_root: PathBuf,
    registration: ProjectWriteRegistration,
) -> StoreResult<ProjectRecord> {
    let runtime_home = context.runtime_home().as_path();
    validate_project_id(&registration.project_internal_id)?;
    validate_project_name(&registration.project_name)?;
    validate_project_alias(&registration.project_alias)?;
    validate_project_status(&registration.status)?;
    validate_json_object("projects.metadata_json", &registration.metadata_json)?;

    let registry_path = registry_db_path(runtime_home);
    let mut registry = open_registry_database_for_mutation(context)?;
    let runtime_home_row = runtime_home_record_from_conn(
        &registry,
        runtime_home.to_path_buf(),
        registry_path.clone(),
    )?
    .ok_or_else(|| StoreError::NotFound {
        entity: "runtime_home",
        id: registry_path.display().to_string(),
    })?;

    let project_home = registration
        .project_home
        .unwrap_or_else(|| project_home_path(runtime_home, &registration.project_internal_id));
    let project_home = validate_project_home_boundary(runtime_home, &repo_root, &project_home)
        .map_err(path_boundary_input)?;
    let state_db_path = project_home.join(PROJECT_STATE_DB_FILE);
    let repo_root_text = path_to_text("repo_root", &repo_root)?;
    let project_home_text = path_to_text("project_home", &project_home)?;
    let state_db_path_text = path_to_text("state_db_path", &state_db_path)?;
    let storage_manifest_json = current_storage_manifest_json()?;

    let mut project_state = create_project_state_database_for_mutation(context, &state_db_path)?;
    {
        let tx = begin_immediate_transaction(&mut project_state)?;
        let existing_updated_at = tx
            .query_row(
                "SELECT updated_at
                   FROM project_state
                  WHERE project_id = ?1",
                params![registration.project_internal_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        if let Some(updated_at) = existing_updated_at.as_deref() {
            let valid_floor = UtcTimestamp::parse(updated_at).and_then(|timestamp| {
                timestamp
                    .ensure_canonical_rfc3339_representable()
                    .map_err(|_| volicord_types::values::UtcTimestampParseError)
            });
            if valid_floor.is_err() {
                return Err(StoreError::corrupt_owner_state_value(
                    "project_state",
                    &registration.project_internal_id,
                    "updated_at",
                ));
            }
        }
        tx.execute(
            "INSERT INTO project_state (
                project_id,
                storage_profile,
                created_at,
                updated_at,
                metadata_json,
                enforcement_profile_json
            )
            VALUES (
                ?1,
                ?2,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                ?3,
                ?4
            )
            ON CONFLICT(project_id) DO UPDATE SET
                metadata_json = excluded.metadata_json",
            params![
                registration.project_internal_id,
                storage_manifest_json,
                registration.metadata_json,
                BASELINE_PROJECT_ENFORCEMENT_PROFILE_JSON
            ],
        )?;
        tx.commit()?;
    }
    validate_project_state_schema(&project_state)?;

    with_immediate_transaction(&mut registry, |tx| {
        tx.execute(
            "INSERT INTO projects (
                project_internal_id,
                project_name,
                project_alias,
                runtime_home_id,
                repo_root,
                project_home,
                state_db_path,
                status,
                metadata_json,
                created_at,
                updated_at
            )
            VALUES (
                ?1,
                ?2,
                ?3,
                ?4,
                ?5,
                ?6,
                ?7,
                ?8,
                ?9,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
            )
            ON CONFLICT(project_internal_id) DO UPDATE SET
                project_name = excluded.project_name,
                project_alias = excluded.project_alias,
                runtime_home_id = excluded.runtime_home_id,
                repo_root = excluded.repo_root,
                project_home = excluded.project_home,
                state_db_path = excluded.state_db_path,
                status = excluded.status,
                metadata_json = excluded.metadata_json,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
            params![
                registration.project_internal_id,
                registration.project_name,
                registration.project_alias,
                runtime_home_row.runtime_home_id,
                repo_root_text,
                project_home_text,
                state_db_path_text,
                registration.status,
                registration.metadata_json
            ],
        )?;
        tx.execute(
            "INSERT INTO project_aliases (
                alias,
                project_internal_id,
                created_at
            )
            VALUES (
                ?1,
                ?2,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
            )
            ON CONFLICT(alias) DO UPDATE SET
                project_internal_id = excluded.project_internal_id",
            params![registration.project_alias, registration.project_internal_id],
        )?;
        Ok(())
    })?;

    project_record_from_conn(&registry, runtime_home, &registration.project_internal_id)?
        .ok_or_else(|| StoreError::NotFound {
            entity: "project",
            id: registration.project_internal_id,
        })
}

fn path_boundary_input(error: crate::runtime_home::RuntimePathBoundaryError) -> StoreError {
    match error {
        RuntimePathBoundaryError::UnsupportedEnvironment { diagnostic } => {
            StoreError::UnsupportedPlatformEnvironment { diagnostic }
        }
        RuntimePathBoundaryError::PlatformUnavailable { diagnostic } => {
            StoreError::PlatformEnvironmentUnavailable { diagnostic }
        }
        error => StoreError::InvalidInput {
            detail: error.to_string(),
        },
    }
}

/// Lists registered projects in deterministic order.
pub fn list_projects(runtime_home: impl AsRef<Path>) -> StoreResult<Vec<ProjectRecord>> {
    let runtime_home = runtime_home.as_ref().to_path_buf();
    let registry_path = registry_db_path(&runtime_home);
    if !registry_path.exists() {
        return Ok(Vec::new());
    }

    let conn = open_registry_database_read_only(registry_path)?;
    let mut stmt = conn.prepare(
        "SELECT
            project_internal_id,
            project_name,
            project_alias,
            runtime_home_id,
            repo_root,
            project_home,
            state_db_path,
            status,
            metadata_json
         FROM projects
         ORDER BY project_name, project_internal_id",
    )?;
    let rows = stmt.query_map([], project_record_from_row)?;
    let mut projects = Vec::new();
    for row in rows {
        let project = row?;
        projects.push(validate_current_project_registration(
            &runtime_home,
            &project,
        )?);
    }
    Ok(projects)
}

/// Reads one registered project from `registry.sqlite`.
pub fn project_record(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
) -> StoreResult<Option<ProjectRecord>> {
    validate_project_reference(project_id)?;
    let runtime_home = runtime_home.as_ref().to_path_buf();
    let registry_path = registry_db_path(&runtime_home);
    if !registry_path.exists() {
        return Ok(None);
    }

    let conn = open_registry_database_read_only(registry_path)?;
    project_record_from_conn(&conn, &runtime_home, project_id)
}

/// Reads one registered project without creating, migrating, or writing registry state.
pub fn project_record_read_only(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
) -> StoreResult<Option<ProjectRecord>> {
    validate_project_reference(project_id)?;
    let runtime_home = runtime_home.as_ref().to_path_buf();
    let registry_path = registry_db_path(&runtime_home);
    if !registry_path.exists() {
        return Ok(None);
    }

    let conn = open_registry_database_read_only(registry_path)?;
    project_record_from_conn(&conn, &runtime_home, project_id)
}

/// Reads one registered project by internal id.
pub fn project_record_by_internal_id(
    runtime_home: impl AsRef<Path>,
    project_internal_id: &str,
) -> StoreResult<Option<ProjectRecord>> {
    project_record(runtime_home, project_internal_id)
}

/// Reads one registered project by repository root.
pub fn project_record_by_repo_root(
    runtime_home: impl AsRef<Path>,
    repo_root: impl AsRef<Path>,
) -> StoreResult<Option<ProjectRecord>> {
    let path_validation = validate_runtime_home_product_repository(runtime_home, repo_root)
        .map_err(path_boundary_input)?;
    let registry_path = registry_db_path(&path_validation.runtime_home);
    if !registry_path.exists() {
        return Ok(None);
    }
    let conn = open_registry_database_read_only(registry_path)?;
    project_record_by_repo_root_from_conn(&conn, path_validation)
}

/// Reads a registered project by repository root through the admitted Runtime Home identity.
pub fn project_record_by_repo_root_admitted(
    context: &RuntimeHomeMutationContext<'_>,
    repo_root: impl AsRef<Path>,
) -> StoreResult<Option<ProjectRecord>> {
    let path_validation =
        validate_runtime_home_product_repository_admitted(context.runtime_home(), repo_root)
            .map_err(path_boundary_input)?;
    let registry_path = registry_db_path(context.runtime_home().as_path());
    if !registry_path.exists() {
        return Ok(None);
    }
    let conn = open_registry_database_read_only(registry_path)?;
    let repo_root_text = path_to_text("repo_root", &path_validation.repo_root)?;
    let project = conn
        .query_row(
            "SELECT
                project_internal_id,
                project_name,
                project_alias,
                runtime_home_id,
                repo_root,
                project_home,
                state_db_path,
                status,
                metadata_json
             FROM projects
             WHERE repo_root = ?1",
            [repo_root_text],
            project_record_from_row,
        )
        .optional()
        .map_err(StoreError::from)?;
    project
        .map(|project| validate_current_project_registration_admitted(context, &project))
        .transpose()
}

/// Reads a registered project by canonical repository root without writing registry state.
pub fn project_record_by_repo_root_read_only(
    runtime_home: impl AsRef<Path>,
    repo_root: impl AsRef<Path>,
) -> StoreResult<Option<ProjectRecord>> {
    let path_validation = validate_runtime_home_product_repository(runtime_home, repo_root)
        .map_err(path_boundary_input)?;
    let registry_path = registry_db_path(&path_validation.runtime_home);
    if !registry_path.exists() {
        return Ok(None);
    }
    let conn = open_registry_database_read_only(registry_path)?;
    project_record_by_repo_root_from_conn(&conn, path_validation)
}

fn project_record_by_repo_root_from_conn(
    conn: &Connection,
    path_validation: crate::runtime_home::RuntimeProductPathValidation,
) -> StoreResult<Option<ProjectRecord>> {
    let repo_root_text = path_to_text("repo_root", &path_validation.repo_root)?;
    let project = conn
        .query_row(
            "SELECT
                project_internal_id,
                project_name,
                project_alias,
                runtime_home_id,
                repo_root,
                project_home,
                state_db_path,
                status,
                metadata_json
             FROM projects
             WHERE repo_root = ?1",
            [repo_root_text],
            project_record_from_row,
        )
        .optional()
        .map_err(StoreError::from)?;
    project
        .map(|project| {
            validate_current_project_registration(path_validation.runtime_home, &project)
        })
        .transpose()
}

/// Updates a project's display name and, optionally, its primary alias.
pub fn rename_project(
    context: &RuntimeHomeMutationContext<'_>,
    project_ref: &str,
    project_name: &str,
    project_alias: Option<&str>,
) -> StoreResult<ProjectRecord> {
    validate_project_reference(project_ref)?;
    validate_project_name(project_name)?;
    if let Some(alias) = project_alias {
        validate_project_alias(alias)?;
    }

    let runtime_home = context.runtime_home().as_path().to_path_buf();
    let mut conn = open_registry_database_for_mutation(context)?;
    let current =
        raw_project_record_from_conn(&conn, project_ref)?.ok_or_else(|| StoreError::NotFound {
            entity: "project",
            id: project_ref.to_owned(),
        })?;
    let next_alias = project_alias.unwrap_or(&current.project_alias);
    let tx = crate::sqlite::begin_immediate_transaction(&mut conn)?;
    tx.execute(
        "UPDATE projects
            SET project_name = ?2,
                project_alias = ?3,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
          WHERE project_internal_id = ?1",
        params![current.project_internal_id, project_name, next_alias],
    )?;
    tx.execute(
        "INSERT INTO project_aliases (
            alias,
            project_internal_id,
            created_at
        )
        VALUES (
            ?1,
            ?2,
            strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
        )
        ON CONFLICT(alias) DO UPDATE SET
            project_internal_id = excluded.project_internal_id",
        params![next_alias, current.project_internal_id],
    )?;
    tx.commit()?;

    project_record_from_conn(&conn, &runtime_home, &current.project_internal_id)?.ok_or_else(|| {
        StoreError::NotFound {
            entity: "project",
            id: current.project_internal_id,
        }
    })
}

/// Removes a project registry row without deleting project-state files.
pub fn forget_project(
    context: &RuntimeHomeMutationContext<'_>,
    project_ref: &str,
) -> StoreResult<bool> {
    validate_project_reference(project_ref)?;
    let registry_path = registry_db_path(context.runtime_home().as_path());
    if !registry_path.exists() {
        return Ok(false);
    }
    let mut conn = open_registry_database_for_mutation(context)?;
    let Some(current) = raw_project_record_from_conn(&conn, project_ref)? else {
        return Ok(false);
    };
    let tx = crate::sqlite::begin_immediate_transaction(&mut conn)?;
    tx.execute(
        "DELETE FROM project_aliases WHERE project_internal_id = ?1",
        [current.project_internal_id.as_str()],
    )?;
    let changed = tx.execute(
        "DELETE FROM projects WHERE project_internal_id = ?1",
        [current.project_internal_id.as_str()],
    )?;
    tx.commit()?;
    Ok(changed > 0)
}

/// Reads one registered project and validates it before execution use.
pub fn project_record_for_execution(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
) -> StoreResult<Option<ProjectRecord>> {
    project_record_read_only(runtime_home, project_id)
}

/// Reads one registered project through the admitted Runtime Home identity.
pub fn project_record_for_execution_admitted(
    context: &RuntimeHomeMutationContext<'_>,
    project_id: &str,
) -> StoreResult<Option<ProjectRecord>> {
    validate_project_reference(project_id)?;
    let registry_path = registry_db_path(context.runtime_home().as_path());
    if !registry_path.exists() {
        return Ok(None);
    }
    let conn = open_registry_database_read_only(registry_path)?;
    let project = raw_project_record_from_conn(&conn, project_id)?;
    project
        .map(|project| validate_current_project_registration_admitted(context, &project))
        .transpose()
}

/// Reads one registered project for execution without registry writes.
pub fn project_record_for_execution_read_only(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
) -> StoreResult<Option<ProjectRecord>> {
    project_record_read_only(runtime_home, project_id)
}

/// Validates a stored project registration for current operational use.
pub fn validate_current_project_registration(
    runtime_home: impl AsRef<Path>,
    project: &ProjectRecord,
) -> StoreResult<ProjectRecord> {
    validate_project_id(&project.project_id).map_err(|error| {
        StoreError::InvalidProjectRegistration {
            project_id: project.project_id.clone(),
            field: "project_id",
            relationship: "invalid_project_id",
            detail: error.to_string(),
        }
    })?;
    let path_validation =
        validate_runtime_home_product_repository(runtime_home.as_ref(), &project.repo_root)
            .map_err(|error| registered_project_path_error(project, "repo_root", error))?;
    let project_home = validate_project_home_boundary(
        &path_validation.runtime_home,
        &path_validation.repo_root,
        &project.project_home,
    )
    .map_err(|error| registered_project_path_error(project, "project_home", error))?;
    let expected_state_db_path = project_home.join(PROJECT_STATE_DB_FILE);
    let stored_state_db_path = normalize_lexical_path("state_db_path", &project.state_db_path)
        .map_err(|error| registered_project_path_error(project, "state_db_path", error))?;
    if !paths_equal_for_boundary(&stored_state_db_path, &expected_state_db_path) {
        return Err(state_db_path_mismatch_error(
            project,
            &stored_state_db_path,
            &expected_state_db_path,
        ));
    }

    Ok(ProjectRecord {
        repo_root: path_validation.repo_root,
        project_home,
        state_db_path: expected_state_db_path,
        ..project.clone()
    })
}

/// Validates a stored project registration against admitted Runtime Home identity.
pub fn validate_current_project_registration_admitted(
    context: &RuntimeHomeMutationContext<'_>,
    project: &ProjectRecord,
) -> StoreResult<ProjectRecord> {
    validate_project_id(&project.project_id).map_err(|error| {
        StoreError::InvalidProjectRegistration {
            project_id: project.project_id.clone(),
            field: "project_id",
            relationship: "invalid_project_id",
            detail: error.to_string(),
        }
    })?;
    let path_validation = validate_runtime_home_product_repository_admitted(
        context.runtime_home(),
        &project.repo_root,
    )
    .map_err(|error| registered_project_path_error(project, "repo_root", error))?;
    let project_home = validate_project_home_boundary_admitted(
        context.runtime_home(),
        &path_validation.repo_root,
        &project.project_home,
    )
    .map_err(|error| registered_project_path_error(project, "project_home", error))?;
    let expected_state_db_path = project_home.join(PROJECT_STATE_DB_FILE);
    let stored_state_db_path = normalize_lexical_path("state_db_path", &project.state_db_path)
        .map_err(|error| registered_project_path_error(project, "state_db_path", error))?;
    if !paths_equal_for_boundary(&stored_state_db_path, &expected_state_db_path) {
        return Err(state_db_path_mismatch_error(
            project,
            &stored_state_db_path,
            &expected_state_db_path,
        ));
    }

    Ok(ProjectRecord {
        repo_root: path_validation.repo_root,
        project_home,
        state_db_path: expected_state_db_path,
        ..project.clone()
    })
}

/// Validates a stored project registration before execution use.
pub fn validate_project_record_for_execution(
    runtime_home: impl AsRef<Path>,
    project: &ProjectRecord,
) -> StoreResult<ProjectRecord> {
    validate_current_project_registration(runtime_home, project)
}

fn registered_project_path_error(
    project: &ProjectRecord,
    field: &'static str,
    error: RuntimePathBoundaryError,
) -> StoreError {
    match error {
        RuntimePathBoundaryError::UnsupportedEnvironment { diagnostic } => {
            StoreError::UnsupportedPlatformEnvironment { diagnostic }
        }
        RuntimePathBoundaryError::PlatformUnavailable { diagnostic } => {
            StoreError::PlatformEnvironmentUnavailable { diagnostic }
        }
        error => {
            let relationship = error
                .violation()
                .map(|violation| violation.as_str())
                .unwrap_or("invalid_path");
            StoreError::InvalidProjectRegistration {
                project_id: project.project_id.clone(),
                field,
                relationship,
                detail: error.to_string(),
            }
        }
    }
}

fn state_db_path_mismatch_error(
    project: &ProjectRecord,
    stored: &Path,
    expected: &Path,
) -> StoreError {
    StoreError::InvalidProjectRegistration {
        project_id: project.project_id.clone(),
        field: "state_db_path",
        relationship: "state_db_path_mismatch",
        detail: format!(
            "state_db_path must match project_home/{PROJECT_STATE_DB_FILE}: stored {}, expected {}",
            stored.display(),
            expected.display()
        ),
    }
}

fn runtime_home_record_from_conn(
    conn: &Connection,
    runtime_home: PathBuf,
    registry_path: PathBuf,
) -> StoreResult<Option<RuntimeHomeRecord>> {
    conn.query_row(
        "SELECT
            runtime_home_id,
            publication_id,
            storage_profile,
            metadata_json,
            created_at,
            updated_at
           FROM runtime_home
          WHERE singleton_id = 1",
        [],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
            ))
        },
    )
    .optional()
    .map_err(StoreError::from)?
    .map(
        |(
            runtime_home_id,
            publication_id,
            storage_profile,
            metadata_json,
            created_at,
            updated_at,
        )| {
            let publication_id = RuntimeHomePublicationId::parse(publication_id).map_err(|_| {
                StoreError::corrupt_stored_value("registry", "runtime_home.publication_id")
            })?;
            Ok(RuntimeHomeRecord {
                runtime_home,
                registry_db_path: registry_path,
                runtime_home_id,
                publication_id,
                storage_profile,
                metadata_json,
                created_at,
                updated_at,
            })
        },
    )
    .transpose()
}

fn project_record_from_conn(
    conn: &Connection,
    runtime_home: &Path,
    project_id: &str,
) -> StoreResult<Option<ProjectRecord>> {
    let project = raw_project_record_from_conn(conn, project_id)?;
    project
        .map(|project| validate_current_project_registration(runtime_home, &project))
        .transpose()
}

pub(crate) fn raw_project_record_from_conn(
    conn: &Connection,
    project_ref: &str,
) -> StoreResult<Option<ProjectRecord>> {
    conn.query_row(
        "SELECT
            p.project_internal_id,
            p.project_name,
            p.project_alias,
            p.runtime_home_id,
            p.repo_root,
            p.project_home,
            p.state_db_path,
            p.status,
            p.metadata_json
         FROM projects AS p
         LEFT JOIN project_aliases AS pa
           ON pa.project_internal_id = p.project_internal_id
          AND pa.alias = ?1
         WHERE p.project_internal_id = ?1
            OR p.project_alias = ?1
            OR pa.alias = ?1
         ORDER BY p.project_internal_id
         LIMIT 1",
        [project_ref],
        project_record_from_row,
    )
    .optional()
    .map_err(StoreError::from)
}

fn project_record_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProjectRecord> {
    let project_internal_id = row.get::<_, String>(0)?;
    Ok(ProjectRecord {
        project_id: project_internal_id.clone(),
        project_internal_id,
        project_name: row.get(1)?,
        project_alias: row.get(2)?,
        runtime_home_id: row.get(3)?,
        repo_root: PathBuf::from(row.get::<_, String>(4)?),
        project_home: PathBuf::from(row.get::<_, String>(5)?),
        state_db_path: PathBuf::from(row.get::<_, String>(6)?),
        status: row.get(7)?,
        metadata_json: row.get(8)?,
    })
}

/// Validates a project id that may become one `projects/{project_id}` path component.
pub fn validate_project_id(project_id: &str) -> StoreResult<()> {
    validate_identifier("project_id", project_id)?;
    validate_path_component("project_id", project_id)
}

fn validate_project_reference(project_ref: &str) -> StoreResult<()> {
    validate_identifier("project_ref", project_ref)?;
    validate_path_component("project_ref", project_ref)
}

fn validate_project_name(name: &str) -> StoreResult<()> {
    validate_identifier("project_name", name)?;
    if name.contains('\0') {
        Err(StoreError::InvalidInput {
            detail: "project_name must not contain NUL".to_owned(),
        })
    } else {
        Ok(())
    }
}

fn validate_project_alias(alias: &str) -> StoreResult<()> {
    validate_identifier("project_alias", alias)?;
    validate_path_component("project_alias", alias)
}

fn validate_identifier(field: &'static str, value: &str) -> StoreResult<()> {
    if value.trim().is_empty() {
        Err(StoreError::InvalidInput {
            detail: format!("{field} must not be empty"),
        })
    } else {
        Ok(())
    }
}

fn validate_command_text(field: &'static str, value: &str) -> StoreResult<()> {
    validate_identifier(field, value)?;
    if value.contains('\0') {
        Err(StoreError::InvalidInput {
            detail: format!("{field} must not contain NUL"),
        })
    } else {
        Ok(())
    }
}

fn validate_path_component(field: &'static str, value: &str) -> StoreResult<()> {
    if value == "."
        || value == ".."
        || value.contains('/')
        || value.contains('\\')
        || value.contains('\0')
    {
        Err(StoreError::InvalidInput {
            detail: format!("{field} must be a single path component"),
        })
    } else {
        Ok(())
    }
}

fn validate_project_status(status: &str) -> StoreResult<()> {
    if status == ACTIVE_PROJECT_STATUS {
        Ok(())
    } else {
        Err(StoreError::InvalidInput {
            detail: format!("project status must be {ACTIVE_PROJECT_STATUS}"),
        })
    }
}

fn validate_connection_mode(mode: &str) -> StoreResult<()> {
    if matches!(mode, "read_only" | "workflow") {
        Ok(())
    } else {
        Err(StoreError::InvalidInput {
            detail: "default_connection_mode must be read_only or workflow".to_owned(),
        })
    }
}

fn validate_json_object(field: &'static str, text: &str) -> StoreResult<()> {
    let value = serde_json::from_str::<Value>(text).map_err(|error| StoreError::InvalidInput {
        detail: format!("{field} must be JSON object text: {error}"),
    })?;

    if value.is_object() {
        Ok(())
    } else {
        Err(StoreError::InvalidInput {
            detail: format!("{field} must be a JSON object"),
        })
    }
}

fn path_to_text(field: &'static str, path: &Path) -> StoreResult<String> {
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| StoreError::InvalidInput {
            detail: format!("{field} must be valid UTF-8"),
        })
}

fn installation_profile_from_conn(conn: &Connection) -> StoreResult<InstallationProfileRecord> {
    installation_profile_from_conn_optional(conn)?.ok_or_else(|| StoreError::NotFound {
        entity: "installation_profile",
        id: "singleton".to_owned(),
    })
}

fn installation_profile_from_conn_optional(
    conn: &Connection,
) -> StoreResult<Option<InstallationProfileRecord>> {
    conn.query_row(
        "SELECT
            installation_id,
            runtime_home_id,
            volicord_command,
            volicord_mcp_command,
            bin_dir,
            default_connection_mode,
            metadata_json,
            created_at,
            updated_at
         FROM installation_profile
         ORDER BY installation_id
         LIMIT 1",
        [],
        |row| {
            Ok(InstallationProfileRecord {
                installation_id: row.get(0)?,
                runtime_home_id: row.get(1)?,
                volicord_command: row.get(2)?,
                volicord_mcp_command: row.get(3)?,
                bin_dir: PathBuf::from(row.get::<_, String>(4)?),
                default_connection_mode: row.get(5)?,
                metadata_json: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        },
    )
    .optional()
    .map_err(StoreError::from)
}

fn project_internal_id_for_repo(repo_root: &Path) -> StoreResult<String> {
    let repo_root_text = path_to_text("repo_root", repo_root)?;
    Ok(stable_internal_id("prj", &repo_root_text))
}

fn default_project_name(repo_root: &Path) -> String {
    repo_root
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or("project")
        .to_owned()
}

fn default_project_alias(project_name: &str, project_internal_id: &str) -> String {
    let mut alias = String::new();
    let mut previous_separator = false;
    for character in project_name.chars() {
        if character.is_ascii_alphanumeric() {
            alias.push(character.to_ascii_lowercase());
            previous_separator = false;
        } else if matches!(character, '-' | '_') {
            if !alias.is_empty() && !previous_separator {
                alias.push(character);
                previous_separator = true;
            }
        } else if !alias.is_empty() && !previous_separator {
            alias.push('-');
            previous_separator = true;
        }
    }
    while alias.ends_with('-') || alias.ends_with('_') {
        alias.pop();
    }
    if alias.is_empty() {
        alias.push_str("project");
    }

    let suffix = project_internal_id
        .strip_prefix("prj_")
        .unwrap_or(project_internal_id)
        .chars()
        .take(8)
        .collect::<String>();
    format!("{alias}-{suffix}")
}

fn stable_internal_id(prefix: &str, input: &str) -> String {
    let digest = Sha256::digest(input.as_bytes());
    let mut suffix = String::with_capacity(24);
    for byte in digest.iter().take(12) {
        suffix.push_str(&format!("{byte:02x}"));
    }
    format!("{prefix}_{suffix}")
}

#[cfg(test)]
mod tests {
    use std::{
        error::Error,
        fs,
        path::{Path, PathBuf},
        sync::{Arc, Barrier},
        thread,
    };

    use crate::{
        agent_connections::{ensure_agent_connection, AgentConnectionRegistration},
        core_pipeline::CoreProjectStore,
        inspection::{inspect_registry_database, DatabaseInspection},
        mutation::TestRuntimeHomeAdmission,
        operational_sessions::{start_mcp_runtime_session_for_test, McpRuntimeSessionStart},
        sqlite::{
            open_project_state_database_for_test, open_read_only_database,
            open_registry_database_for_test,
        },
    };
    use volicord_test_support::TempRuntimeHome;
    use volicord_types::ids::ProjectId;
    use volicord_types::integration_revision::McpRuntimeSessionSource;

    use super::*;

    type SqliteMasterRow = (String, String, Option<String>);

    #[test]
    fn runtime_path_platform_identity_is_preserved_by_store_routing() {
        let runtime_error = RuntimePathBoundaryError::PlatformUnavailable {
            diagnostic: volicord_platform_fs::PlatformDiagnostic::new(
                volicord_platform_fs::PlatformDiagnosticKind::PlatformObservationFailure,
                "a required platform observation failed",
            ),
        };

        let store_error = path_boundary_input(runtime_error);
        assert_eq!(
            store_error
                .platform_diagnostic()
                .map(volicord_platform_fs::PlatformDiagnostic::code),
            Some("platform.observation.failed")
        );
        assert_eq!(
            store_error.to_string(),
            "platform.observation.failed: a required platform observation failed"
        );
    }

    #[test]
    fn fresh_runtime_home_is_staged_and_atomically_published() -> Result<(), Box<dyn Error>> {
        let fixture = TempRuntimeHome::new("bootstrap-staged-publish")?;
        let setup = TestRuntimeHomeAdmission::exclusive(fixture.path())?;
        let context = setup.context()?;
        assert!(matches!(
            inspect_runtime_home_bootstrap(fixture.path())?,
            RuntimeHomeBootstrapState::Absent
        ));

        let prepared = prepare_runtime_home(&context, "runtime_home_staged_publish", "{}")?;
        assert!(!fixture.path().exists());
        assert_eq!(staging_directories(fixture.path())?.len(), 1);

        let outcome = commit_runtime_home(&context, prepared)?;
        let RuntimeHomePublicationOutcome::PublishedByThisInvocation { mut publication } = outcome
        else {
            panic!("fresh publication must be owned by this invocation");
        };
        assert_eq!(publication.final_path(), fixture.path());
        let record = publication.confirm(&context)?;
        assert_eq!(record.runtime_home, fixture.path());
        assert_eq!(record.registry_db_path, fixture.registry_db_path());
        assert!(fixture.registry_db_path().is_file());
        assert!(staging_directories(fixture.path())?.is_empty());
        assert!(matches!(
            inspect_runtime_home_bootstrap(fixture.path())?,
            RuntimeHomeBootstrapState::Ready(_)
        ));
        Ok(())
    }

    #[test]
    fn staged_creation_failures_leave_no_final_path_or_staging_directory(
    ) -> Result<(), Box<dyn Error>> {
        for (label, failed_phase) in [
            ("schema", RuntimeHomeBootstrapPhase::SchemaCreation),
            ("singleton", RuntimeHomeBootstrapPhase::SingletonInsert),
            ("manifest", RuntimeHomeBootstrapPhase::ManifestValidation),
        ] {
            let fixture = TempRuntimeHome::new(&format!("bootstrap-fail-{label}"))?;
            let setup = TestRuntimeHomeAdmission::exclusive(fixture.path())?;
            let context = setup.context()?;
            let mut hook = |phase| {
                if phase == failed_phase {
                    Err(StoreError::InvalidInput {
                        detail: format!("injected {label} failure"),
                    })
                } else {
                    Ok(())
                }
            };
            prepare_runtime_home_inner(
                &context,
                &format!("runtime_home_fail_{label}"),
                "{}",
                None,
                &mut hook,
            )
            .expect_err("injected staged bootstrap failure");

            assert!(!fixture.path().exists());
            assert!(staging_directories(fixture.path())?.is_empty());
        }
        Ok(())
    }

    #[test]
    fn failure_before_atomic_rename_cleans_staging_and_preserves_absence(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = TempRuntimeHome::new("bootstrap-fail-before-rename")?;
        let setup = TestRuntimeHomeAdmission::exclusive(fixture.path())?;
        let context = setup.context()?;
        let prepared = prepare_runtime_home(&context, "runtime_home_fail_before_rename", "{}")?;
        let mut hook = |phase| {
            if phase == RuntimeHomeBootstrapPhase::AtomicRename {
                Err(StoreError::InvalidInput {
                    detail: "injected publication failure".to_owned(),
                })
            } else {
                Ok(())
            }
        };

        commit_runtime_home_inner(prepared, &mut hook)
            .expect_err("publication failure must not expose the final path");

        assert!(!fixture.path().exists());
        assert!(staging_directories(fixture.path())?.is_empty());
        Ok(())
    }

    #[test]
    fn installation_profile_is_present_at_fresh_publication() -> Result<(), Box<dyn Error>> {
        let fixture = TempRuntimeHome::new("bootstrap-with-installation")?;
        let setup = TestRuntimeHomeAdmission::exclusive(fixture.path())?;
        let context = setup.context()?;
        let (_, expected) = initialize_runtime_home_with_installation(
            &context,
            "runtime_home_with_installation",
            "{}",
            installation_registration(fixture.path()),
        )?;

        let actual = installation_profile_read_only(fixture.path())?
            .expect("installation profile must publish with the Registry");
        assert_eq!(actual, expected);
        assert!(staging_directories(fixture.path())?.is_empty());
        Ok(())
    }

    #[test]
    fn existing_current_home_is_accepted_read_only_without_mtime_or_byte_changes(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = TempRuntimeHome::new("bootstrap-current-read-only")?;
        let setup = TestRuntimeHomeAdmission::exclusive(fixture.path())?;
        let context = setup.context()?;
        initialize_runtime_home(&context, "runtime_home_current_read_only", "{}")?;
        let before_bytes = fs::read(fixture.registry_db_path())?;
        let before_modified = fs::metadata(fixture.registry_db_path())?.modified()?;

        let state = inspect_runtime_home_bootstrap(fixture.path())?;

        assert!(matches!(state, RuntimeHomeBootstrapState::Ready(_)));
        assert_eq!(fs::read(fixture.registry_db_path())?, before_bytes);
        assert_eq!(
            fs::metadata(fixture.registry_db_path())?.modified()?,
            before_modified
        );
        Ok(())
    }

    #[test]
    fn noncurrent_manifest_is_incompatible_and_preserved_read_only() -> Result<(), Box<dyn Error>> {
        let fixture = TempRuntimeHome::new("bootstrap-noncurrent-manifest")?;
        let setup = TestRuntimeHomeAdmission::exclusive(fixture.path())?;
        let context = setup.context()?;
        initialize_runtime_home(&context, "runtime_home_noncurrent_manifest", "{}")?;
        let current = current_storage_manifest()?;
        let noncurrent = StorageManifest::new(
            volicord_types::storage_contract::STORAGE_CONTRACT_ID,
            format!("sha256:{}", "a".repeat(64)),
            format!("sha256:{}", "b".repeat(64)),
            current.enabled_capabilities.clone(),
        )?;
        let noncurrent = canonical_json_string(&noncurrent)?;
        let conn = Connection::open(fixture.registry_db_path())?;
        conn.execute(
            "UPDATE runtime_home SET storage_profile = ?1",
            [&noncurrent],
        )?;
        drop(conn);
        let before_bytes = fs::read(fixture.registry_db_path())?;
        let before_modified = fs::metadata(fixture.registry_db_path())?.modified()?;

        let state = inspect_runtime_home_bootstrap(fixture.path())?;

        let RuntimeHomeBootstrapState::Incompatible(mismatch) = state else {
            panic!("noncurrent manifest must be incompatible");
        };
        assert!(mismatch.storage_profile_mismatch);
        assert!(mismatch.observed_manifest_digest.is_some());
        assert!(mismatch.existing_state_preserved);
        assert_eq!(fs::read(fixture.registry_db_path())?, before_bytes);
        assert_eq!(
            fs::metadata(fixture.registry_db_path())?.modified()?,
            before_modified
        );
        assert!(
            initialize_runtime_home(&context, "runtime_home_noncurrent_manifest", "{}").is_err()
        );
        assert_eq!(fs::read(fixture.registry_db_path())?, before_bytes);
        Ok(())
    }

    #[test]
    fn corrupt_sqlite_is_distinct_from_an_incompatible_schema() -> Result<(), Box<dyn Error>> {
        let fixture = TempRuntimeHome::new("bootstrap-corrupt-sqlite")?;
        fs::create_dir_all(fixture.path())?;
        fs::write(fixture.registry_db_path(), b"not a sqlite database")?;

        let state = inspect_runtime_home_bootstrap(fixture.path())?;

        assert!(matches!(
            state,
            RuntimeHomeBootstrapState::Corrupt(RuntimeHomeCorruption {
                kind: RuntimeHomeCorruptionKind::SqliteInvalid,
                ..
            })
        ));
        assert_eq!(
            fs::read(fixture.registry_db_path())?,
            b"not a sqlite database"
        );
        Ok(())
    }

    #[test]
    fn mismatch_report_contains_manifest_and_relation_facts() -> Result<(), Box<dyn Error>> {
        let fixture = TempRuntimeHome::new("bootstrap-relation-facts")?;
        let setup = TestRuntimeHomeAdmission::exclusive(fixture.path())?;
        let context = setup.context()?;
        initialize_runtime_home(&context, "runtime_home_relation_facts", "{}")?;
        let conn = Connection::open(fixture.registry_db_path())?;
        conn.execute_batch(
            "DROP TABLE project_aliases;
             CREATE TABLE runtime_extension (id TEXT PRIMARY KEY);",
        )?;
        drop(conn);

        let state = inspect_runtime_home_bootstrap(fixture.path())?;

        let RuntimeHomeBootstrapState::Incompatible(mismatch) = state else {
            panic!("changed relations must be incompatible");
        };
        assert!(mismatch
            .missing_relations
            .iter()
            .any(|name| name == "project_aliases"));
        assert!(mismatch
            .unexpected_relations
            .iter()
            .any(|name| name == "runtime_extension"));
        assert!(mismatch
            .changed_relation_categories
            .contains(&RuntimeHomeSchemaCategory::Relation));
        assert!(mismatch.expected_manifest_digest.starts_with("sha256:"));
        assert_eq!(mismatch.expected_manifest_digest.len(), 71);
        assert!(!mismatch.storage_profile_mismatch);
        assert!(mismatch.existing_state_preserved);
        let diagnostic = mismatch.to_string();
        assert!(diagnostic.contains("choose a fresh explicit --home"));
        assert!(!diagnostic.contains("missing canonical SQLite relation"));
        Ok(())
    }

    #[test]
    fn concurrent_creators_publish_once_without_replacing_the_winner() -> Result<(), Box<dyn Error>>
    {
        let fixture = TempRuntimeHome::new("bootstrap-concurrent-creators")?;
        let setup = TestRuntimeHomeAdmission::exclusive(fixture.path())?;
        let context = setup.context()?;
        let first = prepare_runtime_home(&context, "runtime_home_creator_first", "{}")?;
        let second = prepare_runtime_home(&context, "runtime_home_creator_second", "{}")?;
        let barrier = Arc::new(Barrier::new(2));
        let first_barrier = Arc::clone(&barrier);
        let second_barrier = Arc::clone(&barrier);
        let (first_outcome, second_outcome) = thread::scope(|scope| {
            let first = scope.spawn(|| {
                first_barrier.wait();
                commit_runtime_home(&context, first)
            });
            let second = scope.spawn(|| {
                second_barrier.wait();
                commit_runtime_home(&context, second)
            });
            (
                first.join().expect("first creator thread"),
                second.join().expect("second creator thread"),
            )
        });
        let first_outcome = first_outcome?;
        let second_outcome = second_outcome?;
        let (mut winner, observed) = match (first_outcome, second_outcome) {
            (
                RuntimeHomePublicationOutcome::PublishedByThisInvocation { publication },
                RuntimeHomePublicationOutcome::ObservedConcurrentWinner { record },
            )
            | (
                RuntimeHomePublicationOutcome::ObservedConcurrentWinner { record },
                RuntimeHomePublicationOutcome::PublishedByThisInvocation { publication },
            ) => (publication, record),
            _ => panic!("exactly one concurrent creator must own publication"),
        };
        let winner_record = winner.confirm(&context)?;

        assert_eq!(winner_record, observed);
        assert!(matches!(
            winner_record.runtime_home_id.as_str(),
            "runtime_home_creator_first" | "runtime_home_creator_second"
        ));
        assert!(staging_directories(fixture.path())?.is_empty());
        assert!(matches!(
            inspect_runtime_home_bootstrap(fixture.path())?,
            RuntimeHomeBootstrapState::Ready(_)
        ));
        assert!(matches!(
            winner.rollback_if_owned(&context)?,
            RuntimeHomePublicationRollbackOutcome::RolledBack {
                durability,
                failure: None,
            } if durability == runtime_home_parent_durability()
        ));
        assert!(!fixture.path().exists());
        Ok(())
    }

    #[test]
    fn equal_runtime_home_ids_receive_distinct_publication_ids() -> Result<(), Box<dyn Error>> {
        let first = TempRuntimeHome::new("bootstrap-distinct-publication-first")?;
        let second = TempRuntimeHome::new("bootstrap-distinct-publication-second")?;
        let first_setup = TestRuntimeHomeAdmission::exclusive(first.path())?;
        let first_context = first_setup.context()?;
        let second_setup = TestRuntimeHomeAdmission::exclusive(second.path())?;
        let second_context = second_setup.context()?;
        let first_prepared =
            prepare_runtime_home(&first_context, "runtime_home_same_identity", "{}")?;
        let second_prepared =
            prepare_runtime_home(&second_context, "runtime_home_same_identity", "{}")?;

        assert_ne!(
            first_prepared.publication_id(),
            second_prepared.publication_id()
        );
        assert_ne!(
            first_prepared.manifest_digest(),
            first_prepared.publication_id().as_str()
        );
        Ok(())
    }

    #[test]
    fn post_rename_confirmation_failures_retain_rollback_ownership() -> Result<(), Box<dyn Error>> {
        for phase in [
            RuntimeHomePublicationConfirmationPhase::ParentDirectorySync,
            RuntimeHomePublicationConfirmationPhase::PublicationReadBack,
            RuntimeHomePublicationConfirmationPhase::PublicationManifestValidation,
        ] {
            let fixture = TempRuntimeHome::new(&format!(
                "bootstrap-post-rename-{}",
                match phase {
                    RuntimeHomePublicationConfirmationPhase::ParentDirectorySync => "parent-sync",
                    RuntimeHomePublicationConfirmationPhase::PublicationReadBack => "read-back",
                    RuntimeHomePublicationConfirmationPhase::PublicationManifestValidation =>
                        "manifest",
                }
            ))?;
            let setup = TestRuntimeHomeAdmission::exclusive(fixture.path())?;
            let context = setup.context()?;
            let prepared =
                prepare_runtime_home(&context, "runtime_home_post_rename_failure", "{}")?;
            let RuntimeHomePublicationOutcome::PublishedByThisInvocation { mut publication } =
                commit_runtime_home(&context, prepared)?
            else {
                panic!("fresh publication must be owned");
            };
            assert!(fixture.path().is_dir());

            let mut hook = |current| {
                if current == phase {
                    Err(StoreError::InvalidInput {
                        detail: format!("injected {phase:?} failure"),
                    })
                } else {
                    Ok(())
                }
            };
            confirm_runtime_home_inner(&context, &mut publication, &mut hook)
                .expect_err("post-rename confirmation fault must be visible");
            assert!(matches!(
                publication.rollback_if_owned(&context)?,
                RuntimeHomePublicationRollbackOutcome::RolledBack {
                    durability,
                    failure: None,
                } if durability == runtime_home_parent_durability()
            ));
            assert!(!fixture.path().exists());
            initialize_runtime_home(&context, "runtime_home_unrelated_replacement", "{}")?;
            assert!(matches!(
                publication.rollback_if_owned(&context)?,
                RuntimeHomePublicationRollbackOutcome::AlreadyRolledBack {
                    durability,
                } if durability == runtime_home_parent_durability()
            ));
            let RuntimeHomeBootstrapState::Ready(replacement) =
                inspect_runtime_home_bootstrap(fixture.path())?
            else {
                panic!("unrelated replacement must remain ready");
            };
            assert_eq!(
                replacement.runtime_home_id,
                "runtime_home_unrelated_replacement"
            );
        }
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn removed_but_unsynchronized_guard_is_terminal_and_preserves_replacement(
    ) -> Result<(), Box<dyn Error>> {
        use volicord_platform_fs::directory_tree_removal_test_support::{
            fail_next_directory_tree_removal, DirectoryTreeRemovalFault,
        };

        let fixture = TempRuntimeHome::new("bootstrap-removed-unsynchronized")?;
        let setup = TestRuntimeHomeAdmission::exclusive(fixture.path())?;
        let context = setup.context()?;
        let prepared = prepare_runtime_home(&context, "runtime_home_removed_unsynchronized", "{}")?;
        let RuntimeHomePublicationOutcome::PublishedByThisInvocation { mut publication } =
            commit_runtime_home(&context, prepared)?
        else {
            panic!("fresh publication must be owned");
        };
        fail_next_directory_tree_removal(DirectoryTreeRemovalFault::ParentDirectorySyncFailure);

        let outcome = publication.rollback_if_owned(&context)?;

        assert!(matches!(
            outcome,
            RuntimeHomePublicationRollbackOutcome::RolledBack {
                durability,
                failure: Some(failure),
            } if durability == failed_runtime_home_parent_durability()
                && failure.effect == DirectoryTreeRemovalEffect::Removed
                && failure.target_state == DirectoryTreeTargetState::Absent
        ));
        assert!(!fixture.path().exists());

        initialize_runtime_home(&context, "runtime_home_replacement", "{}")?;
        assert!(matches!(
            publication.rollback_if_owned(&context)?,
            RuntimeHomePublicationRollbackOutcome::AlreadyRolledBack {
                durability,
            } if durability == failed_runtime_home_parent_durability()
        ));
        let RuntimeHomeBootstrapState::Ready(replacement) =
            inspect_runtime_home_bootstrap(fixture.path())?
        else {
            panic!("replacement must remain ready");
        };
        assert_eq!(replacement.runtime_home_id, "runtime_home_replacement");
        Ok(())
    }

    #[test]
    fn incomplete_removal_is_terminal_and_does_not_blindly_retry() -> Result<(), Box<dyn Error>> {
        use volicord_platform_fs::directory_tree_removal_test_support::{
            fail_next_directory_tree_removal, DirectoryTreeRemovalFault,
        };

        for (fault, expected_effect, expected_state) in [
            (
                DirectoryTreeRemovalFault::BeforeRecursiveRemoval,
                DirectoryTreeRemovalEffect::NotRemoved,
                DirectoryTreeTargetState::Present,
            ),
            (
                DirectoryTreeRemovalFault::RecursiveRemovalAfterPartialEffect,
                DirectoryTreeRemovalEffect::PartiallyRemovedOrUnknown,
                DirectoryTreeTargetState::Present,
            ),
            (
                DirectoryTreeRemovalFault::PostRemovalInspectionFailure,
                DirectoryTreeRemovalEffect::PartiallyRemovedOrUnknown,
                DirectoryTreeTargetState::Unknown,
            ),
        ] {
            let fixture = TempRuntimeHome::new(&format!(
                "bootstrap-incomplete-removal-{}",
                expected_effect.as_str()
            ))?;
            let setup = TestRuntimeHomeAdmission::exclusive(fixture.path())?;
            let context = setup.context()?;
            let prepared = prepare_runtime_home(&context, "runtime_home_incomplete_removal", "{}")?;
            let RuntimeHomePublicationOutcome::PublishedByThisInvocation { mut publication } =
                commit_runtime_home(&context, prepared)?
            else {
                panic!("fresh publication must be owned");
            };
            fail_next_directory_tree_removal(fault);

            let first = publication.rollback_if_owned(&context)?;
            let RuntimeHomePublicationRollbackOutcome::RemovalIncomplete { failure } = first else {
                panic!("fault must retain incomplete removal");
            };
            assert_eq!(failure.effect, expected_effect);
            assert_eq!(failure.target_state, expected_state);
            assert!(fixture.path().is_dir());
            fs::write(fixture.path().join("replacement-marker"), b"preserve")?;

            let second = publication.rollback_if_owned(&context)?;
            assert!(matches!(
                second,
                RuntimeHomePublicationRollbackOutcome::RemovalIncomplete { failure }
                    if failure.effect == expected_effect
                        && failure.target_state == expected_state
            ));
            assert_eq!(
                fs::read(fixture.path().join("replacement-marker"))?,
                b"preserve"
            );
        }
        Ok(())
    }

    #[test]
    fn confirmation_failure_retains_every_rollback_result() -> Result<(), Box<dyn Error>> {
        use volicord_platform_fs::directory_tree_removal_test_support::{
            fail_next_directory_tree_removal, DirectoryTreeRemovalFault,
        };

        #[derive(Clone, Copy)]
        enum RollbackFixture {
            Synchronized,
            #[cfg(unix)]
            ParentSyncFailure,
            Preserved,
            OwnershipLost,
            Incomplete,
        }

        let rollback_fixtures = [
            RollbackFixture::Synchronized,
            #[cfg(unix)]
            RollbackFixture::ParentSyncFailure,
            RollbackFixture::Preserved,
            RollbackFixture::OwnershipLost,
            RollbackFixture::Incomplete,
        ];
        for rollback_fixture in rollback_fixtures {
            let label = match rollback_fixture {
                RollbackFixture::Synchronized => "synchronized",
                #[cfg(unix)]
                RollbackFixture::ParentSyncFailure => "parent-sync-failure",
                RollbackFixture::Preserved => "preserved",
                RollbackFixture::OwnershipLost => "ownership-lost",
                RollbackFixture::Incomplete => "incomplete",
            };
            let fixture =
                TempRuntimeHome::new(&format!("bootstrap-confirmation-composite-{label}"))?;
            let setup = TestRuntimeHomeAdmission::exclusive(fixture.path())?;
            let context = setup.context()?;
            let prepared =
                prepare_runtime_home(&context, "runtime_home_confirmation_composite", "{}")?;
            let mut confirmation_hook = |phase| {
                if phase == RuntimeHomePublicationConfirmationPhase::PublicationReadBack {
                    Err(StoreError::InvalidInput {
                        detail: "injected primary confirmation failure".to_owned(),
                    })
                } else {
                    Ok(())
                }
            };
            let mut displaced = None;
            let mut before_rollback = |publication: &mut RuntimeHomePublicationGuard| {
                match rollback_fixture {
                    RollbackFixture::Synchronized => {}
                    #[cfg(unix)]
                    RollbackFixture::ParentSyncFailure => {
                        fail_next_directory_tree_removal(
                            DirectoryTreeRemovalFault::ParentDirectorySyncFailure,
                        );
                    }
                    RollbackFixture::Preserved => publication.preserve(),
                    RollbackFixture::OwnershipLost => {
                        let path = publication.final_path();
                        let replacement = path.with_extension("displaced");
                        fs::rename(path, &replacement)?;
                        displaced = Some(replacement);
                    }
                    RollbackFixture::Incomplete => {
                        fail_next_directory_tree_removal(
                            DirectoryTreeRemovalFault::BeforeRecursiveRemoval,
                        );
                    }
                }
                Ok(())
            };

            let error = publish_and_confirm_runtime_home_inner(
                &context,
                prepared,
                &mut confirmation_hook,
                &mut before_rollback,
            )
            .expect_err("confirmation failure must retain rollback");
            let StoreError::RuntimeHomePublicationConfirmation(failure) = error else {
                panic!("confirmation failure must use the composite Store error");
            };

            assert_eq!(
                failure.primary.phase,
                RuntimeHomePublicationConfirmationPhase::PublicationReadBack
            );
            assert!(failure
                .primary
                .store_error()
                .to_string()
                .contains("injected primary confirmation failure"));
            assert!(failure.publication_occurred);
            match rollback_fixture {
                RollbackFixture::Synchronized => {
                    assert_eq!(failure.final_path_state, RuntimeHomeFinalPathState::Absent);
                    assert_eq!(
                        failure.rollback_parent_durability,
                        runtime_home_parent_durability()
                    );
                    assert!(matches!(
                        failure.rollback,
                        RuntimeHomePublicationRollbackAttempt::Completed(
                            RuntimeHomePublicationRollbackOutcome::RolledBack { failure: None, .. }
                        )
                    ));
                }
                #[cfg(unix)]
                RollbackFixture::ParentSyncFailure => {
                    assert_eq!(failure.final_path_state, RuntimeHomeFinalPathState::Absent);
                    assert_eq!(
                        failure.rollback_parent_durability,
                        failed_runtime_home_parent_durability()
                    );
                    assert!(matches!(
                        failure.rollback,
                        RuntimeHomePublicationRollbackAttempt::Completed(
                            RuntimeHomePublicationRollbackOutcome::RolledBack {
                                failure: Some(_),
                                ..
                            }
                        )
                    ));
                }
                RollbackFixture::Preserved => {
                    assert_eq!(failure.final_path_state, RuntimeHomeFinalPathState::Present);
                    assert!(matches!(
                        failure.rollback,
                        RuntimeHomePublicationRollbackAttempt::Completed(
                            RuntimeHomePublicationRollbackOutcome::Preserved {
                                reason: RuntimeHomePublicationPreservationReason::SetupPolicy,
                            }
                        )
                    ));
                }
                RollbackFixture::OwnershipLost => {
                    assert_eq!(failure.final_path_state, RuntimeHomeFinalPathState::Absent);
                    assert!(matches!(
                        failure.rollback,
                        RuntimeHomePublicationRollbackAttempt::Completed(
                            RuntimeHomePublicationRollbackOutcome::OwnershipLost {
                                reason: RuntimeHomePublicationOwnershipLoss::FinalPathMissing,
                            }
                        )
                    ));
                    assert!(displaced.as_ref().is_some_and(|path| path.is_dir()));
                }
                RollbackFixture::Incomplete => {
                    assert_eq!(failure.final_path_state, RuntimeHomeFinalPathState::Present);
                    assert!(matches!(
                        failure.rollback,
                        RuntimeHomePublicationRollbackAttempt::Completed(
                            RuntimeHomePublicationRollbackOutcome::RemovalIncomplete {
                                failure,
                            }
                        ) if failure.effect == DirectoryTreeRemovalEffect::NotRemoved
                    ));
                }
            }
        }
        Ok(())
    }

    #[test]
    fn rollback_preserves_publication_when_exact_identity_is_lost() -> Result<(), Box<dyn Error>> {
        for (field, expected_reason) in [
            (
                "publication_id",
                RuntimeHomePublicationOwnershipLoss::PublicationIdMismatch,
            ),
            (
                "runtime_home_id",
                RuntimeHomePublicationOwnershipLoss::RuntimeHomeIdMismatch,
            ),
            (
                "storage_profile",
                RuntimeHomePublicationOwnershipLoss::ManifestDigestMismatch,
            ),
        ] {
            let fixture = TempRuntimeHome::new(&format!("bootstrap-ownership-loss-{field}"))?;
            let setup = TestRuntimeHomeAdmission::exclusive(fixture.path())?;
            let context = setup.context()?;
            let prepared = prepare_runtime_home(&context, "runtime_home_ownership_loss", "{}")?;
            let RuntimeHomePublicationOutcome::PublishedByThisInvocation { mut publication } =
                commit_runtime_home(&context, prepared)?
            else {
                panic!("fresh publication must be owned");
            };
            let conn = Connection::open(fixture.registry_db_path())?;
            match field {
                "publication_id" => {
                    let replacement = RuntimeHomePublicationId::generate()?;
                    conn.execute(
                        "UPDATE runtime_home SET publication_id = ?1 WHERE singleton_id = 1",
                        [replacement.as_str()],
                    )?;
                }
                "runtime_home_id" => {
                    conn.execute(
                        "UPDATE runtime_home SET runtime_home_id = 'runtime_home_replacement'
                          WHERE singleton_id = 1",
                        [],
                    )?;
                }
                "storage_profile" => {
                    let current = current_storage_manifest()?;
                    let replacement = StorageManifest::new(
                        volicord_types::storage_contract::STORAGE_CONTRACT_ID,
                        format!("sha256:{}", "c".repeat(64)),
                        format!("sha256:{}", "d".repeat(64)),
                        current.enabled_capabilities.clone(),
                    )?;
                    conn.execute(
                        "UPDATE runtime_home SET storage_profile = ?1 WHERE singleton_id = 1",
                        [canonical_json_string(&replacement)?],
                    )?;
                }
                _ => unreachable!(),
            }
            drop(conn);

            assert!(matches!(
                publication.rollback_if_owned(&context)?,
                RuntimeHomePublicationRollbackOutcome::OwnershipLost { reason }
                    if reason == expected_reason
            ));
            assert!(fixture.path().is_dir(), "{field}");
        }
        Ok(())
    }

    #[test]
    fn pre_removal_absence_is_terminal_ownership_loss() -> Result<(), Box<dyn Error>> {
        let fixture = TempRuntimeHome::new("bootstrap-pre-removal-absence")?;
        let setup = TestRuntimeHomeAdmission::exclusive(fixture.path())?;
        let context = setup.context()?;
        let prepared = prepare_runtime_home(&context, "runtime_home_pre_removal_absence", "{}")?;
        let RuntimeHomePublicationOutcome::PublishedByThisInvocation { mut publication } =
            commit_runtime_home(&context, prepared)?
        else {
            panic!("fresh publication must be owned");
        };
        let displaced = fixture.root_path().join("displaced-runtime-home");
        fs::rename(fixture.path(), &displaced)?;

        assert!(matches!(
            publication.rollback_if_owned(&context)?,
            RuntimeHomePublicationRollbackOutcome::OwnershipLost {
                reason: RuntimeHomePublicationOwnershipLoss::FinalPathMissing,
            }
        ));
        initialize_runtime_home(&context, "runtime_home_later_replacement", "{}")?;
        assert!(matches!(
            publication.rollback_if_owned(&context)?,
            RuntimeHomePublicationRollbackOutcome::OwnershipLost {
                reason: RuntimeHomePublicationOwnershipLoss::FinalPathMissing,
            }
        ));
        let RuntimeHomeBootstrapState::Ready(replacement) =
            inspect_runtime_home_bootstrap(fixture.path())?
        else {
            panic!("replacement must remain ready");
        };
        assert_eq!(
            replacement.runtime_home_id,
            "runtime_home_later_replacement"
        );
        assert!(displaced.is_dir());
        Ok(())
    }

    #[test]
    fn managed_host_consumption_preserves_the_owned_publication() -> Result<(), Box<dyn Error>> {
        let fixture = TempRuntimeHome::new("bootstrap-managed-consumption-preserves")?;
        let setup = TestRuntimeHomeAdmission::exclusive(fixture.path())?;
        let context = setup.context()?;
        let prepared = prepare_runtime_home(&context, "runtime_home_managed_consumption", "{}")?;
        let RuntimeHomePublicationOutcome::PublishedByThisInvocation { mut publication } =
            commit_runtime_home(&context, prepared)?
        else {
            panic!("fresh publication must be owned");
        };
        publication.confirm(&context)?;
        ensure_agent_connection(
            &context,
            AgentConnectionRegistration {
                connection_internal_id: "connection_managed_consumption".to_owned(),
                host_kind: "codex".to_owned(),
                intent: "personal".to_owned(),
                host_scope: "user".to_owned(),
                server_name: "volicord".to_owned(),
                config_target: fixture
                    .root_path()
                    .join("config.toml")
                    .display()
                    .to_string(),
                mode: "workflow".to_owned(),
                enabled: true,
                managed_fingerprint: "managed-consumption-fingerprint".to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;
        start_mcp_runtime_session_for_test(
            &context,
            McpRuntimeSessionStart {
                connection_internal_id: "connection_managed_consumption".to_owned(),
                session_source: McpRuntimeSessionSource::ManagedHost,
                observed_host_executable_version: None,
                process_id: 42,
                process_started_at: "2026-07-25T00:00:00Z".to_owned(),
            },
        )?;

        assert!(matches!(
            publication.rollback_if_owned(&context)?,
            RuntimeHomePublicationRollbackOutcome::Preserved {
                reason: RuntimeHomePublicationPreservationReason::ManagedHostConsumption,
            }
        ));
        assert!(fixture.path().is_dir());
        assert!(matches!(
            publication.rollback_if_owned(&context)?,
            RuntimeHomePublicationRollbackOutcome::Preserved {
                reason: RuntimeHomePublicationPreservationReason::ManagedHostConsumption,
            }
        ));
        Ok(())
    }

    #[test]
    fn writable_registry_open_does_not_bootstrap_a_final_path() -> Result<(), Box<dyn Error>> {
        let fixture = TempRuntimeHome::new("bootstrap-no-direct-open")?;

        open_registry_database_for_test(fixture.registry_db_path())
            .expect_err("ordinary writable open must not create a Registry");

        assert!(!fixture.path().exists());
        assert!(!fixture.registry_db_path().exists());
        Ok(())
    }

    #[test]
    fn project_id_validator_rejects_unsafe_path_components() {
        for invalid in ["", "   ", ".", "..", "a/b", "a\\b", "a\0b"] {
            let error = validate_project_id(invalid).expect_err("project_id should be rejected");
            assert!(
                matches!(error, StoreError::InvalidInput { .. }),
                "unexpected error for {invalid:?}: {error}"
            );
        }
    }

    #[test]
    fn project_id_validator_accepts_normal_ascii_and_utf8() -> Result<(), Box<dyn Error>> {
        validate_project_id("project_alpha")?;
        validate_project_id("프로젝트")?;
        Ok(())
    }

    #[test]
    fn project_registration_uses_project_id_validator_even_with_custom_home(
    ) -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("store-project-id-validation")?;
        let setup = TestRuntimeHomeAdmission::exclusive(runtime_home.path())?;
        let context = setup.context()?;
        let repo_root = runtime_home.create_product_repo("repo")?;
        initialize_runtime_home(&context, "runtime_home_validation", "{}")?;

        let error = register_project(
            &context,
            ProjectRegistration {
                project_id: "a/b".to_owned(),
                repo_root,
                project_home: Some(runtime_home.path().join("custom-project-home")),
                status: ACTIVE_PROJECT_STATUS.to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )
        .expect_err("invalid project_id should be rejected before registration");

        assert!(matches!(error, StoreError::InvalidInput { .. }));
        assert!(!runtime_home.path().join("custom-project-home").exists());
        Ok(())
    }

    #[test]
    fn project_registration_rejects_same_runtime_home_and_repository() -> Result<(), Box<dyn Error>>
    {
        let runtime_home = TempRuntimeHome::new("store-same-runtime-repo")?;
        let setup = TestRuntimeHomeAdmission::exclusive(runtime_home.path())?;
        let context = setup.context()?;
        initialize_runtime_home(&context, "runtime_home_same_repo", "{}")?;

        let error = register_project(
            &context,
            ProjectRegistration {
                project_id: "project_same".to_owned(),
                repo_root: runtime_home.path().to_path_buf(),
                project_home: None,
                status: ACTIVE_PROJECT_STATUS.to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )
        .expect_err("same Runtime Home and Product Repository should be rejected");

        assert!(error.to_string().contains("same path"));
        assert!(!runtime_home.project_state_db_path("project_same").exists());
        Ok(())
    }

    #[test]
    fn installation_profile_accepts_executable_paths() -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("store-installation-profile")?;
        let setup = TestRuntimeHomeAdmission::exclusive(runtime_home.path())?;
        let context = setup.context()?;
        initialize_runtime_home(&context, "runtime_home_installation", "{}")?;

        let profile = write_installation_profile(
            &context,
            InstallationProfileRegistration {
                installation_id: "default".to_owned(),
                volicord_command: "/opt/volicord/bin/volicord".to_owned(),
                volicord_mcp_command: "/opt/volicord/bin/volicord".to_owned(),
                bin_dir: PathBuf::from("/opt/volicord/bin"),
                default_connection_mode: "workflow".to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;

        assert_eq!(profile.volicord_command, "/opt/volicord/bin/volicord");
        assert_eq!(profile.default_connection_mode, "workflow");
        assert_eq!(
            require_installation_profile(runtime_home.path())?.volicord_mcp_command,
            "/opt/volicord/bin/volicord"
        );
        Ok(())
    }

    #[test]
    fn installation_profile_read_only_does_not_create_a_missing_registry(
    ) -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("store-read-only-profile-missing")?;
        assert!(!runtime_home.path().exists());

        assert!(installation_profile_read_only(runtime_home.path())?.is_none());

        assert!(!runtime_home.path().exists());
        assert!(!registry_db_path(runtime_home.path()).exists());
        Ok(())
    }

    #[test]
    fn installation_profile_read_only_rejects_zero_byte_registry_without_initializing_it(
    ) -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("store-read-only-profile-zero-byte")?;
        fs::create_dir_all(runtime_home.path())?;
        let registry_path = registry_db_path(runtime_home.path());
        fs::write(&registry_path, [])?;
        let bytes_before = fs::read(&registry_path)?;
        let modified_before = fs::metadata(&registry_path)?.modified()?;
        let entries_before = directory_entries(runtime_home.path())?;

        installation_profile_read_only(runtime_home.path())
            .expect_err("zero-byte Registry must not be initialized by a read");

        assert_eq!(fs::read(&registry_path)?, bytes_before);
        assert!(bytes_before.is_empty());
        assert_eq!(fs::metadata(&registry_path)?.len(), 0);
        assert_eq!(fs::metadata(&registry_path)?.modified()?, modified_before);
        assert_eq!(directory_entries(runtime_home.path())?, entries_before);
        Ok(())
    }

    #[test]
    fn installation_profile_read_only_rejects_empty_sqlite_without_creating_schema(
    ) -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("store-read-only-profile-empty-sqlite")?;
        fs::create_dir_all(runtime_home.path())?;
        let registry_path = registry_db_path(runtime_home.path());
        let conn = rusqlite::Connection::open(&registry_path)?;
        conn.execute_batch("VACUUM")?;
        drop(conn);
        let schema_before = sqlite_master_rows(&registry_path)?;
        let bytes_before = fs::read(&registry_path)?;
        assert!(
            !bytes_before.is_empty(),
            "VACUUM should materialize an empty valid SQLite database"
        );
        let modified_before = fs::metadata(&registry_path)?.modified()?;
        let entries_before = directory_entries(runtime_home.path())?;

        installation_profile_read_only(runtime_home.path())
            .expect_err("empty SQLite must not be accepted as a Volicord Registry");

        assert_eq!(sqlite_master_rows(&registry_path)?, schema_before);
        assert!(schema_before.is_empty());
        assert_eq!(fs::read(&registry_path)?, bytes_before);
        assert_eq!(fs::metadata(&registry_path)?.modified()?, modified_before);
        assert_eq!(directory_entries(runtime_home.path())?, entries_before);
        Ok(())
    }

    #[test]
    fn installation_profile_read_only_preserves_valid_registry_without_profile(
    ) -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("store-read-only-profile-absent")?;
        let setup = TestRuntimeHomeAdmission::exclusive(runtime_home.path())?;
        let context = setup.context()?;
        initialize_runtime_home(&context, "runtime_home_profile_absent", "{}")?;
        let registry_path = registry_db_path(runtime_home.path());
        let bytes_before = fs::read(&registry_path)?;
        let modified_before = fs::metadata(&registry_path)?.modified()?;
        let entries_before = directory_entries(runtime_home.path())?;

        assert!(installation_profile_read_only(runtime_home.path())?.is_none());

        assert_eq!(fs::read(&registry_path)?, bytes_before);
        assert_eq!(fs::metadata(&registry_path)?.modified()?, modified_before);
        assert_eq!(directory_entries(runtime_home.path())?, entries_before);
        Ok(())
    }

    #[test]
    fn installation_profile_read_only_returns_exact_profile_without_writing(
    ) -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("store-read-only-profile-present")?;
        let setup = TestRuntimeHomeAdmission::exclusive(runtime_home.path())?;
        let context = setup.context()?;
        initialize_runtime_home(&context, "runtime_home_profile_present", "{}")?;
        let expected = write_installation_profile(
            &context,
            InstallationProfileRegistration {
                installation_id: "default".to_owned(),
                volicord_command: "/opt/volicord/bin/volicord".to_owned(),
                volicord_mcp_command: "/opt/volicord/bin/volicord".to_owned(),
                bin_dir: PathBuf::from("/opt/volicord/bin"),
                default_connection_mode: "workflow".to_owned(),
                metadata_json: r#"{"source":"test"}"#.to_owned(),
            },
        )?;
        let registry_path = registry_db_path(runtime_home.path());
        let bytes_before = fs::read(&registry_path)?;
        let modified_before = fs::metadata(&registry_path)?.modified()?;
        let entries_before = directory_entries(runtime_home.path())?;

        let actual = installation_profile_read_only(runtime_home.path())?
            .expect("stored installation profile should be returned");

        assert_eq!(actual, expected);
        assert_eq!(fs::read(&registry_path)?, bytes_before);
        assert_eq!(fs::metadata(&registry_path)?.modified()?, modified_before);
        assert_eq!(directory_entries(runtime_home.path())?, entries_before);
        Ok(())
    }

    #[test]
    fn installation_profile_rejects_impossible_command_text() -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("store-installation-profile-invalid")?;
        let setup = TestRuntimeHomeAdmission::exclusive(runtime_home.path())?;
        let context = setup.context()?;
        initialize_runtime_home(&context, "runtime_home_installation_invalid", "{}")?;

        let error = write_installation_profile(
            &context,
            InstallationProfileRegistration {
                installation_id: "default".to_owned(),
                volicord_command: "volicord\0bad".to_owned(),
                volicord_mcp_command: "volicord".to_owned(),
                bin_dir: PathBuf::from("/opt/volicord/bin"),
                default_connection_mode: "workflow".to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )
        .expect_err("NUL command should be rejected");

        assert!(error
            .to_string()
            .contains("volicord_command must not contain NUL"));
        Ok(())
    }

    #[test]
    fn project_registration_rejects_repository_inside_runtime_home() -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("store-repo-inside-runtime")?;
        let setup = TestRuntimeHomeAdmission::exclusive(runtime_home.path())?;
        let context = setup.context()?;
        initialize_runtime_home(&context, "runtime_home_contains_repo", "{}")?;
        let repo_root = runtime_home.path().join("repo");
        fs::create_dir_all(&repo_root)?;

        let error = register_project(
            &context,
            ProjectRegistration {
                project_id: "project_repo_inside".to_owned(),
                repo_root,
                project_home: None,
                status: ACTIVE_PROJECT_STATUS.to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )
        .expect_err("repository under Runtime Home should be rejected");

        assert!(error
            .to_string()
            .contains("Product Repository must not be inside Volicord Runtime Home"));
        assert!(!runtime_home
            .project_state_db_path("project_repo_inside")
            .exists());
        Ok(())
    }

    #[test]
    fn project_registration_rejects_runtime_home_inside_repository() -> Result<(), Box<dyn Error>> {
        let root = TempRuntimeHome::new("store-runtime-inside-repo")?;
        let repo_root = root.create_product_repo("repo")?;
        let runtime_home = repo_root.join(".volicord");
        let setup = TestRuntimeHomeAdmission::exclusive(&runtime_home)?;
        let context = setup.context()?;
        initialize_runtime_home(&context, "runtime_home_inside_repo", "{}")?;

        let error = register_project(
            &context,
            ProjectRegistration {
                project_id: "project_runtime_inside".to_owned(),
                repo_root,
                project_home: None,
                status: ACTIVE_PROJECT_STATUS.to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )
        .expect_err("Runtime Home under repository should be rejected");

        assert!(error
            .to_string()
            .contains("Volicord Runtime Home must not be inside Product Repository"));
        assert!(!project_home_path(&runtime_home, "project_runtime_inside").exists());
        Ok(())
    }

    #[test]
    fn project_registration_accepts_separate_sibling_paths() -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("store-sibling-paths")?;
        let setup = TestRuntimeHomeAdmission::exclusive(runtime_home.path())?;
        let context = setup.context()?;
        let repo_root = runtime_home.create_product_repo("repo")?;
        initialize_runtime_home(&context, "runtime_home_sibling", "{}")?;

        let record = register_project(
            &context,
            ProjectRegistration {
                project_id: "project_sibling".to_owned(),
                repo_root: repo_root.clone(),
                project_home: None,
                status: ACTIVE_PROJECT_STATUS.to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;

        assert_eq!(record.repo_root, fs::canonicalize(repo_root)?);
        assert!(record.project_home.starts_with(runtime_home.path()));
        assert!(record.state_db_path.exists());
        Ok(())
    }

    #[test]
    fn project_reregistration_preserves_clock_floor_and_rejects_corrupt_floor(
    ) -> Result<(), Box<dyn Error>> {
        let project_id = "project_reregister_clock_floor";
        let (runtime_home, repo_root) =
            registered_project("store-reregister-clock-floor", project_id)?;
        let mutation = TestRuntimeHomeAdmission::shared(runtime_home.path())?;
        let context = mutation.context()?;
        let record = project_record(runtime_home.path(), project_id)?
            .expect("registered project should remain available");
        let conn = open_project_state_database_for_test(&record.state_db_path)?;
        let future_floor = "2999-07-13T12:34:56.789Z";
        conn.execute(
            "UPDATE project_state SET updated_at = ?2 WHERE project_id = ?1",
            params![project_id, future_floor],
        )?;
        drop(conn);

        register_project(
            &context,
            ProjectRegistration {
                project_id: project_id.to_owned(),
                repo_root: repo_root.clone(),
                project_home: None,
                status: ACTIVE_PROJECT_STATUS.to_owned(),
                metadata_json: r#"{"reregistered":true}"#.to_owned(),
            },
        )?;
        let conn = open_project_state_database_for_test(&record.state_db_path)?;
        let preserved = conn.query_row(
            "SELECT updated_at FROM project_state WHERE project_id = ?1",
            [project_id],
            |row| row.get::<_, String>(0),
        )?;
        assert_eq!(preserved, future_floor);

        drop(conn);
        for corrupt_floor in ["not-a-timestamp", "9999-12-31T23:59:59-23:59"] {
            let conn = open_project_state_database_for_test(&record.state_db_path)?;
            conn.execute(
                "UPDATE project_state SET updated_at = ?2 WHERE project_id = ?1",
                params![project_id, corrupt_floor],
            )?;
            drop(conn);
            let error = register_project(
                &context,
                ProjectRegistration {
                    project_id: project_id.to_owned(),
                    repo_root: repo_root.clone(),
                    project_home: None,
                    status: ACTIVE_PROJECT_STATUS.to_owned(),
                    metadata_json: r#"{"must_not_apply":true}"#.to_owned(),
                },
            )
            .expect_err("corrupt project clock floor must fail closed");
            assert!(matches!(
                error,
                StoreError::CorruptOwnerStateValue {
                    table: "project_state",
                    logical_column: "updated_at",
                    ..
                }
            ));
            let corrupt = open_project_state_database_for_test(&record.state_db_path)?.query_row(
                "SELECT updated_at FROM project_state WHERE project_id = ?1",
                [project_id],
                |row| row.get::<_, String>(0),
            )?;
            assert_eq!(corrupt, corrupt_floor);
        }
        Ok(())
    }

    #[test]
    fn repo_project_registration_uses_basename_with_unique_safe_aliases(
    ) -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("store-repo-project-basename")?;
        let setup = TestRuntimeHomeAdmission::exclusive(runtime_home.path())?;
        let context = setup.context()?;
        let repo_a = runtime_home.create_product_repo("left/repo")?;
        let repo_b = runtime_home.create_product_repo("right/repo")?;
        initialize_runtime_home(&context, "runtime_home_repo_project", "{}")?;

        let first = ensure_project_for_repo(
            &context,
            RepoProjectRegistration {
                project_name: None,
                project_alias: None,
                repo_root: repo_a,
                project_home: None,
                status: ACTIVE_PROJECT_STATUS.to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;
        let second = ensure_project_for_repo(
            &context,
            RepoProjectRegistration {
                project_name: None,
                project_alias: None,
                repo_root: repo_b,
                project_home: None,
                status: ACTIVE_PROJECT_STATUS.to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;

        assert_eq!(first.project_name, "repo");
        assert_eq!(second.project_name, "repo");
        assert_ne!(first.project_internal_id, second.project_internal_id);
        assert_ne!(first.project_alias, second.project_alias);
        assert!(first.project_alias.starts_with("repo-"));
        assert!(second.project_alias.starts_with("repo-"));
        Ok(())
    }

    #[test]
    fn repo_project_registration_reuses_existing_project_without_renaming(
    ) -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("store-repo-project-reuse")?;
        let setup = TestRuntimeHomeAdmission::exclusive(runtime_home.path())?;
        let context = setup.context()?;
        let repo_root = runtime_home.create_product_repo("repo")?;
        initialize_runtime_home(&context, "runtime_home_repo_reuse", "{}")?;

        let original = ensure_project_for_repo(
            &context,
            RepoProjectRegistration {
                project_name: None,
                project_alias: None,
                repo_root: repo_root.clone(),
                project_home: None,
                status: ACTIVE_PROJECT_STATUS.to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;
        rename_project(
            &context,
            &original.project_internal_id,
            "Renamed Project",
            None,
        )?;

        let reused = ensure_project_for_repo(
            &context,
            RepoProjectRegistration {
                project_name: None,
                project_alias: None,
                repo_root,
                project_home: None,
                status: ACTIVE_PROJECT_STATUS.to_owned(),
                metadata_json: "{\"ignored\":true}".to_owned(),
            },
        )?;

        assert_eq!(reused.project_internal_id, original.project_internal_id);
        assert_eq!(reused.project_name, "Renamed Project");
        assert_eq!(reused.metadata_json, "{}");
        Ok(())
    }

    #[test]
    fn project_registration_accepts_valid_custom_project_home() -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("store-custom-project-home")?;
        let setup = TestRuntimeHomeAdmission::exclusive(runtime_home.path())?;
        let context = setup.context()?;
        let repo_root = runtime_home.create_product_repo("repo")?;
        let project_home = runtime_home.path().join("custom-projects/project_custom");
        initialize_runtime_home(&context, "runtime_home_custom_project", "{}")?;

        let record = register_project(
            &context,
            ProjectRegistration {
                project_id: "project_custom".to_owned(),
                repo_root,
                project_home: Some(project_home.clone()),
                status: ACTIVE_PROJECT_STATUS.to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;
        let project = project_record_for_execution(runtime_home.path(), "project_custom")?
            .expect("project should be registered");
        let store = CoreProjectStore::open_read_only(
            runtime_home.path(),
            &ProjectId::new("project_custom"),
        )?;

        assert_eq!(record.project_home, project_home);
        assert_eq!(project.project_home, project_home);
        assert_eq!(
            project.state_db_path,
            project_home.join(PROJECT_STATE_DB_FILE)
        );
        assert_eq!(store.project_record().state_db_path, project.state_db_path);
        assert!(project.state_db_path.exists());
        Ok(())
    }

    #[test]
    fn project_registration_rejects_custom_home_outside_runtime_home() -> Result<(), Box<dyn Error>>
    {
        let runtime_home = TempRuntimeHome::new("store-project-home-outside")?;
        let setup = TestRuntimeHomeAdmission::exclusive(runtime_home.path())?;
        let context = setup.context()?;
        let repo_root = runtime_home.create_product_repo("repo")?;
        let project_home = runtime_home
            .path()
            .parent()
            .expect("runtime home has parent")
            .join("outside-project-home");
        initialize_runtime_home(&context, "runtime_home_project_home_outside", "{}")?;

        let error = register_project(
            &context,
            ProjectRegistration {
                project_id: "project_home_outside".to_owned(),
                repo_root,
                project_home: Some(project_home.clone()),
                status: ACTIVE_PROJECT_STATUS.to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )
        .expect_err("project_home outside Runtime Home should be rejected");

        assert!(error.to_string().contains("project_home must be inside"));
        assert!(!project_home.exists());
        Ok(())
    }

    #[test]
    fn project_registration_rejects_custom_home_overlapping_repository(
    ) -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("store-project-home-repo-overlap")?;
        let setup = TestRuntimeHomeAdmission::exclusive(runtime_home.path())?;
        let context = setup.context()?;
        let repo_root = runtime_home.create_product_repo("repo")?;
        let project_home = repo_root.join(".volicord-project");
        initialize_runtime_home(&context, "runtime_home_project_home_overlap", "{}")?;

        let error = register_project(
            &context,
            ProjectRegistration {
                project_id: "project_home_overlap".to_owned(),
                repo_root,
                project_home: Some(project_home.clone()),
                status: ACTIVE_PROJECT_STATUS.to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )
        .expect_err("project_home overlapping Product Repository should be rejected");

        assert!(error
            .to_string()
            .contains("project_home must not overlap Product Repository"));
        assert!(!project_home.exists());
        Ok(())
    }

    #[test]
    fn checked_project_record_accepts_valid_existing_registration() -> Result<(), Box<dyn Error>> {
        let (runtime_home, repo_root) = registered_project("store-checked-valid", "project_valid")?;

        let project = project_record_for_execution(runtime_home.path(), "project_valid")?
            .expect("project should be registered");
        let store = CoreProjectStore::open_read_only(
            runtime_home.path(),
            &ProjectId::new("project_valid"),
        )?;

        assert_eq!(project.repo_root, fs::canonicalize(repo_root)?);
        assert_eq!(
            project.state_db_path,
            project.project_home.join(PROJECT_STATE_DB_FILE)
        );
        assert_eq!(store.project_record().project_id, "project_valid");
        assert_eq!(store.project_record().state_db_path, project.state_db_path);
        Ok(())
    }

    #[test]
    fn checked_project_list_rejects_unsafe_stored_project_id() -> Result<(), Box<dyn Error>> {
        let original_project_id = "project_unsafe_id_original";
        let damaged_project_id = "project/unsafe";
        let (runtime_home, _) = registered_project("store-checked-unsafe-id", original_project_id)?;
        let original = project_record(runtime_home.path(), original_project_id)?
            .expect("project should be registered");

        replace_project_id(runtime_home.path(), original_project_id, damaged_project_id)?;

        let list_error = list_projects(runtime_home.path())
            .expect_err("project listing should reject unsafe stored project_id");
        assert_invalid_project_registration(list_error, "invalid_project_id");
        let damaged = raw_project_record(runtime_home.path(), damaged_project_id)?;
        assert_eq!(damaged.project_id, damaged_project_id);
        assert_eq!(damaged.project_home, original.project_home);
        assert_eq!(damaged.state_db_path, original.state_db_path);
        assert_registry_record_unchanged_and_visible(
            runtime_home.path(),
            damaged_project_id,
            &damaged,
        )?;
        Ok(())
    }

    #[test]
    fn checked_project_record_rejects_state_db_path_mismatch_before_alternate_creation(
    ) -> Result<(), Box<dyn Error>> {
        let project_id = "project_state_db_mismatch_missing";
        let (runtime_home, _) = registered_project("store-state-db-missing-alt", project_id)?;
        let original =
            project_record(runtime_home.path(), project_id)?.expect("project should be registered");
        let expected_state_path = original.project_home.join(PROJECT_STATE_DB_FILE);
        let alternate_state_path = runtime_home.path().join("alternate/missing-state.sqlite");

        replace_project_state_db_path(runtime_home.path(), project_id, &alternate_state_path)?;
        assert!(!alternate_state_path.exists());

        let lookup_error = project_record(runtime_home.path(), project_id)
            .expect_err("mismatched state_db_path should be rejected by project lookup");
        assert_state_db_path_mismatch(lookup_error, &alternate_state_path, &expected_state_path);
        let list_error = list_projects(runtime_home.path())
            .expect_err("mismatched state_db_path should be rejected by project listing");
        assert_state_db_path_mismatch(list_error, &alternate_state_path, &expected_state_path);
        let error = project_record_for_execution(runtime_home.path(), project_id)
            .expect_err("mismatched state_db_path should be rejected for execution");
        assert_state_db_path_mismatch(error, &alternate_state_path, &expected_state_path);
        let open_error =
            CoreProjectStore::open_read_only(runtime_home.path(), &ProjectId::new(project_id))
                .expect_err("Core store open should reject mismatched state_db_path");
        assert_state_db_path_mismatch(open_error, &alternate_state_path, &expected_state_path);
        assert!(!alternate_state_path.exists());

        let damaged = raw_project_record(runtime_home.path(), project_id)?;
        assert_eq!(damaged.state_db_path, alternate_state_path);
        assert_registry_record_unchanged_and_visible(runtime_home.path(), project_id, &damaged)?;
        Ok(())
    }

    #[test]
    fn checked_project_record_rejects_existing_alternate_without_mutating_alternate(
    ) -> Result<(), Box<dyn Error>> {
        let project_id = "project_state_db_mismatch_existing";
        let (runtime_home, _) = registered_project("store-state-db-existing-alt", project_id)?;
        let original =
            project_record(runtime_home.path(), project_id)?.expect("project should be registered");
        let expected_state_path = original.project_home.join(PROJECT_STATE_DB_FILE);
        let alternate_state_path = runtime_home.path().join("alternate/existing-state.sqlite");
        fs::create_dir_all(
            alternate_state_path
                .parent()
                .expect("alternate state path has parent"),
        )?;
        let conn = open_project_state_database_for_test(&alternate_state_path)?;
        drop(conn);
        let metadata_before = fs::metadata(&alternate_state_path)?;
        let modified_before = metadata_before.modified()?;

        replace_project_state_db_path(runtime_home.path(), project_id, &alternate_state_path)?;

        let open_error =
            CoreProjectStore::open_read_only(runtime_home.path(), &ProjectId::new(project_id))
                .expect_err("Core store open should reject mismatched state_db_path");
        assert_state_db_path_mismatch(open_error, &alternate_state_path, &expected_state_path);
        let metadata_after = fs::metadata(&alternate_state_path)?;
        assert_eq!(metadata_after.len(), metadata_before.len());
        assert_eq!(metadata_after.modified()?, modified_before);
        let lookup_error = project_record(runtime_home.path(), project_id)
            .expect_err("mismatched state_db_path should be rejected by project lookup");
        assert_state_db_path_mismatch(lookup_error, &alternate_state_path, &expected_state_path);
        let list_error = list_projects(runtime_home.path())
            .expect_err("mismatched state_db_path should be rejected by project listing");
        assert_state_db_path_mismatch(list_error, &alternate_state_path, &expected_state_path);
        let damaged = raw_project_record(runtime_home.path(), project_id)?;
        assert_eq!(damaged.state_db_path, alternate_state_path);
        assert_registry_record_unchanged_and_visible(runtime_home.path(), project_id, &damaged)?;
        Ok(())
    }

    #[test]
    fn checked_project_record_rejects_same_path_registration_for_operational_reads(
    ) -> Result<(), Box<dyn Error>> {
        let project_id = "project_same_path_invalid";
        let (runtime_home, _) = registered_project("store-checked-same", project_id)?;
        replace_project_repo_root(runtime_home.path(), project_id, runtime_home.path())?;

        let error = project_record_for_execution(runtime_home.path(), project_id)
            .expect_err("same-path registration should be rejected for execution");
        assert_invalid_project_registration(error, "same_path");
        let open_error =
            CoreProjectStore::open_read_only(runtime_home.path(), &ProjectId::new(project_id))
                .expect_err("Core store open should reject same-path registration");
        assert_invalid_project_registration(open_error, "same_path");

        let lookup_error = project_record(runtime_home.path(), project_id)
            .expect_err("project lookup should reject same-path registration");
        assert_invalid_project_registration(lookup_error, "same_path");
        let list_error = list_projects(runtime_home.path())
            .expect_err("project listing should reject same-path registration");
        assert_invalid_project_registration(list_error, "same_path");
        let damaged = raw_project_record(runtime_home.path(), project_id)?;
        assert_eq!(damaged.repo_root, runtime_home.path());
        assert_registry_record_unchanged_and_visible(runtime_home.path(), project_id, &damaged)?;
        Ok(())
    }

    #[test]
    fn checked_project_record_rejects_repository_inside_runtime_home() -> Result<(), Box<dyn Error>>
    {
        let project_id = "project_repo_inside_runtime_home";
        let (runtime_home, _) = registered_project("store-checked-repo-inside", project_id)?;
        let repo_root = runtime_home.path().join("invalid-product-repo");
        fs::create_dir_all(&repo_root)?;
        replace_project_repo_root(runtime_home.path(), project_id, &repo_root)?;

        let error =
            CoreProjectStore::open_read_only(runtime_home.path(), &ProjectId::new(project_id))
                .expect_err("repository under Runtime Home should be rejected for execution");

        assert_invalid_project_registration(error, "runtime_home_contains_product_repository");
        let lookup_error = project_record(runtime_home.path(), project_id)
            .expect_err("project lookup should reject repository under Runtime Home");
        assert_invalid_project_registration(
            lookup_error,
            "runtime_home_contains_product_repository",
        );
        let list_error = list_projects(runtime_home.path())
            .expect_err("project listing should reject repository under Runtime Home");
        assert_invalid_project_registration(list_error, "runtime_home_contains_product_repository");
        let damaged = raw_project_record(runtime_home.path(), project_id)?;
        assert_eq!(damaged.repo_root, repo_root);
        assert_registry_record_unchanged_and_visible(runtime_home.path(), project_id, &damaged)?;
        Ok(())
    }

    #[test]
    fn checked_project_record_rejects_runtime_home_inside_repository() -> Result<(), Box<dyn Error>>
    {
        let project_id = "project_runtime_home_inside_repo";
        let (runtime_home, _) = registered_project("store-checked-runtime-inside", project_id)?;
        let repo_root = runtime_home
            .path()
            .parent()
            .expect("runtime home has parent")
            .to_path_buf();
        replace_project_repo_root(runtime_home.path(), project_id, &repo_root)?;

        let error =
            CoreProjectStore::open_read_only(runtime_home.path(), &ProjectId::new(project_id))
                .expect_err("Runtime Home under repository should be rejected for execution");

        assert_invalid_project_registration(error, "product_repository_contains_runtime_home");
        let lookup_error = project_record(runtime_home.path(), project_id)
            .expect_err("project lookup should reject Runtime Home under repository");
        assert_invalid_project_registration(
            lookup_error,
            "product_repository_contains_runtime_home",
        );
        let list_error = list_projects(runtime_home.path())
            .expect_err("project listing should reject Runtime Home under repository");
        assert_invalid_project_registration(list_error, "product_repository_contains_runtime_home");
        let damaged = raw_project_record(runtime_home.path(), project_id)?;
        assert_registry_record_unchanged_and_visible(runtime_home.path(), project_id, &damaged)?;
        Ok(())
    }

    #[test]
    fn checked_project_record_rejects_project_home_outside_runtime_home(
    ) -> Result<(), Box<dyn Error>> {
        let project_id = "project_home_outside_damaged";
        let (runtime_home, _) = registered_project("store-checked-project-home", project_id)?;
        let original =
            project_record(runtime_home.path(), project_id)?.expect("project should be registered");
        let outside_project_home = runtime_home
            .path()
            .parent()
            .expect("runtime home has parent")
            .join("outside-project-home-damaged");

        replace_project_home(runtime_home.path(), project_id, &outside_project_home)?;

        let lookup_error = project_record(runtime_home.path(), project_id)
            .expect_err("project lookup should reject project_home outside Runtime Home");
        assert_invalid_project_registration(lookup_error, "project_home_outside_runtime_home");
        let list_error = list_projects(runtime_home.path())
            .expect_err("project listing should reject project_home outside Runtime Home");
        assert_invalid_project_registration(list_error, "project_home_outside_runtime_home");
        let open_error =
            CoreProjectStore::open_read_only(runtime_home.path(), &ProjectId::new(project_id))
                .expect_err("Core store open should reject project_home outside Runtime Home");
        assert_invalid_project_registration(open_error, "project_home_outside_runtime_home");
        let damaged = raw_project_record(runtime_home.path(), project_id)?;
        assert_eq!(damaged.project_home, outside_project_home);
        assert_eq!(damaged.state_db_path, original.state_db_path);
        assert_registry_record_unchanged_and_visible(runtime_home.path(), project_id, &damaged)?;
        Ok(())
    }

    fn registered_project(
        prefix: &str,
        project_id: &str,
    ) -> Result<(TempRuntimeHome, PathBuf), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new(prefix)?;
        let setup = TestRuntimeHomeAdmission::exclusive(runtime_home.path())?;
        let context = setup.context()?;
        let repo_root = runtime_home.create_product_repo("repo")?;
        initialize_runtime_home(&context, &format!("runtime_home_{project_id}"), "{}")?;
        register_project(
            &context,
            ProjectRegistration {
                project_id: project_id.to_owned(),
                repo_root: repo_root.clone(),
                project_home: None,
                status: ACTIVE_PROJECT_STATUS.to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;
        Ok((runtime_home, repo_root))
    }

    fn directory_entries(root: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
        let mut entries = fs::read_dir(root)?
            .map(|entry| entry.map(|entry| entry.file_name().into()))
            .collect::<Result<Vec<PathBuf>, _>>()?;
        entries.sort();
        Ok(entries)
    }

    fn staging_directories(runtime_home: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
        let parent = runtime_home.parent().expect("Runtime Home has parent");
        let mut staging = fs::read_dir(parent)?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".volicord-runtime-staging-")
            })
            .map(|entry| entry.path())
            .collect::<Vec<_>>();
        staging.sort();
        Ok(staging)
    }

    fn installation_registration(runtime_home: &Path) -> InstallationProfileRegistration {
        InstallationProfileRegistration {
            installation_id: "default".to_owned(),
            volicord_command: "/opt/volicord/bin/volicord".to_owned(),
            volicord_mcp_command: "/opt/volicord/bin/volicord".to_owned(),
            bin_dir: runtime_home.join("bin"),
            default_connection_mode: "workflow".to_owned(),
            metadata_json: "{}".to_owned(),
        }
    }

    fn sqlite_master_rows(path: &Path) -> Result<Vec<SqliteMasterRow>, Box<dyn Error>> {
        let conn = open_read_only_database(path)?;
        let mut statement =
            conn.prepare("SELECT type, name, sql FROM sqlite_master ORDER BY type, name")?;
        let rows = statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    fn replace_project_repo_root(
        runtime_home: &Path,
        project_id: &str,
        repo_root: &Path,
    ) -> Result<(), Box<dyn Error>> {
        let conn = open_registry_database_for_test(registry_db_path(runtime_home))?;
        conn.execute(
            "UPDATE projects SET repo_root = ?2 WHERE project_internal_id = ?1",
            rusqlite::params![project_id, repo_root.to_string_lossy().as_ref()],
        )?;
        Ok(())
    }

    fn replace_project_id(
        runtime_home: &Path,
        old_project_id: &str,
        new_project_id: &str,
    ) -> Result<(), Box<dyn Error>> {
        let conn = rusqlite::Connection::open(registry_db_path(runtime_home))?;
        conn.pragma_update(None, "foreign_keys", "OFF")?;
        conn.execute(
            "UPDATE projects SET project_internal_id = ?2 WHERE project_internal_id = ?1",
            rusqlite::params![old_project_id, new_project_id],
        )?;
        conn.execute(
            "UPDATE project_aliases SET project_internal_id = ?2 WHERE project_internal_id = ?1",
            rusqlite::params![old_project_id, new_project_id],
        )?;
        Ok(())
    }

    fn replace_project_state_db_path(
        runtime_home: &Path,
        project_id: &str,
        state_db_path: &Path,
    ) -> Result<(), Box<dyn Error>> {
        let conn = open_registry_database_for_test(registry_db_path(runtime_home))?;
        conn.execute(
            "UPDATE projects SET state_db_path = ?2 WHERE project_internal_id = ?1",
            rusqlite::params![project_id, state_db_path.to_string_lossy().as_ref()],
        )?;
        Ok(())
    }

    fn replace_project_home(
        runtime_home: &Path,
        project_id: &str,
        project_home: &Path,
    ) -> Result<(), Box<dyn Error>> {
        let conn = open_registry_database_for_test(registry_db_path(runtime_home))?;
        conn.execute(
            "UPDATE projects SET project_home = ?2 WHERE project_internal_id = ?1",
            rusqlite::params![project_id, project_home.to_string_lossy().as_ref()],
        )?;
        Ok(())
    }

    fn assert_registry_record_unchanged_and_visible(
        runtime_home: &Path,
        project_id: &str,
        expected: &ProjectRecord,
    ) -> StoreResult<()> {
        let project = raw_project_record(runtime_home, project_id)?;
        assert_eq!(&project, expected);

        let inspection = inspect_registry_database(runtime_home);
        let DatabaseInspection::Present(snapshot) = inspection else {
            panic!("expected present registry inspection, got {inspection:?}");
        };
        assert!(
            snapshot.projects.iter().any(|project| {
                project.project_id == expected.project_id
                    && project.repo_root == expected.repo_root
                    && project.project_home == expected.project_home
                    && project.state_db_path == expected.state_db_path
            }),
            "invalid registry record should remain inspectable"
        );
        Ok(())
    }

    fn raw_project_record(runtime_home: &Path, project_id: &str) -> StoreResult<ProjectRecord> {
        let conn = open_read_only_database(registry_db_path(runtime_home))?;
        Ok(conn.query_row(
            "SELECT
                project_internal_id,
                project_name,
                project_alias,
                runtime_home_id,
                repo_root,
                project_home,
                state_db_path,
                status,
                metadata_json
             FROM projects
             WHERE project_internal_id = ?1",
            rusqlite::params![project_id],
            project_record_from_row,
        )?)
    }

    fn assert_invalid_project_registration(error: StoreError, relationship: &str) {
        match error {
            StoreError::InvalidProjectRegistration {
                relationship: actual,
                detail,
                ..
            } => {
                assert_eq!(actual, relationship);
                assert!(!detail.is_empty());
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    fn assert_state_db_path_mismatch(error: StoreError, stored: &Path, expected: &Path) {
        match error {
            StoreError::InvalidProjectRegistration {
                field,
                relationship,
                detail,
                ..
            } => {
                assert_eq!(field, "state_db_path");
                assert_eq!(relationship, "state_db_path_mismatch");
                assert!(detail.contains(&stored.display().to_string()));
                assert!(detail.contains(&expected.display().to_string()));
            }
            other => panic!("unexpected error: {other}"),
        }
    }
}
