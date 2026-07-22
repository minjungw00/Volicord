//! Typed scope and subject identities for operational diagnostics.

use std::{
    ffi::OsString,
    fs,
    path::{Component, Path, PathBuf},
};

use sha2::{Digest, Sha256};
use volicord_types::{
    DiagnosticScope, DiagnosticScopeKind, DiagnosticSubject, GuardHookPhase, GuardManagedArtifact,
};

const SUBJECT_IDENTITY_DOMAIN: &[u8] = b"volicord.operational-diagnostic-subject";
const SUBJECT_IDENTITY_VERSION: u16 = 1;

mod sealed {
    pub trait Sealed {}
}

/// Scope, canonical identity, and safe report projection owned by a typed subject.
pub trait OperationalSubject: sealed::Sealed {
    fn scope(&self) -> &DiagnosticScope;
    fn canonical_identity_bytes(&self) -> &[u8];
    fn safe_display_subject(&self) -> &DiagnosticSubject;
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SubjectIdentity {
    scope: DiagnosticScope,
    canonical_identity_bytes: Vec<u8>,
    safe_display_subject: DiagnosticSubject,
}

impl SubjectIdentity {
    fn opaque_path(
        connection_id: &str,
        kind: &'static str,
        identity_prefix: &[u8],
        display_prefix: &str,
        path: &Path,
    ) -> Result<Self, String> {
        let canonical = canonical_identity_path(path)?;
        let mut identity = identity_prefix.to_vec();
        push_length_prefixed(&mut identity, canonical.as_os_str().as_encoded_bytes());
        let canonical_identity_bytes = canonical_subject_bytes(kind, &identity);
        let reference = format!(
            "{display_prefix}.sha256:{}",
            lowercase_sha256(&canonical_identity_bytes)
        );
        Self::new(connection_id, kind, canonical_identity_bytes, reference)
    }

    fn stable_reference(
        connection_id: &str,
        kind: &'static str,
        identity_prefix: &[u8],
        reference: &str,
    ) -> Result<Self, String> {
        let mut identity = identity_prefix.to_vec();
        push_length_prefixed(&mut identity, reference.as_bytes());
        Self::new(
            connection_id,
            kind,
            canonical_subject_bytes(kind, &identity),
            reference,
        )
    }

    fn new(
        connection_id: &str,
        kind: &'static str,
        canonical_identity_bytes: Vec<u8>,
        safe_reference: impl Into<String>,
    ) -> Result<Self, String> {
        Ok(Self {
            scope: DiagnosticScope::try_new(DiagnosticScopeKind::Connection, connection_id)
                .map_err(|error| error.to_string())?,
            canonical_identity_bytes,
            safe_display_subject: DiagnosticSubject::try_new(kind, safe_reference)
                .map_err(|error| error.to_string())?,
        })
    }

    fn installation(reference: &str) -> Result<Self, String> {
        let identity = canonical_subject_bytes("installation", reference.as_bytes());
        Ok(Self {
            scope: DiagnosticScope::try_new(DiagnosticScopeKind::Installation, reference)
                .map_err(|error| error.to_string())?,
            canonical_identity_bytes: identity,
            safe_display_subject: DiagnosticSubject::try_new("installation", reference)
                .map_err(|error| error.to_string())?,
        })
    }
}

macro_rules! typed_subject {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct $name(SubjectIdentity);

        impl sealed::Sealed for $name {}

        impl OperationalSubject for $name {
            fn scope(&self) -> &DiagnosticScope {
                &self.0.scope
            }

            fn canonical_identity_bytes(&self) -> &[u8] {
                &self.0.canonical_identity_bytes
            }

            fn safe_display_subject(&self) -> &DiagnosticSubject {
                &self.0.safe_display_subject
            }
        }
    };
}

typed_subject!(ManagedConfigurationTarget);
typed_subject!(ProductRepositorySubject);
typed_subject!(GuardManagedArtifactSubject);
typed_subject!(GuardPhaseSubject);
typed_subject!(GuardInstallationSubject);
typed_subject!(GuardEventSubject);
typed_subject!(IntegrationRevisionSubject);
typed_subject!(VerificationToolSubject);
typed_subject!(InstallationSubject);
typed_subject!(TrustSubject);

impl ManagedConfigurationTarget {
    pub fn for_connection(connection_id: &str, path: impl AsRef<Path>) -> Result<Self, String> {
        SubjectIdentity::opaque_path(
            connection_id,
            "managed_config_target",
            b"managed_configuration_target",
            "managed_config_target",
            path.as_ref(),
        )
        .map(Self)
    }
}

impl ProductRepositorySubject {
    pub fn for_connection(connection_id: &str, path: impl AsRef<Path>) -> Result<Self, String> {
        SubjectIdentity::opaque_path(
            connection_id,
            "product_repository",
            b"product_repository",
            "product_repository",
            path.as_ref(),
        )
        .map(Self)
    }
}

impl GuardManagedArtifactSubject {
    pub fn for_connection(
        connection_id: &str,
        artifact: GuardManagedArtifact,
        path: impl AsRef<Path>,
    ) -> Result<Self, String> {
        let artifact_kind = guard_artifact_kind(artifact);
        SubjectIdentity::opaque_path(
            connection_id,
            "guard_managed_artifact",
            artifact_kind.as_bytes(),
            &format!("guard_managed_artifact.{artifact_kind}"),
            path.as_ref(),
        )
        .map(Self)
    }
}

impl GuardPhaseSubject {
    pub fn for_connection(connection_id: &str, phase: GuardHookPhase) -> Result<Self, String> {
        SubjectIdentity::stable_reference(
            connection_id,
            "guard_phase",
            b"guard_phase",
            phase.as_str(),
        )
        .map(Self)
    }
}

impl GuardInstallationSubject {
    pub fn for_connection(connection_id: &str, installation_id: &str) -> Result<Self, String> {
        SubjectIdentity::stable_reference(
            connection_id,
            "guard_installation",
            b"guard_installation",
            installation_id,
        )
        .map(Self)
    }

    pub fn inventory_for_connection(connection_id: &str) -> Result<Self, String> {
        SubjectIdentity::stable_reference(
            connection_id,
            "guard_installation",
            b"guard_installation_inventory",
            connection_id,
        )
        .map(Self)
    }
}

impl GuardEventSubject {
    pub fn for_connection(connection_id: &str, event_id: &str) -> Result<Self, String> {
        SubjectIdentity::stable_reference(connection_id, "guard_event", b"guard_event", event_id)
            .map(Self)
    }
}

impl IntegrationRevisionSubject {
    pub fn for_runtime_session(
        connection_id: &str,
        runtime_session_id: &str,
    ) -> Result<Self, String> {
        SubjectIdentity::stable_reference(
            connection_id,
            "integration_revision",
            b"runtime_session",
            runtime_session_id,
        )
        .map(Self)
    }

    pub fn for_guard_installation(
        connection_id: &str,
        installation_id: &str,
    ) -> Result<Self, String> {
        SubjectIdentity::stable_reference(
            connection_id,
            "integration_revision",
            b"guard_installation",
            installation_id,
        )
        .map(Self)
    }
}

impl VerificationToolSubject {
    pub fn for_runtime_session(
        connection_id: &str,
        runtime_session_id: &str,
    ) -> Result<Self, String> {
        SubjectIdentity::stable_reference(
            connection_id,
            "verification_tool",
            b"runtime_session",
            runtime_session_id,
        )
        .map(Self)
    }
}

impl InstallationSubject {
    pub fn current() -> Result<Self, String> {
        SubjectIdentity::installation("current").map(Self)
    }
}

impl TrustSubject {
    pub fn for_repository(
        connection_id: &str,
        repository_path: impl AsRef<Path>,
    ) -> Result<Self, String> {
        ProductRepositorySubject::for_connection(connection_id, repository_path)
            .map(|repository| Self(repository.0))
    }
}

pub(crate) fn guard_artifact_kind(artifact: GuardManagedArtifact) -> String {
    match artifact {
        GuardManagedArtifact::HostHookWrapper(phase) => {
            format!("host_hook_wrapper:{}", phase.as_str())
        }
        artifact => artifact.kind().as_str().to_owned(),
    }
}

fn canonical_subject_bytes(kind: &str, identity: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    push_length_prefixed(&mut bytes, SUBJECT_IDENTITY_DOMAIN);
    bytes.extend_from_slice(&SUBJECT_IDENTITY_VERSION.to_be_bytes());
    push_length_prefixed(&mut bytes, kind.as_bytes());
    push_length_prefixed(&mut bytes, identity);
    bytes
}

fn push_length_prefixed(output: &mut Vec<u8>, value: &[u8]) {
    output.extend_from_slice(&(value.len() as u64).to_be_bytes());
    output.extend_from_slice(value);
}

fn lowercase_sha256(value: &[u8]) -> String {
    Sha256::digest(value)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn canonical_identity_path(path: &Path) -> Result<PathBuf, String> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| format!("could not resolve current directory: {error}"))?
            .join(path)
    };
    let normalized = lexical_normalize(&absolute)?;
    if let Ok(canonical) = fs::canonicalize(&normalized) {
        return lexical_normalize(&canonical);
    }

    let mut ancestor = normalized.clone();
    let mut tail = Vec::<OsString>::new();
    while !ancestor.exists() {
        let Some(name) = ancestor.file_name().map(ToOwned::to_owned) else {
            break;
        };
        tail.push(name);
        if !ancestor.pop() {
            break;
        }
    }
    let mut canonical = fs::canonicalize(&ancestor).unwrap_or(ancestor);
    for component in tail.into_iter().rev() {
        canonical.push(component);
    }
    lexical_normalize(&canonical)
}

fn lexical_normalize(path: &Path) -> Result<PathBuf, String> {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    return Err(format!(
                        "path cannot be normalized above its root: {}",
                        path.display()
                    ));
                }
            }
            Component::Normal(value) => normalized.push(value),
        }
    }
    if !normalized.is_absolute() {
        return Err(format!(
            "operational subject path is not absolute after normalization: {}",
            path.display()
        ));
    }
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use volicord_types::GuardHookPhase;

    use super::*;

    #[test]
    fn canonical_path_aliases_have_one_identity_and_safe_projection() {
        let canonical = ManagedConfigurationTarget::for_connection(
            "connection_subject",
            "/tmp/volicord-subject/config.toml",
        )
        .expect("canonical subject");
        let alias = ManagedConfigurationTarget::for_connection(
            "connection_subject",
            "/tmp/volicord-subject/child/.././config.toml",
        )
        .expect("alias subject");

        assert_eq!(
            canonical.canonical_identity_bytes(),
            alias.canonical_identity_bytes()
        );
        assert_eq!(
            canonical.safe_display_subject(),
            alias.safe_display_subject()
        );
        assert!(!canonical
            .safe_display_subject()
            .reference()
            .contains("/tmp/"));
    }

    #[test]
    fn guard_phases_keep_closed_distinct_identities() {
        let pre = GuardPhaseSubject::for_connection("connection_subject", GuardHookPhase::PreTool)
            .expect("pre-tool subject");
        let post =
            GuardPhaseSubject::for_connection("connection_subject", GuardHookPhase::PostTool)
                .expect("post-tool subject");

        assert_ne!(
            pre.canonical_identity_bytes(),
            post.canonical_identity_bytes()
        );
        assert_ne!(pre.safe_display_subject(), post.safe_display_subject());
    }
}
