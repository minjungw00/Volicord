use std::{error::Error, fmt, path::Path};

use volicord_platform_fs::{
    RuntimeHomeMutationLease, RuntimeHomeMutationLeaseMode, RuntimeHomeMutationLeaseOutcome,
    RuntimeHomeMutationWaitPolicy,
};
use volicord_store::{RuntimeHomeMutationContext, RuntimeHomeMutationSetupInProgress, StoreError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliMutationAdmissionError {
    SetupInProgress(RuntimeHomeMutationSetupInProgress),
    Acquisition {
        mutation_domain: &'static str,
        stage: &'static str,
        detail: String,
    },
    Operation(String),
}

impl fmt::Display for CliMutationAdmissionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SetupInProgress(condition) => write!(formatter, "{condition}"),
            Self::Acquisition {
                mutation_domain,
                stage,
                detail,
            } => write!(
                formatter,
                "Runtime Home mutation admission failed for {mutation_domain} during {stage}: {detail}"
            ),
            Self::Operation(message) => formatter.write_str(message),
        }
    }
}

impl Error for CliMutationAdmissionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::SetupInProgress(condition) => Some(condition),
            Self::Acquisition { .. } => None,
            Self::Operation(_) => None,
        }
    }
}

pub fn with_cli_runtime_home_mutation<T>(
    runtime_home: &Path,
    mutation_domain: &'static str,
    operation: impl FnOnce(&RuntimeHomeMutationContext<'_>) -> Result<T, CliMutationAdmissionError>,
) -> Result<T, CliMutationAdmissionError> {
    with_cli_runtime_home_mutation_result(runtime_home, mutation_domain, operation)?
}

pub fn with_cli_runtime_home_mutation_result<T, E>(
    runtime_home: &Path,
    mutation_domain: &'static str,
    operation: impl FnOnce(&RuntimeHomeMutationContext<'_>) -> Result<T, E>,
) -> Result<Result<T, E>, CliMutationAdmissionError> {
    let outcome = RuntimeHomeMutationLease::acquire(
        runtime_home,
        RuntimeHomeMutationLeaseMode::SharedWriter,
        RuntimeHomeMutationWaitPolicy::Immediate,
    )
    .map_err(|source| CliMutationAdmissionError::Acquisition {
        mutation_domain,
        stage: source.stage(),
        detail: source.to_string(),
    })?;
    let lease = match outcome {
        RuntimeHomeMutationLeaseOutcome::Acquired(lease) => lease,
        RuntimeHomeMutationLeaseOutcome::Busy(busy) => {
            return Err(CliMutationAdmissionError::SetupInProgress(
                RuntimeHomeMutationSetupInProgress::from_busy(mutation_domain, busy),
            ));
        }
    };
    let context = RuntimeHomeMutationContext::new(lease.permit(), runtime_home)
        .map_err(|error| CliMutationAdmissionError::Operation(error.to_string()))?;
    Ok(operation(&context))
}

impl From<StoreError> for CliMutationAdmissionError {
    fn from(error: StoreError) -> Self {
        Self::Operation(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use volicord_platform_fs::{
        RuntimeHomeMutationLease, RuntimeHomeMutationLeaseMode, RuntimeHomeMutationLeaseOutcome,
        RuntimeHomeMutationWaitPolicy,
    };
    use volicord_store::{
        bootstrap::initialize_runtime_home,
        diagnostics::{
            diagnostics_db_path, start_diagnostic_session, DiagnosticSessionStart,
            DiagnosticTransport,
        },
    };
    use volicord_test_support::{with_test_runtime_home_setup, TempRuntimeHome};

    use super::*;

    #[test]
    fn diagnostic_writer_is_not_invoked_while_setup_is_exclusive_and_succeeds_after_release(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("cli-diagnostic-admission")?;
        with_test_runtime_home_setup(fixture.path(), |context| {
            initialize_runtime_home(
                context,
                fixture.path(),
                "runtime_home_cli_diagnostic_admission",
                "{}",
            )?;
            Ok(())
        })?;
        let outcome = RuntimeHomeMutationLease::acquire(
            fixture.path(),
            RuntimeHomeMutationLeaseMode::ExclusiveSetup,
            RuntimeHomeMutationWaitPolicy::Immediate,
        )?;
        let RuntimeHomeMutationLeaseOutcome::Acquired(exclusive) = outcome else {
            panic!("test setup must acquire ExclusiveSetup");
        };

        let error = with_cli_runtime_home_mutation(
            fixture.path(),
            "cli.diagnostics.persistence",
            |context| {
                start_diagnostic_session(
                    context,
                    DiagnosticSessionStart {
                        session_id: "diagnostic_setup_busy",
                        connection_id: None,
                        project_id: None,
                        transport: DiagnosticTransport::CliInbox,
                        host_kind: None,
                        package_version: "test",
                        build_id: "test",
                    },
                )
                .map_err(Into::into)
            },
        )
        .expect_err("diagnostic writer must be rejected before Store persistence");
        let CliMutationAdmissionError::SetupInProgress(condition) = error else {
            panic!("diagnostic writer must return the typed setup condition");
        };
        assert_eq!(condition.code(), "runtime_home.mutation.setup_in_progress");
        assert_eq!(condition.mutation_domain(), "cli.diagnostics.persistence");
        assert!(!diagnostics_db_path(fixture.path()).exists());
        drop(exclusive);

        with_cli_runtime_home_mutation(fixture.path(), "cli.diagnostics.persistence", |context| {
            start_diagnostic_session(
                context,
                DiagnosticSessionStart {
                    session_id: "diagnostic_after_setup",
                    connection_id: None,
                    project_id: None,
                    transport: DiagnosticTransport::CliInbox,
                    host_kind: None,
                    package_version: "test",
                    build_id: "test",
                },
            )
            .map_err(Into::into)
        })?;
        assert!(diagnostics_db_path(fixture.path()).is_file());
        Ok(())
    }
}
