use std::collections::{BTreeMap, BTreeSet};

use volicord_mcp_protocol::ProtocolRegistry;
use volicord_types::{
    mcp_verification_evidence::{
        McpActiveVerificationEvidence, McpActiveVerificationSource, McpEvidenceCheckStatus,
        McpHostCompatibilityEvidence, McpProbeEvidence, McpRevisionConformance, McpSideEffectKind,
    },
    values::{GuardHookPhase, UtcTimestamp},
};

use crate::{
    connection_command::McpStage,
    guard_integration::audit::{
        HookPathSafetyAssessment, HookPathSafetyEvidenceReason, HookPathSafetyEvidenceSource,
        HookPathSafetyState, MAX_HOOK_PATH_SAFETY_EVIDENCE,
    },
};

use super::semantics::{ActiveVerificationState, StorageWriteabilityState};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct McpActiveVerificationHumanProjection {
    pub(super) state: ActiveVerificationState,
    pub(super) storage_writeability: StorageWriteabilityState,
    pub(super) observed_at: Option<UtcTimestamp>,
    pub(super) source: Option<McpActiveVerificationSource>,
    pub(super) store: Option<StoreWriteabilityHumanProjection>,
    pub(super) side_effect_groups: Vec<&'static str>,
    pub(super) protocol: Option<ProtocolConformanceHumanProjection>,
    pub(super) host: Option<HostCompatibilityHumanProjection>,
}

impl McpActiveVerificationHumanProjection {
    pub(super) const fn not_run() -> Self {
        Self {
            state: ActiveVerificationState::NotRun,
            storage_writeability: StorageWriteabilityState::NotChecked,
            observed_at: None,
            source: None,
            store: None,
            side_effect_groups: Vec::new(),
            protocol: None,
            host: None,
        }
    }

    pub(super) fn try_from_evidence(
        evidence: &McpActiveVerificationEvidence,
    ) -> Result<Self, String> {
        let store = StoreWriteabilityHumanProjection::from_evidence(evidence);
        let protocol =
            ProtocolConformanceHumanProjection::try_from_evidence(evidence.protocol_conformance())?;
        let host =
            HostCompatibilityHumanProjection::try_from_evidence(evidence.host_compatibility())?;
        let active_passed = store.all_passed
            && protocol.passed_count == protocol.revisions.len()
            && host.passed_count == host.profiles.len();

        let mut side_effect_groups = Vec::new();
        if evidence.side_effects().iter().any(|effect| {
            matches!(
                effect,
                McpSideEffectKind::RollbackOnlyRegistryWriteProbe
                    | McpSideEffectKind::RollbackOnlyProjectWriteProbe
            )
        }) {
            side_effect_groups.push("rollback-only Store probes");
        }
        if evidence.side_effects().iter().any(|effect| {
            matches!(
                effect,
                McpSideEffectKind::DisposableProtocolConformance
                    | McpSideEffectKind::DisposableHostCompatibility
            )
        }) {
            side_effect_groups.push("disposable conformance sessions");
        }

        Ok(Self {
            state: if active_passed {
                ActiveVerificationState::Passed
            } else {
                ActiveVerificationState::Failed
            },
            storage_writeability: if store.all_passed {
                StorageWriteabilityState::Passed
            } else {
                StorageWriteabilityState::Failed
            },
            observed_at: Some(evidence.observed_at().clone()),
            source: Some(evidence.source()),
            store: Some(store),
            side_effect_groups,
            protocol: Some(protocol),
            host: Some(host),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StoreWriteabilityHumanProjection {
    pub(super) registry: McpEvidenceCheckStatus,
    pub(super) projects: Vec<ProjectWriteabilityHumanProjection>,
    pub(super) passed_count: usize,
    pub(super) all_passed: bool,
}

impl StoreWriteabilityHumanProjection {
    fn from_evidence(evidence: &McpActiveVerificationEvidence) -> Self {
        let mut projects = evidence
            .project_writes()
            .iter()
            .map(|project| ProjectWriteabilityHumanProjection {
                project_id: project.project_id().to_owned(),
                status: project.state_write(),
            })
            .collect::<Vec<_>>();
        projects.sort_by(|left, right| left.project_id.cmp(&right.project_id));
        let registry = evidence.registry_write();
        let passed_count = usize::from(registry == McpEvidenceCheckStatus::Passed)
            + projects
                .iter()
                .filter(|project| project.status == McpEvidenceCheckStatus::Passed)
                .count();
        let all_passed = passed_count == projects.len() + 1;
        Self {
            registry,
            projects,
            passed_count,
            all_passed,
        }
    }

    pub(super) fn total_count(&self) -> usize {
        self.projects.len() + 1
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ProjectWriteabilityHumanProjection {
    pub(super) project_id: String,
    pub(super) status: McpEvidenceCheckStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ProbeDisclosureState {
    Passed,
    Failed,
    Contradictory,
}

impl ProbeDisclosureState {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Contradictory => "contradictory",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ProtocolConformanceHumanProjection {
    pub(super) passed_count: usize,
    pub(super) revisions: Vec<ProtocolRevisionHumanProjection>,
    pub(super) designated_read_only_tool: Option<String>,
}

impl ProtocolConformanceHumanProjection {
    fn try_from_evidence(evidence: &[McpRevisionConformance]) -> Result<Self, String> {
        let canonical_order = ProtocolRegistry::production()
            .oldest_to_newest()
            .enumerate()
            .map(|(index, profile)| (profile.revision().as_str(), index))
            .collect::<BTreeMap<_, _>>();
        let mut seen = BTreeSet::new();
        let mut revisions = Vec::with_capacity(evidence.len());
        for item in evidence {
            let Some(order) = canonical_order.get(item.revision()).copied() else {
                return Err(format!(
                    "active-verification protocol evidence names unsupported revision {}",
                    item.revision()
                ));
            };
            if !seen.insert(item.revision()) {
                return Err(format!(
                    "active-verification protocol evidence repeats revision {}",
                    item.revision()
                ));
            }
            revisions.push(ProtocolRevisionHumanProjection::try_from_evidence(
                order, item,
            )?);
        }
        revisions.sort_by_key(|revision| revision.canonical_order);
        let passed_count = revisions
            .iter()
            .filter(|revision| revision.disclosure == ProbeDisclosureState::Passed)
            .count();
        let tools = revisions
            .iter()
            .map(|revision| revision.probe.safe_read_only_tool.as_str())
            .collect::<BTreeSet<_>>();
        let designated_read_only_tool =
            (tools.len() == 1).then(|| tools.first().copied().unwrap_or_default().to_owned());
        Ok(Self {
            passed_count,
            revisions,
            designated_read_only_tool,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ProtocolRevisionHumanProjection {
    canonical_order: usize,
    pub(super) revision: String,
    pub(super) disclosure: ProbeDisclosureState,
    pub(super) probe: ProbeHumanProjection,
}

impl ProtocolRevisionHumanProjection {
    fn try_from_evidence(
        canonical_order: usize,
        evidence: &McpRevisionConformance,
    ) -> Result<Self, String> {
        let probe = ProbeHumanProjection::try_from_evidence(evidence.probe())?;
        let disclosure = probe.disclosure(Some(evidence.revision()));
        Ok(Self {
            canonical_order,
            revision: evidence.revision().to_owned(),
            disclosure,
            probe,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct HostCompatibilityHumanProjection {
    pub(super) passed_count: usize,
    pub(super) profiles: Vec<HostProfileHumanProjection>,
}

impl HostCompatibilityHumanProjection {
    fn try_from_evidence(evidence: &[McpHostCompatibilityEvidence]) -> Result<Self, String> {
        let mut seen = BTreeSet::new();
        let mut profiles = Vec::with_capacity(evidence.len());
        for item in evidence {
            let identity = (item.profile(), item.fixture());
            if !seen.insert(identity) {
                return Err(format!(
                    "active-verification host evidence repeats profile {} fixture {}",
                    item.profile(),
                    item.fixture()
                ));
            }
            let probe = ProbeHumanProjection::try_from_evidence(item.probe())?;
            let disclosure = probe.disclosure(None);
            profiles.push(HostProfileHumanProjection {
                profile: item.profile().to_owned(),
                fixture: item.fixture().to_owned(),
                disclosure,
                probe,
            });
        }
        profiles.sort_by(|left, right| {
            (&left.profile, &left.fixture).cmp(&(&right.profile, &right.fixture))
        });
        let passed_count = profiles
            .iter()
            .filter(|profile| profile.disclosure == ProbeDisclosureState::Passed)
            .count();
        Ok(Self {
            passed_count,
            profiles,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct HostProfileHumanProjection {
    pub(super) profile: String,
    pub(super) fixture: String,
    pub(super) disclosure: ProbeDisclosureState,
    pub(super) probe: ProbeHumanProjection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ProbeHumanProjection {
    pub(super) status: McpEvidenceCheckStatus,
    pub(super) requested_revision: Option<String>,
    pub(super) negotiated_revision: Option<String>,
    pub(super) initialize: bool,
    pub(super) initialized_notification: bool,
    pub(super) schema_validation: bool,
    pub(super) tools_list_observed: bool,
    pub(super) tools_returned: Option<usize>,
    pub(super) required_tools_validated: bool,
    pub(super) safe_read_only_tool: String,
    pub(super) safe_read_only_tool_completed: bool,
    pub(super) shutdown_completed: bool,
    pub(super) diagnostic_code: Option<String>,
    pub(super) failure_stage: Option<McpStage>,
    pub(super) finding_id: Option<String>,
}

impl ProbeHumanProjection {
    fn try_from_evidence(evidence: &McpProbeEvidence) -> Result<Self, String> {
        let failure_stage = evidence
            .failure_stage()
            .map(|stage| match stage {
                "startup" => Ok(McpStage::Startup),
                "initialize" => Ok(McpStage::Initialize),
                "tools_list" => Ok(McpStage::ToolsList),
                "safe_tool_call" => Ok(McpStage::SafeToolCall),
                "shutdown" => Ok(McpStage::Shutdown),
                _ => Err(format!(
                    "active-verification probe has unknown failure stage {stage}"
                )),
            })
            .transpose()?;
        Ok(Self {
            status: evidence.status(),
            requested_revision: evidence.requested_revision().map(str::to_owned),
            negotiated_revision: evidence.negotiated_revision().map(str::to_owned),
            initialize: evidence.initialize(),
            initialized_notification: evidence.initialized_notification(),
            schema_validation: evidence.schema_validation(),
            tools_list_observed: evidence.tools_list_observed(),
            tools_returned: evidence.tools_returned(),
            required_tools_validated: evidence.required_tools_validated(),
            safe_read_only_tool: evidence.safe_read_only_tool().to_owned(),
            safe_read_only_tool_completed: evidence.safe_read_only_tool_completed(),
            shutdown_completed: evidence.shutdown_completed(),
            diagnostic_code: evidence.diagnostic_code().map(str::to_owned),
            failure_stage,
            finding_id: evidence.finding_id().map(str::to_owned),
        })
    }

    fn disclosure(&self, expected_revision: Option<&str>) -> ProbeDisclosureState {
        let revisions_consistent = self.requested_revision.as_deref().is_some_and(|requested| {
            self.negotiated_revision.as_deref() == Some(requested)
                && expected_revision.is_none_or(|expected| expected == requested)
        });
        let lifecycle_complete = revisions_consistent
            && self.initialize
            && self.initialized_notification
            && self.schema_validation
            && self.tools_list_observed
            && self.tools_returned.is_some()
            && self.required_tools_validated
            && !self.safe_read_only_tool.is_empty()
            && self.safe_read_only_tool_completed
            && self.shutdown_completed;
        let has_failure = self.failure_stage.is_some()
            || self.diagnostic_code.is_some()
            || self.finding_id.is_some();
        match (self.status, lifecycle_complete, has_failure) {
            (McpEvidenceCheckStatus::Passed, true, false) => ProbeDisclosureState::Passed,
            (McpEvidenceCheckStatus::Failed, false, _) => ProbeDisclosureState::Failed,
            _ => ProbeDisclosureState::Contradictory,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct HookPathSafetyHumanProjection {
    pub(super) state: HookPathSafetyState,
    pub(super) cwd_independence: HookPathSafetyState,
    pub(super) subdirectory_safety: HookPathSafetyState,
    pub(super) evidence_count: usize,
    pub(super) verified_count: usize,
    pub(super) source_counts: BTreeMap<HookPathSafetyEvidenceSource, usize>,
    pub(super) expanded_evidence: Vec<HookPathSafetyEvidenceHumanProjection>,
    pub(super) healthy: bool,
    pub(super) contradictory: bool,
    pub(super) collection_at_limit: bool,
}

impl HookPathSafetyHumanProjection {
    pub(super) fn from_assessment(assessment: &HookPathSafetyAssessment) -> Self {
        let evidence = assessment.evidence();
        let all_evidence_verified = !evidence.is_empty()
            && evidence
                .iter()
                .all(|item| item.state() == HookPathSafetyState::Verified);
        let dimensions_verified = assessment.state() == HookPathSafetyState::Verified
            && assessment.cwd_independence() == HookPathSafetyState::Verified
            && assessment.subdirectory_safety() == HookPathSafetyState::Verified;
        let healthy = dimensions_verified && all_evidence_verified;
        let has_applicable = evidence
            .iter()
            .any(|item| item.state() != HookPathSafetyState::NotApplicable);
        let has_not_applicable = evidence
            .iter()
            .any(|item| item.state() == HookPathSafetyState::NotApplicable);
        let mixed_applicability = has_applicable && has_not_applicable;
        let mut expanded_evidence = evidence
            .iter()
            .filter(|item| {
                matches!(
                    item.state(),
                    HookPathSafetyState::Failed
                        | HookPathSafetyState::NotRecorded
                        | HookPathSafetyState::NotChecked
                ) || (item.state() == HookPathSafetyState::NotApplicable && mixed_applicability)
            })
            .map(HookPathSafetyEvidenceHumanProjection::from_evidence)
            .collect::<Vec<_>>();
        let child_evidence_verified = assessment.cwd_independence()
            == HookPathSafetyState::Verified
            && assessment.subdirectory_safety() == HookPathSafetyState::Verified
            && all_evidence_verified;
        let contradictory =
            (assessment.state() == HookPathSafetyState::Verified) != child_evidence_verified;
        if !healthy && expanded_evidence.is_empty() && contradictory {
            expanded_evidence = evidence
                .iter()
                .map(HookPathSafetyEvidenceHumanProjection::from_evidence)
                .collect();
        }
        expanded_evidence.sort_by(|left, right| {
            (
                hook_path_safety_state_rank(left.state),
                left.source,
                left.reason,
                &left.installation_id,
                left.phase,
                &left.path,
            )
                .cmp(&(
                    hook_path_safety_state_rank(right.state),
                    right.source,
                    right.reason,
                    &right.installation_id,
                    right.phase,
                    &right.path,
                ))
        });
        let mut source_counts = BTreeMap::new();
        for item in evidence {
            *source_counts.entry(item.source()).or_default() += 1;
        }
        let verified_count = evidence
            .iter()
            .filter(|item| item.state() == HookPathSafetyState::Verified)
            .count();
        Self {
            state: assessment.state(),
            cwd_independence: assessment.cwd_independence(),
            subdirectory_safety: assessment.subdirectory_safety(),
            evidence_count: evidence.len(),
            verified_count,
            source_counts,
            expanded_evidence,
            healthy,
            contradictory,
            collection_at_limit: evidence.len() == MAX_HOOK_PATH_SAFETY_EVIDENCE,
        }
    }
}

fn hook_path_safety_state_rank(state: HookPathSafetyState) -> u8 {
    match state {
        HookPathSafetyState::Failed => 0,
        HookPathSafetyState::NotRecorded => 1,
        HookPathSafetyState::NotChecked => 2,
        HookPathSafetyState::NotApplicable => 3,
        HookPathSafetyState::Verified => 4,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct HookPathSafetyEvidenceHumanProjection {
    pub(super) state: HookPathSafetyState,
    pub(super) source: HookPathSafetyEvidenceSource,
    pub(super) reason: HookPathSafetyEvidenceReason,
    pub(super) installation_id: Option<String>,
    pub(super) phase: Option<GuardHookPhase>,
    pub(super) path: Option<String>,
}

impl HookPathSafetyEvidenceHumanProjection {
    fn from_evidence(evidence: &crate::guard_integration::audit::HookPathSafetyEvidence) -> Self {
        Self {
            state: evidence.state(),
            source: evidence.source(),
            reason: evidence.reason(),
            installation_id: evidence.installation_id().map(str::to_owned),
            phase: evidence.phase(),
            path: evidence.path().map(str::to_owned),
        }
    }
}
