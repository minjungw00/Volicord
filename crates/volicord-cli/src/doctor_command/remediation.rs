//! Canonical remediation planning for the Doctor administrative command.

use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use serde::Serialize;
#[cfg(test)]
use volicord_types::diagnostics::DiagnosticAction as FindingAction;
use volicord_types::diagnostics::{
    DiagnosticCode, DiagnosticFinding, DiagnosticFindingId, DiagnosticSeverity,
};

const MAX_DOCTOR_REMEDIATION_ACTIONS: usize = 128;
const MAX_DOCTOR_ACTION_PROVENANCE: usize = 128;
const MAX_DOCTOR_CHECK_ID_BYTES: usize = 128;
const MAX_DOCTOR_COMMAND_BYTES: usize = 4_096;

/// Whether Doctor requires a remediation or recommends it as follow-up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum DoctorRemediationUrgency {
    Recommended,
    Required,
}

impl DoctorRemediationUrgency {
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::Recommended => "recommended",
            Self::Required => "required",
        }
    }
}

/// Closed semantic priority used after urgency and before canonical code order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum DoctorRemediationPriority {
    FollowUp,
    Standard,
    High,
    Immediate,
}

impl DoctorRemediationPriority {
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::FollowUp => "follow_up",
            Self::Standard => "standard",
            Self::High => "high",
            Self::Immediate => "immediate",
        }
    }
}

/// Bounded source coordinates retained by a finalized Doctor action.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(super) enum DoctorRemediationProvenance {
    Finding { finding_id: DiagnosticFindingId },
    Check { check_id: String },
}

/// Doctor-owned action meanings that do not have a structured finding owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DoctorDirectAction {
    InitializeConnection,
    ProtectPersonalLocalFiles,
    RepairIntegrationIntentDrift,
    RepairProjectPolicy,
    RepairGuardFiles,
    RepairGuardHookPathSafety,
    RepairMcpCommand,
    MakeProfileCommandsAvailable,
    RepairCommandLinks,
    AddCommandLinksToPath,
    CreateCommandLinks,
}

impl DoctorDirectAction {
    const fn code(self) -> &'static str {
        match self {
            Self::InitializeConnection => "action.setup.initialize_connection",
            Self::ProtectPersonalLocalFiles => "action.repository.protect_personal_local_files",
            Self::RepairIntegrationIntentDrift => {
                "action.connection.repair_integration_intent_drift"
            }
            Self::RepairProjectPolicy => "action.policy.repair_project_authority",
            Self::RepairGuardFiles => "action.guard.repair_files",
            Self::RepairGuardHookPathSafety => "action.guard.repair_hook_path_safety",
            Self::RepairMcpCommand => "action.installation.repair_mcp_command",
            Self::MakeProfileCommandsAvailable => {
                "action.installation.make_profile_commands_available"
            }
            Self::RepairCommandLinks => "action.installation.repair_command_links",
            Self::AddCommandLinksToPath => "action.installation.add_command_links_to_path",
            Self::CreateCommandLinks => "action.installation.create_command_links",
        }
    }

    const fn summary(self) -> &'static str {
        match self {
            Self::InitializeConnection => {
                "Initialize the Codex connection from the Product Repository"
            }
            Self::ProtectPersonalLocalFiles => {
                "Review affected repositories, restore repository-local excludes with the intended connection setup, and remove local-only paths from the Git index without deleting their working-tree files"
            }
            Self::RepairIntegrationIntentDrift => {
                "Rerun Codex Record initialization with the intended connection intent so local policy and enabled inventory converge"
            }
            Self::RepairProjectPolicy => {
                "Inspect the authoritative project policy and apply one validated canonical policy file"
            }
            Self::RepairGuardFiles => {
                "Reinstall the Codex Record Guard files for the affected Product Repository"
            }
            Self::RepairGuardHookPathSafety => {
                "Regenerate cwd-independent Codex Record Guard commands for the affected Product Repository"
            }
            Self::RepairMcpCommand => {
                "Select an executable MCP launch command and rerun initialization with that command"
            }
            Self::MakeProfileCommandsAvailable => {
                "Make the installation-profile commands resolve on PATH and reload existing agent hosts after PATH or command-link changes"
            }
            Self::RepairCommandLinks => {
                "Repair the installation command links or reinstall the Volicord executable on PATH, then reload existing agent hosts"
            }
            Self::AddCommandLinksToPath => {
                "Add the installation command-link directory to PATH before starting new shells or agent hosts"
            }
            Self::CreateCommandLinks => {
                "Install the Volicord executable in a command directory kept on PATH, then reload existing agent hosts"
            }
        }
    }

    const fn priority(self) -> DoctorRemediationPriority {
        match self {
            Self::InitializeConnection | Self::RepairGuardFiles | Self::RepairMcpCommand => {
                DoctorRemediationPriority::Immediate
            }
            Self::RepairProjectPolicy | Self::RepairGuardHookPathSafety => {
                DoctorRemediationPriority::High
            }
            Self::ProtectPersonalLocalFiles
            | Self::RepairIntegrationIntentDrift
            | Self::MakeProfileCommandsAvailable
            | Self::RepairCommandLinks
            | Self::AddCommandLinksToPath
            | Self::CreateCommandLinks => DoctorRemediationPriority::Standard,
        }
    }

    const fn urgency(self) -> DoctorRemediationUrgency {
        match self {
            Self::InitializeConnection
            | Self::RepairProjectPolicy
            | Self::RepairGuardFiles
            | Self::RepairGuardHookPathSafety
            | Self::RepairMcpCommand => DoctorRemediationUrgency::Required,
            Self::ProtectPersonalLocalFiles
            | Self::RepairIntegrationIntentDrift
            | Self::MakeProfileCommandsAvailable
            | Self::RepairCommandLinks
            | Self::AddCommandLinksToPath
            | Self::CreateCommandLinks => DoctorRemediationUrgency::Recommended,
        }
    }
}

#[derive(Debug, Clone)]
enum CandidateActionCode {
    Typed(DiagnosticCode),
    Registered(&'static str),
}

impl CandidateActionCode {
    fn finalize(self) -> Result<DiagnosticCode, DoctorRemediationError> {
        let code = match self {
            Self::Typed(code) => code,
            Self::Registered(code) => DiagnosticCode::parse(code)
                .map_err(|error| DoctorRemediationError::InvalidActionCode(error.to_string()))?,
        };
        if code.namespace() != "action" {
            return Err(DoctorRemediationError::InvalidActionCode(format!(
                "{} is not in the action namespace",
                code.as_str()
            )));
        }
        Ok(code)
    }
}

/// One unfinalized action from a Doctor-owned check or another typed owner.
#[derive(Debug, Clone)]
pub(super) struct DoctorActionCandidate {
    code: CandidateActionCode,
    summary: String,
    command: Option<String>,
    urgency: DoctorRemediationUrgency,
    priority: DoctorRemediationPriority,
    provenance: DoctorRemediationProvenance,
}

impl DoctorActionCandidate {
    pub(super) fn direct(
        action: DoctorDirectAction,
        check_id: impl Into<String>,
        command: Option<String>,
    ) -> Self {
        Self {
            code: CandidateActionCode::Registered(action.code()),
            summary: action.summary().to_owned(),
            command,
            urgency: action.urgency(),
            priority: action.priority(),
            provenance: DoctorRemediationProvenance::Check {
                check_id: check_id.into(),
            },
        }
    }

    #[cfg(test)]
    pub(super) fn registered(
        action_code: &'static str,
        summary: &'static str,
        urgency: DoctorRemediationUrgency,
        priority: DoctorRemediationPriority,
        check_id: impl Into<String>,
        command: Option<String>,
    ) -> Self {
        Self {
            code: CandidateActionCode::Registered(action_code),
            summary: summary.to_owned(),
            command,
            urgency,
            priority,
            provenance: DoctorRemediationProvenance::Check {
                check_id: check_id.into(),
            },
        }
    }

    #[cfg(test)]
    pub(super) fn from_finding_action(
        action: &FindingAction,
        urgency: DoctorRemediationUrgency,
        priority: DoctorRemediationPriority,
        check_id: impl Into<String>,
        command: Option<String>,
    ) -> Self {
        Self {
            code: CandidateActionCode::Typed(action.code().clone()),
            summary: action.summary().to_owned(),
            command,
            urgency,
            priority,
            provenance: DoctorRemediationProvenance::Check {
                check_id: check_id.into(),
            },
        }
    }

    #[cfg(test)]
    pub(super) fn with_summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = summary.into();
        self
    }
}

/// One normalized action in the finalized Doctor remediation plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct DoctorRemediationAction {
    code: DiagnosticCode,
    summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    command: Option<String>,
    urgency: DoctorRemediationUrgency,
    priority: DoctorRemediationPriority,
    provenance: BTreeSet<DoctorRemediationProvenance>,
}

impl DoctorRemediationAction {
    pub(super) fn code(&self) -> &DiagnosticCode {
        &self.code
    }

    pub(super) fn summary(&self) -> &str {
        &self.summary
    }

    pub(super) fn command(&self) -> Option<&str> {
        self.command.as_deref()
    }

    pub(super) const fn urgency(&self) -> DoctorRemediationUrgency {
        self.urgency
    }

    pub(super) const fn priority(&self) -> DoctorRemediationPriority {
        self.priority
    }

    pub(super) fn has_check_provenance(&self, check_id: &str) -> bool {
        self.provenance.iter().any(|provenance| {
            matches!(
                provenance,
                DoctorRemediationProvenance::Check {
                    check_id: candidate
                } if candidate == check_id
            )
        })
    }

    pub(super) fn has_finding_provenance(&self, finding_ids: &BTreeSet<&str>) -> bool {
        self.provenance.iter().any(|provenance| {
            matches!(
                provenance,
                DoctorRemediationProvenance::Finding { finding_id }
                    if finding_ids.contains(finding_id.as_str())
            )
        })
    }
}

/// The single deterministic source for every Doctor remediation projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DoctorRemediationPlan {
    actions: Vec<DoctorRemediationAction>,
}

impl DoctorRemediationPlan {
    pub(super) fn finalize(
        findings: &[DiagnosticFinding],
        direct_candidates: Vec<DoctorActionCandidate>,
    ) -> Result<Self, DoctorRemediationError> {
        let mut candidates = Vec::new();
        for finding in findings {
            let (urgency, priority) = finding_action_policy(finding.severity());
            candidates.extend(
                finding
                    .actions()
                    .iter()
                    .map(|action| DoctorActionCandidate {
                        code: CandidateActionCode::Typed(action.code().clone()),
                        summary: action.summary().to_owned(),
                        command: None,
                        urgency,
                        priority,
                        provenance: DoctorRemediationProvenance::Finding {
                            finding_id: finding.id().clone(),
                        },
                    }),
            );
        }
        candidates.extend(direct_candidates);

        let mut merged = BTreeMap::<DiagnosticCode, DoctorRemediationAction>::new();
        for candidate in candidates {
            let code = candidate.code.finalize()?;
            let summary = normalize_summary(&candidate.summary)?;
            let command = normalize_command(candidate.command)?;
            validate_provenance(&candidate.provenance)?;
            let action = merged
                .entry(code.clone())
                .or_insert_with(|| DoctorRemediationAction {
                    code,
                    summary: summary.clone(),
                    command: command.clone(),
                    urgency: candidate.urgency,
                    priority: candidate.priority,
                    provenance: BTreeSet::new(),
                });
            if action.summary != summary {
                return Err(DoctorRemediationError::ConflictingSummaries {
                    code: action.code.as_str().to_owned(),
                    first: action.summary.clone(),
                    second: summary,
                });
            }
            action.command = merge_commands(action.code.as_str(), action.command.take(), command)?;
            action.urgency = action.urgency.max(candidate.urgency);
            action.priority = action.priority.max(candidate.priority);
            action.provenance.insert(candidate.provenance);
            if action.provenance.len() > MAX_DOCTOR_ACTION_PROVENANCE {
                return Err(DoctorRemediationError::TooManyProvenanceEntries {
                    code: action.code.as_str().to_owned(),
                });
            }
        }
        if merged.len() > MAX_DOCTOR_REMEDIATION_ACTIONS {
            return Err(DoctorRemediationError::TooManyActions);
        }

        let mut actions = merged.into_values().collect::<Vec<_>>();
        actions.sort_by(remediation_action_order);
        let plan = Self { actions };
        plan.validate_finding_coverage(findings)?;
        Ok(plan)
    }

    pub(super) fn actions(&self) -> &[DoctorRemediationAction] {
        &self.actions
    }

    pub(super) fn required_actions(&self) -> impl Iterator<Item = &DoctorRemediationAction> {
        self.actions
            .iter()
            .filter(|action| action.urgency == DoctorRemediationUrgency::Required)
    }

    pub(super) fn recommended_actions(&self) -> impl Iterator<Item = &DoctorRemediationAction> {
        self.actions
            .iter()
            .filter(|action| action.urgency == DoctorRemediationUrgency::Recommended)
    }

    pub(super) fn primary_action(&self) -> Option<&DoctorRemediationAction> {
        self.actions.first()
    }

    pub(super) fn has_required_actions(&self) -> bool {
        self.required_actions().next().is_some()
    }

    pub(super) fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    pub(super) fn validate_check_provenance(
        &self,
        check_ids: &BTreeSet<&str>,
    ) -> Result<(), DoctorRemediationError> {
        for provenance in self
            .actions
            .iter()
            .flat_map(|action| action.provenance.iter())
        {
            let DoctorRemediationProvenance::Check { check_id } = provenance else {
                continue;
            };
            if !check_ids.contains(check_id.as_str()) {
                return Err(DoctorRemediationError::UnknownCheckId(check_id.clone()));
            }
        }
        Ok(())
    }

    fn validate_finding_coverage(
        &self,
        findings: &[DiagnosticFinding],
    ) -> Result<(), DoctorRemediationError> {
        let plan_codes = self
            .actions
            .iter()
            .map(|action| action.code())
            .collect::<BTreeSet<_>>();
        for action in findings.iter().flat_map(DiagnosticFinding::actions) {
            if !plan_codes.contains(action.code()) {
                return Err(DoctorRemediationError::FindingActionMissing {
                    code: action.code().as_str().to_owned(),
                });
            }
        }
        Ok(())
    }
}

fn finding_action_policy(
    severity: DiagnosticSeverity,
) -> (DoctorRemediationUrgency, DoctorRemediationPriority) {
    match severity {
        DiagnosticSeverity::Error => (
            DoctorRemediationUrgency::Required,
            DoctorRemediationPriority::High,
        ),
        DiagnosticSeverity::Warning => (
            DoctorRemediationUrgency::Recommended,
            DoctorRemediationPriority::Standard,
        ),
        DiagnosticSeverity::Info => (
            DoctorRemediationUrgency::Recommended,
            DoctorRemediationPriority::FollowUp,
        ),
    }
}

fn normalize_summary(summary: &str) -> Result<String, DoctorRemediationError> {
    let summary = summary.trim();
    if summary.is_empty() {
        return Err(DoctorRemediationError::InvalidSummary);
    }
    Ok(summary.to_owned())
}

fn normalize_command(command: Option<String>) -> Result<Option<String>, DoctorRemediationError> {
    let Some(command) = command else {
        return Ok(None);
    };
    let command = command.trim();
    if command.is_empty() || command.len() > MAX_DOCTOR_COMMAND_BYTES {
        return Err(DoctorRemediationError::InvalidCommand);
    }
    Ok(Some(command.to_owned()))
}

fn validate_provenance(
    provenance: &DoctorRemediationProvenance,
) -> Result<(), DoctorRemediationError> {
    let DoctorRemediationProvenance::Check { check_id } = provenance else {
        return Ok(());
    };
    let valid = !check_id.is_empty()
        && check_id.len() <= MAX_DOCTOR_CHECK_ID_BYTES
        && check_id
            .bytes()
            .enumerate()
            .all(|(index, byte)| match byte {
                b'a'..=b'z' => true,
                b'0'..=b'9' | b'_' => index > 0,
                _ => false,
            });
    if valid {
        Ok(())
    } else {
        Err(DoctorRemediationError::InvalidCheckId(check_id.clone()))
    }
}

fn merge_commands(
    code: &str,
    first: Option<String>,
    second: Option<String>,
) -> Result<Option<String>, DoctorRemediationError> {
    match (first, second) {
        (None, command) | (command, None) => Ok(command),
        (Some(first), Some(second)) if first == second => Ok(Some(first)),
        (Some(first), Some(second)) => Err(DoctorRemediationError::ConflictingCommands {
            code: code.to_owned(),
            first,
            second,
        }),
    }
}

fn remediation_action_order(
    left: &DoctorRemediationAction,
    right: &DoctorRemediationAction,
) -> Ordering {
    right
        .urgency
        .cmp(&left.urgency)
        .then_with(|| right.priority.cmp(&left.priority))
        .then_with(|| left.code.cmp(&right.code))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum DoctorRemediationError {
    InvalidActionCode(String),
    InvalidSummary,
    InvalidCommand,
    InvalidCheckId(String),
    UnknownCheckId(String),
    ConflictingSummaries {
        code: String,
        first: String,
        second: String,
    },
    ConflictingCommands {
        code: String,
        first: String,
        second: String,
    },
    TooManyActions,
    TooManyProvenanceEntries {
        code: String,
    },
    FindingActionMissing {
        code: String,
    },
}

impl fmt::Display for DoctorRemediationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidActionCode(detail) => {
                write!(formatter, "invalid Doctor remediation action code: {detail}")
            }
            Self::InvalidSummary => {
                formatter.write_str("Doctor remediation summary must not be empty")
            }
            Self::InvalidCommand => formatter.write_str(
                "Doctor remediation command must be non-empty and within the size bound",
            ),
            Self::InvalidCheckId(check_id) => {
                write!(formatter, "invalid Doctor remediation check ID: {check_id}")
            }
            Self::UnknownCheckId(check_id) => write!(
                formatter,
                "Doctor remediation provenance references unknown check ID: {check_id}"
            ),
            Self::ConflictingSummaries {
                code,
                first,
                second,
            } => write!(
                formatter,
                "Doctor remediation action {code} has conflicting summaries: {first:?} and {second:?}"
            ),
            Self::ConflictingCommands {
                code,
                first,
                second,
            } => write!(
                formatter,
                "Doctor remediation action {code} has conflicting commands: {first:?} and {second:?}"
            ),
            Self::TooManyActions => formatter.write_str(
                "Doctor remediation plan exceeds the bounded action count",
            ),
            Self::TooManyProvenanceEntries { code } => write!(
                formatter,
                "Doctor remediation action {code} exceeds the bounded provenance count"
            ),
            Self::FindingActionMissing { code } => write!(
                formatter,
                "Doctor finding action {code} is absent from the finalized remediation plan"
            ),
        }
    }
}

impl std::error::Error for DoctorRemediationError {}

#[cfg(test)]
mod tests {
    use serde_json::to_value;
    use volicord_types::{
        diagnostics::{
            DiagnosticAction, DiagnosticDomain, DiagnosticFacts, DiagnosticSource, DiagnosticStage,
            DiagnosticSubject,
        },
        values::UtcTimestamp,
    };

    use super::*;

    fn action(code: &str, summary: &str) -> DiagnosticAction {
        DiagnosticAction::try_new(
            DiagnosticCode::parse(code).expect("test action code"),
            summary,
        )
        .expect("test action")
    }

    fn finding(
        id: &str,
        severity: DiagnosticSeverity,
        action: DiagnosticAction,
    ) -> DiagnosticFinding {
        DiagnosticFinding::try_new(
            DiagnosticFindingId::parse(id).expect("test finding ID"),
            DiagnosticCode::parse("test.remediation_condition").expect("test diagnostic code"),
            DiagnosticDomain::parse("test").expect("test domain"),
            DiagnosticStage::parse("evaluation").expect("test stage"),
            severity,
            DiagnosticSource::parse("doctor_test").expect("test source"),
            DiagnosticSubject::try_new("installation", "current").expect("test subject"),
            DiagnosticFacts::empty(),
            UtcTimestamp::parse("2026-07-31T00:00:00Z").expect("test timestamp"),
        )
        .expect("test finding")
        .with_actions(vec![action])
        .expect("test finding action")
    }

    #[test]
    fn finding_and_direct_sources_merge_by_typed_code() {
        let shared = action("action.test.repair", "Repair the test condition");
        let finding = finding(
            "finding.test_repair",
            DiagnosticSeverity::Warning,
            shared.clone(),
        );
        let direct = DoctorActionCandidate::from_finding_action(
            &shared,
            DoctorRemediationUrgency::Required,
            DoctorRemediationPriority::Immediate,
            "test_check",
            Some("volicord test repair".to_owned()),
        );

        let plan = DoctorRemediationPlan::finalize(&[finding], vec![direct]).expect("merged plan");
        assert_eq!(plan.actions().len(), 1);
        let action = plan.primary_action().expect("primary action");
        assert_eq!(action.code().as_str(), "action.test.repair");
        assert_eq!(action.command(), Some("volicord test repair"));
        assert_eq!(action.urgency(), DoctorRemediationUrgency::Required);
        assert_eq!(action.provenance.len(), 2);
    }

    #[test]
    fn conflicting_summaries_fail_report_assembly() {
        let shared = action("action.test.repair", "Repair the test condition");
        let first = DoctorActionCandidate::from_finding_action(
            &shared,
            DoctorRemediationUrgency::Recommended,
            DoctorRemediationPriority::Standard,
            "first_check",
            None,
        );
        let second = first.clone().with_summary("Perform unrelated repair");

        assert!(matches!(
            DoctorRemediationPlan::finalize(&[], vec![first, second]),
            Err(DoctorRemediationError::ConflictingSummaries { .. })
        ));
    }

    #[test]
    fn conflicting_commands_fail_report_assembly() {
        let shared = action("action.test.repair", "Repair the test condition");
        let first = DoctorActionCandidate::from_finding_action(
            &shared,
            DoctorRemediationUrgency::Recommended,
            DoctorRemediationPriority::Standard,
            "first_check",
            Some("volicord repair one".to_owned()),
        );
        let second = DoctorActionCandidate::from_finding_action(
            &shared,
            DoctorRemediationUrgency::Recommended,
            DoctorRemediationPriority::Standard,
            "second_check",
            Some("volicord repair two".to_owned()),
        );

        assert!(matches!(
            DoctorRemediationPlan::finalize(&[], vec![first, second]),
            Err(DoctorRemediationError::ConflictingCommands { .. })
        ));
    }

    #[test]
    fn input_order_does_not_change_actions_or_primary_selection() {
        let required = DoctorActionCandidate::direct(
            DoctorDirectAction::RepairProjectPolicy,
            "project_policy_authority",
            None,
        );
        let recommended = DoctorActionCandidate::direct(
            DoctorDirectAction::ProtectPersonalLocalFiles,
            "personal_local_git_tracking",
            None,
        );
        let high = DoctorActionCandidate::direct(
            DoctorDirectAction::RepairGuardFiles,
            "guard_files",
            None,
        );

        let first = DoctorRemediationPlan::finalize(
            &[],
            vec![required.clone(), recommended.clone(), high.clone()],
        )
        .expect("first plan");
        let second = DoctorRemediationPlan::finalize(&[], vec![high, recommended, required])
            .expect("second plan");

        assert_eq!(
            to_value(first.actions()).expect("first JSON"),
            to_value(second.actions()).expect("second JSON")
        );
        assert_eq!(
            first.primary_action().map(|action| action.code().as_str()),
            Some("action.guard.repair_files")
        );
        assert_eq!(
            first.primary_action().map(|action| action.code()),
            second.primary_action().map(|action| action.code())
        );
    }

    #[test]
    fn doctor_owned_direct_action_needs_no_finding() {
        let candidate = DoctorActionCandidate::direct(
            DoctorDirectAction::MakeProfileCommandsAvailable,
            "command_availability",
            None,
        );
        let plan =
            DoctorRemediationPlan::finalize(&[], vec![candidate]).expect("direct action plan");

        assert_eq!(plan.actions().len(), 1);
        assert_eq!(plan.required_actions().count(), 0);
        assert_eq!(plan.recommended_actions().count(), 1);
        assert_eq!(
            plan.primary_action().map(|action| action.code().as_str()),
            Some("action.installation.make_profile_commands_available")
        );
    }

    #[test]
    fn noncanonical_action_namespace_fails_finalization() {
        let candidate = DoctorActionCandidate::registered(
            "diagnostic.test.repair",
            "Repair the test condition",
            DoctorRemediationUrgency::Recommended,
            DoctorRemediationPriority::Standard,
            "test_check",
            None,
        );

        assert!(matches!(
            DoctorRemediationPlan::finalize(&[], vec![candidate]),
            Err(DoctorRemediationError::InvalidActionCode(_))
        ));
    }

    #[test]
    fn malformed_action_code_fails_finalization() {
        let candidate = DoctorActionCandidate::registered(
            "action invalid",
            "Repair the test condition",
            DoctorRemediationUrgency::Recommended,
            DoctorRemediationPriority::Standard,
            "test_check",
            None,
        );

        assert!(matches!(
            DoctorRemediationPlan::finalize(&[], vec![candidate]),
            Err(DoctorRemediationError::InvalidActionCode(_))
        ));
    }
}
