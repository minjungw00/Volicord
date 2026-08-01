use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use volicord_store::{
    agent_connections::{AgentConnectionRecord, ConnectionProjectRecord},
    core_pipeline::CoreProjectStore,
    guards::GuardInstallationRecord,
    inspection::{
        AgentConnectionInspectionRecord, GuardInstallationInspectionRecord, ProjectInspectionRecord,
    },
};
use volicord_types::guard_manifest::{
    guard_manifest_from_json, guard_manifest_matches_owner_binding, GuardCommand,
    GuardCommandInvocation, GuardCommandInvocationSet, GuardCommandSet, GuardManagedArtifact,
    GuardManagedArtifactKind, GuardManifestOwnerBinding, ManagedFileExpectation,
};
use volicord_types::ids::ProjectId;
use volicord_types::integration_revision::{
    ConnectionIntegrationRevisionBasis, IntegrationRevision,
};
use volicord_types::values::{GuardHookPhase, IntegrationProfile};

use crate::host_integration::{
    contracts::{
        contract_for, hook_event_for_phase, validate_contract_config, HostContractConfigKind,
    },
    guard_phase_capability_name, HostKind, MANAGED_WRAPPER_ENV, MANAGED_WRAPPER_VALUE,
};

use super::{
    git_exclude::git_exclude_path,
    hooks::{codex_dispatch_wrapper_script_content, codex_guard_hook_script, shell_word},
    policy::{required_guard_phase_names, validate_policy_schema, validate_workflow_policy},
};

pub(crate) const HOOK_WRAPPER_MARKER: &str = "VOLICORD_MANAGED_HOOK_WRAPPER";

pub(crate) const MAX_HOOK_PATH_SAFETY_EVIDENCE: usize = 16;
const MAX_HOOK_PATH_SAFETY_EVIDENCE_TEXT_CHARS: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum HookPathSafetyState {
    Verified,
    Failed,
    NotRecorded,
    NotChecked,
    NotApplicable,
}

impl HookPathSafetyState {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Verified => "verified",
            Self::Failed => "failed",
            Self::NotRecorded => "not_recorded",
            Self::NotChecked => "not_checked",
            Self::NotApplicable => "not_applicable",
        }
    }

    fn aggregate(left: Self, right: Self) -> Self {
        let states = [left, right];
        if states.contains(&Self::Failed) {
            Self::Failed
        } else if states.contains(&Self::NotRecorded) {
            Self::NotRecorded
        } else if states.contains(&Self::NotChecked) {
            Self::NotChecked
        } else if states.contains(&Self::Verified) {
            Self::Verified
        } else {
            Self::NotApplicable
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum HookPathSafetyEvidenceSource {
    GuardManifest,
    ProjectPolicy,
    HostHookConfig,
    HostHookDispatch,
    HostHookWrapper,
}

impl HookPathSafetyEvidenceSource {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::GuardManifest => "guard_manifest",
            Self::ProjectPolicy => "project_policy",
            Self::HostHookConfig => "host_hook_config",
            Self::HostHookDispatch => "host_hook_dispatch",
            Self::HostHookWrapper => "host_hook_wrapper",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum HookPathSafetyEvidenceReason {
    CurrentContractVerified,
    ManifestMalformed,
    OwnerBindingMismatch,
    RequiredPhaseMissing,
    RequiredArtifactMissing,
    ArtifactUnavailable,
    ArtifactMissing,
    ArtifactMalformed,
    ContentMismatch,
    ManagedCommandBindingMismatch,
    PermissionMismatch,
    NoncanonicalHookCommand,
    NoncanonicalRootResolution,
    NoncanonicalManagedCommand,
    PolicyHashMismatch,
    HostOutputMismatch,
    AuditNotRun,
}

impl HookPathSafetyEvidenceReason {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::CurrentContractVerified => "current_contract_verified",
            Self::ManifestMalformed => "manifest_malformed",
            Self::OwnerBindingMismatch => "owner_binding_mismatch",
            Self::RequiredPhaseMissing => "required_phase_missing",
            Self::RequiredArtifactMissing => "required_artifact_missing",
            Self::ArtifactUnavailable => "artifact_unavailable",
            Self::ArtifactMissing => "artifact_missing",
            Self::ArtifactMalformed => "artifact_malformed",
            Self::ContentMismatch => "content_mismatch",
            Self::ManagedCommandBindingMismatch => "managed_command_binding_mismatch",
            Self::PermissionMismatch => "permission_mismatch",
            Self::NoncanonicalHookCommand => "noncanonical_hook_command",
            Self::NoncanonicalRootResolution => "noncanonical_root_resolution",
            Self::NoncanonicalManagedCommand => "noncanonical_managed_command",
            Self::PolicyHashMismatch => "policy_hash_mismatch",
            Self::HostOutputMismatch => "host_output_mismatch",
            Self::AuditNotRun => "audit_not_run",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct HookPathSafetyEvidence {
    state: HookPathSafetyState,
    source: HookPathSafetyEvidenceSource,
    reason: HookPathSafetyEvidenceReason,
    #[serde(skip_serializing_if = "Option::is_none")]
    installation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    phase: Option<GuardHookPhase>,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
}

impl HookPathSafetyEvidence {
    pub(crate) const fn state(&self) -> HookPathSafetyState {
        self.state
    }

    pub(crate) const fn source(&self) -> HookPathSafetyEvidenceSource {
        self.source
    }

    pub(crate) const fn reason(&self) -> HookPathSafetyEvidenceReason {
        self.reason
    }

    pub(crate) fn installation_id(&self) -> Option<&str> {
        self.installation_id.as_deref()
    }

    pub(crate) const fn phase(&self) -> Option<GuardHookPhase> {
        self.phase
    }

    pub(crate) fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct HookPathSafetyAssessment {
    pub(crate) state: HookPathSafetyState,
    pub(crate) cwd_independence: HookPathSafetyState,
    pub(crate) subdirectory_safety: HookPathSafetyState,
    pub(crate) evidence: Vec<HookPathSafetyEvidence>,
    #[serde(skip)]
    assessment_count: usize,
    #[serde(skip)]
    installation_id: Option<String>,
}

impl Default for HookPathSafetyAssessment {
    fn default() -> Self {
        Self {
            state: HookPathSafetyState::NotChecked,
            cwd_independence: HookPathSafetyState::NotChecked,
            subdirectory_safety: HookPathSafetyState::NotChecked,
            evidence: Vec::new(),
            assessment_count: 0,
            installation_id: None,
        }
    }
}

impl HookPathSafetyAssessment {
    pub(crate) const fn state(&self) -> HookPathSafetyState {
        self.state
    }

    pub(crate) const fn cwd_independence(&self) -> HookPathSafetyState {
        self.cwd_independence
    }

    pub(crate) const fn subdirectory_safety(&self) -> HookPathSafetyState {
        self.subdirectory_safety
    }

    pub(crate) fn evidence(&self) -> &[HookPathSafetyEvidence] {
        &self.evidence
    }

    pub(crate) fn not_checked() -> Self {
        Self::from_state(
            HookPathSafetyState::NotChecked,
            HookPathSafetyEvidenceReason::AuditNotRun,
        )
    }

    #[cfg(test)]
    pub(crate) fn test_state(state: HookPathSafetyState) -> Self {
        Self {
            state,
            cwd_independence: state,
            subdirectory_safety: state,
            evidence: Vec::new(),
            assessment_count: 1,
            installation_id: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn test_failed(reason: HookPathSafetyEvidenceReason) -> Self {
        Self::from_state(HookPathSafetyState::Failed, reason)
    }

    fn from_state(state: HookPathSafetyState, reason: HookPathSafetyEvidenceReason) -> Self {
        Self {
            state,
            cwd_independence: state,
            subdirectory_safety: state,
            evidence: vec![HookPathSafetyEvidence {
                state,
                source: HookPathSafetyEvidenceSource::GuardManifest,
                reason,
                installation_id: None,
                phase: None,
                path: None,
            }],
            assessment_count: 1,
            installation_id: None,
        }
    }

    fn mark_applicable(&mut self, installation_id: Option<&str>) {
        self.state = HookPathSafetyState::NotRecorded;
        self.cwd_independence = HookPathSafetyState::NotRecorded;
        self.subdirectory_safety = HookPathSafetyState::NotRecorded;
        self.assessment_count = 1;
        self.installation_id = installation_id.map(bounded_hook_path_evidence_text);
    }

    fn record(
        &mut self,
        state: HookPathSafetyState,
        source: HookPathSafetyEvidenceSource,
        reason: HookPathSafetyEvidenceReason,
        phase: Option<GuardHookPhase>,
        path: Option<&Path>,
    ) {
        match state {
            HookPathSafetyState::Failed => {
                self.state = HookPathSafetyState::Failed;
                self.cwd_independence = HookPathSafetyState::Failed;
                self.subdirectory_safety = HookPathSafetyState::Failed;
            }
            HookPathSafetyState::NotChecked if self.state != HookPathSafetyState::Failed => {
                self.state = HookPathSafetyState::NotChecked;
                self.cwd_independence = HookPathSafetyState::NotChecked;
                self.subdirectory_safety = HookPathSafetyState::NotChecked;
            }
            HookPathSafetyState::Verified
            | HookPathSafetyState::NotRecorded
            | HookPathSafetyState::NotApplicable
            | HookPathSafetyState::NotChecked => {}
        }
        self.evidence.push(HookPathSafetyEvidence {
            state,
            source,
            reason,
            installation_id: self.installation_id.clone(),
            phase,
            path: path.map(|path| bounded_hook_path_evidence_text(&path.display().to_string())),
        });
    }

    fn mark_verified(&mut self, manifest: &volicord_types::guard_manifest::GuardManifest) {
        if matches!(
            self.state,
            HookPathSafetyState::Failed | HookPathSafetyState::NotChecked
        ) {
            return;
        }
        self.state = HookPathSafetyState::Verified;
        self.cwd_independence = HookPathSafetyState::Verified;
        self.subdirectory_safety = HookPathSafetyState::Verified;
        for (artifact, source) in [
            (
                GuardManagedArtifact::VolicordPolicy,
                HookPathSafetyEvidenceSource::ProjectPolicy,
            ),
            (
                GuardManagedArtifact::HostHookConfig,
                HookPathSafetyEvidenceSource::HostHookConfig,
            ),
            (
                GuardManagedArtifact::HostHookDispatch,
                HookPathSafetyEvidenceSource::HostHookDispatch,
            ),
        ] {
            let path = manifest
                .managed_files
                .iter()
                .find(|file| file.artifact() == artifact)
                .map(ManagedFileExpectation::path);
            self.record(
                HookPathSafetyState::Verified,
                source,
                HookPathSafetyEvidenceReason::CurrentContractVerified,
                None,
                path,
            );
        }
        for phase in GuardHookPhase::REQUIRED {
            let path = manifest
                .managed_files
                .iter()
                .find(|file| file.artifact() == GuardManagedArtifact::HostHookWrapper(phase))
                .map(ManagedFileExpectation::path);
            self.record(
                HookPathSafetyState::Verified,
                HookPathSafetyEvidenceSource::HostHookWrapper,
                HookPathSafetyEvidenceReason::CurrentContractVerified,
                Some(phase),
                path,
            );
        }
    }

    pub(crate) fn merge(&mut self, other: Self) {
        if other.assessment_count == 0 {
            return;
        }
        if self.assessment_count == 0 {
            *self = other;
            return;
        }
        self.state = HookPathSafetyState::aggregate(self.state, other.state);
        self.cwd_independence =
            HookPathSafetyState::aggregate(self.cwd_independence, other.cwd_independence);
        self.subdirectory_safety =
            HookPathSafetyState::aggregate(self.subdirectory_safety, other.subdirectory_safety);
        self.assessment_count += other.assessment_count;
        self.evidence.extend(other.evidence);
        self.canonicalize();
    }

    fn canonicalize(&mut self) {
        self.evidence.sort_by(|left, right| {
            (
                hook_path_evidence_state_rank(left.state),
                left.source,
                left.reason,
                &left.installation_id,
                left.phase,
                &left.path,
            )
                .cmp(&(
                    hook_path_evidence_state_rank(right.state),
                    right.source,
                    right.reason,
                    &right.installation_id,
                    right.phase,
                    &right.path,
                ))
        });
        self.evidence.dedup();
        self.evidence.truncate(MAX_HOOK_PATH_SAFETY_EVIDENCE);
    }
}

fn hook_path_evidence_state_rank(state: HookPathSafetyState) -> u8 {
    match state {
        HookPathSafetyState::Failed => 0,
        HookPathSafetyState::NotRecorded => 1,
        HookPathSafetyState::NotChecked => 2,
        HookPathSafetyState::NotApplicable => 3,
        HookPathSafetyState::Verified => 4,
    }
}

fn bounded_hook_path_evidence_text(value: &str) -> String {
    value
        .chars()
        .take(MAX_HOOK_PATH_SAFETY_EVIDENCE_TEXT_CHARS)
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum GuardArtifactIssue {
    Missing,
    Unavailable,
    Malformed,
    ContentMismatch,
    OwnershipMismatch,
    PermissionMismatch,
    HookContractMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GuardArtifactFinding {
    pub(crate) artifact: GuardManagedArtifact,
    pub(crate) path: PathBuf,
    pub(crate) issue: GuardArtifactIssue,
    pub(crate) details: Option<Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum GuardManifestIssue {
    Malformed,
    OwnershipMismatch,
}

#[derive(Debug, Default)]
pub(crate) struct GuardAuditFacts {
    pub(crate) findings: Vec<GuardArtifactFinding>,
    pub(crate) manifest_issues: Vec<GuardManifestIssue>,
    audited_artifacts: BTreeSet<GuardManagedArtifact>,
    pub(crate) guard_profiles: Vec<IntegrationProfile>,
    pub(crate) direct_file_write_matcher_coverage_values: Vec<bool>,
    pub(crate) missing_required_phases: Vec<GuardHookPhase>,
    pub(crate) hook_path_safety: HookPathSafetyAssessment,
    pub(crate) prompt_capture_configured: bool,
    pub(crate) prompt_capture_host_supported: bool,
    pub(crate) rule_file_supported: bool,
}

impl GuardAuditFacts {
    pub(crate) fn merge(&mut self, other: GuardAuditFacts) {
        self.findings.extend(other.findings);
        self.manifest_issues.extend(other.manifest_issues);
        self.audited_artifacts.extend(other.audited_artifacts);
        self.guard_profiles.extend(other.guard_profiles);
        self.direct_file_write_matcher_coverage_values
            .extend(other.direct_file_write_matcher_coverage_values);
        self.missing_required_phases
            .extend(other.missing_required_phases);
        self.hook_path_safety.merge(other.hook_path_safety);
        self.prompt_capture_configured |= other.prompt_capture_configured;
        self.prompt_capture_host_supported |= other.prompt_capture_host_supported;
        self.rule_file_supported |= other.rule_file_supported;
    }

    pub(crate) fn sort_dedup(&mut self) {
        self.findings.sort_by(|left, right| {
            (&left.artifact, &left.path, left.issue).cmp(&(
                &right.artifact,
                &right.path,
                right.issue,
            ))
        });
        self.findings.dedup_by(|left, right| {
            left.artifact == right.artifact && left.path == right.path && left.issue == right.issue
        });
        self.manifest_issues.sort();
        self.manifest_issues.dedup();
        self.guard_profiles.sort();
        self.guard_profiles.dedup();
        self.missing_required_phases.sort();
        self.missing_required_phases.dedup();
        self.hook_path_safety.canonicalize();
    }

    fn record_finding(
        &mut self,
        artifact: GuardManagedArtifact,
        path: impl Into<PathBuf>,
        issue: GuardArtifactIssue,
    ) {
        let path = path.into();
        if let Some(source) = hook_path_safety_source(artifact) {
            let state = if issue == GuardArtifactIssue::Unavailable {
                HookPathSafetyState::NotChecked
            } else {
                HookPathSafetyState::Failed
            };
            self.hook_path_safety.record(
                state,
                source,
                hook_path_safety_reason(artifact, issue),
                hook_path_safety_phase(artifact),
                Some(&path),
            );
        }
        self.findings.push(GuardArtifactFinding {
            artifact,
            path,
            issue,
            details: None,
        });
    }

    fn record_manifest_issue(&mut self, issue: GuardManifestIssue) {
        let reason = match issue {
            GuardManifestIssue::Malformed => HookPathSafetyEvidenceReason::ManifestMalformed,
            GuardManifestIssue::OwnershipMismatch => {
                HookPathSafetyEvidenceReason::OwnerBindingMismatch
            }
        };
        self.hook_path_safety.record(
            HookPathSafetyState::Failed,
            HookPathSafetyEvidenceSource::GuardManifest,
            reason,
            None,
            None,
        );
        self.manifest_issues.push(issue);
    }

    pub(crate) fn affected_paths(&self) -> Vec<PathBuf> {
        let mut paths = self
            .findings
            .iter()
            .map(|finding| finding.path.clone())
            .collect::<Vec<_>>();
        paths.sort();
        paths.dedup();
        paths
    }

    pub(crate) fn artifact_kind_audited(&self, kind: GuardManagedArtifactKind) -> bool {
        self.audited_artifacts
            .iter()
            .any(|artifact| artifact.kind() == kind)
    }

    pub(crate) fn artifact_issues(
        &self,
        kind: GuardManagedArtifactKind,
    ) -> BTreeSet<GuardArtifactIssue> {
        self.findings
            .iter()
            .filter(|finding| finding.artifact.kind() == kind)
            .map(|finding| finding.issue)
            .collect()
    }

    pub(crate) fn generated_config_verified(&self) -> bool {
        self.findings.is_empty()
            && self.manifest_issues.is_empty()
            && volicord_types::guard_manifest::GUARD_MANAGED_ARTIFACT_SPECS
                .iter()
                .filter(|spec| !spec.optional_under_git_owner)
                .all(|spec| self.audited_artifacts.contains(&spec.artifact))
    }

    pub(crate) fn direct_file_write_matcher_coverage(&self) -> bool {
        self.generated_config_verified()
            && all_recorded_values_true(&self.direct_file_write_matcher_coverage_values)
    }
}

fn hook_path_safety_source(artifact: GuardManagedArtifact) -> Option<HookPathSafetyEvidenceSource> {
    match artifact {
        GuardManagedArtifact::VolicordPolicy => Some(HookPathSafetyEvidenceSource::ProjectPolicy),
        GuardManagedArtifact::HostHookConfig => Some(HookPathSafetyEvidenceSource::HostHookConfig),
        GuardManagedArtifact::HostHookDispatch => {
            Some(HookPathSafetyEvidenceSource::HostHookDispatch)
        }
        GuardManagedArtifact::HostHookWrapper(_) => {
            Some(HookPathSafetyEvidenceSource::HostHookWrapper)
        }
        GuardManagedArtifact::GitInfoExclude
        | GuardManagedArtifact::HostRuleInstruction
        | GuardManagedArtifact::AgentsManagedBlock => None,
    }
}

fn hook_path_safety_phase(artifact: GuardManagedArtifact) -> Option<GuardHookPhase> {
    match artifact {
        GuardManagedArtifact::HostHookWrapper(phase) => Some(phase),
        _ => None,
    }
}

fn hook_path_safety_reason(
    artifact: GuardManagedArtifact,
    issue: GuardArtifactIssue,
) -> HookPathSafetyEvidenceReason {
    match issue {
        GuardArtifactIssue::Missing => HookPathSafetyEvidenceReason::ArtifactMissing,
        GuardArtifactIssue::Unavailable => HookPathSafetyEvidenceReason::ArtifactUnavailable,
        GuardArtifactIssue::Malformed => HookPathSafetyEvidenceReason::ArtifactMalformed,
        GuardArtifactIssue::ContentMismatch => HookPathSafetyEvidenceReason::ContentMismatch,
        GuardArtifactIssue::OwnershipMismatch => {
            HookPathSafetyEvidenceReason::ManagedCommandBindingMismatch
        }
        GuardArtifactIssue::PermissionMismatch => HookPathSafetyEvidenceReason::PermissionMismatch,
        GuardArtifactIssue::HookContractMismatch => match artifact {
            GuardManagedArtifact::HostHookConfig => {
                HookPathSafetyEvidenceReason::NoncanonicalHookCommand
            }
            GuardManagedArtifact::HostHookDispatch => {
                HookPathSafetyEvidenceReason::NoncanonicalRootResolution
            }
            GuardManagedArtifact::HostHookWrapper(_) => {
                HookPathSafetyEvidenceReason::NoncanonicalManagedCommand
            }
            _ => HookPathSafetyEvidenceReason::NoncanonicalManagedCommand,
        },
    }
}

pub(crate) fn all_recorded_values_true(values: &[bool]) -> bool {
    !values.is_empty() && values.iter().all(|value| *value)
}

#[derive(Debug, Clone, Copy)]
struct GuardAuthorityContext<'a> {
    guard_installation_id: &'a str,
    connection_internal_id: &'a str,
    project_id: &'a str,
    connection_host_kind: &'a str,
    connection_server_name: &'a str,
    connection_integration_revision: &'a str,
    project_repo_root: &'a Path,
}

pub(crate) fn guard_file_findings_for_installation(
    runtime_home: &Path,
    installation: &GuardInstallationRecord,
    connection: &AgentConnectionRecord,
    projects: &[ConnectionProjectRecord],
) -> GuardAuditFacts {
    let matched_projects = projects
        .iter()
        .filter(|project| installation.project_internal_id == project.project_internal_id)
        .collect::<Vec<_>>();
    let Ok(revision) =
        volicord_store::operational_sessions::connection_integration_revision(connection)
    else {
        return broken_manifest_findings();
    };
    let [project] = matched_projects.as_slice() else {
        return broken_manifest_findings();
    };
    let context = GuardAuthorityContext {
        guard_installation_id: &installation.guard_installation_id,
        connection_internal_id: &connection.connection_internal_id,
        project_id: &installation.project_id,
        connection_host_kind: &connection.host_kind,
        connection_server_name: &connection.server_name,
        connection_integration_revision: revision.as_str(),
        project_repo_root: &project.project.repo_root,
    };
    let mut findings = guard_file_findings_with_context(&installation.manifest_json, Some(context));
    audit_authoritative_project_policy(
        runtime_home,
        &project.project.project_id,
        &project.project.repo_root,
        &connection.intent,
        &mut findings,
    );
    findings
}

fn audit_authoritative_project_policy(
    runtime_home: &Path,
    project_id: &str,
    repo_root: &Path,
    connection_intent: &str,
    findings: &mut GuardAuditFacts,
) {
    let artifact = GuardManagedArtifact::VolicordPolicy;
    let path = artifact
        .expected_path(repo_root, None)
        .expect("the Guard policy has a repository-owned path");
    let issue = (|| {
        let store = CoreProjectStore::open_read_only(runtime_home, &ProjectId::new(project_id))
            .map_err(|_| GuardArtifactIssue::Unavailable)?;
        let authority = store
            .project_workflow_policy()
            .map_err(|_| GuardArtifactIssue::Unavailable)?
            .ok_or(GuardArtifactIssue::OwnershipMismatch)?;
        let text = super::files::read_managed_text(repo_root, &path)
            .map_err(|_| GuardArtifactIssue::Unavailable)?
            .ok_or(GuardArtifactIssue::Missing)?;
        let policy =
            serde_json::from_str::<Value>(&text).map_err(|_| GuardArtifactIssue::Malformed)?;
        validate_policy_schema(&policy, connection_intent)
            .map_err(|_| GuardArtifactIssue::Malformed)?;
        let fingerprint = policy_hash(&policy).map_err(|_| GuardArtifactIssue::Malformed)?;
        if fingerprint != authority.policy_fingerprint {
            return Err(GuardArtifactIssue::OwnershipMismatch);
        }
        Ok(())
    })()
    .err();
    if let Some(issue) = issue {
        findings.record_finding(artifact, path, issue);
    }
}

pub(crate) fn guard_manifest_binding_valid_for_installation(
    installation: &GuardInstallationRecord,
    connection: &AgentConnectionRecord,
    projects: &[ConnectionProjectRecord],
) -> bool {
    let matched_projects = projects
        .iter()
        .filter(|project| installation.project_internal_id == project.project_internal_id)
        .collect::<Vec<_>>();
    let [project] = matched_projects.as_slice() else {
        return false;
    };
    let Ok(revision) =
        volicord_store::operational_sessions::connection_integration_revision(connection)
    else {
        return false;
    };
    let context = GuardAuthorityContext {
        guard_installation_id: &installation.guard_installation_id,
        connection_internal_id: &connection.connection_internal_id,
        project_id: &installation.project_id,
        connection_host_kind: &connection.host_kind,
        connection_server_name: &connection.server_name,
        connection_integration_revision: revision.as_str(),
        project_repo_root: &project.project.repo_root,
    };
    serde_json::from_str::<Value>(&installation.manifest_json)
        .ok()
        .is_some_and(|value| guard_manifest_matches_authority_context(&value, context))
}

pub(crate) fn guard_file_findings_for_inspection(
    installation: &GuardInstallationInspectionRecord,
    connection: &AgentConnectionInspectionRecord,
    projects: &[ProjectInspectionRecord],
) -> GuardAuditFacts {
    let matched_projects = projects
        .iter()
        .filter(|project| installation.project_internal_id == project.project_internal_id)
        .collect::<Vec<_>>();
    let [project] = matched_projects.as_slice() else {
        return broken_manifest_findings();
    };
    let Ok(revision) = inspection_connection_revision(connection) else {
        return broken_manifest_findings();
    };
    let context = GuardAuthorityContext {
        guard_installation_id: &installation.guard_installation_id,
        connection_internal_id: &connection.connection_internal_id,
        project_id: &installation.project_id,
        connection_host_kind: &connection.host_kind,
        connection_server_name: &connection.server_name,
        connection_integration_revision: revision.as_str(),
        project_repo_root: &project.repo_root,
    };
    guard_file_findings_with_context(&installation.manifest_json, Some(context))
}

pub(crate) fn guard_manifest_binding_valid_for_inspection(
    installation: &GuardInstallationInspectionRecord,
    connection: &AgentConnectionInspectionRecord,
    projects: &[ProjectInspectionRecord],
) -> bool {
    let matched_projects = projects
        .iter()
        .filter(|project| installation.project_internal_id == project.project_internal_id)
        .collect::<Vec<_>>();
    let [project] = matched_projects.as_slice() else {
        return false;
    };
    let Ok(revision) = inspection_connection_revision(connection) else {
        return false;
    };
    let context = GuardAuthorityContext {
        guard_installation_id: &installation.guard_installation_id,
        connection_internal_id: &connection.connection_internal_id,
        project_id: &installation.project_id,
        connection_host_kind: &connection.host_kind,
        connection_server_name: &connection.server_name,
        connection_integration_revision: revision.as_str(),
        project_repo_root: &project.repo_root,
    };
    serde_json::from_str::<Value>(&installation.manifest_json)
        .ok()
        .is_some_and(|value| guard_manifest_matches_authority_context(&value, context))
}

fn guard_manifest_matches_authority_context(
    value: &Value,
    context: GuardAuthorityContext<'_>,
) -> bool {
    let project_git_info_exclude_path = match git_exclude_path(context.project_repo_root) {
        Ok(path) => path,
        Err(_) => return false,
    };
    guard_manifest_matches_owner_binding(
        value,
        GuardManifestOwnerBinding {
            row_guard_installation_id: context.guard_installation_id,
            row_connection_id: context.connection_internal_id,
            row_project_id: context.project_id,
            connection_host_kind: context.connection_host_kind,
            connection_integration_revision: context.connection_integration_revision,
            project_repo_root: context.project_repo_root,
            project_git_info_exclude_path: project_git_info_exclude_path.as_deref(),
        },
    )
}

fn inspection_connection_revision(
    connection: &AgentConnectionInspectionRecord,
) -> Result<IntegrationRevision, volicord_types::integration_revision::IntegrationRevisionError> {
    IntegrationRevision::for_connection(ConnectionIntegrationRevisionBasis {
        connection_internal_id: &connection.connection_internal_id,
        integration_instance_id: &connection.integration_instance_id,
        host_kind: &connection.host_kind,
        intent: &connection.intent,
        host_scope: &connection.host_scope,
        mode: &connection.mode,
        server_name: &connection.server_name,
        config_target: &connection.config_target,
        managed_configuration_fingerprint: &connection.managed_fingerprint,
        integration_generation: connection.integration_generation,
    })
}

fn broken_manifest_findings() -> GuardAuditFacts {
    let mut findings = GuardAuditFacts::default();
    findings.hook_path_safety.mark_applicable(None);
    findings.record_manifest_issue(GuardManifestIssue::OwnershipMismatch);
    findings
}

pub(crate) fn missing_required_hooks_from_manifest_json(manifest_json: &str) -> Vec<String> {
    guard_manifest_from_json(manifest_json)
        .ok()
        .map(|manifest| missing_required_hooks_from_manifest(&manifest))
        .unwrap_or_else(|| {
            required_guard_phase_names()
                .into_iter()
                .map(str::to_owned)
                .collect()
        })
}

fn guard_file_findings_with_context(
    manifest_json: &str,
    context: Option<GuardAuthorityContext<'_>>,
) -> GuardAuditFacts {
    let mut findings = GuardAuditFacts::default();
    findings.hook_path_safety.mark_applicable(None);
    let Ok(manifest) = guard_manifest_from_json(manifest_json) else {
        findings.record_manifest_issue(GuardManifestIssue::Malformed);
        return findings;
    };
    findings
        .hook_path_safety
        .mark_applicable(Some(manifest.guard_installation_id.as_str()));
    let value = serde_json::to_value(&manifest).expect("typed Guard manifest serializes");
    if context.is_some_and(|context| !guard_manifest_matches_authority_context(&value, context)) {
        findings.record_manifest_issue(GuardManifestIssue::OwnershipMismatch);
        return findings;
    }
    findings.prompt_capture_configured = manifest
        .required_hook_phases
        .contains(&GuardHookPhase::PromptCapture);
    findings.prompt_capture_host_supported = true;
    findings.rule_file_supported = manifest.managed_files.iter().any(|file| {
        file.artifact() == volicord_types::guard_manifest::GuardManagedArtifact::HostRuleInstruction
    });
    findings.guard_profiles.push(manifest.integration_profile);
    findings
        .direct_file_write_matcher_coverage_values
        .push(true);
    findings.missing_required_phases = missing_required_phases_from_manifest(&manifest);
    for phase in findings.missing_required_phases.iter().copied() {
        findings.hook_path_safety.record(
            HookPathSafetyState::Failed,
            HookPathSafetyEvidenceSource::GuardManifest,
            HookPathSafetyEvidenceReason::RequiredPhaseMissing,
            Some(phase),
            None,
        );
    }

    for expectation in &manifest.managed_files {
        findings.audited_artifacts.insert(expectation.artifact());
        verify_guard_file(expectation, &manifest, context, &mut findings);
    }
    for artifact in required_hook_path_safety_artifacts() {
        if !findings.audited_artifacts.contains(&artifact) {
            findings.hook_path_safety.record(
                HookPathSafetyState::Failed,
                hook_path_safety_source(artifact)
                    .expect("required Hook path-safety artifacts have a source"),
                HookPathSafetyEvidenceReason::RequiredArtifactMissing,
                hook_path_safety_phase(artifact),
                None,
            );
        }
    }
    if context.is_some() {
        findings.hook_path_safety.mark_verified(&manifest);
    }
    findings
}

fn required_hook_path_safety_artifacts() -> impl Iterator<Item = GuardManagedArtifact> {
    [
        GuardManagedArtifact::VolicordPolicy,
        GuardManagedArtifact::HostHookConfig,
        GuardManagedArtifact::HostHookDispatch,
    ]
    .into_iter()
    .chain(
        GuardHookPhase::REQUIRED
            .into_iter()
            .map(GuardManagedArtifact::HostHookWrapper),
    )
}

fn missing_required_hooks_from_manifest(
    manifest: &volicord_types::guard_manifest::GuardManifest,
) -> Vec<String> {
    missing_required_phases_from_manifest(manifest)
        .into_iter()
        .map(|phase| guard_phase_capability_name(phase).to_owned())
        .collect()
}

fn missing_required_phases_from_manifest(
    manifest: &volicord_types::guard_manifest::GuardManifest,
) -> Vec<GuardHookPhase> {
    GuardHookPhase::REQUIRED
        .into_iter()
        .filter(|phase| !manifest.required_hook_phases.contains(phase))
        .collect()
}

fn verify_guard_file(
    file: &ManagedFileExpectation,
    manifest: &volicord_types::guard_manifest::GuardManifest,
    context: Option<GuardAuthorityContext<'_>>,
    findings: &mut GuardAuditFacts,
) {
    let artifact = file.artifact();
    let path = file.path();
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            findings.record_finding(artifact, path, GuardArtifactIssue::Missing);
            return;
        }
        Err(_) => {
            findings.record_finding(artifact, path, GuardArtifactIssue::Unavailable);
            return;
        }
    };
    match file {
        ManagedFileExpectation::GitInfoExclude { .. }
        | ManagedFileExpectation::HostRuleInstruction { .. }
        | ManagedFileExpectation::AgentsManagedBlock { .. } => {
            verify_managed_block_file(file, &text, findings)
        }
        ManagedFileExpectation::VolicordPolicy { .. }
        | ManagedFileExpectation::HostHookConfig { .. } => {
            verify_managed_json_file(file, manifest, context, &text, findings)
        }
        ManagedFileExpectation::HostHookDispatch { .. }
        | ManagedFileExpectation::HostHookWrapper { .. } => {
            verify_managed_script_file(file, manifest, &text, findings)
        }
    }
}

fn verify_managed_block_file(
    file: &ManagedFileExpectation,
    text: &str,
    findings: &mut GuardAuditFacts,
) {
    let artifact = file.artifact();
    let path = file.path();
    let Some((start_marker, end_marker)) = file.block_markers() else {
        findings.record_finding(artifact, path, GuardArtifactIssue::Malformed);
        return;
    };
    if marker_count(text, start_marker) != 1 || marker_count(text, end_marker) != 1 {
        findings.record_finding(artifact, path, GuardArtifactIssue::Malformed);
        return;
    }
    let Some(block) = managed_block_slice(text, start_marker, end_marker) else {
        findings.record_finding(artifact, path, GuardArtifactIssue::Malformed);
        return;
    };
    if sha256_text(block) != file.content_hash().as_str() {
        findings.record_finding(artifact, path, GuardArtifactIssue::ContentMismatch);
    }
}

fn verify_managed_json_file(
    file: &ManagedFileExpectation,
    manifest: &volicord_types::guard_manifest::GuardManifest,
    context: Option<GuardAuthorityContext<'_>>,
    text: &str,
    findings: &mut GuardAuditFacts,
) {
    let artifact = file.artifact();
    let path = file.path();
    if sha256_text(text) != file.content_hash().as_str() {
        findings.record_finding(artifact, path, GuardArtifactIssue::ContentMismatch);
    }
    if artifact == GuardManagedArtifact::HostHookConfig {
        match serde_json::from_str::<Value>(text) {
            Ok(value) if is_volicord_codex_hook_config(&value) => {}
            Ok(_) | Err(_) => {
                findings.record_finding(artifact, path, GuardArtifactIssue::Malformed);
                return;
            }
        }
        if let Some(context) = context {
            let validation_matches =
                volicord_host_contract::McpServerKey::parse(context.connection_server_name)
                    .ok()
                    .is_some_and(|server| {
                        validate_contract_config(
                            HostKind::Codex,
                            HostContractConfigKind::HookConfig,
                            text,
                            Some(&server),
                        )
                        .is_ok()
                    });
            if !validation_matches {
                findings.record_finding(artifact, path, GuardArtifactIssue::HookContractMismatch);
            }
        }
        return;
    }
    let policy = match serde_json::from_str::<Value>(text) {
        Ok(policy) => policy,
        Err(_) => {
            findings.record_finding(artifact, path, GuardArtifactIssue::Malformed);
            return;
        }
    };
    let Some(connection_intent) = policy.get("connection_intent").and_then(Value::as_str) else {
        findings.record_finding(artifact, path, GuardArtifactIssue::Malformed);
        return;
    };
    if let Err(issue) = validate_workflow_policy(&policy, Some(connection_intent)) {
        if issue.code == "POLICY_BINDING_MISMATCH" {
            findings.record_finding(artifact, path, GuardArtifactIssue::OwnershipMismatch);
        } else {
            findings.record_finding(artifact, path, GuardArtifactIssue::Malformed);
            return;
        }
    }
    match policy_hash(&policy) {
        Ok(actual) if actual == manifest.policy_hash.as_str() => {}
        Ok(_) => {
            findings.record_finding(artifact, path, GuardArtifactIssue::ContentMismatch);
        }
        Err(_) => {
            findings.record_finding(artifact, path, GuardArtifactIssue::Malformed);
            return;
        }
    }
    let command_owner_fields_match = policy_command_invocations(&policy)
        .zip(runtime_command_invocations(manifest))
        .is_some_and(|(policy_commands, runtime_commands)| {
            let policy_command = policy_commands.get(GuardHookPhase::PreTool);
            let runtime_command = runtime_commands.get(GuardHookPhase::PreTool);
            policy_command.repo_root == runtime_command.repo_root
                && policy.get("repo_root").and_then(Value::as_str)
                    == Some(policy_command.repo_root.as_str())
        });
    let owner_fields_match = command_owner_fields_match
        && policy.get("connection_id").and_then(Value::as_str)
            == Some(manifest.connection_id.as_str())
        && policy.get("guard_installation_id").and_then(Value::as_str)
            == Some(manifest.guard_installation_id.as_str())
        && policy.get("host").and_then(Value::as_str) == Some(manifest.host_kind.as_str())
        && policy.get("selected_profile").and_then(Value::as_str)
            == Some(manifest.integration_profile.as_str());
    if !owner_fields_match || !policy_runtime_commands_match(&policy, manifest) {
        findings.record_finding(artifact, path, GuardArtifactIssue::OwnershipMismatch);
    }
}

fn policy_runtime_commands_match(
    policy: &Value,
    manifest: &volicord_types::guard_manifest::GuardManifest,
) -> bool {
    policy_command_invocations(policy)
        .zip(runtime_command_invocations(manifest))
        .is_some_and(|(policy, runtime)| policy.fields_match_except_policy_hash(&runtime))
}

fn policy_command_invocations(policy: &Value) -> Option<GuardCommandInvocationSet> {
    let commands = policy
        .get("host_hook")
        .and_then(|hook| hook.get("commands"))
        .and_then(|value| serde_json::from_value::<GuardCommandSet>(value.clone()).ok())?;
    GuardCommandInvocationSet::from_policy_commands(&commands).ok()
}

fn runtime_command_invocations(
    manifest: &volicord_types::guard_manifest::GuardManifest,
) -> Option<GuardCommandInvocationSet> {
    GuardCommandInvocationSet::from_runtime_commands(
        &manifest.runtime_commands,
        &manifest.policy_hash,
    )
    .ok()
}

fn verify_managed_script_file(
    file: &ManagedFileExpectation,
    manifest: &volicord_types::guard_manifest::GuardManifest,
    text: &str,
    findings: &mut GuardAuditFacts,
) {
    let artifact = file.artifact();
    let path = file.path();
    let marker = match file {
        ManagedFileExpectation::HostHookDispatch { managed_marker, .. }
        | ManagedFileExpectation::HostHookWrapper { managed_marker, .. } => managed_marker,
        _ => {
            findings.record_finding(artifact, path, GuardArtifactIssue::Malformed);
            return;
        }
    };
    if marker != HOOK_WRAPPER_MARKER
        || !text
            .lines()
            .any(|line| line == format!("# {HOOK_WRAPPER_MARKER}"))
    {
        findings.record_finding(artifact, path, GuardArtifactIssue::Malformed);
        return;
    }
    if matches!(file, ManagedFileExpectation::HostHookDispatch { .. }) {
        verify_managed_dispatch_script_file(file, text, findings);
        return;
    }
    let ManagedFileExpectation::HostHookWrapper {
        managed_script_command: expected_command,
        host_kind,
        phase,
        purpose,
        connection_id,
        guard_installation_id,
        policy_hash,
        host_output,
        executable_required,
        ..
    } = file
    else {
        findings.record_finding(artifact, path, GuardArtifactIssue::Malformed);
        return;
    };
    if hook_wrapper_exec_command(text) != Some(expected_command) {
        findings.record_finding(artifact, path, GuardArtifactIssue::HookContractMismatch);
    }
    if !has_exact_current_managed_wrapper_shape(text, file, expected_command) {
        findings.record_finding(artifact, path, GuardArtifactIssue::HookContractMismatch);
    }
    if !generated_managed_command_shape_verified(file, expected_command) {
        findings.record_finding(artifact, path, GuardArtifactIssue::HookContractMismatch);
    }
    if policy_hash != &manifest.policy_hash
        || hook_wrapper_comment_value(text, "policy_hash") != Some(policy_hash.as_str())
    {
        findings.record_finding(artifact, path, GuardArtifactIssue::OwnershipMismatch);
        findings.hook_path_safety.record(
            HookPathSafetyState::Failed,
            HookPathSafetyEvidenceSource::HostHookWrapper,
            HookPathSafetyEvidenceReason::PolicyHashMismatch,
            Some(*phase),
            Some(path),
        );
    }
    if hook_wrapper_comment_value(text, "host_output") != Some(host_output.as_str()) {
        findings.record_finding(artifact, path, GuardArtifactIssue::OwnershipMismatch);
        findings.hook_path_safety.record(
            HookPathSafetyState::Failed,
            HookPathSafetyEvidenceSource::HostHookWrapper,
            HookPathSafetyEvidenceReason::HostOutputMismatch,
            Some(*phase),
            Some(path),
        );
    }
    let owner_fields = [
        ("host_kind", host_kind.as_str()),
        ("phase", phase.as_str()),
        (
            "purpose",
            match purpose {
                volicord_types::guard_manifest::GuardManagedScriptPurpose::Guard => "guard",
            },
        ),
        ("connection_id", connection_id.as_str()),
        ("guard_installation_id", guard_installation_id.as_str()),
    ];
    for (key, expected) in owner_fields {
        if hook_wrapper_comment_value(text, key) != Some(expected) {
            findings.record_finding(artifact, path, GuardArtifactIssue::OwnershipMismatch);
            findings.hook_path_safety.record(
                HookPathSafetyState::Failed,
                HookPathSafetyEvidenceSource::HostHookWrapper,
                HookPathSafetyEvidenceReason::ManagedCommandBindingMismatch,
                Some(*phase),
                Some(path),
            );
        }
    }
    if sha256_text(text) != file.content_hash().as_str() {
        findings.record_finding(artifact, path, GuardArtifactIssue::ContentMismatch);
    }
    if *executable_required && !script_is_executable(path) {
        findings.record_finding(artifact, path, GuardArtifactIssue::PermissionMismatch);
    }
}

fn verify_managed_dispatch_script_file(
    file: &ManagedFileExpectation,
    text: &str,
    findings: &mut GuardAuditFacts,
) {
    let artifact = file.artifact();
    let path = file.path();
    let ManagedFileExpectation::HostHookDispatch {
        managed_script_role,
        host_kind,
        phase,
        executable_required,
        ..
    } = file
    else {
        findings.record_finding(artifact, path, GuardArtifactIssue::Malformed);
        return;
    };
    if *managed_script_role != volicord_types::guard_manifest::GuardManagedScriptRole::CodexDispatch
        || hook_wrapper_comment_value(text, "host_kind") != Some(host_kind.as_str())
        || hook_wrapper_comment_value(text, "phase") != Some("dispatch")
        || hook_wrapper_comment_value(text, "script_role") != Some("codex_dispatch")
        || *phase != volicord_types::guard_manifest::GuardDispatchPhase::Dispatch
    {
        findings.record_finding(artifact, path, GuardArtifactIssue::OwnershipMismatch);
        return;
    }
    if text != codex_dispatch_wrapper_script_content() {
        findings.record_finding(artifact, path, GuardArtifactIssue::HookContractMismatch);
    }
    if sha256_text(text) != file.content_hash().as_str() {
        findings.record_finding(artifact, path, GuardArtifactIssue::ContentMismatch);
    }
    if *executable_required && !script_is_executable(path) {
        findings.record_finding(artifact, path, GuardArtifactIssue::PermissionMismatch);
    }
}

fn marker_count(text: &str, marker: &str) -> usize {
    text.match_indices(marker).count()
}

fn managed_block_slice<'a>(text: &'a str, start_marker: &str, end_marker: &str) -> Option<&'a str> {
    let start = text.find(start_marker)?;
    let end = start + text[start..].find(end_marker)? + end_marker.len();
    let end = if text[end..].starts_with('\n') {
        end + 1
    } else {
        end
    };
    text.get(start..end)
}

pub(crate) fn is_volicord_codex_hook_config(value: &Value) -> bool {
    let Some(root) = value.as_object() else {
        return false;
    };
    if root.keys().any(|key| key != "hooks") {
        return false;
    }
    let Some(hooks) = root.get("hooks").and_then(Value::as_object) else {
        return false;
    };
    let Some(contract) = contract_for(HostKind::Codex) else {
        return false;
    };
    if hooks.len() != GuardHookPhase::REQUIRED.len() {
        return false;
    }
    let phases: &[GuardHookPhase] = &GuardHookPhase::REQUIRED;
    phases.iter().all(|phase| {
        let Some(event) = hook_event_for_phase(contract, *phase) else {
            return false;
        };
        let Some(groups) = hooks.get(event.event_name).and_then(Value::as_array) else {
            return false;
        };
        groups.len() == 1
            && groups
                .first()
                .is_some_and(|group| is_volicord_codex_hook_group(*phase, group))
    })
}

fn is_volicord_codex_hook_group(phase: GuardHookPhase, group: &Value) -> bool {
    let Some(group) = group.as_object() else {
        return false;
    };
    let expected_keys = if phase == GuardHookPhase::PromptCapture {
        &["hooks"][..]
    } else {
        &["hooks", "matcher"][..]
    };
    if group.len() != expected_keys.len()
        || group
            .keys()
            .any(|key| !expected_keys.contains(&key.as_str()))
    {
        return false;
    }
    let Some(handlers) = group.get("hooks").and_then(Value::as_array) else {
        return false;
    };
    handlers.len() == 1
        && handlers
            .first()
            .is_some_and(|handler| is_volicord_codex_hook_handler(phase, handler))
}

fn is_volicord_codex_hook_handler(phase: GuardHookPhase, handler: &Value) -> bool {
    let Some(object) = handler.as_object() else {
        return false;
    };
    let expected_keys = if phase == GuardHookPhase::PromptCapture {
        &["command", "timeout", "type"][..]
    } else {
        &["command", "statusMessage", "timeout", "type"][..]
    };
    let expected_status = match phase {
        GuardHookPhase::PreTool => Some("Checking Volicord write"),
        GuardHookPhase::PostTool => Some("Recording Volicord write"),
        GuardHookPhase::PromptCapture => None,
    };
    let expected_command = format!("sh -c {}", shell_word(&codex_guard_hook_script(phase)));
    object.len() == expected_keys.len()
        && object
            .keys()
            .all(|key| expected_keys.contains(&key.as_str()))
        && object.get("type").and_then(Value::as_str) == Some("command")
        && object.get("command").and_then(Value::as_str) == Some(expected_command.as_str())
        && object.get("timeout").and_then(Value::as_u64) == Some(30)
        && object.get("statusMessage").and_then(Value::as_str) == expected_status
}

pub(crate) fn hook_wrapper_exec_command(content: &str) -> Option<&str> {
    content
        .lines()
        .find_map(|line| line.strip_prefix("exec "))
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn generated_shell_words(command: &str) -> Option<Vec<String>> {
    let mut chars = command.chars().peekable();
    let mut words = Vec::new();
    while chars.peek().is_some() {
        while chars
            .peek()
            .is_some_and(|character| character.is_whitespace())
        {
            chars.next();
        }
        if chars.peek().is_none() {
            break;
        }
        let mut word = String::new();
        let mut consumed = false;
        while chars
            .peek()
            .is_some_and(|character| !character.is_whitespace())
        {
            match chars.next()? {
                '\'' => {
                    consumed = true;
                    loop {
                        match chars.next() {
                            Some('\'') => break,
                            Some(character) => word.push(character),
                            None => return None,
                        }
                    }
                }
                '\\' => {
                    consumed = true;
                    word.push(chars.next()?);
                }
                character
                    if character.is_ascii_alphanumeric()
                        || matches!(character, '_' | '-' | '.' | '/' | ':' | '=') =>
                {
                    consumed = true;
                    word.push(character);
                }
                _ => return None,
            }
        }
        if !consumed {
            return None;
        }
        words.push(word);
    }
    Some(words)
}

fn generated_managed_command_shape_verified(file: &ManagedFileExpectation, command: &str) -> bool {
    let ManagedFileExpectation::HostHookWrapper {
        purpose,
        policy_hash,
        phase,
        host_kind,
        connection_id,
        guard_installation_id,
        host_output,
        ..
    } = file
    else {
        return false;
    };
    if *purpose != volicord_types::guard_manifest::GuardManagedScriptPurpose::Guard {
        return false;
    }
    let Some(words) = generated_shell_words(command) else {
        return false;
    };
    let Some((executable, args)) = words.split_first() else {
        return false;
    };
    let command = GuardCommand {
        command: executable.clone(),
        args: args.to_vec(),
    };
    let Ok(invocation) =
        GuardCommandInvocation::from_runtime_command_with_policy_hash(&command, policy_hash)
    else {
        return false;
    };
    *phase == invocation.phase
        && *host_kind == invocation.host_kind
        && connection_id == &invocation.connection_id
        && guard_installation_id == &invocation.guard_installation_id
        && *host_output == invocation.host_output
}

fn has_current_managed_process_binding(content: &str) -> bool {
    let binding_export = format!("export {MANAGED_WRAPPER_ENV}");
    let binding_assignment = format!("{MANAGED_WRAPPER_ENV}={MANAGED_WRAPPER_VALUE}");
    if hook_wrapper_comment_value(content, "runtime_home_binding")
        != Some("selected_init_runtime_home")
        || content
            .lines()
            .filter(|line| *line == "export VOLICORD_HOME")
            .count()
            != 1
        || content
            .lines()
            .filter(|line| *line == binding_export)
            .count()
            != 1
        || content
            .lines()
            .filter(|line| *line == binding_assignment)
            .count()
            != 1
    {
        return false;
    }
    let mut assignments = content
        .lines()
        .filter(|line| line.starts_with("VOLICORD_HOME="));
    let Some(assignment) = assignments.next() else {
        return false;
    };
    if assignments.next().is_some() {
        return false;
    }
    generated_shell_words(assignment)
        .filter(|words| words.len() == 1)
        .and_then(|words| words.into_iter().next())
        .and_then(|word| word.strip_prefix("VOLICORD_HOME=").map(str::to_owned))
        .is_some_and(|runtime_home| {
            !runtime_home.is_empty() && Path::new(&runtime_home).is_absolute()
        })
}

fn has_exact_current_managed_wrapper_shape(
    content: &str,
    file: &ManagedFileExpectation,
    expected_command: &str,
) -> bool {
    let ManagedFileExpectation::HostHookWrapper {
        host_kind,
        phase,
        purpose,
        connection_id,
        guard_installation_id,
        policy_hash,
        host_output,
        ..
    } = file
    else {
        return false;
    };
    if !has_current_managed_process_binding(content) {
        return false;
    }
    let Some(runtime_assignment) = content
        .lines()
        .find(|line| line.starts_with("VOLICORD_HOME="))
    else {
        return false;
    };
    let Some(runtime_home) = generated_shell_words(runtime_assignment)
        .filter(|words| words.len() == 1)
        .and_then(|words| words.into_iter().next())
        .and_then(|word| word.strip_prefix("VOLICORD_HOME=").map(str::to_owned))
    else {
        return false;
    };
    if runtime_assignment != format!("VOLICORD_HOME={}", shell_word(&runtime_home)) {
        return false;
    }
    let purpose = match purpose {
        volicord_types::guard_manifest::GuardManagedScriptPurpose::Guard => "guard",
    };
    let expected = [
        "#!/bin/sh".to_owned(),
        format!("# {HOOK_WRAPPER_MARKER}"),
        format!("# host_kind={}", host_kind.as_str()),
        format!("# phase={}", phase.as_str()),
        format!("# purpose={purpose}"),
        format!("# connection_id={}", connection_id.as_str()),
        format!("# guard_installation_id={}", guard_installation_id.as_str()),
        format!("# policy_hash={}", policy_hash.as_str()),
        format!("# host_output={}", host_output.as_str()),
        "# runtime_home_binding=selected_init_runtime_home".to_owned(),
        runtime_assignment.to_owned(),
        format!("{MANAGED_WRAPPER_ENV}={MANAGED_WRAPPER_VALUE}"),
        "export VOLICORD_HOME".to_owned(),
        format!("export {MANAGED_WRAPPER_ENV}"),
        format!("exec {expected_command}"),
    ];
    content.lines().eq(expected.iter().map(String::as_str)) && content.ends_with('\n')
}

pub(crate) fn hook_wrapper_comment_value<'a>(content: &'a str, key: &str) -> Option<&'a str> {
    let prefix = format!("# {key}=");
    content
        .lines()
        .find_map(|line| line.strip_prefix(&prefix))
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

pub(crate) fn policy_hash(policy: &Value) -> Result<String, serde_json::Error> {
    volicord_types::canonical::canonical_json_sha256(policy).map(|hash| hash.into_inner())
}

pub(crate) fn sha256_text(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    format!("sha256:{}", hex_bytes(&hasher.finalize()))
}

fn hex_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

#[cfg(unix)]
pub(crate) fn script_is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    fs::metadata(path)
        .map(|metadata| metadata.permissions().mode() & 0o100 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
pub(crate) fn script_is_executable(_path: &Path) -> bool {
    true
}

#[cfg(test)]
mod tests {
    use std::fs;

    use serde_json::Value;
    use volicord_store::{
        agent_connections::agent_connection_record_read_only,
        operational_sessions::connection_integration_revision,
    };
    use volicord_test_support::core_fixtures::CoreFixture;
    use volicord_types::guard_manifest::GuardManagedArtifact;
    use volicord_types::values::{GuardHookPhase, IntegrationProfile};

    use super::{
        guard_file_findings_with_context, GuardArtifactIssue, GuardAuditFacts,
        GuardAuthorityContext, GuardManifestIssue, HookPathSafetyAssessment,
        HookPathSafetyEvidenceReason, HookPathSafetyState, MAX_HOOK_PATH_SAFETY_EVIDENCE,
    };
    use crate::{
        guard_integration::{
            apply_guard_integration,
            manifest::guard_manifest_json,
            plan::{plan_guard_integration, GuardIntegrationPlanRequest},
        },
        host_integration::{ConnectionIntent, HostKind},
    };
    use volicord_mcp::ManagedMcpLaunchSpec;

    #[test]
    fn manifest_audit_accepts_projection_and_detects_owned_file_drift(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = CoreFixture::new("guard-manifest-audit")?;
        let repo_root = fixture.product_repo_path();
        fs::create_dir_all(repo_root.join(".git"))?;
        let unrelated_path = repo_root.join("README.user.md");
        fs::write(&unrelated_path, "user-owned\n")?;
        let volicord_command = fixture.runtime_home_path().join("bin/volicord");
        let mcp_entry = ManagedMcpLaunchSpec::shared_repository(HostKind::Codex)?;
        let plan = plan_guard_integration(GuardIntegrationPlanRequest {
            host_kind: HostKind::Codex,
            profile: IntegrationProfile::Record,
            server_name: "volicord-test",
            runtime_home: fixture.runtime_home_path(),
            volicord_command: &volicord_command,
            repo_root: &repo_root,
            connection_id: fixture.connection_id(),
            guard_installation_id: "guard_manifest_audit",
            mcp_entry: &mcp_entry,
            connection_intent: ConnectionIntent::Shared,
        })?;
        let connection = agent_connection_record_read_only(
            fixture.runtime_home_path(),
            fixture.connection_id(),
        )?
        .expect("fixture connection");
        let revision = connection_integration_revision(&connection)?;
        let manifest_json = guard_manifest_json(&connection, fixture.project_id(), &plan)?;
        apply_guard_integration(plan)?;
        let context = GuardAuthorityContext {
            guard_installation_id: "guard_manifest_audit",
            connection_internal_id: fixture.connection_id(),
            project_id: fixture.project_id(),
            connection_host_kind: &connection.host_kind,
            connection_server_name: &connection.server_name,
            connection_integration_revision: revision.as_str(),
            project_repo_root: &repo_root,
        };
        let has_issue = |facts: &GuardAuditFacts,
                         artifact: GuardManagedArtifact,
                         path: &std::path::Path,
                         issue: GuardArtifactIssue| {
            facts.findings.iter().any(|finding| {
                finding.artifact == artifact && finding.path == path && finding.issue == issue
            })
        };

        let valid = guard_file_findings_with_context(&manifest_json, Some(context));
        assert!(valid.generated_config_verified());
        assert!(valid.findings.is_empty());
        assert!(valid.manifest_issues.is_empty());
        assert_eq!(valid.hook_path_safety.state, HookPathSafetyState::Verified);
        assert_eq!(
            valid.hook_path_safety.cwd_independence,
            HookPathSafetyState::Verified
        );
        assert_eq!(
            valid.hook_path_safety.subdirectory_safety,
            HookPathSafetyState::Verified
        );
        assert!(valid.hook_path_safety.evidence.iter().all(|evidence| {
            evidence.state == HookPathSafetyState::Verified
                && evidence.reason == HookPathSafetyEvidenceReason::CurrentContractVerified
        }));

        let applicable_without_authority = guard_file_findings_with_context(&manifest_json, None);
        assert_eq!(
            applicable_without_authority.hook_path_safety.state,
            HookPathSafetyState::NotRecorded
        );
        assert_eq!(
            applicable_without_authority
                .hook_path_safety
                .cwd_independence,
            HookPathSafetyState::NotRecorded
        );
        assert_eq!(
            applicable_without_authority
                .hook_path_safety
                .subdirectory_safety,
            HookPathSafetyState::NotRecorded
        );

        let mut hash_mismatch_manifest: Value = serde_json::from_str(&manifest_json)?;
        hash_mismatch_manifest["runtime_commands"]["post_tool"]["args"][13] = Value::String(
            "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_owned(),
        );
        let hash_mismatch = guard_file_findings_with_context(
            &serde_json::to_string(&hash_mismatch_manifest)?,
            Some(context),
        );
        assert!(hash_mismatch
            .manifest_issues
            .contains(&GuardManifestIssue::Malformed));

        let policy_path = repo_root.join(".volicord/policy.json");
        let policy_text = fs::read_to_string(&policy_path)?;
        let mut policy: Value = serde_json::from_str(&policy_text)?;
        policy["connection_id"] = Value::String("connection_other".to_owned());
        fs::write(&policy_path, serde_json::to_string(&policy)?)?;
        let command_owner_mismatch =
            guard_file_findings_with_context(&manifest_json, Some(context));
        assert!(has_issue(
            &command_owner_mismatch,
            GuardManagedArtifact::VolicordPolicy,
            &policy_path,
            GuardArtifactIssue::OwnershipMismatch,
        ));
        policy = serde_json::from_str(&policy_text)?;
        policy["connection_intent"] = Value::String("personal".to_owned());
        fs::write(&policy_path, serde_json::to_string(&policy)?)?;
        let changed_policy = guard_file_findings_with_context(&manifest_json, Some(context));
        assert!(has_issue(
            &changed_policy,
            GuardManagedArtifact::VolicordPolicy,
            &policy_path,
            GuardArtifactIssue::ContentMismatch,
        ));
        fs::write(&policy_path, policy_text)?;

        let wrapper_path = repo_root.join(".codex/hooks/volicord-pre-tool.sh");
        let wrapper_text = fs::read_to_string(&wrapper_path)?;
        let changed_policy_hash =
            wrapper_text.replacen("# policy_hash=sha256:", "# policy_hash=sha256:1", 1);
        assert_ne!(changed_policy_hash, wrapper_text);
        fs::write(&wrapper_path, changed_policy_hash)?;
        let policy_hash_mismatch = guard_file_findings_with_context(&manifest_json, Some(context));
        assert_eq!(
            policy_hash_mismatch.hook_path_safety.state,
            HookPathSafetyState::Failed
        );
        assert!(policy_hash_mismatch
            .hook_path_safety
            .evidence
            .iter()
            .any(|evidence| evidence.reason == HookPathSafetyEvidenceReason::PolicyHashMismatch));
        fs::write(&wrapper_path, &wrapper_text)?;

        let changed_wrapper_text = wrapper_text.replacen("exec ", "exec false # ", 1);
        assert_ne!(changed_wrapper_text, wrapper_text);
        fs::write(&wrapper_path, changed_wrapper_text)?;
        let changed_wrapper = guard_file_findings_with_context(&manifest_json, Some(context));
        assert!(has_issue(
            &changed_wrapper,
            GuardManagedArtifact::HostHookWrapper(GuardHookPhase::PreTool),
            &wrapper_path,
            GuardArtifactIssue::HookContractMismatch,
        ));
        assert!(has_issue(
            &changed_wrapper,
            GuardManagedArtifact::HostHookWrapper(GuardHookPhase::PreTool),
            &wrapper_path,
            GuardArtifactIssue::ContentMismatch,
        ));
        fs::write(&wrapper_path, &wrapper_text)?;

        let wrapper_without_marker =
            fs::read_to_string(&wrapper_path)?.replace("# VOLICORD_MANAGED_HOOK_WRAPPER\n", "");
        fs::write(&wrapper_path, wrapper_without_marker)?;
        let missing_marker = guard_file_findings_with_context(&manifest_json, Some(context));
        assert!(has_issue(
            &missing_marker,
            GuardManagedArtifact::HostHookWrapper(GuardHookPhase::PreTool),
            &wrapper_path,
            GuardArtifactIssue::Malformed,
        ));
        fs::write(&wrapper_path, &wrapper_text)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let mut permissions = fs::metadata(&wrapper_path)?.permissions();
            let original_mode = permissions.mode();
            permissions.set_mode(original_mode & !0o111);
            fs::set_permissions(&wrapper_path, permissions)?;
            let permission_mismatch =
                guard_file_findings_with_context(&manifest_json, Some(context));
            assert!(has_issue(
                &permission_mismatch,
                GuardManagedArtifact::HostHookWrapper(GuardHookPhase::PreTool),
                &wrapper_path,
                GuardArtifactIssue::PermissionMismatch,
            ));
            let mut permissions = fs::metadata(&wrapper_path)?.permissions();
            permissions.set_mode(original_mode);
            fs::set_permissions(&wrapper_path, permissions)?;
        }

        let hook_config_path = repo_root.join(".codex/hooks.json");
        let hook_config_text = fs::read_to_string(&hook_config_path)?;
        fs::write(&hook_config_path, "not-json")?;
        let malformed = guard_file_findings_with_context(&manifest_json, Some(context));
        assert!(has_issue(
            &malformed,
            GuardManagedArtifact::HostHookConfig,
            &hook_config_path,
            GuardArtifactIssue::Malformed,
        ));
        fs::write(&hook_config_path, hook_config_text)?;

        let dispatch_path = repo_root.join(".codex/hooks/volicord-dispatch.sh");
        let dispatch_text = fs::read_to_string(&dispatch_path)?;
        let wrong_dispatch_root = dispatch_text.replacen("git rev-parse --show-toplevel", "pwd", 1);
        assert_ne!(wrong_dispatch_root, dispatch_text);
        fs::write(&dispatch_path, wrong_dispatch_root)?;
        let wrong_dispatch = guard_file_findings_with_context(&manifest_json, Some(context));
        assert_eq!(
            wrong_dispatch.hook_path_safety.state,
            HookPathSafetyState::Failed
        );
        assert!(wrong_dispatch
            .hook_path_safety
            .evidence
            .iter()
            .any(|evidence| {
                evidence.reason == HookPathSafetyEvidenceReason::NoncanonicalRootResolution
            }));
        fs::write(&dispatch_path, dispatch_text)?;

        let owner_binding_context = GuardAuthorityContext {
            connection_internal_id: "connection_other",
            ..context
        };
        let owner_binding_mismatch =
            guard_file_findings_with_context(&manifest_json, Some(owner_binding_context));
        assert_eq!(
            owner_binding_mismatch.hook_path_safety.state,
            HookPathSafetyState::Failed
        );
        assert!(owner_binding_mismatch
            .hook_path_safety
            .evidence
            .iter()
            .any(|evidence| evidence.reason == HookPathSafetyEvidenceReason::OwnerBindingMismatch));

        let missing_path = repo_root.join(".codex/rules/volicord.rules");
        fs::remove_file(&missing_path)?;
        let missing = guard_file_findings_with_context(&manifest_json, Some(context));
        assert!(has_issue(
            &missing,
            GuardManagedArtifact::HostRuleInstruction,
            &missing_path,
            GuardArtifactIssue::Missing,
        ));

        let mut owner_mismatch: Value = serde_json::from_str(&manifest_json)?;
        owner_mismatch["connection_id"] = Value::String("connection_other".to_owned());
        let owner_mismatch = guard_file_findings_with_context(
            &serde_json::to_string(&owner_mismatch)?,
            Some(context),
        );
        assert!(owner_mismatch
            .manifest_issues
            .contains(&GuardManifestIssue::Malformed));
        assert_eq!(fs::read_to_string(unrelated_path)?, "user-owned\n");
        Ok(())
    }

    #[test]
    fn hook_path_safety_states_and_aggregation_preserve_exact_knowledge() {
        let not_checked = HookPathSafetyAssessment::not_checked();
        assert_eq!(not_checked.state, HookPathSafetyState::NotChecked);
        assert!(not_checked
            .evidence
            .iter()
            .any(|evidence| evidence.reason == HookPathSafetyEvidenceReason::AuditNotRun));

        let not_applicable =
            HookPathSafetyAssessment::test_state(HookPathSafetyState::NotApplicable);
        assert_eq!(
            not_applicable.cwd_independence,
            HookPathSafetyState::NotApplicable
        );
        assert_eq!(
            not_applicable.subdirectory_safety,
            HookPathSafetyState::NotApplicable
        );

        let verified = HookPathSafetyAssessment::test_state(HookPathSafetyState::Verified);
        let failed = HookPathSafetyAssessment::test_failed(
            HookPathSafetyEvidenceReason::NoncanonicalManagedCommand,
        );
        let not_recorded = HookPathSafetyAssessment::test_state(HookPathSafetyState::NotRecorded);

        let mut failed_aggregate = verified.clone();
        failed_aggregate.merge(failed.clone());
        assert_eq!(failed_aggregate.state, HookPathSafetyState::Failed);

        let mut not_recorded_aggregate = verified.clone();
        not_recorded_aggregate.merge(not_recorded.clone());
        assert_eq!(
            not_recorded_aggregate.state,
            HookPathSafetyState::NotRecorded
        );

        let mut verified_with_not_applicable = verified.clone();
        verified_with_not_applicable.merge(not_applicable.clone());
        assert_eq!(
            verified_with_not_applicable.state,
            HookPathSafetyState::Verified
        );

        let mut all_not_applicable = HookPathSafetyAssessment::default();
        all_not_applicable.merge(not_applicable.clone());
        all_not_applicable.merge(not_applicable);
        assert_eq!(all_not_applicable.state, HookPathSafetyState::NotApplicable);

        let mut unavailable_aggregate = verified.clone();
        unavailable_aggregate.merge(HookPathSafetyAssessment::test_state(
            HookPathSafetyState::NotChecked,
        ));
        assert_eq!(unavailable_aggregate.state, HookPathSafetyState::NotChecked);

        let mut forward = HookPathSafetyAssessment::default();
        for assessment in [verified.clone(), not_recorded.clone(), failed.clone()] {
            forward.merge(assessment);
        }
        let mut reverse = HookPathSafetyAssessment::default();
        for assessment in [failed, not_recorded, verified] {
            reverse.merge(assessment);
        }
        assert_eq!(forward, reverse);
        assert_eq!(
            serde_json::to_value(forward).expect("forward assessment JSON"),
            serde_json::to_value(reverse).expect("reverse assessment JSON")
        );

        let mut bounded = HookPathSafetyAssessment::default();
        for index in (0..20).rev() {
            let mut assessment = HookPathSafetyAssessment::test_failed(
                HookPathSafetyEvidenceReason::ContentMismatch,
            );
            assessment.evidence[0].path = Some(format!("artifact-{index:02}"));
            bounded.merge(assessment);
        }
        assert_eq!(bounded.evidence.len(), MAX_HOOK_PATH_SAFETY_EVIDENCE);
        assert!(bounded.evidence.windows(2).all(|pair| pair[0] <= pair[1]));
    }
}
