//! Current managed-host authority validation at MCP startup and call boundaries.

use std::{
    fs::File,
    io::{self, Read},
    path::Path,
    time::SystemTime,
};

use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use volicord_core::{validate_host_verification_receipt, ValidatedHostVerificationReceipt};
use volicord_platform_fs::{observe_local_platform_boundary, observe_parent_process_binding};
use volicord_store::{
    agent_connections::{
        agent_connection_project_access_read_only, agent_connection_record_read_only,
        CONNECTION_INTENT_PERSONAL, CONNECTION_INTENT_SHARED, CONNECTION_MODE_READ_ONLY,
        CONNECTION_MODE_WORKFLOW, HOST_KIND_CODEX, HOST_SCOPE_PROJECT, HOST_SCOPE_USER,
    },
    core_pipeline::CoreProjectStore,
    managed_host_authority::managed_host_authority_read_only,
};
use volicord_types::{
    generated_managed_artifacts_digest, lookup_embedded_codex_support_entry, AgentConnectionId,
    ConfigurationTargetOwner, CurrentHostReceiptContext, IntegrationProfile,
    ManagedConnectionScope, ProjectId, UtcTimestamp,
};

use crate::errors::McpAdapterError;

pub(crate) fn validate_current_managed_host_authority(
    runtime_home: &Path,
    connection_internal_id: &str,
    project_internal_id: &str,
) -> Result<ValidatedHostVerificationReceipt, McpAdapterError> {
    let connection = agent_connection_record_read_only(runtime_home, connection_internal_id)
        .map_err(|error| {
            authority_failure(
                error.classification().category,
                format!("current Agent Connection authority is unavailable: {error}"),
            )
        })?
        .ok_or_else(|| {
            authority_failure(
                "host_receipt_connection_missing",
                format!("connection {connection_internal_id} is not registered"),
            )
        })?;
    if !connection.enabled {
        return Err(authority_failure(
            "host_receipt_connection_disabled",
            format!("connection {connection_internal_id} is disabled"),
        ));
    }
    if connection.host_kind != HOST_KIND_CODEX
        || !matches!(
            connection.mode.as_str(),
            CONNECTION_MODE_READ_ONLY | CONNECTION_MODE_WORKFLOW
        )
    {
        return Err(authority_failure(
            "host_receipt_connection_stale",
            "the current Agent Connection host or mode is not supported",
        ));
    }
    let access = agent_connection_project_access_read_only(
        runtime_home,
        connection_internal_id,
        project_internal_id,
    )
    .map_err(|error| {
        authority_failure(
            error.classification().category,
            format!("current connection/project access is unavailable: {error}"),
        )
    })?
    .ok_or_else(|| {
        authority_failure(
            "host_receipt_project_missing",
            format!(
                "connection {connection_internal_id} has no current access record for project {project_internal_id}"
            ),
        )
    })?;
    if !access.connection_enabled || !access.project_allowed || access.project.is_none() {
        return Err(authority_failure(
            "host_receipt_project_stale",
            format!(
                "project {project_internal_id} is not currently allowed for connection {connection_internal_id}"
            ),
        ));
    }

    let authority = managed_host_authority_read_only(
        runtime_home,
        connection_internal_id,
        project_internal_id,
    )
    .map_err(|error| {
        authority_failure(
            error.classification().category,
            format!("current managed-host authority is unavailable: {error}"),
        )
    })?
    .ok_or_else(|| {
        authority_failure(
            "host_receipt_missing",
            format!(
                "connection {connection_internal_id} has no current receipt for project {project_internal_id}"
            ),
        )
    })?;

    let expected_connection = match authority.managed_host_binding.connection_scope {
        ManagedConnectionScope::Personal => (CONNECTION_INTENT_PERSONAL, HOST_SCOPE_USER),
        ManagedConnectionScope::Shared => (CONNECTION_INTENT_SHARED, HOST_SCOPE_PROJECT),
    };
    let expected_target_owner = match connection.host_scope.as_str() {
        HOST_SCOPE_USER => ConfigurationTargetOwner::User,
        HOST_SCOPE_PROJECT => ConfigurationTargetOwner::Project,
        _ => {
            return Err(authority_failure(
                "host_receipt_connection_stale",
                "the current Agent Connection host scope is unknown",
            ))
        }
    };
    if connection.intent != expected_connection.0
        || connection.host_scope != expected_connection.1
        || connection.config_target != authority.managed_host_binding.configuration_target.path
        || authority.managed_host_binding.configuration_target.owner != expected_target_owner
    {
        return Err(authority_failure(
            "host_receipt_connection_stale",
            "the current Agent Connection scope or configuration target changed after verification",
        ));
    }

    let platform = observe_local_platform_boundary()
        .map_err(|error| authority_failure(error.reason(), error.detail().to_owned()))?;
    let parent = observe_parent_process_binding()
        .map_err(|error| authority_failure(error.reason(), error.detail().to_owned()))?;
    if parent != authority.managed_host_binding.process_binding {
        return Err(authority_failure(
            "host_receipt_process_binding_stale",
            "the current MCP parent process does not match the stored managed Codex binding",
        ));
    }
    if platform.environment != authority.managed_host_binding.platform_environment
        || platform.release_coordinate != authority.managed_host_binding.platform_release_coordinate
    {
        return Err(authority_failure(
            "host_receipt_platform_stale",
            "the current platform boundary does not match the stored managed Codex binding",
        ));
    }

    let mut current_artifacts = Vec::with_capacity(authority.generated_artifacts.len());
    for artifact in &authority.generated_artifacts {
        let digest = sha256_regular_file(Path::new(&artifact.path)).map_err(|error| {
            authority_failure(
                "host_receipt_generated_artifact_unavailable",
                format!("managed artifact {} is unavailable: {error}", artifact.path),
            )
        })?;
        if digest != artifact.digest {
            return Err(authority_failure(
                "host_receipt_generated_artifacts_stale",
                format!(
                    "managed artifact {} changed after verification",
                    artifact.path
                ),
            ));
        }
        current_artifacts.push(volicord_types::GeneratedManagedArtifact {
            path: artifact.path.clone(),
            digest,
        });
    }
    let generated_artifacts_digest =
        generated_managed_artifacts_digest(platform.environment, &current_artifacts)
            .map_err(|error| authority_failure(error.reason(), error.to_string()))?;
    if generated_artifacts_digest != authority.generated_artifacts_digest {
        return Err(authority_failure(
            "host_receipt_generated_artifacts_stale",
            "the complete current managed-artifact inventory no longer matches its stored identity",
        ));
    }

    lookup_embedded_codex_support_entry(
        &parent.executable_digest,
        platform.environment,
        &platform.release_coordinate,
        &authority.managed_host_binding.required_capabilities,
        IntegrationProfile::Record,
    )
    .map_err(|error| {
        authority_failure(
            error.reason(),
            "no exact embedded Codex support-catalog entry matches the current parent executable",
        )
    })?;

    let store =
        CoreProjectStore::open_read_only(runtime_home, &ProjectId::new(project_internal_id))
            .map_err(|error| {
                authority_failure("host_receipt_store_unavailable", error.to_string())
            })?;
    let policy = store
        .project_workflow_policy()
        .map_err(|error| authority_failure("host_receipt_policy_unavailable", error.to_string()))?
        .ok_or_else(|| {
            authority_failure(
                "host_receipt_policy_missing",
                "the selected project has no current authoritative workflow policy",
            )
        })?;
    let verifier_path = std::env::current_exe()
        .map_err(|error| authority_failure("verifier_build_unavailable", error.to_string()))?;
    let verifier_build_digest = sha256_regular_file(&verifier_path)
        .map_err(|error| authority_failure("verifier_build_unavailable", error.to_string()))?;

    let current = CurrentHostReceiptContext {
        project_id: ProjectId::new(project_internal_id),
        connection_id: AgentConnectionId::new(connection_internal_id),
        host_kind: authority.managed_host_binding.host_kind,
        integration_profile: IntegrationProfile::Record,
        platform_environment: platform.environment,
        platform_release_coordinate: platform.release_coordinate,
        required_capabilities: authority.managed_host_binding.required_capabilities.clone(),
        binding_digest: authority.binding_digest,
        generated_artifacts_digest,
        executable_digest: parent.executable_digest,
        policy_digest: policy.policy_fingerprint,
        verifier_build_digest,
    };
    let current_time = UtcTimestamp::from_datetime(DateTime::<Utc>::from(SystemTime::now()));
    validate_host_verification_receipt(authority.host_verification_receipt, &current, &current_time)
        .map_err(|error| authority_failure(error.reason(), error.to_string()))
}

fn sha256_regular_file(path: &Path) -> io::Result<String> {
    let metadata = std::fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "path is not a regular non-symlink file",
        ));
    }
    let mut file = File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn authority_failure(reason: &'static str, detail: impl Into<String>) -> McpAdapterError {
    McpAdapterError::Environment(format!("{reason}: {}", detail.into()))
}
