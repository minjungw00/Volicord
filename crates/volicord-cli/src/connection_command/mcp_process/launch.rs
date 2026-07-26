use std::{collections::BTreeMap, path::Path};

use volicord_mcp::{
    ManagedMcpInvocationPurpose, ManagedMcpLaunchSpec, ManagedMcpMaterializationInput,
    ManagedMcpWorkingDirectory, MaterializedManagedMcpLaunch, VOLICORD_HOME_ENV,
};

pub(in crate::connection_command) fn materialize_connection_invocation(
    launch: &ManagedMcpLaunchSpec,
    runtime_home: &Path,
    repo_root: &Path,
    purpose: ManagedMcpInvocationPurpose,
) -> Result<MaterializedManagedMcpLaunch, volicord_mcp::ManagedMcpLaunchError> {
    let mut forwarded_environment = BTreeMap::new();
    if launch
        .environment()
        .forwarded_names()
        .contains(VOLICORD_HOME_ENV)
    {
        forwarded_environment.insert(
            VOLICORD_HOME_ENV.to_owned(),
            runtime_home.as_os_str().to_owned(),
        );
    }
    let working_directory = match launch.host_scope() {
        volicord_types::host_configuration::HostScope::User => {
            ManagedMcpWorkingDirectory::Inherited
        }
        volicord_types::host_configuration::HostScope::Project => {
            ManagedMcpWorkingDirectory::ProductRepository(repo_root.to_path_buf())
        }
    };
    launch.materialize(ManagedMcpMaterializationInput::new(
        purpose,
        forwarded_environment,
        working_directory,
    ))
}

#[cfg(test)]
mod tests {
    use std::{ffi::OsString, path::PathBuf};

    use super::*;

    #[test]
    fn shared_verification_materializes_selected_runtime_home_and_repository() {
        let launch =
            ManagedMcpLaunchSpec::shared_repository(volicord_types::values::HostKind::Codex)
                .expect("shared launch");
        let materialized = materialize_connection_invocation(
            &launch,
            Path::new("/selected/runtime-home"),
            Path::new("/workspace/product"),
            ManagedMcpInvocationPurpose::CliStdioHandshake,
        )
        .expect("shared verification launch");
        assert_eq!(
            materialized.environment().get(VOLICORD_HOME_ENV),
            Some(&OsString::from("/selected/runtime-home"))
        );
        assert_eq!(
            materialized.working_directory(),
            &ManagedMcpWorkingDirectory::ProductRepository(PathBuf::from("/workspace/product"))
        );
    }

    #[test]
    fn personal_verification_uses_static_runtime_home_and_repository_independent_cwd() {
        let launch = ManagedMcpLaunchSpec::personal(
            Path::new("/opt/volicord"),
            Path::new("/contract/runtime-home"),
            "connection_alpha",
        )
        .expect("personal launch");
        let materialized = materialize_connection_invocation(
            &launch,
            Path::new("/decoy/selected-runtime-home"),
            Path::new("/workspace/product"),
            ManagedMcpInvocationPurpose::CliStdioHandshake,
        )
        .expect("personal verification launch");
        assert_eq!(
            materialized.environment().get(VOLICORD_HOME_ENV),
            Some(&OsString::from("/contract/runtime-home"))
        );
        assert_eq!(
            materialized.working_directory(),
            &ManagedMcpWorkingDirectory::Inherited
        );
    }
}
