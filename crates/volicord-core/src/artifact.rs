use std::path::{Component, Path};
use volicord_types::canonical::canonical_git_object_id;
use volicord_types::ids::{ArtifactId, ProjectId, StorageRef, TaskId};
use volicord_types::schema::{ArtifactRef, SourceRef, ToolEnvelope};
use volicord_types::values::{ArtifactAvailability, ArtifactIntegrityStatus, StateRecordKind};

use volicord_store::{
    artifacts::{PersistentArtifactStatus, PersistentArtifactVerificationStatus},
    core_pipeline::{CoreProjectStore, ProjectStateHeader, StoredArtifactRecord},
};

use crate::pipeline::{CorePipelineError, CoreResult};
use crate::record_refs::state_ref;

#[derive(Debug)]
pub(crate) enum ArtifactPolicyError {
    Core(CorePipelineError),
    Validation {
        field: &'static str,
        message: &'static str,
    },
}

impl From<CorePipelineError> for ArtifactPolicyError {
    fn from(error: CorePipelineError) -> Self {
        Self::Core(error)
    }
}

pub(crate) fn artifact_ref_from_verified_record(
    store: &CoreProjectStore,
    record: &StoredArtifactRecord,
    display_name: Option<String>,
    created_by_run_state_version: Option<u64>,
) -> CoreResult<ArtifactRef> {
    let verification_status = persistent_artifact_verification_status(store, record)?;
    let task_id = TaskId::new(record.task_id.clone());
    let integrity_status = effective_artifact_integrity_status(record, verification_status)?;
    Ok(ArtifactRef {
        artifact_id: ArtifactId::new(record.artifact_id.clone()),
        project_id: ProjectId::new(record.project_id.clone()),
        task_id: task_id.clone(),
        display_name: display_name
            .or_else(|| record.producer.display_name.clone())
            .unwrap_or_else(|| record.artifact_id.clone()),
        content_type: sanitized_artifact_content_type(record, integrity_status).into(),
        sha256: sanitized_artifact_sha256(record, integrity_status).into(),
        size_bytes: record.size_bytes.into(),
        integrity_status,
        redaction_state: record.redaction_state,
        availability: artifact_availability_for_verification_status(record, verification_status)?,
        created_by_run_ref: Some(state_ref(
            StateRecordKind::Run,
            record.provenance.producer_run_id.as_str(),
            &ProjectId::new(record.project_id.clone()),
            Some(&task_id),
            created_by_run_state_version,
        ))
        .into(),
        created_by_actor_source: Some(record.producer.created_by_actor_source.clone()).into(),
        storage_ref: Some(StorageRef::new(record.uri.clone())).into(),
    })
}

pub(crate) fn persistent_artifact_is_verified_current(
    store: &CoreProjectStore,
    record: &StoredArtifactRecord,
) -> CoreResult<bool> {
    Ok(persistent_artifact_verification_status(store, record)?
        == PersistentArtifactVerificationStatus::VerifiedCurrent)
}

pub(crate) fn persistent_artifact_verification_status(
    store: &CoreProjectStore,
    record: &StoredArtifactRecord,
) -> CoreResult<PersistentArtifactVerificationStatus> {
    store
        .verify_persistent_artifact_body(record)
        .map(|verification| verification.status)
        .map_err(CorePipelineError::from)
}

pub(crate) fn artifact_availability_for_verification_status(
    record: &StoredArtifactRecord,
    verification_status: PersistentArtifactVerificationStatus,
) -> CoreResult<ArtifactAvailability> {
    match verification_status {
        PersistentArtifactVerificationStatus::VerifiedCurrent => {
            Ok(ArtifactAvailability::Available)
        }
        PersistentArtifactVerificationStatus::Missing => Ok(ArtifactAvailability::Missing),
        PersistentArtifactVerificationStatus::IntegrityFailed => {
            Ok(ArtifactAvailability::IntegrityFailed)
        }
        PersistentArtifactVerificationStatus::Unavailable => match record.status {
            PersistentArtifactStatus::Missing => Ok(ArtifactAvailability::Missing),
            PersistentArtifactStatus::IntegrityFailed => Ok(ArtifactAvailability::IntegrityFailed),
            PersistentArtifactStatus::Available | PersistentArtifactStatus::Unavailable => {
                Ok(ArtifactAvailability::Unavailable)
            }
        },
        PersistentArtifactVerificationStatus::BoundaryViolation => {
            Ok(ArtifactAvailability::Unusable)
        }
    }
}

pub(crate) fn effective_artifact_integrity_status(
    record: &StoredArtifactRecord,
    verification_status: PersistentArtifactVerificationStatus,
) -> CoreResult<ArtifactIntegrityStatus> {
    match verification_status {
        PersistentArtifactVerificationStatus::VerifiedCurrent => {
            Ok(ArtifactIntegrityStatus::Verified)
        }
        PersistentArtifactVerificationStatus::IntegrityFailed
        | PersistentArtifactVerificationStatus::BoundaryViolation => {
            Ok(ArtifactIntegrityStatus::Corrupt)
        }
        PersistentArtifactVerificationStatus::Missing
        | PersistentArtifactVerificationStatus::Unavailable => Ok(record.integrity_status),
    }
}

pub(crate) fn sanitized_artifact_content_type(
    record: &StoredArtifactRecord,
    integrity_status: ArtifactIntegrityStatus,
) -> Option<String> {
    match integrity_status {
        ArtifactIntegrityStatus::Verified => record.content_type.clone(),
        ArtifactIntegrityStatus::Corrupt => record
            .content_type
            .as_ref()
            .filter(|value| !value.trim().is_empty())
            .cloned(),
    }
}

pub(crate) fn sanitized_artifact_sha256(
    record: &StoredArtifactRecord,
    integrity_status: ArtifactIntegrityStatus,
) -> Option<String> {
    match integrity_status {
        ArtifactIntegrityStatus::Verified => record.sha256.clone(),
        ArtifactIntegrityStatus::Corrupt => record
            .sha256
            .as_ref()
            .filter(|value| artifact_sha256_is_lowercase_hex(value))
            .cloned(),
    }
}

pub(crate) fn artifact_sha256_is_lowercase_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

pub(crate) fn normalize_source_refs(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    envelope: &ToolEnvelope,
    task_id: &TaskId,
    field: &'static str,
    refs: &[SourceRef],
) -> Result<Vec<SourceRef>, ArtifactPolicyError> {
    normalize_source_refs_with_carried_artifact_task(
        store,
        project_state,
        envelope,
        task_id,
        field,
        refs,
        None,
    )
}

pub(crate) fn normalize_source_refs_with_carried_artifact_task(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    envelope: &ToolEnvelope,
    task_id: &TaskId,
    field: &'static str,
    refs: &[SourceRef],
    carried_artifact_task_id: Option<&TaskId>,
) -> Result<Vec<SourceRef>, ArtifactPolicyError> {
    refs.iter()
        .cloned()
        .map(|source_ref| {
            normalize_source_ref(
                store,
                project_state,
                envelope,
                task_id,
                field,
                source_ref,
                carried_artifact_task_id,
            )
        })
        .collect()
}

pub(crate) fn normalize_source_ref(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    envelope: &ToolEnvelope,
    task_id: &TaskId,
    field: &'static str,
    source_ref: SourceRef,
    carried_artifact_task_id: Option<&TaskId>,
) -> Result<SourceRef, ArtifactPolicyError> {
    match source_ref {
        SourceRef::RepositoryFile(mut source) => {
            source.repository_path = match normalize_source_repository_path(&source.repository_path)
            {
                Some(path) => path,
                None => {
                    return source_ref_error(
                        envelope,
                        project_state,
                        field,
                        "repository_path must be a normalized Product Repository relative path",
                    )
                }
            };
            source.baseline_commit_sha = match canonical_git_object_id(&source.baseline_commit_sha)
            {
                Ok(value) => value,
                Err(_) => {
                    return source_ref_error(
                        envelope,
                        project_state,
                        field,
                        "Git object ids must be exactly 40 or 64 ASCII hexadecimal characters",
                    )
                }
            };
            if !artifact_sha256_is_lowercase_hex(&source.content_sha256) {
                return source_ref_error(
                    envelope,
                    project_state,
                    field,
                    "content_sha256 must be a lowercase 64-character SHA-256 hex string",
                );
            }
            if source
                .line_range
                .as_ref()
                .is_some_and(|range| range.start_line == 0 || range.end_line < range.start_line)
            {
                return source_ref_error(
                    envelope,
                    project_state,
                    field,
                    "line_range must be one-based, inclusive, and ordered",
                );
            }
            Ok(SourceRef::RepositoryFile(source))
        }
        SourceRef::GitCommit(mut source) => {
            source.commit_sha = match canonical_git_object_id(&source.commit_sha) {
                Ok(value) => value,
                Err(_) => {
                    return source_ref_error(
                        envelope,
                        project_state,
                        field,
                        "Git object ids must be exactly 40 or 64 ASCII hexadecimal characters",
                    )
                }
            };
            Ok(SourceRef::GitCommit(source))
        }
        SourceRef::GitDiff(mut source) => {
            source.base_commit_sha = match canonical_git_object_id(&source.base_commit_sha) {
                Ok(value) => value,
                Err(_) => {
                    return source_ref_error(
                        envelope,
                        project_state,
                        field,
                        "Git object ids must be exactly 40 or 64 ASCII hexadecimal characters",
                    )
                }
            };
            source.head_commit_sha = match canonical_git_object_id(&source.head_commit_sha) {
                Ok(value) => value,
                Err(_) => {
                    return source_ref_error(
                        envelope,
                        project_state,
                        field,
                        "Git object ids must be exactly 40 or 64 ASCII hexadecimal characters",
                    )
                }
            };
            if let Some(artifact_ref) = source.diff_artifact_ref.as_ref() {
                source.diff_artifact_ref = Some(canonical_source_artifact_ref(
                    store,
                    project_state,
                    envelope,
                    task_id,
                    field,
                    artifact_ref,
                    carried_artifact_task_id,
                )?)
                .into();
            }
            Ok(SourceRef::GitDiff(source))
        }
        SourceRef::Command(mut source) => {
            if source.invocation_id.trim().is_empty() || source.command_summary.trim().is_empty() {
                return source_ref_error(
                    envelope,
                    project_state,
                    field,
                    "command source identifiers and summaries must not be empty",
                );
            }
            source.command_summary = source
                .command_summary
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            if let Some(artifact_ref) = source.output_artifact_ref.as_ref() {
                source.output_artifact_ref = Some(canonical_source_artifact_ref(
                    store,
                    project_state,
                    envelope,
                    task_id,
                    field,
                    artifact_ref,
                    carried_artifact_task_id,
                )?)
                .into();
            }
            Ok(SourceRef::Command(source))
        }
        SourceRef::ExternalUri(source) => {
            if !external_source_uri_is_valid(&source.uri) {
                return source_ref_error(
                    envelope,
                    project_state,
                    field,
                    "external_uri must be an absolute http or https URI without user information",
                );
            }
            if !artifact_sha256_is_lowercase_hex(&source.content_sha256) {
                return source_ref_error(
                    envelope,
                    project_state,
                    field,
                    "content_sha256 must be a lowercase 64-character SHA-256 hex string",
                );
            }
            Ok(SourceRef::ExternalUri(source))
        }
        SourceRef::UserContext(source) => {
            if source.context_id.trim().is_empty() {
                return source_ref_error(
                    envelope,
                    project_state,
                    field,
                    "user context ids must not be empty",
                );
            }
            Ok(SourceRef::UserContext(source))
        }
    }
}

pub(crate) fn source_ref_error<T>(
    _envelope: &ToolEnvelope,
    _project_state: &ProjectStateHeader,
    field: &'static str,
    message: &'static str,
) -> Result<T, ArtifactPolicyError> {
    Err(ArtifactPolicyError::Validation { field, message })
}

pub(crate) fn normalize_source_repository_path(raw: &str) -> Option<String> {
    if raw.trim().is_empty() || raw.contains('\\') || has_windows_drive_prefix(raw) {
        return None;
    }
    let path = Path::new(raw);
    if path.is_absolute() {
        return None;
    }
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                parts.pop()?;
            }
            Component::Normal(value) => parts.push(value.to_str()?.to_owned()),
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    (!parts.is_empty()).then(|| parts.join("/"))
}

pub(crate) fn has_windows_drive_prefix(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

pub(crate) fn external_source_uri_is_valid(value: &str) -> bool {
    if value
        .bytes()
        .any(|byte| byte.is_ascii_whitespace() || byte.is_ascii_control())
    {
        return false;
    }
    let Some(rest) = value
        .strip_prefix("https://")
        .or_else(|| value.strip_prefix("http://"))
    else {
        return false;
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    !authority.is_empty() && !authority.contains('@')
}

pub(crate) fn canonical_source_artifact_ref(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    envelope: &ToolEnvelope,
    task_id: &TaskId,
    field: &'static str,
    submitted: &ArtifactRef,
    carried_artifact_task_id: Option<&TaskId>,
) -> Result<ArtifactRef, ArtifactPolicyError> {
    let artifact_task_id = if submitted.task_id == *task_id {
        task_id
    } else if carried_artifact_task_id == Some(&submitted.task_id) {
        carried_artifact_task_id.expect("matched carried artifact Task")
    } else {
        return source_ref_error(
            envelope,
            project_state,
            field,
            "source artifact refs must belong to the request Task or the explicitly carried predecessor Task",
        );
    };
    if submitted.project_id != envelope.project_id {
        return source_ref_error(
            envelope,
            project_state,
            field,
            "source artifact refs must belong to the request project",
        );
    }
    let record = store
        .artifact_record(submitted.artifact_id.as_str())
        .map_err(CorePipelineError::from)?;
    let Some(record) = record else {
        return source_ref_error(
            envelope,
            project_state,
            field,
            "source artifact refs must identify an existing artifact",
        );
    };
    let owner_link = store
        .artifact_has_task_owner_link(submitted.artifact_id.as_str(), artifact_task_id.as_str())
        .map_err(CorePipelineError::from)?;
    if record.project_id != envelope.project_id.as_str()
        || record.task_id != artifact_task_id.as_str()
        || !owner_link
    {
        return source_ref_error(
            envelope,
            project_state,
            field,
            "source artifact refs must identify an artifact owned by the request project and Task",
        );
    }
    let integrity_status = record.integrity_status;
    let availability = match record.status {
        PersistentArtifactStatus::Available => ArtifactAvailability::Available,
        PersistentArtifactStatus::Missing => ArtifactAvailability::Missing,
        PersistentArtifactStatus::IntegrityFailed => ArtifactAvailability::IntegrityFailed,
        PersistentArtifactStatus::Unavailable => ArtifactAvailability::Unavailable,
    };
    Ok(ArtifactRef {
        artifact_id: ArtifactId::new(record.artifact_id.clone()),
        project_id: envelope.project_id.clone(),
        task_id: artifact_task_id.clone(),
        display_name: record
            .producer
            .display_name
            .clone()
            .unwrap_or_else(|| record.artifact_id.clone()),
        content_type: record.content_type.clone().into(),
        sha256: record.sha256.clone().into(),
        size_bytes: record.size_bytes.into(),
        integrity_status,
        redaction_state: record.redaction_state,
        availability,
        created_by_run_ref: Some(state_ref(
            StateRecordKind::Run,
            record.provenance.producer_run_id.as_str(),
            &envelope.project_id,
            Some(artifact_task_id),
            Some(project_state.state_version),
        ))
        .into(),
        created_by_actor_source: Some(record.producer.created_by_actor_source.clone()).into(),
        storage_ref: Some(StorageRef::new(record.uri)).into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_repository_paths_normalize_only_within_the_product_repository() {
        assert_eq!(
            normalize_source_repository_path("./src/feature/../lib.rs"),
            Some("src/lib.rs".to_owned())
        );
        assert_eq!(normalize_source_repository_path("../outside.rs"), None);
        assert_eq!(normalize_source_repository_path("/absolute.rs"), None);
        assert_eq!(normalize_source_repository_path("C:\\outside.rs"), None);
    }

    #[test]
    fn external_sources_require_bounded_http_identity_without_credentials() {
        assert!(external_source_uri_is_valid(
            "https://example.test/evidence?id=1"
        ));
        assert!(!external_source_uri_is_valid(
            "https://user@example.test/evidence"
        ));
        assert!(!external_source_uri_is_valid("file:///tmp/evidence"));
        assert!(!external_source_uri_is_valid(
            "https://example.test/evidence value"
        ));
    }

    #[test]
    fn artifact_sha256_accepts_only_lowercase_full_length_hex() {
        assert!(artifact_sha256_is_lowercase_hex(&"a".repeat(64)));
        assert!(!artifact_sha256_is_lowercase_hex(&"A".repeat(64)));
        assert!(!artifact_sha256_is_lowercase_hex(&"a".repeat(63)));
        assert!(!artifact_sha256_is_lowercase_hex(&"g".repeat(64)));
    }
}
