use std::{path::Path, time::Duration};

use volicord_platform_fs::{
    CanonicalRuntimeHomePath, RuntimeHomeMutationBusy, RuntimeHomeMutationLeaseMode,
    RuntimeHomeMutationPermit, RuntimeHomeMutationWaitPolicy,
};

use crate::{StoreError, StoreResult};

/// Stable condition code for an ordinary writer rejected while setup is exclusive.
pub const RUNTIME_HOME_MUTATION_SETUP_IN_PROGRESS: &str = "runtime_home.mutation.setup_in_progress";

/// Bounded, typed projection of an ordinary mutation blocked by setup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeHomeMutationSetupInProgress {
    runtime_home: CanonicalRuntimeHomePath,
    mutation_domain: String,
    requested_mode: RuntimeHomeMutationLeaseMode,
    wait_policy: RuntimeHomeMutationWaitPolicy,
    elapsed: Duration,
}

impl RuntimeHomeMutationSetupInProgress {
    /// Constructs the stable higher-layer condition from platform busy facts.
    pub fn from_busy(mutation_domain: impl Into<String>, busy: RuntimeHomeMutationBusy) -> Self {
        Self {
            runtime_home: busy.target().clone(),
            mutation_domain: mutation_domain.into(),
            requested_mode: busy.requested_mode(),
            wait_policy: busy.wait_policy(),
            elapsed: busy.elapsed(),
        }
    }

    /// Stable machine-readable condition code.
    pub const fn code(&self) -> &'static str {
        RUNTIME_HOME_MUTATION_SETUP_IN_PROGRESS
    }

    /// Exact canonical Runtime Home whose setup is in progress.
    pub fn runtime_home(&self) -> &CanonicalRuntimeHomePath {
        &self.runtime_home
    }

    /// Higher-layer operation family that requested admission.
    pub fn mutation_domain(&self) -> &str {
        &self.mutation_domain
    }

    /// Lease mode requested by the rejected writer.
    pub const fn requested_mode(&self) -> RuntimeHomeMutationLeaseMode {
        self.requested_mode
    }

    /// Bounded wait policy used by the writer.
    pub const fn wait_policy(&self) -> RuntimeHomeMutationWaitPolicy {
        self.wait_policy
    }

    /// Time spent attempting admission.
    pub const fn elapsed(&self) -> Duration {
        self.elapsed
    }

    /// Setup-busy mutations are safe to retry after setup completes.
    pub const fn retryable(&self) -> bool {
        true
    }
}

impl std::fmt::Display for RuntimeHomeMutationSetupInProgress {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{}: Runtime Home {} is in exclusive setup; {} may be retried after setup completes",
            self.code(),
            self.runtime_home.as_path().display(),
            self.mutation_domain
        )
    }
}

impl std::error::Error for RuntimeHomeMutationSetupInProgress {}

/// Store capability authorizing mutations in one exact Volicord Runtime Home.
///
/// The context borrows one live platform mutation permit. It carries no user
/// authority, Task authority, Product Repository permission, or security
/// guarantee.
///
/// ```compile_fail
/// use std::path::Path;
/// use volicord_platform_fs::{
///     RuntimeHomeMutationLease, RuntimeHomeMutationLeaseMode,
///     RuntimeHomeMutationLeaseOutcome, RuntimeHomeMutationWaitPolicy,
/// };
/// use volicord_store::RuntimeHomeMutationContext;
///
/// fn detached_context(path: &Path) -> RuntimeHomeMutationContext<'static> {
///     let RuntimeHomeMutationLeaseOutcome::Acquired(lease) =
///         RuntimeHomeMutationLease::acquire(
///             path,
///             RuntimeHomeMutationLeaseMode::SharedWriter,
///             RuntimeHomeMutationWaitPolicy::Immediate,
///         )
///         .unwrap()
///     else {
///         unreachable!()
///     };
///     RuntimeHomeMutationContext::new(lease.permit(), path).unwrap()
/// }
/// ```
pub struct RuntimeHomeMutationContext<'lease> {
    permit: RuntimeHomeMutationPermit<'lease>,
    runtime_home: CanonicalRuntimeHomePath,
}

#[cfg(test)]
pub(crate) struct TestRuntimeHomeAdmission {
    lease: volicord_platform_fs::RuntimeHomeMutationLease,
    runtime_home: std::path::PathBuf,
}

#[cfg(test)]
impl TestRuntimeHomeAdmission {
    pub(crate) fn shared(runtime_home: impl AsRef<Path>) -> StoreResult<Self> {
        Self::acquire(
            runtime_home.as_ref(),
            RuntimeHomeMutationLeaseMode::SharedWriter,
        )
    }

    pub(crate) fn exclusive(runtime_home: impl AsRef<Path>) -> StoreResult<Self> {
        Self::acquire(
            runtime_home.as_ref(),
            RuntimeHomeMutationLeaseMode::ExclusiveSetup,
        )
    }

    fn acquire(runtime_home: &Path, mode: RuntimeHomeMutationLeaseMode) -> StoreResult<Self> {
        use volicord_platform_fs::{RuntimeHomeMutationLease, RuntimeHomeMutationLeaseOutcome};

        let outcome = RuntimeHomeMutationLease::acquire(
            runtime_home,
            mode,
            RuntimeHomeMutationWaitPolicy::Immediate,
        )
        .map_err(|error| StoreError::InvalidInput {
            detail: error.to_string(),
        })?;
        let RuntimeHomeMutationLeaseOutcome::Acquired(lease) = outcome else {
            return Err(StoreError::Conflict {
                entity: "runtime_home_mutation",
                id: runtime_home.display().to_string(),
                detail: "test Runtime Home mutation admission is busy".to_owned(),
            });
        };
        Ok(Self {
            lease,
            runtime_home: runtime_home.to_path_buf(),
        })
    }

    pub(crate) fn context(&self) -> StoreResult<RuntimeHomeMutationContext<'_>> {
        RuntimeHomeMutationContext::new(self.lease.permit(), &self.runtime_home)
    }
}

#[cfg(test)]
pub(crate) fn with_test_runtime_home_setup<T>(
    runtime_home: &Path,
    operation: impl FnOnce(&RuntimeHomeMutationContext<'_>) -> StoreResult<T>,
) -> StoreResult<T> {
    let admission = TestRuntimeHomeAdmission::exclusive(runtime_home)?;
    let context = admission.context()?;
    operation(&context)
}

impl<'lease> RuntimeHomeMutationContext<'lease> {
    /// Constructs a Store mutation context after verifying the selected Runtime Home.
    pub fn new(
        permit: RuntimeHomeMutationPermit<'lease>,
        runtime_home: impl AsRef<Path>,
    ) -> StoreResult<Self> {
        let runtime_home = runtime_home.as_ref();
        let matches_target =
            permit
                .matches_target(runtime_home)
                .map_err(|error| StoreError::InvalidInput {
                    detail: format!(
                        "Runtime Home mutation context target could not be verified: {error}"
                    ),
                })?;
        if !matches_target {
            return Err(StoreError::InvalidInput {
                detail: format!(
                    "Runtime Home mutation permit targets {}, not {}",
                    permit.target().as_path().display(),
                    runtime_home.display()
                ),
            });
        }
        let runtime_home = permit.target().clone();
        Ok(Self {
            permit,
            runtime_home,
        })
    }

    /// Returns the exact canonical Runtime Home authorized by this context.
    pub fn runtime_home(&self) -> &CanonicalRuntimeHomePath {
        &self.runtime_home
    }

    /// Returns the admission mode backing this context.
    pub const fn mode(&self) -> RuntimeHomeMutationLeaseMode {
        self.permit.mode()
    }

    /// Returns whether ordinary Runtime Home mutations are authorized.
    pub const fn authorizes_runtime_home_mutation(&self) -> bool {
        matches!(
            self.mode(),
            RuntimeHomeMutationLeaseMode::SharedWriter
                | RuntimeHomeMutationLeaseMode::ExclusiveSetup
        )
    }

    pub(crate) fn reborrow(&self) -> RuntimeHomeMutationContext<'lease> {
        RuntimeHomeMutationContext {
            permit: self.permit.reborrow(),
            runtime_home: self.runtime_home.clone(),
        }
    }

    /// Requires the exclusive setup capability for staging, publication, or recovery.
    pub(crate) fn require_exclusive_setup(&self) -> StoreResult<()> {
        if self.mode() == RuntimeHomeMutationLeaseMode::ExclusiveSetup {
            Ok(())
        } else {
            Err(StoreError::InvalidInput {
                detail: "setup mutation requires ExclusiveSetup admission".to_owned(),
            })
        }
    }

    /// Requires a selected path to resolve to this context's exact Runtime Home.
    pub fn ensure_runtime_home(&self, runtime_home: &Path) -> StoreResult<()> {
        let matches_target =
            self.permit
                .matches_target(runtime_home)
                .map_err(|error| StoreError::InvalidInput {
                    detail: format!("Runtime Home mutation target could not be verified: {error}"),
                })?;
        if matches_target {
            Ok(())
        } else {
            Err(StoreError::InvalidInput {
                detail: format!(
                    "mutation context for {} cannot authorize Runtime Home {}",
                    self.runtime_home.as_path().display(),
                    runtime_home.display()
                ),
            })
        }
    }
}

impl std::fmt::Debug for RuntimeHomeMutationContext<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RuntimeHomeMutationContext")
            .field("runtime_home", &self.runtime_home)
            .field("mode", &self.mode())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use volicord_platform_fs::{
        RuntimeHomeMutationLease, RuntimeHomeMutationLeaseMode, RuntimeHomeMutationLeaseOutcome,
        RuntimeHomeMutationWaitPolicy,
    };
    use volicord_test_support::TempRuntimeHome;

    use super::*;
    use crate::bootstrap::{
        initialize_runtime_home, installation_profile_read_only, prepare_runtime_home,
        runtime_home_record_read_only, write_installation_profile, InstallationProfileRegistration,
    };

    #[test]
    fn context_is_exactly_target_bound() -> Result<(), Box<dyn std::error::Error>> {
        let first = TempRuntimeHome::new("mutation-context-target-first")?;
        let second = TempRuntimeHome::new("mutation-context-target-second")?;
        let admission = TestRuntimeHomeAdmission::shared(first.path())?;

        let error = RuntimeHomeMutationContext::new(admission.lease.permit(), second.path())
            .expect_err("a permit for Runtime Home A must not authorize Runtime Home B");

        assert!(matches!(error, StoreError::InvalidInput { .. }));
        assert!(error.to_string().contains("not"));
        Ok(())
    }

    #[test]
    fn shared_writer_context_authorizes_an_ordinary_store_mutation(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("mutation-context-shared-writer")?;
        with_test_runtime_home_setup(fixture.path(), |context| {
            initialize_runtime_home(context, fixture.path(), "runtime_home_shared_writer", "{}")?;
            Ok(())
        })?;
        let admission = TestRuntimeHomeAdmission::shared(fixture.path())?;
        let context = admission.context()?;

        let written = write_installation_profile(
            &context,
            InstallationProfileRegistration {
                installation_id: "default".to_owned(),
                volicord_command: "volicord".to_owned(),
                volicord_mcp_command: "volicord".to_owned(),
                bin_dir: fixture.path().join("bin"),
                default_connection_mode: "workflow".to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;

        assert_eq!(
            installation_profile_read_only(fixture.path())?,
            Some(written)
        );
        Ok(())
    }

    #[test]
    fn shared_writer_context_cannot_invoke_setup_publication(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("mutation-context-shared-setup")?;
        let admission = TestRuntimeHomeAdmission::shared(fixture.path())?;
        let context = admission.context()?;

        let error =
            prepare_runtime_home(&context, fixture.path(), "runtime_home_shared_setup", "{}")
                .expect_err("setup staging requires ExclusiveSetup");

        assert!(matches!(error, StoreError::InvalidInput { .. }));
        assert!(error.to_string().contains("ExclusiveSetup"));
        assert!(!fixture.path().exists());
        Ok(())
    }

    #[test]
    fn exclusive_setup_context_authorizes_setup_and_read_only_stays_available(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("mutation-context-exclusive-setup")?;
        with_test_runtime_home_setup(fixture.path(), |context| {
            assert_eq!(context.mode(), RuntimeHomeMutationLeaseMode::ExclusiveSetup);
            initialize_runtime_home(
                context,
                fixture.path(),
                "runtime_home_exclusive_setup",
                "{}",
            )?;
            write_installation_profile(
                context,
                InstallationProfileRegistration {
                    installation_id: "default".to_owned(),
                    volicord_command: "volicord".to_owned(),
                    volicord_mcp_command: "volicord".to_owned(),
                    bin_dir: fixture.path().join("bin"),
                    default_connection_mode: "workflow".to_owned(),
                    metadata_json: "{}".to_owned(),
                },
            )?;
            Ok(())
        })?;
        let exclusive = TestRuntimeHomeAdmission::exclusive(fixture.path())?;

        assert_eq!(
            runtime_home_record_read_only(fixture.path())?
                .expect("published Runtime Home")
                .runtime_home_id,
            "runtime_home_exclusive_setup"
        );
        assert_eq!(
            exclusive.context()?.mode(),
            RuntimeHomeMutationLeaseMode::ExclusiveSetup
        );
        assert!(installation_profile_read_only(fixture.path())?.is_some());
        Ok(())
    }

    #[test]
    fn setup_busy_condition_is_stable_bounded_and_retryable(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("mutation-context-setup-busy")?;
        let _exclusive = TestRuntimeHomeAdmission::exclusive(fixture.path())?;
        let outcome = RuntimeHomeMutationLease::acquire(
            fixture.path(),
            RuntimeHomeMutationLeaseMode::SharedWriter,
            RuntimeHomeMutationWaitPolicy::Immediate,
        )?;
        let RuntimeHomeMutationLeaseOutcome::Busy(busy) = outcome else {
            panic!("ordinary writer must be rejected while setup is exclusive");
        };
        let condition = RuntimeHomeMutationSetupInProgress::from_busy("store.test_writer", busy);

        assert_eq!(condition.code(), RUNTIME_HOME_MUTATION_SETUP_IN_PROGRESS);
        assert_eq!(condition.mutation_domain(), "store.test_writer");
        assert_eq!(
            condition.requested_mode(),
            RuntimeHomeMutationLeaseMode::SharedWriter
        );
        assert_eq!(
            condition.wait_policy(),
            RuntimeHomeMutationWaitPolicy::Immediate
        );
        assert!(condition.retryable());
        assert!(condition.elapsed() < Duration::from_secs(1));
        assert!(!condition.to_string().contains(".lock"));
        Ok(())
    }
}
