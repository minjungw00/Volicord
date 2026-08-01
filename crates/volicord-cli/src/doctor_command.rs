use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::SystemTime,
};

use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::{json, Value};
use volicord_command_model::DoctorArgs;
use volicord_mcp::{BuildInfo, BuildProvenanceAssessment};
use volicord_store::{
    agent_connections::{CONNECTION_MODE_READ_ONLY, CONNECTION_MODE_WORKFLOW},
    core_pipeline::CoreProjectStore,
    guards::{guard_installation, guard_observation_summary},
    inspection::{
        inspect_runtime_home, DatabaseInspection, InspectionSchemaState,
        InstallationProfileInspectionRecord, RegistryInspectionSnapshot, RuntimeHomeInspection,
    },
    operational_diagnostics::{
        runtime_home_diagnostic_finding, store_diagnostic_finding_from_kind, RuntimeHomeDiagnostic,
        RuntimeHomeDiagnosticFacts, StoreDiagnostic, StoreDiagnosticFacts,
    },
    runtime_home::{resolve_runtime_home, RuntimeHomeResolutionError},
    StoreError, StoreFailureRoute,
};
use volicord_types::canonical::canonical_json_sha256;
use volicord_types::connection_verification::ConnectionCheckKind;
use volicord_types::diagnostics::DiagnosticFinding;
use volicord_types::guard_manifest::{
    guard_manifest_from_json, GuardManagedArtifact, GuardManagedArtifactKind,
};
use volicord_types::ids::ProjectId;
use volicord_types::schema::SummaryCard;
use volicord_types::values::{GuardHookPhase, IntegrationProfile, UtcTimestamp};
use volicord_types::workflow_policy::ProjectWorkflowPolicySource;

use crate::{
    guard_integration::audit::{
        guard_file_findings_for_inspection, guard_manifest_binding_valid_for_inspection,
        missing_required_hooks_from_manifest_json, GuardArtifactIssue, GuardAuditFacts,
        GuardManifestIssue, HookPathSafetyAssessment, HookPathSafetyState,
    },
    guard_integration::git_exclude::{always_local_paths, git_exclude_path, personal_only_paths},
    guard_integration::policy::validate_policy_schema,
    host_integration::HostKind,
    operational_diagnostics::{
        occurrence_finding, InstallationDiagnostic, InstallationFacts, InstallationSubject,
        OperationalCheckState, OperationalDiagnostic,
    },
    policy_command::read_validated_policy_file,
    presentation::{ActionHint, BulletList, Document, Element, Field, HumanValue, Section, YesNo},
    setup_command::{path_text, CommandOutcome, CommandStatus},
    shell_path::{
        detect_command_on_path, is_executable_file, mcp_binary_name, path_directory_is_on_path,
        paths_equivalent, volicord_binary_name, PATH_ENV,
    },
    summary_card::DIAGNOSTIC_SUMMARY_GUARANTEE,
};

mod remediation;

use remediation::{
    DoctorActionCandidate, DoctorDirectAction, DoctorRemediationAction, DoctorRemediationPlan,
};

const GUARD_FILES_CHECK_ID: &str = ConnectionCheckKind::GuardFiles.as_str();
const GUARD_OBSERVATION_CHECK_ID: &str = ConnectionCheckKind::GuardObservation.as_str();

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DoctorCommandError {
    Usage(String),
    Runtime(String),
}

impl std::fmt::Display for DoctorCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Usage(message) | Self::Runtime(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for DoctorCommandError {}

impl From<RuntimeHomeResolutionError> for DoctorCommandError {
    fn from(error: RuntimeHomeResolutionError) -> Self {
        Self::Runtime(error.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputFormat {
    Compact,
    Verbose,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DoctorOptions {
    output: OutputFormat,
    privacy_footprint: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct DiagnosticCheck {
    id: String,
    status: String,
    summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<Value>,
    #[serde(skip)]
    hook_path_safety: Option<HookPathSafetyAssessment>,
}

impl DiagnosticCheck {
    fn passed(id: impl Into<String>, summary: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            status: "passed".to_owned(),
            summary: summary.into(),
            details: None,
            hook_path_safety: None,
        }
    }

    fn warning(id: impl Into<String>, summary: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            status: "warning".to_owned(),
            summary: summary.into(),
            details: None,
            hook_path_safety: None,
        }
    }

    fn skipped(id: impl Into<String>, summary: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            status: "skipped".to_owned(),
            summary: summary.into(),
            details: None,
            hook_path_safety: None,
        }
    }

    fn failed(id: impl Into<String>, summary: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            status: "failed".to_owned(),
            summary: summary.into(),
            details: None,
            hook_path_safety: None,
        }
    }

    fn with_details(mut self, details: Value) -> Self {
        self.details = Some(details);
        self
    }

    fn with_hook_path_safety(mut self, assessment: HookPathSafetyAssessment) -> Self {
        let details = self.details.get_or_insert_with(|| json!({}));
        if let Some(details) = details.as_object_mut() {
            details.insert("hook_path_safety".to_owned(), json!(&assessment));
        }
        self.hook_path_safety = Some(assessment);
        self
    }
}

#[derive(Debug, Clone)]
struct DoctorReport {
    status: CommandStatus,
    status_meaning: DoctorStatusMeaning,
    runtime_home: PathBuf,
    build: BuildInfo,
    summary_card: SummaryCard,
    checks: Vec<DiagnosticCheck>,
    findings: Vec<DiagnosticFinding>,
    remediation_plan: DoctorRemediationPlan,
}

impl DoctorReport {
    fn finalize(
        status: CommandStatus,
        runtime_home: PathBuf,
        build: BuildInfo,
        checks: Vec<DiagnosticCheck>,
        findings: Vec<DiagnosticFinding>,
        direct_action_candidates: Vec<DoctorActionCandidate>,
    ) -> Result<Self, DoctorCommandError> {
        let remediation_plan = DoctorRemediationPlan::finalize(&findings, direct_action_candidates)
            .map_err(|error| DoctorCommandError::Runtime(error.to_string()))?;
        let check_ids = checks
            .iter()
            .map(|check| check.id.as_str())
            .collect::<BTreeSet<_>>();
        remediation_plan
            .validate_check_provenance(&check_ids)
            .map_err(|error| DoctorCommandError::Runtime(error.to_string()))?;
        if status == CommandStatus::Complete && remediation_plan.has_required_actions() {
            return Err(DoctorCommandError::Runtime(
                "Doctor report is complete but its remediation plan contains a required action"
                    .to_owned(),
            ));
        }
        let status_meaning =
            DoctorStatusMeaning::derive(status, warning_count(&checks), &remediation_plan);
        let summary_card = doctor_summary_card(&checks, &remediation_plan);
        Ok(Self {
            status,
            status_meaning,
            runtime_home,
            build,
            summary_card,
            checks,
            findings,
            remediation_plan,
        })
    }

    fn warning_count(&self) -> usize {
        warning_count(&self.checks)
    }
}

fn warning_count(checks: &[DiagnosticCheck]) -> usize {
    checks
        .iter()
        .filter(|check| check.status == "warning")
        .count()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DoctorStatusMeaning {
    CompleteReady,
    CompleteRecommendedFollowUp,
    CompleteWarningsWithRecommendedRemediation,
    CompleteWarningsReviewOnly,
    ActionRequiredRemediationAvailable,
    ActionRequiredNoDefinedRemediation,
    FailedRemediationAvailable,
    FailedNoDefinedRemediation,
}

impl DoctorStatusMeaning {
    fn derive(
        status: CommandStatus,
        warning_count: usize,
        remediation_plan: &DoctorRemediationPlan,
    ) -> Self {
        let has_actions = !remediation_plan.is_empty();
        let has_required_actions = remediation_plan.has_required_actions();
        match status {
            CommandStatus::Complete if warning_count == 0 && !has_actions => Self::CompleteReady,
            CommandStatus::Complete if warning_count == 0 => Self::CompleteRecommendedFollowUp,
            CommandStatus::Complete if has_actions => {
                Self::CompleteWarningsWithRecommendedRemediation
            }
            CommandStatus::Complete => Self::CompleteWarningsReviewOnly,
            CommandStatus::ActionRequired if has_required_actions => {
                Self::ActionRequiredRemediationAvailable
            }
            CommandStatus::ActionRequired => Self::ActionRequiredNoDefinedRemediation,
            CommandStatus::Failed if has_required_actions => Self::FailedRemediationAvailable,
            CommandStatus::Failed => Self::FailedNoDefinedRemediation,
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::CompleteReady => {
                "installation profile is usable; no warnings or remediation actions are present"
            }
            Self::CompleteRecommendedFollowUp => {
                "installation profile is usable; recommended remediation is available"
            }
            Self::CompleteWarningsWithRecommendedRemediation => {
                "installation profile is usable; warnings have recommended remediation actions"
            }
            Self::CompleteWarningsReviewOnly => {
                "installation profile is usable; warnings require review and have no automatic remediation"
            }
            Self::ActionRequiredRemediationAvailable => {
                "local remediation is required before Volicord workflows are usable"
            }
            Self::ActionRequiredNoDefinedRemediation => {
                "local remediation is required, but no required remediation action is programmatically defined"
            }
            Self::FailedRemediationAvailable => {
                "a blocking diagnostic failed; the remediation plan names the next action"
            }
            Self::FailedNoDefinedRemediation => {
                "a blocking diagnostic failed without a programmatically defined required remediation action"
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct PrivacyRecordCounts {
    projects: usize,
    agent_connections: usize,
    connection_projects: usize,
    guard_installations: usize,
    project_state_databases: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PrivacyFootprintClaimKind {
    Stores,
    DoesNotStore,
    DoesNotProve,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
enum PrivacyFootprintClaimId {
    RuntimeHomeMetadata,
    ProductRepositoryRegistrations,
    GuardObservationMetadata,
    ProjectStateRecords,
    DiagnosticObservations,
    GuardPromptCapture,
    DiagnosticPayloads,
    ActorAttribution,
    WritePrevention,
    TamperProofAudit,
    FullFilesystemMonitoring,
    OperatingSystemEnforcementOrIsolation,
    ProductAssurance,
}

impl PrivacyFootprintClaimId {
    const fn kind(self) -> PrivacyFootprintClaimKind {
        match self {
            Self::RuntimeHomeMetadata
            | Self::ProductRepositoryRegistrations
            | Self::GuardObservationMetadata
            | Self::ProjectStateRecords
            | Self::DiagnosticObservations => PrivacyFootprintClaimKind::Stores,
            Self::GuardPromptCapture | Self::DiagnosticPayloads => {
                PrivacyFootprintClaimKind::DoesNotStore
            }
            Self::ActorAttribution
            | Self::WritePrevention
            | Self::TamperProofAudit
            | Self::FullFilesystemMonitoring
            | Self::OperatingSystemEnforcementOrIsolation
            | Self::ProductAssurance => PrivacyFootprintClaimKind::DoesNotProve,
        }
    }

    const fn text(self) -> &'static str {
        match self {
            Self::RuntimeHomeMetadata => {
                "Runtime Home identity, registry path, storage profile, installation profile, command paths, and setup metadata"
            }
            Self::ProductRepositoryRegistrations => {
                "Product Repository registrations, project home paths, project state database paths, and Agent Connection records"
            }
            Self::GuardObservationMetadata => {
                "Codex Record Guard installation records, capability metadata, policy hashes, hook observation timestamps, and prompt-capture availability state"
            }
            Self::ProjectStateRecords => {
                "project state records for tasks, change units, write tickets, evidence metadata, close-readiness records, User Channel actions, and artifacts when those features are used"
            }
            Self::DiagnosticObservations => {
                "bounded diagnostics.sqlite session, connection, project, transport, host, build, tool, categorical outcome, counter, byte-size, and latency observations when diagnostics are present"
            }
            Self::GuardPromptCapture => {
                "Guard prompt-capture observations do not include raw prompt text by default"
            }
            Self::DiagnosticPayloads => {
                "diagnostics.sqlite does not store prompt bodies, Product Repository paths or file contents, error bodies, secrets, or user-action question, choice, rationale, note, or observation-summary text"
            }
            Self::ActorAttribution => "actor attribution",
            Self::WritePrevention => "write prevention",
            Self::TamperProofAudit => "tamper-proof audit",
            Self::FullFilesystemMonitoring => "full filesystem monitoring",
            Self::OperatingSystemEnforcementOrIsolation => {
                "OS enforcement or security isolation"
            }
            Self::ProductAssurance => {
                "product correctness, test sufficiency, human review, final acceptance, or residual-risk acceptance"
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PrivacyFootprintClaim {
    id: PrivacyFootprintClaimId,
}

impl PrivacyFootprintClaim {
    const fn new(id: PrivacyFootprintClaimId) -> Self {
        Self { id }
    }

    const fn kind(self) -> PrivacyFootprintClaimKind {
        self.id.kind()
    }

    const fn text(self) -> &'static str {
        self.id.text()
    }
}

const PRIVACY_FOOTPRINT_CLAIMS: [PrivacyFootprintClaim; 13] = [
    PrivacyFootprintClaim::new(PrivacyFootprintClaimId::RuntimeHomeMetadata),
    PrivacyFootprintClaim::new(PrivacyFootprintClaimId::ProductRepositoryRegistrations),
    PrivacyFootprintClaim::new(PrivacyFootprintClaimId::GuardObservationMetadata),
    PrivacyFootprintClaim::new(PrivacyFootprintClaimId::ProjectStateRecords),
    PrivacyFootprintClaim::new(PrivacyFootprintClaimId::DiagnosticObservations),
    PrivacyFootprintClaim::new(PrivacyFootprintClaimId::GuardPromptCapture),
    PrivacyFootprintClaim::new(PrivacyFootprintClaimId::DiagnosticPayloads),
    PrivacyFootprintClaim::new(PrivacyFootprintClaimId::ActorAttribution),
    PrivacyFootprintClaim::new(PrivacyFootprintClaimId::WritePrevention),
    PrivacyFootprintClaim::new(PrivacyFootprintClaimId::TamperProofAudit),
    PrivacyFootprintClaim::new(PrivacyFootprintClaimId::FullFilesystemMonitoring),
    PrivacyFootprintClaim::new(PrivacyFootprintClaimId::OperatingSystemEnforcementOrIsolation),
    PrivacyFootprintClaim::new(PrivacyFootprintClaimId::ProductAssurance),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PrivacyFootprintDefinition {
    claims: &'static [PrivacyFootprintClaim],
    doctor_output_scope: &'static str,
}

impl PrivacyFootprintDefinition {
    const fn current() -> Self {
        Self {
            claims: &PRIVACY_FOOTPRINT_CLAIMS,
            doctor_output_scope:
                "Category and count summary only; stored row bodies are not printed.",
        }
    }

    fn claim_texts(self, kind: PrivacyFootprintClaimKind) -> Vec<&'static str> {
        self.claims
            .iter()
            .copied()
            .filter(|claim| claim.kind() == kind)
            .map(PrivacyFootprintClaim::text)
            .collect()
    }

    fn project(
        self,
        registry_state: &'static str,
        registry_db_path: String,
        record_counts: Option<PrivacyRecordCounts>,
    ) -> PrivacyFootprint {
        PrivacyFootprint {
            registry_state,
            registry_db_path,
            record_counts,
            stores: self.claim_texts(PrivacyFootprintClaimKind::Stores),
            does_not_store: self.claim_texts(PrivacyFootprintClaimKind::DoesNotStore),
            does_not_prove: self.claim_texts(PrivacyFootprintClaimKind::DoesNotProve),
            doctor_output_scope: self.doctor_output_scope,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct PrivacyFootprint {
    registry_state: &'static str,
    registry_db_path: String,
    record_counts: Option<PrivacyRecordCounts>,
    stores: Vec<&'static str>,
    does_not_store: Vec<&'static str>,
    does_not_prove: Vec<&'static str>,
    doctor_output_scope: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct PrivacyFootprintReport {
    status: &'static str,
    runtime_home: String,
    privacy_footprint: PrivacyFootprint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProjectPolicyAuthorityState {
    Matches,
    AuthorityMissing,
    AuthorityCorrupt,
    AuthorityUnavailable,
    ManagedFileMissing,
    ManagedFileInvalid,
    ManagedFileUnavailable,
    ManagedFileStale,
}

impl ProjectPolicyAuthorityState {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Matches => "matches",
            Self::AuthorityMissing => "authority_missing",
            Self::AuthorityCorrupt => "authority_corrupt",
            Self::AuthorityUnavailable => "authority_unavailable",
            Self::ManagedFileMissing => "managed_file_missing",
            Self::ManagedFileInvalid => "managed_file_invalid",
            Self::ManagedFileUnavailable => "managed_file_unavailable",
            Self::ManagedFileStale => "managed_file_stale",
        }
    }
}

pub fn run_doctor_command<F>(
    args: DoctorArgs,
    env_var: F,
    current_dir: &Path,
) -> Result<CommandOutcome, DoctorCommandError>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    let options = DoctorOptions {
        output: if args.output.json {
            OutputFormat::Json
        } else if args.output.verbose {
            OutputFormat::Verbose
        } else {
            OutputFormat::Compact
        },
        privacy_footprint: args.privacy_footprint,
    };
    let runtime_home = resolve_runtime_home(&env_var, current_dir)?;
    let inspection = inspect_runtime_home(&runtime_home);
    if options.privacy_footprint {
        return Ok(CommandOutcome {
            status: CommandStatus::Complete,
            output: render_privacy_footprint_output(options.output, &runtime_home, &inspection)?,
        });
    }
    let mut checks = Vec::new();
    let mut direct_action_candidates = Vec::new();
    let mut findings = Vec::new();
    let observed_at = doctor_current_timestamp();

    let build = volicord_mcp::build_info();
    let (build_check, build_diagnostic) = inspect_build_identity(&build);
    checks.push(build_check);
    if let Some(diagnostic) = build_diagnostic {
        findings.push(doctor_installation_finding(
            diagnostic,
            observed_at.clone(),
        )?);
    }
    if let Some((diagnostic, facts)) = inspect_runtime_home_path(&runtime_home, &mut checks) {
        findings.push(
            runtime_home_diagnostic_finding(
                diagnostic,
                format!("finding.doctor.{}", diagnostic.code().replace('.', "_")),
                &facts,
                observed_at.clone(),
            )
            .map_err(|error| DoctorCommandError::Runtime(error.to_string()))?,
        );
    }
    let mut profile = None;
    let mut absent_profile_state = "missing";
    let mut absent_profile_summary = "installation profile is missing";
    let mut project_count = None;
    let mut connection_count = None;
    let mut guard_installation_count = None;
    let registry_is_missing = matches!(&inspection.registry, DatabaseInspection::Missing { .. });

    match &inspection.registry {
        DatabaseInspection::Missing { path } => {
            checks.push(
                DiagnosticCheck::failed("registry", "Runtime Home registry is missing")
                    .with_details(json!({ "path": path_text(path) })),
            );
            let diagnostic = RuntimeHomeDiagnostic::RegistryMissing;
            findings.push(
                runtime_home_diagnostic_finding(
                    diagnostic,
                    "finding.doctor.runtime_home_registry_missing",
                    &RuntimeHomeDiagnosticFacts {
                        observed_state: Some("missing"),
                        ..RuntimeHomeDiagnosticFacts::default()
                    },
                    observed_at.clone(),
                )
                .map_err(|error| DoctorCommandError::Runtime(error.to_string()))?,
            );
        }
        DatabaseInspection::Present(snapshot) => {
            inspect_registry_snapshot(snapshot, &mut checks);
            profile = snapshot.installation_profile.as_ref();
            project_count = Some(snapshot.projects.len());
            connection_count = Some(snapshot.agent_connections.len());
            guard_installation_count = Some(snapshot.guard_installations.len());
            inspect_guard_installations(snapshot, &mut checks, &mut direct_action_candidates);
            inspect_personal_local_git_tracking(
                snapshot,
                &mut checks,
                &mut direct_action_candidates,
            );
            inspect_integration_intent_drift(snapshot, &mut checks, &mut direct_action_candidates);
            inspect_project_policy_authority(
                &runtime_home,
                snapshot,
                &mut checks,
                &mut direct_action_candidates,
            );
        }
        DatabaseInspection::Unsupported { path, .. } => {
            absent_profile_state = "corrupt";
            absent_profile_summary =
                "installation profile cannot be read from an invalid registry schema";
            checks.push(
                DiagnosticCheck::failed(
                    "registry",
                    "Runtime Home registry uses an unsupported schema",
                )
                .with_details(json!({
                    "path": path_text(path),
                    "diagnostic_code": StoreDiagnostic::SchemaMismatch.code(),
                })),
            );
            findings.push(doctor_store_finding(
                StoreDiagnostic::SchemaMismatch,
                "unsupported",
                observed_at.clone(),
            )?);
        }
        DatabaseInspection::Malformed { path, .. } => {
            absent_profile_state = "corrupt";
            absent_profile_summary = "installation profile cannot be read from a corrupt registry";
            checks.push(
                DiagnosticCheck::failed("registry", "Runtime Home registry is malformed")
                    .with_details(json!({
                        "path": path_text(path),
                        "diagnostic_code": StoreDiagnostic::IntegrityOrCorruptionFailure.code(),
                    })),
            );
            findings.push(doctor_store_finding(
                StoreDiagnostic::IntegrityOrCorruptionFailure,
                "malformed",
                observed_at.clone(),
            )?);
        }
        DatabaseInspection::Unreadable { path, .. } => {
            absent_profile_state = "unavailable";
            absent_profile_summary = "installation profile is currently unavailable";
            checks.push(
                DiagnosticCheck::failed("registry", "Runtime Home registry is unreadable")
                    .with_details(json!({
                        "path": path_text(path),
                        "diagnostic_code": volicord_types::diagnostics::INTERNAL_UNEXPECTED_FAILURE_CODE,
                    })),
            );
            findings.push(doctor_store_finding(
                StoreDiagnostic::Unexpected,
                "unreadable",
                observed_at.clone(),
            )?);
        }
    }

    if let Some(profile) = profile {
        for diagnostic in inspect_installation_profile(
            profile,
            &env_var,
            &mut checks,
            &mut direct_action_candidates,
        ) {
            findings.push(doctor_installation_finding(
                diagnostic,
                observed_at.clone(),
            )?);
        }
    } else {
        checks.push(
            DiagnosticCheck::failed("installation_profile", absent_profile_summary).with_details(
                json!({
                    "state": absent_profile_state,
                    "runtime_home": path_text(&runtime_home),
                }),
            ),
        );
        if absent_profile_state == "missing" && !registry_is_missing {
            direct_action_candidates.push(direct_action_candidate(
                DoctorDirectAction::InitializeConnection,
                "installation_profile",
                None,
            ));
        }
        checks.push(DiagnosticCheck::skipped(
            "volicord_command",
            "volicord command check needs an installation profile",
        ));
        checks.push(DiagnosticCheck::skipped(
            "volicord_mcp_command",
            "MCP launch command check needs an installation profile",
        ));
        checks.push(DiagnosticCheck::skipped(
            "path_or_shim",
            "PATH and shim check needs an installation profile",
        ));
    }

    checks.push(host_detection_check());
    if let (Some(projects), Some(connections), Some(guard_installations)) =
        (project_count, connection_count, guard_installation_count)
    {
        checks.push(
            DiagnosticCheck::passed("registry_counts", "registry records are readable")
                .with_details(json!({
                    "projects": projects,
                    "connections": connections,
                    "guard_installations": guard_installations,
                })),
        );
    } else {
        checks.push(DiagnosticCheck::skipped(
            "registry_counts",
            "project and connection counts are unavailable until the registry is readable",
        ));
    }

    let status = doctor_status(&checks);
    let report = DoctorReport::finalize(
        status,
        runtime_home,
        build,
        checks,
        findings,
        direct_action_candidates,
    )?;
    Ok(CommandOutcome {
        status,
        output: render_doctor_output(options.output, &report)?,
    })
}

fn inspect_build_identity(build: &BuildInfo) -> (DiagnosticCheck, Option<InstallationDiagnostic>) {
    let assessment = build.assess_provenance();
    let details = match &assessment {
        BuildProvenanceAssessment::UsableCleanExactProfile => {
            json!({ "state": "usable_clean", "profile_precision": "exact" })
        }
        BuildProvenanceAssessment::UsableCleanProfileClassOnly => {
            json!({ "state": "usable_clean", "profile_precision": "class_only" })
        }
        BuildProvenanceAssessment::DirtySource { profile_precision } => json!({
            "state": "dirty_source",
            "profile_precision": profile_precision,
        }),
        BuildProvenanceAssessment::Unavailable { gaps } => json!({
            "state": "unavailable",
            "missing_or_incomplete": gaps,
        }),
    };
    let (check, diagnostic) = match assessment {
        BuildProvenanceAssessment::UsableCleanExactProfile => (
            DiagnosticCheck::passed(
                "build_identity",
                "build provenance identifies a clean source commit and exact Cargo profile",
            ),
            None,
        ),
        BuildProvenanceAssessment::UsableCleanProfileClassOnly => (
            DiagnosticCheck::passed(
                "build_identity",
                "build provenance identifies a clean source commit; profile precision: class only",
            ),
            None,
        ),
        BuildProvenanceAssessment::DirtySource { .. } => (
            DiagnosticCheck::warning(
                "build_identity",
                "build source is dirty; the recorded commit does not identify the working-tree changes",
            ),
            Some(InstallationDiagnostic::BuildSourceNotReproducible),
        ),
        BuildProvenanceAssessment::Unavailable { .. } => (
            DiagnosticCheck::warning(
                "build_identity",
                "build identity is unavailable because required provenance metadata is incomplete",
            ),
            Some(InstallationDiagnostic::BuildIdentityUnavailable),
        ),
    };
    (check.with_details(details), diagnostic)
}

fn render_privacy_footprint_output(
    output: OutputFormat,
    runtime_home: &Path,
    inspection: &RuntimeHomeInspection,
) -> Result<String, DoctorCommandError> {
    let definition = PrivacyFootprintDefinition::current();
    let report = PrivacyFootprintReport {
        status: CommandStatus::Complete.as_str(),
        runtime_home: path_text(runtime_home),
        privacy_footprint: definition.project(
            privacy_registry_state(&inspection.registry),
            path_text(&inspection.registry_db_path),
            privacy_record_counts(&inspection.registry),
        ),
    };

    match output {
        OutputFormat::Json => serde_json::to_string_pretty(&report)
            .map(|text| format!("{text}\n"))
            .map_err(|error| DoctorCommandError::Runtime(error.to_string())),
        OutputFormat::Compact => Ok(render_privacy_footprint_text(&report)),
        OutputFormat::Verbose => Err(DoctorCommandError::Usage(
            "--privacy-footprint and --verbose cannot be used together".to_owned(),
        )),
    }
}

fn render_privacy_footprint_text(report: &PrivacyFootprintReport) -> String {
    let footprint = &report.privacy_footprint;
    let count_elements = footprint.record_counts.as_ref().map_or_else(
        || vec![Field::new("Availability", HumanValue::text("unavailable")).into()],
        |counts| {
            vec![
                Field::new("Projects", HumanValue::Count(counts.projects)).into(),
                Field::new("Connections", HumanValue::Count(counts.agent_connections)).into(),
                Field::new(
                    "Connection memberships",
                    HumanValue::Count(counts.connection_projects),
                )
                .into(),
                Field::new(
                    "Guard installations",
                    HumanValue::Count(counts.guard_installations),
                )
                .into(),
                Field::new(
                    "Project state databases",
                    HumanValue::Count(counts.project_state_databases),
                )
                .into(),
            ]
        },
    );
    Document::new(
        "Volicord Runtime Home privacy footprint",
        vec![
            Section::new(
                "Runtime Home",
                vec![
                    Field::new("Path", HumanValue::text(&report.runtime_home)).into(),
                    Field::new("Registry", HumanValue::text(footprint.registry_state)).into(),
                ],
            )
            .into(),
            Section::new("Record counts", count_elements).into(),
            Section::new(
                "Stores",
                vec![BulletList::new(footprint.stores.iter().copied()).into()],
            )
            .into(),
            Section::new(
                "Does not store",
                vec![BulletList::new(footprint.does_not_store.iter().copied()).into()],
            )
            .into(),
            Section::new(
                "Does not prove",
                vec![BulletList::new(footprint.does_not_prove.iter().copied()).into()],
            )
            .into(),
            Section::new(
                "Output scope",
                vec![Field::new("Summary", HumanValue::text(footprint.doctor_output_scope)).into()],
            )
            .into(),
        ],
    )
    .render()
}

fn privacy_registry_state(
    registry: &DatabaseInspection<RegistryInspectionSnapshot>,
) -> &'static str {
    match registry {
        DatabaseInspection::Missing { .. } => "missing",
        DatabaseInspection::Present(_) => "present",
        DatabaseInspection::Unsupported { .. } => "unsupported",
        DatabaseInspection::Malformed { .. } => "malformed",
        DatabaseInspection::Unreadable { .. } => "unreadable",
    }
}

fn privacy_record_counts(
    registry: &DatabaseInspection<RegistryInspectionSnapshot>,
) -> Option<PrivacyRecordCounts> {
    match registry {
        DatabaseInspection::Present(snapshot) => Some(PrivacyRecordCounts {
            projects: snapshot.projects.len(),
            agent_connections: snapshot.agent_connections.len(),
            connection_projects: snapshot.connection_projects.len(),
            guard_installations: snapshot.guard_installations.len(),
            project_state_databases: snapshot.projects.len(),
        }),
        _ => None,
    }
}

fn inspect_runtime_home_path(
    runtime_home: &Path,
    checks: &mut Vec<DiagnosticCheck>,
) -> Option<(RuntimeHomeDiagnostic, RuntimeHomeDiagnosticFacts)> {
    match fs::metadata(runtime_home) {
        Ok(metadata) if metadata.is_dir() => {
            checks.push(
                DiagnosticCheck::passed(
                    "runtime_home_access",
                    "Runtime Home directory is accessible",
                )
                .with_details(json!({ "path": path_text(runtime_home) })),
            );
            None
        }
        Ok(_) => {
            checks.push(
                DiagnosticCheck::failed(
                    "runtime_home_access",
                    "Runtime Home path is not a directory",
                )
                .with_details(json!({ "path": path_text(runtime_home) })),
            );
            Some((
                RuntimeHomeDiagnostic::InvalidPath,
                RuntimeHomeDiagnosticFacts {
                    observed_state: Some("not_directory"),
                    path_role: Some("runtime_home"),
                    ..RuntimeHomeDiagnosticFacts::default()
                },
            ))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            checks.push(
                DiagnosticCheck::failed("runtime_home_access", "Runtime Home directory is missing")
                    .with_details(json!({ "path": path_text(runtime_home) })),
            );
            Some((
                RuntimeHomeDiagnostic::MissingPath,
                RuntimeHomeDiagnosticFacts {
                    observed_state: Some("missing"),
                    path_role: Some("runtime_home"),
                    io_error_kind: Some("not_found"),
                    ..RuntimeHomeDiagnosticFacts::default()
                },
            ))
        }
        Err(error) => {
            checks.push(
                DiagnosticCheck::failed(
                    "runtime_home_access",
                    "Runtime Home directory is not accessible",
                )
                .with_details(json!({
                    "path": path_text(runtime_home),
                    "io_error_kind": doctor_io_error_kind(error.kind()),
                })),
            );
            RuntimeHomeDiagnostic::from_io_kind(error.kind()).map(|diagnostic| {
                (
                    diagnostic,
                    RuntimeHomeDiagnosticFacts {
                        observed_state: Some("unavailable"),
                        path_role: Some("runtime_home"),
                        io_error_kind: Some(doctor_io_error_kind(error.kind())),
                        ..RuntimeHomeDiagnosticFacts::default()
                    },
                )
            })
        }
    }
}

const fn doctor_io_error_kind(kind: std::io::ErrorKind) -> &'static str {
    match kind {
        std::io::ErrorKind::NotFound => "not_found",
        std::io::ErrorKind::PermissionDenied => "permission_denied",
        std::io::ErrorKind::InvalidInput => "invalid_input",
        std::io::ErrorKind::InvalidData => "invalid_data",
        std::io::ErrorKind::Unsupported => "unsupported",
        std::io::ErrorKind::TimedOut => "timed_out",
        std::io::ErrorKind::Interrupted => "interrupted",
        _ => "other",
    }
}

fn inspect_registry_snapshot(
    snapshot: &RegistryInspectionSnapshot,
    checks: &mut Vec<DiagnosticCheck>,
) {
    match snapshot.schema {
        InspectionSchemaState::Current => checks.push(
            DiagnosticCheck::passed("registry_schema", "Runtime Home registry schema is current")
                .with_details(json!({
                    "path": path_text(&snapshot.path),
                    "storage_profile": snapshot.runtime_home.storage_profile,
                })),
        ),
    }
}

const MAX_PERSONAL_GIT_PROJECTS: usize = 32;
const MAX_PERSONAL_GIT_FINDINGS: usize = 64;
const MAX_LOCAL_POLICY_BYTES: u64 = 1024 * 1024;

fn inspect_personal_local_git_tracking(
    snapshot: &RegistryInspectionSnapshot,
    checks: &mut Vec<DiagnosticCheck>,
    direct_action_candidates: &mut Vec<DoctorActionCandidate>,
) {
    let connected_connections = snapshot
        .agent_connections
        .iter()
        .map(|connection| connection.connection_internal_id.as_str())
        .collect::<BTreeSet<_>>();
    let mut project_internal_ids = snapshot
        .connection_projects
        .iter()
        .filter(|membership| {
            connected_connections.contains(membership.connection_internal_id.as_str())
        })
        .map(|membership| membership.project_internal_id.as_str())
        .collect::<BTreeSet<_>>();
    project_internal_ids.extend(
        snapshot
            .agent_connections
            .iter()
            .filter_map(|connection| connection.project_internal_id.as_deref()),
    );
    let mut projects = snapshot
        .projects
        .iter()
        .filter(|project| project_internal_ids.contains(project.project_internal_id.as_str()))
        .collect::<Vec<_>>();
    projects.sort_by(|left, right| left.project_id.cmp(&right.project_id));
    if projects.is_empty() {
        checks.push(DiagnosticCheck::skipped(
            "personal_local_git_tracking",
            "no repository connection is recorded",
        ));
        return;
    }

    let project_count = projects.len();
    let mut truncated = project_count > MAX_PERSONAL_GIT_PROJECTS;
    projects.truncate(MAX_PERSONAL_GIT_PROJECTS);
    let mut tracked_paths = Vec::new();
    let mut unignored_paths = Vec::new();
    let mut audit_errors = Vec::new();
    let mut effective_personal_project_count = 0usize;
    let mut local_paths = match always_local_paths() {
        Ok(paths) => paths,
        Err(error) => {
            checks.push(
                DiagnosticCheck::failed(
                    "personal_local_git_tracking",
                    "Guard managed-artifact path policy is invalid",
                )
                .with_details(json!({ "detail": error.to_string() })),
            );
            return;
        }
    };
    match personal_only_paths() {
        Ok(paths) => local_paths.extend(paths),
        Err(error) => {
            checks.push(
                DiagnosticCheck::failed(
                    "personal_local_git_tracking",
                    "Guard managed-artifact path policy is invalid",
                )
                .with_details(json!({ "detail": error.to_string() })),
            );
            return;
        }
    }
    let policy_local_path = local_paths
        .iter()
        .find(|path| path.artifact() == GuardManagedArtifact::VolicordPolicy)
        .expect("always-local policy includes the typed Volicord policy coordinate");

    'projects: for project in &projects {
        let exclude_path = match git_exclude_path(&project.repo_root) {
            Ok(Some(path)) => path,
            Ok(None) => continue,
            Err(error) => {
                push_bounded_git_finding(
                    &mut audit_errors,
                    json!({
                        "project_id": project.project_id,
                        "repo_root": path_text(&project.repo_root),
                        "detail": error.to_string(),
                    }),
                    &mut truncated,
                );
                continue;
            }
        };
        match local_policy_connection_intent(&project.repo_root) {
            Ok(intent) => {
                let personal = intent == "personal";
                effective_personal_project_count += usize::from(personal);
            }
            Err(detail) => {
                push_bounded_git_finding(
                    &mut audit_errors,
                    json!({
                        "project_id": project.project_id,
                        "repo_root": path_text(&project.repo_root),
                        "path": policy_local_path.ignore_probe_pattern(),
                        "detail": detail,
                    }),
                    &mut truncated,
                );
            }
        }
        // Audit both intent projections regardless of the policy's current
        // intent. A failed or interrupted migration can leave an opposite-
        // intent local file behind, and that file must not become trackable.
        for local_path in &local_paths {
            if tracked_paths.len() + unignored_paths.len() + audit_errors.len()
                >= MAX_PERSONAL_GIT_FINDINGS
            {
                truncated = true;
                break 'projects;
            }
            let pathspec = local_path.tracking_path();
            let ignore_probe = local_path.ignore_probe();
            let tracked = match git_path_predicate(
                &project.repo_root,
                true,
                &["ls-files", "--error-unmatch", "--", pathspec],
            ) {
                Ok(value) => value,
                Err(detail) => {
                    push_bounded_git_finding(
                        &mut audit_errors,
                        json!({
                            "project_id": project.project_id,
                            "repo_root": path_text(&project.repo_root),
                            "detail": detail,
                        }),
                        &mut truncated,
                    );
                    continue 'projects;
                }
            };
            if tracked {
                push_bounded_git_finding(
                    &mut tracked_paths,
                    json!({
                        "project_id": project.project_id,
                        "repo_root": path_text(&project.repo_root),
                        "path": local_path.pattern(),
                        "exclude_path": path_text(&exclude_path),
                    }),
                    &mut truncated,
                );
                continue;
            }
            match fs::symlink_metadata(project.repo_root.join(pathspec)) {
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => {
                    push_bounded_git_finding(
                        &mut audit_errors,
                        json!({
                            "project_id": project.project_id,
                            "repo_root": path_text(&project.repo_root),
                            "path": local_path.pattern(),
                            "detail": format!("failed to inspect the local path: {error}"),
                        }),
                        &mut truncated,
                    );
                    continue;
                }
            }
            let ignored = match git_path_predicate(
                &project.repo_root,
                false,
                &["check-ignore", "--quiet", "--no-index", "--", ignore_probe],
            ) {
                Ok(value) => value,
                Err(detail) => {
                    push_bounded_git_finding(
                        &mut audit_errors,
                        json!({
                            "project_id": project.project_id,
                            "repo_root": path_text(&project.repo_root),
                            "path": local_path.pattern(),
                            "detail": detail,
                        }),
                        &mut truncated,
                    );
                    continue 'projects;
                }
            };
            if !ignored {
                push_bounded_git_finding(
                    &mut unignored_paths,
                    json!({
                        "project_id": project.project_id,
                        "repo_root": path_text(&project.repo_root),
                        "path": local_path.pattern(),
                        "exclude_path": path_text(&exclude_path),
                    }),
                    &mut truncated,
                );
            }
        }
    }

    let details = json!({
        "connected_project_count": project_count,
        "effective_personal_project_count": effective_personal_project_count,
        "audited_project_count": projects.len(),
        "tracked_paths": tracked_paths,
        "unignored_existing_paths": unignored_paths,
        "audit_errors": audit_errors,
        "truncated": truncated,
        "reads_local_policy_file": true,
        "does_not_read_other_local_integration_file_contents": true,
    });
    let has_warning = details["tracked_paths"]
        .as_array()
        .is_some_and(|values| !values.is_empty())
        || details["unignored_existing_paths"]
            .as_array()
            .is_some_and(|values| !values.is_empty())
        || details["audit_errors"]
            .as_array()
            .is_some_and(|values| !values.is_empty())
        || truncated;
    let check = if has_warning {
        direct_action_candidates.push(direct_action_candidate(
            DoctorDirectAction::ProtectPersonalLocalFiles,
            "personal_local_git_tracking",
            None,
        ));
        DiagnosticCheck::warning(
            "personal_local_git_tracking",
            "local integration files need Git tracking follow-up",
        )
    } else {
        DiagnosticCheck::passed(
            "personal_local_git_tracking",
            "local integration files are outside the Git index and ignored",
        )
    };
    checks.push(check.with_details(details));
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LocalPolicyAudit {
    host: String,
    connection_intent: String,
    selected_profile: String,
    connection_id: String,
    guard_installation_id: String,
}

fn local_policy_connection_intent(repo_root: &Path) -> Result<String, String> {
    local_policy_audit(repo_root).map(|policy| policy.connection_intent)
}

fn local_policy_audit(repo_root: &Path) -> Result<LocalPolicyAudit, String> {
    let policy_relative_path = GuardManagedArtifact::VolicordPolicy
        .repository_relative_path()
        .map_err(|error| error.to_string())?;
    let policy_path = repo_root.join(&policy_relative_path);
    let policy_dir = policy_path
        .parent()
        .ok_or_else(|| "the canonical local policy path has no parent directory".to_owned())?;
    let directory_metadata = fs::symlink_metadata(policy_dir)
        .map_err(|error| format!("failed to inspect the local policy directory: {error}"))?;
    if directory_metadata.file_type().is_symlink() || !directory_metadata.is_dir() {
        return Err("the local policy directory is not a regular directory".to_owned());
    }
    let metadata = fs::symlink_metadata(&policy_path)
        .map_err(|error| format!("failed to inspect the local policy file: {error}"))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err("the local policy file is not a regular file".to_owned());
    }
    if metadata.len() > MAX_LOCAL_POLICY_BYTES {
        return Err(format!(
            "the local policy file exceeds the {MAX_LOCAL_POLICY_BYTES}-byte audit limit"
        ));
    }
    let text = fs::read_to_string(&policy_path)
        .map_err(|error| format!("failed to read the local policy file: {error}"))?;
    let policy = serde_json::from_str::<Value>(&text)
        .map_err(|error| format!("the local policy file is not valid JSON: {error}"))?;
    let connection_intent = policy
        .get("connection_intent")
        .and_then(Value::as_str)
        .ok_or_else(|| "the local policy is missing connection_intent".to_owned())?;
    validate_policy_schema(&policy, connection_intent)
        .map_err(|error| format!("the local policy schema is invalid: {error}"))?;
    let required = |field: &str| {
        policy
            .get(field)
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .ok_or_else(|| format!("the local policy is missing {field}"))
    };
    Ok(LocalPolicyAudit {
        host: required("host")?,
        connection_intent: connection_intent.to_owned(),
        selected_profile: required("selected_profile")?,
        connection_id: required("connection_id")?,
        guard_installation_id: required("guard_installation_id")?,
    })
}

const MAX_INTENT_DRIFT_FINDINGS: usize = 64;

fn inspect_integration_intent_drift(
    snapshot: &RegistryInspectionSnapshot,
    checks: &mut Vec<DiagnosticCheck>,
    direct_action_candidates: &mut Vec<DoctorActionCandidate>,
) {
    let mut projects = connected_enabled_projects(snapshot);
    projects.sort_by(|left, right| left.project_id.cmp(&right.project_id));
    if projects.is_empty() {
        checks.push(DiagnosticCheck::skipped(
            "integration_intent_drift",
            "no enabled repository integration is recorded",
        ));
        return;
    }

    let connected_project_count = projects.len();
    let mut truncated = projects.len() > MAX_PERSONAL_GIT_PROJECTS;
    projects.truncate(MAX_PERSONAL_GIT_PROJECTS);
    let mut findings = Vec::new();
    let mut audit_errors = Vec::new();
    let active_installations = active_guard_installations(snapshot);

    for project in &projects {
        let policy = match local_policy_audit(&project.repo_root) {
            Ok(policy) => policy,
            Err(detail) => {
                push_bounded_intent_finding(
                    &mut audit_errors,
                    json!({
                        "project_id": project.project_id,
                        "repo_root": path_text(&project.repo_root),
                        "detail": detail,
                    }),
                    &mut truncated,
                );
                continue;
            }
        };
        if policy.host != HostKind::Codex.as_str()
            || policy.selected_profile != IntegrationProfile::Record.as_str()
        {
            push_bounded_intent_finding(
                &mut findings,
                json!({
                    "project_id": project.project_id,
                    "repo_root": path_text(&project.repo_root),
                    "kind": "policy_scope_mismatch",
                    "policy_host": policy.host,
                    "selected_profile": policy.selected_profile,
                    "expected_host": HostKind::Codex.as_str(),
                    "expected_profile": IntegrationProfile::Record.as_str(),
                }),
                &mut truncated,
            );
        }

        let attached = snapshot
            .agent_connections
            .iter()
            .filter(|connection| {
                connection.enabled
                    && connection.host_kind == HostKind::Codex.as_str()
                    && matches!(connection.intent.as_str(), "personal" | "shared")
                    && connection_is_attached_to_project(snapshot, connection, project)
            })
            .collect::<Vec<_>>();
        let expected = attached
            .iter()
            .find(|connection| connection.connection_internal_id == policy.connection_id);
        match expected {
            None => push_bounded_intent_finding(
                &mut findings,
                json!({
                    "project_id": project.project_id,
                    "repo_root": path_text(&project.repo_root),
                    "kind": "policy_connection_missing",
                    "policy_connection_id": policy.connection_id,
                    "policy_intent": policy.connection_intent,
                    "host": policy.host,
                    "policy_host": policy.host,
                }),
                &mut truncated,
            ),
            Some(connection) if connection.host_kind != HostKind::Codex.as_str() => {
                push_bounded_intent_finding(
                    &mut findings,
                    json!({
                        "project_id": project.project_id,
                        "repo_root": path_text(&project.repo_root),
                        "kind": "policy_connection_host_mismatch",
                        "policy_connection_id": policy.connection_id,
                        "policy_host": policy.host,
                        "recorded_host": connection.host_kind,
                    }),
                    &mut truncated,
                );
            }
            Some(connection) if connection.intent != policy.connection_intent => {
                push_bounded_intent_finding(
                    &mut findings,
                    json!({
                        "project_id": project.project_id,
                        "repo_root": path_text(&project.repo_root),
                        "kind": "policy_connection_intent_mismatch",
                        "policy_intent": policy.connection_intent,
                        "recorded_intent": connection.intent,
                        "host": policy.host,
                        "policy_host": policy.host,
                    }),
                    &mut truncated,
                );
            }
            Some(_) => {}
        }
        for connection in attached.iter().filter(|connection| {
            connection.connection_internal_id != policy.connection_id
                || connection.intent != policy.connection_intent
        }) {
            push_bounded_intent_finding(
                &mut findings,
                json!({
                    "project_id": project.project_id,
                    "repo_root": path_text(&project.repo_root),
                    "kind": "additional_active_intent_projection",
                    "policy_connection_id": policy.connection_id,
                    "connection_id": connection.connection_internal_id,
                    "policy_intent": policy.connection_intent,
                    "recorded_intent": connection.intent,
                    "host": HostKind::Codex.as_str(),
                }),
                &mut truncated,
            );
        }

        let guard_matches = active_installations.iter().any(|installation| {
            let manifest = guard_manifest_from_json(&installation.manifest_json).ok();
            installation.guard_installation_id == policy.guard_installation_id
                && installation.connection_internal_id == policy.connection_id
                && manifest.as_ref().is_some_and(|manifest| {
                    manifest.integration_profile == IntegrationProfile::Record
                })
                && installation.project_internal_id == project.project_internal_id
        });
        if !guard_matches {
            push_bounded_intent_finding(
                &mut findings,
                json!({
                    "project_id": project.project_id,
                    "repo_root": path_text(&project.repo_root),
                    "kind": "policy_guard_inventory_mismatch",
                    "guard_installation_id": policy.guard_installation_id,
                    "selected_profile": policy.selected_profile,
                    "host": policy.host,
                }),
                &mut truncated,
            );
        }
    }

    let has_warning = !findings.is_empty() || !audit_errors.is_empty() || truncated;
    let details = json!({
        "connected_project_count": connected_project_count,
        "audited_project_count": projects.len(),
        "findings": findings,
        "audit_errors": audit_errors,
        "truncated": truncated,
        "max_projects": MAX_PERSONAL_GIT_PROJECTS,
        "max_findings": MAX_INTENT_DRIFT_FINDINGS,
    });
    if has_warning {
        direct_action_candidates.push(direct_action_candidate(
            DoctorDirectAction::RepairIntegrationIntentDrift,
            "integration_intent_drift",
            None,
        ));
        checks.push(
            DiagnosticCheck::warning(
                "integration_intent_drift",
                "one or more Codex Record repository integrations have intent or inventory drift",
            )
            .with_details(details),
        );
    } else {
        checks.push(
            DiagnosticCheck::passed(
                "integration_intent_drift",
                "Codex Record repository integration intent and inventory are converged",
            )
            .with_details(details),
        );
    }
}

fn connected_enabled_projects(
    snapshot: &RegistryInspectionSnapshot,
) -> Vec<&volicord_store::inspection::ProjectInspectionRecord> {
    snapshot
        .projects
        .iter()
        .filter(|project| {
            snapshot.agent_connections.iter().any(|connection| {
                connection.enabled
                    && connection.host_kind == HostKind::Codex.as_str()
                    && matches!(connection.intent.as_str(), "personal" | "shared")
                    && connection_is_attached_to_project(snapshot, connection, project)
            })
        })
        .collect()
}

fn inspect_project_policy_authority(
    runtime_home: &Path,
    snapshot: &RegistryInspectionSnapshot,
    checks: &mut Vec<DiagnosticCheck>,
    direct_action_candidates: &mut Vec<DoctorActionCandidate>,
) {
    let mut projects = connected_enabled_projects(snapshot);
    projects.sort_by(|left, right| left.project_id.cmp(&right.project_id));
    let project_count = projects.len();
    let mut truncated = project_count > MAX_PERSONAL_GIT_PROJECTS;
    projects.truncate(MAX_PERSONAL_GIT_PROJECTS);
    let mut findings = Vec::new();
    let mut repair_commands = BTreeSet::new();
    let policy_relative_path = match GuardManagedArtifact::VolicordPolicy.repository_relative_path()
    {
        Ok(path) => path,
        Err(error) => {
            checks.push(
                DiagnosticCheck::failed(
                    "project_policy_authority",
                    "Guard managed-artifact path policy is invalid",
                )
                .with_details(json!({ "detail": error.to_string() })),
            );
            return;
        }
    };
    for project in projects {
        let file_path = project.repo_root.join(&policy_relative_path);
        let state = project_policy_authority_state(runtime_home, project, &file_path);
        if state != ProjectPolicyAuthorityState::Matches {
            truncated |= findings.len() >= MAX_INTENT_DRIFT_FINDINGS;
            if findings.len() < MAX_INTENT_DRIFT_FINDINGS {
                findings.push(json!({
                    "project_id": project.project_id,
                    "repo_root": path_text(&project.repo_root),
                    "status": state.as_str(),
                }));
            }
            repair_commands.insert(format!(
                "volicord policy show --repo {} --json",
                doctor_shell_word(&path_text(&project.repo_root))
            ));
        }
    }
    if !findings.is_empty() {
        let command = (repair_commands.len() == 1)
            .then(|| repair_commands.into_iter().next())
            .flatten();
        direct_action_candidates.push(direct_action_candidate(
            DoctorDirectAction::RepairProjectPolicy,
            "project_policy_authority",
            command,
        ));
    }
    let details = json!({
        "project_count": project_count,
        "findings": findings,
        "truncated": truncated,
        "scan_state": if truncated { "bounded_incomplete" } else { "complete" },
    });
    let finding_count = details["findings"].as_array().map_or(0, Vec::len);
    match project_policy_authority_result(finding_count, truncated) {
        "passed" => checks.push(
            DiagnosticCheck::passed(
                "project_policy_authority",
                "managed project policies match authoritative database fingerprints",
            )
            .with_details(details),
        ),
        "warning" => checks.push(
            DiagnosticCheck::warning(
                "project_policy_authority",
                "the bounded project-policy audit did not inspect every connected project",
            )
            .with_details(details),
        ),
        _ => checks.push(
            DiagnosticCheck::failed(
                "project_policy_authority",
                "one or more managed project policies do not match database authority",
            )
            .with_details(details),
        ),
    }
}

fn project_policy_authority_result(finding_count: usize, truncated: bool) -> &'static str {
    if finding_count > 0 {
        "failed"
    } else if truncated {
        "warning"
    } else {
        "passed"
    }
}

fn project_policy_authority_state(
    runtime_home: &Path,
    project: &volicord_store::inspection::ProjectInspectionRecord,
    file_path: &Path,
) -> ProjectPolicyAuthorityState {
    let store = match CoreProjectStore::open_read_only(
        runtime_home,
        &ProjectId::new(project.project_id.clone()),
    ) {
        Ok(store) => store,
        Err(error) => return project_policy_store_failure_state(&error),
    };
    let authority = match store.project_workflow_policy() {
        Ok(Some(authority)) => authority,
        Ok(None) => return ProjectPolicyAuthorityState::AuthorityMissing,
        Err(error) => return project_policy_store_failure_state(&error),
    };
    if authority.source != ProjectWorkflowPolicySource::ProjectDatabase {
        return ProjectPolicyAuthorityState::AuthorityCorrupt;
    }
    let authority_value = match serde_json::to_value(&authority.policy) {
        Ok(value) => value,
        Err(_) => return ProjectPolicyAuthorityState::AuthorityCorrupt,
    };
    let Some(connection_intent) = authority_value
        .get("connection_intent")
        .and_then(Value::as_str)
    else {
        return ProjectPolicyAuthorityState::AuthorityCorrupt;
    };
    if validate_policy_schema(&authority_value, connection_intent).is_err() {
        return ProjectPolicyAuthorityState::AuthorityCorrupt;
    }
    let authority_fingerprint = match canonical_json_sha256(&authority_value) {
        Ok(fingerprint) => fingerprint,
        Err(_) => return ProjectPolicyAuthorityState::AuthorityCorrupt,
    };
    if authority_fingerprint.as_str() != authority.policy_fingerprint {
        return ProjectPolicyAuthorityState::AuthorityCorrupt;
    }

    match fs::symlink_metadata(file_path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return ProjectPolicyAuthorityState::ManagedFileMissing;
        }
        Err(_) => return ProjectPolicyAuthorityState::ManagedFileUnavailable,
        Ok(_) => {}
    }
    let managed = match read_validated_policy_file(file_path) {
        Ok(managed) => managed,
        Err(crate::policy_command::PolicyCommandError::Validation { .. }) => {
            return ProjectPolicyAuthorityState::ManagedFileInvalid;
        }
        Err(_) => return ProjectPolicyAuthorityState::ManagedFileUnavailable,
    };
    if managed.fingerprint == authority.policy_fingerprint {
        ProjectPolicyAuthorityState::Matches
    } else {
        ProjectPolicyAuthorityState::ManagedFileStale
    }
}

fn project_policy_store_failure_state(error: &StoreError) -> ProjectPolicyAuthorityState {
    project_policy_store_failure_route(error.classification().route)
}

fn project_policy_store_failure_route(route: StoreFailureRoute) -> ProjectPolicyAuthorityState {
    match route {
        StoreFailureRoute::PersistedDataCorrupt => ProjectPolicyAuthorityState::AuthorityCorrupt,
        StoreFailureRoute::OperationalUnavailable
        | StoreFailureRoute::InvalidEnvironment
        | StoreFailureRoute::InvocationContextMismatch => {
            ProjectPolicyAuthorityState::AuthorityUnavailable
        }
    }
}

fn connection_is_attached_to_project(
    snapshot: &RegistryInspectionSnapshot,
    connection: &volicord_store::inspection::AgentConnectionInspectionRecord,
    project: &volicord_store::inspection::ProjectInspectionRecord,
) -> bool {
    connection.project_internal_id.as_deref() == Some(project.project_internal_id.as_str())
        || snapshot.connection_projects.iter().any(|membership| {
            membership.connection_internal_id == connection.connection_internal_id
                && membership.project_internal_id == project.project_internal_id
        })
}

fn active_guard_installations(
    snapshot: &RegistryInspectionSnapshot,
) -> Vec<volicord_store::inspection::GuardInstallationInspectionRecord> {
    snapshot
        .guard_installations
        .iter()
        .filter(|installation| {
            let Some(connection) = snapshot.agent_connections.iter().find(|connection| {
                connection.enabled
                    && connection.connection_internal_id == installation.connection_internal_id
            }) else {
                return false;
            };
            {
                let project_internal_id = installation.project_internal_id.as_str();
                connection.project_internal_id.as_deref() == Some(project_internal_id)
                    || snapshot.connection_projects.iter().any(|membership| {
                        membership.connection_internal_id == connection.connection_internal_id
                            && membership.project_internal_id == project_internal_id
                    })
            }
        })
        .cloned()
        .collect()
}

fn doctor_shell_word(value: &str) -> String {
    if !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_@%+=:,./-".contains(&byte))
    {
        value.to_owned()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

fn push_bounded_intent_finding(values: &mut Vec<Value>, value: Value, truncated: &mut bool) {
    if values.len() < MAX_INTENT_DRIFT_FINDINGS {
        values.push(value);
    } else {
        *truncated = true;
    }
}

fn push_bounded_git_finding(values: &mut Vec<Value>, value: Value, truncated: &mut bool) {
    if values.len() < MAX_PERSONAL_GIT_FINDINGS {
        values.push(value);
    } else {
        *truncated = true;
    }
}

fn git_path_predicate(
    repo_root: &Path,
    literal_pathspecs: bool,
    args: &[&str],
) -> Result<bool, String> {
    let mut command = Command::new("git");
    if literal_pathspecs {
        command.arg("--literal-pathspecs");
    }
    command
        .args(args)
        .current_dir(repo_root)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    for name in [
        "GIT_COMMON_DIR",
        "GIT_DIR",
        "GIT_INDEX_FILE",
        "GIT_OBJECT_DIRECTORY",
        "GIT_WORK_TREE",
    ] {
        command.env_remove(name);
    }
    let status = command
        .status()
        .map_err(|error| format!("failed to run the local Git tracking check: {error}"))?;
    match status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        code => Err(format!(
            "the local Git tracking check exited with status {}",
            code.map_or_else(|| "signal".to_owned(), |value| value.to_string())
        )),
    }
}

fn inspect_guard_installations(
    snapshot: &RegistryInspectionSnapshot,
    checks: &mut Vec<DiagnosticCheck>,
    direct_action_candidates: &mut Vec<DoctorActionCandidate>,
) {
    let installations = active_guard_installations(snapshot);
    if installations.is_empty() {
        checks.push(
            DiagnosticCheck::skipped(
                GUARD_FILES_CHECK_ID,
                "no Codex Record Guard manifest is recorded",
            )
            .with_hook_path_safety(HookPathSafetyAssessment::not_checked()),
        );
        checks.push(DiagnosticCheck::skipped(
            GUARD_OBSERVATION_CHECK_ID,
            "no Guard observation is recorded",
        ));
        checks.push(
            DiagnosticCheck::skipped("control_surface", "no integration profile is recorded")
                .with_details(json!({
                    "selected_profile": "not_checked",
                    "control_surface": {
                        "selected_profile": "not_checked",
                        "host_hooks_active": false,
                        "cooperative_pre_tool_warning_available": false,
                        "cooperative_pre_tool_denial_available": false,
                        "unrecorded_changes_detectable": false,
                        "actor_identity_provable": false,
                        "os_enforced": false,
                    },
                })),
        );
        return;
    }

    let invalid_scope_count = installations
        .iter()
        .filter(|installation| {
            guard_manifest_from_json(&installation.manifest_json).is_err_and(|_| true)
                || guard_manifest_from_json(&installation.manifest_json).is_ok_and(|manifest| {
                    manifest.host_kind.as_str() != HostKind::Codex.as_str()
                        || manifest.integration_profile != IntegrationProfile::Record
                })
        })
        .count();
    let binding_valid_installations = installations
        .iter()
        .filter(|installation| {
            snapshot
                .agent_connections
                .iter()
                .find(|connection| {
                    connection.connection_internal_id == installation.connection_internal_id
                })
                .is_some_and(|connection| {
                    guard_manifest_binding_valid_for_inspection(
                        installation,
                        connection,
                        &snapshot.projects,
                    )
                })
        })
        .collect::<Vec<_>>();
    let binding_invalid_count = installations.len() - binding_valid_installations.len();

    let mut file_findings = GuardAuditFacts::default();
    for installation in &installations {
        let connection = snapshot
            .agent_connections
            .iter()
            .find(|connection| {
                connection.connection_internal_id == installation.connection_internal_id
            })
            .expect("validated inspection snapshot retains the Guard owner");
        file_findings.merge(guard_file_findings_for_inspection(
            installation,
            connection,
            &snapshot.projects,
        ));
    }
    file_findings.sort_dedup();
    let (missing_files, unavailable_files, stale_files, broken_files) =
        projected_doctor_guard_file_paths(&file_findings);

    let missing_required_hooks = binding_valid_installations
        .iter()
        .flat_map(|installation| {
            missing_required_hooks_from_manifest_json(&installation.manifest_json)
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let observation_summaries = binding_valid_installations
        .iter()
        .filter_map(|installation| guard_observation(snapshot, installation))
        .collect::<Vec<_>>();
    let observed_count = observation_summaries
        .iter()
        .filter(|summary| summary.all_required_phases_observed())
        .count();
    let incompatible_observations = observation_summaries
        .iter()
        .map(|summary| summary.incompatible_event_ids.len())
        .sum::<usize>();
    let prompt_capture_configured = binding_valid_installations
        .iter()
        .filter(|installation| guard_prompt_capture_configured(&installation.manifest_json))
        .count();
    let prompt_capture_observed = observation_summaries
        .iter()
        .filter(|summary| summary.prompt_capture_observed())
        .count();
    let host_hooks_active = invalid_scope_count == 0
        && binding_invalid_count == 0
        && missing_required_hooks.is_empty()
        && observed_count == installations.len()
        && installations
            .iter()
            .all(|installation| guard_effective_active(snapshot, installation))
        && file_findings.generated_config_verified()
        && file_findings.hook_path_safety.state == HookPathSafetyState::Verified
        && file_findings.direct_file_write_matcher_coverage();
    let selected_profile = if invalid_scope_count == 0 {
        IntegrationProfile::Record.as_str()
    } else {
        "invalid"
    };
    let control_check = if invalid_scope_count == 0 {
        DiagnosticCheck::passed("control_surface", "selected integration profile is record")
    } else {
        DiagnosticCheck::warning(
            "control_surface",
            "one or more Guard records are outside the Codex Record release scope",
        )
    };
    checks.push(control_check.with_details(json!({
        "selected_profile": selected_profile,
        "control_surface": {
            "selected_profile": selected_profile,
            "host_hooks_active": host_hooks_active,
            "cooperative_pre_tool_warning_available": host_hooks_active,
            "cooperative_pre_tool_denial_available": host_hooks_active,
            "unrecorded_changes_detectable": host_hooks_active,
            "actor_identity_provable": false,
            "os_enforced": false,
        },
    })));

    let path_safety_failed = file_findings.hook_path_safety.state == HookPathSafetyState::Failed;
    let path_safety_incomplete = matches!(
        file_findings.hook_path_safety.state,
        HookPathSafetyState::NotRecorded | HookPathSafetyState::NotChecked
    );
    let file_problem = invalid_scope_count > 0
        || binding_invalid_count > 0
        || !missing_required_hooks.is_empty()
        || !missing_files.is_empty()
        || !stale_files.is_empty()
        || !broken_files.is_empty()
        || path_safety_failed;
    let file_check = if file_problem {
        direct_action_candidates.push(direct_action_candidate(
            DoctorDirectAction::RepairGuardFiles,
            GUARD_FILES_CHECK_ID,
            None,
        ));
        DiagnosticCheck::failed(
            GUARD_FILES_CHECK_ID,
            "one or more Codex Record Guard files are missing, stale, or broken",
        )
    } else if path_safety_incomplete || !unavailable_files.is_empty() {
        DiagnosticCheck::warning(
            GUARD_FILES_CHECK_ID,
            format!(
                "Codex Record Guard files are present, but Hook path-safety evidence is {}",
                file_findings
                    .hook_path_safety
                    .state
                    .as_str()
                    .replace('_', " ")
            ),
        )
    } else {
        DiagnosticCheck::passed(
            GUARD_FILES_CHECK_ID,
            "Codex Record Guard files and Hook path safety are verified",
        )
    };
    let mut file_details = doctor_guard_file_details(&file_findings);
    if let Some(details) = file_details.as_object_mut() {
        details.insert(
            "missing_required_hooks".to_owned(),
            json!(missing_required_hooks),
        );
        details.insert("binding_invalid".to_owned(), json!(binding_invalid_count));
        details.insert(
            "outside_release_scope".to_owned(),
            json!(invalid_scope_count),
        );
    }
    checks.push(
        file_check
            .with_details(file_details)
            .with_hook_path_safety(file_findings.hook_path_safety.clone()),
    );

    if path_safety_failed {
        direct_action_candidates.push(direct_action_candidate(
            DoctorDirectAction::RepairGuardHookPathSafety,
            GUARD_FILES_CHECK_ID,
            None,
        ));
    }

    if binding_invalid_count == 0
        && incompatible_observations == 0
        && observed_count == installations.len()
    {
        checks.push(
            DiagnosticCheck::passed(
                GUARD_OBSERVATION_CHECK_ID,
                "all current Codex Record Guard hook phases were observed",
            )
            .with_details(json!({
                "observed": observed_count,
                "installations": installations.len(),
                "incompatible_events": incompatible_observations,
                "prompt_capture_configured": prompt_capture_configured,
                "prompt_capture_observed": prompt_capture_observed,
            })),
        );
    } else {
        let observation_check = if incompatible_observations > 0 {
            DiagnosticCheck::failed(
                GUARD_OBSERVATION_CHECK_ID,
                "a current Guard event reported a malformed or incompatible hook contract",
            )
        } else {
            DiagnosticCheck::warning(
                GUARD_OBSERVATION_CHECK_ID,
                "one or more Codex Record Guard installations are awaiting current hook observations",
            )
        };
        checks.push(observation_check.with_details(json!({
            "observed": observed_count,
            "installations": installations.len(),
            "binding_invalid": binding_invalid_count,
            "incompatible_events": incompatible_observations,
            "prompt_capture_configured": prompt_capture_configured,
            "prompt_capture_observed": prompt_capture_observed,
        })));
    }
}

fn projected_doctor_guard_file_paths(
    facts: &GuardAuditFacts,
) -> (Vec<String>, Vec<String>, Vec<String>, Vec<String>) {
    let affected_paths = facts.affected_paths();
    let paths_for = |issues: &[GuardArtifactIssue]| {
        affected_paths
            .iter()
            .filter(|path| {
                facts
                    .findings
                    .iter()
                    .any(|finding| finding.path == **path && issues.contains(&finding.issue))
            })
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
    };
    let missing = paths_for(&[GuardArtifactIssue::Missing]);
    let unavailable = paths_for(&[GuardArtifactIssue::Unavailable]);
    let stale = paths_for(&[
        GuardArtifactIssue::ContentMismatch,
        GuardArtifactIssue::OwnershipMismatch,
        GuardArtifactIssue::PermissionMismatch,
        GuardArtifactIssue::HookContractMismatch,
    ]);
    let mut broken = paths_for(&[GuardArtifactIssue::Malformed]);
    if facts
        .manifest_issues
        .contains(&GuardManifestIssue::Malformed)
    {
        broken.push("manifest_json".to_owned());
    }
    if facts
        .manifest_issues
        .contains(&GuardManifestIssue::OwnershipMismatch)
    {
        broken.push("manifest_json:binding".to_owned());
    }
    broken.sort();
    broken.dedup();
    (missing, unavailable, stale, broken)
}

fn projected_doctor_guard_kind_state(
    facts: &GuardAuditFacts,
    kind: GuardManagedArtifactKind,
) -> String {
    if !facts.artifact_kind_audited(kind) {
        return "not_configured".to_owned();
    }
    let issues = facts.artifact_issues(kind);
    if issues.contains(&GuardArtifactIssue::Unavailable) {
        "not_checked"
    } else if issues.contains(&GuardArtifactIssue::Malformed) {
        "broken"
    } else if issues.contains(&GuardArtifactIssue::Missing) {
        "missing"
    } else if !issues.is_empty() {
        "stale"
    } else {
        "installed"
    }
    .to_owned()
}

fn doctor_guard_file_details(findings: &GuardAuditFacts) -> Value {
    let (missing_files, unavailable_files, stale_files, broken_files) =
        projected_doctor_guard_file_paths(findings);
    json!({
        "missing_files": missing_files,
        "unavailable_files": unavailable_files,
        "stale_files": stale_files,
        "broken_files": broken_files,
        "file_states": doctor_guard_file_states(findings),
        "selected_profiles": &findings.guard_profiles,
        "generated_config_verified": findings.generated_config_verified(),
        "direct_file_write_matcher_coverage": findings.direct_file_write_matcher_coverage(),
    })
}

fn doctor_guard_file_states(findings: &GuardAuditFacts) -> BTreeMap<String, String> {
    let mut states = BTreeMap::new();
    if findings
        .manifest_issues
        .contains(&GuardManifestIssue::Malformed)
    {
        return states;
    }
    for kind in [
        GuardManagedArtifactKind::AgentsManagedBlock,
        GuardManagedArtifactKind::VolicordPolicy,
        GuardManagedArtifactKind::HostHookConfig,
        GuardManagedArtifactKind::HostHookDispatch,
        GuardManagedArtifactKind::HostHookWrapper,
        GuardManagedArtifactKind::HostRuleInstruction,
        GuardManagedArtifactKind::GitInfoExclude,
    ] {
        let state = projected_doctor_guard_kind_state(findings, kind);
        if state != "not_configured" {
            states.insert(kind.as_str().to_owned(), state);
        }
    }
    states
}

fn guard_observation(
    snapshot: &RegistryInspectionSnapshot,
    installation: &volicord_store::inspection::GuardInstallationInspectionRecord,
) -> Option<volicord_store::guards::GuardObservationSummary> {
    guard_installation(
        &snapshot.runtime_home.runtime_home_path,
        &installation.guard_installation_id,
    )
    .ok()
    .flatten()
    .and_then(|record| {
        guard_observation_summary(
            &snapshot.runtime_home.runtime_home_path,
            &installation.project_id,
            &record,
        )
        .ok()
    })
}

fn guard_observation_current(
    snapshot: &RegistryInspectionSnapshot,
    installation: &volicord_store::inspection::GuardInstallationInspectionRecord,
) -> bool {
    guard_observation(snapshot, installation)
        .is_some_and(|summary| summary.all_required_phases_observed())
}

fn guard_configuration_healthy(
    installation: &volicord_store::inspection::GuardInstallationInspectionRecord,
) -> bool {
    guard_manifest_from_json(&installation.manifest_json).is_ok()
        && missing_required_hooks_from_manifest_json(&installation.manifest_json).is_empty()
}

fn guard_effective_active(
    snapshot: &RegistryInspectionSnapshot,
    installation: &volicord_store::inspection::GuardInstallationInspectionRecord,
) -> bool {
    guard_configuration_healthy(installation) && guard_observation_current(snapshot, installation)
}

fn guard_prompt_capture_configured(manifest_json: &str) -> bool {
    guard_manifest_from_json(manifest_json).is_ok_and(|manifest| {
        manifest
            .required_hook_phases
            .contains(&GuardHookPhase::PromptCapture)
    })
}

fn inspect_installation_profile<F>(
    profile: &InstallationProfileInspectionRecord,
    env_var: &F,
    checks: &mut Vec<DiagnosticCheck>,
    direct_action_candidates: &mut Vec<DoctorActionCandidate>,
) -> Vec<InstallationDiagnostic>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    let mut diagnostics = Vec::new();
    let mode_supported = matches!(
        profile.default_connection_mode.as_str(),
        CONNECTION_MODE_WORKFLOW | CONNECTION_MODE_READ_ONLY
    );
    if mode_supported {
        checks.push(
            DiagnosticCheck::passed("installation_profile", "installation profile is present")
                .with_details(json!({
                    "state": "present",
                    "installation_id": profile.installation_id,
                    "default_connection_mode": profile.default_connection_mode,
                    "bin_dir": path_text(&profile.bin_dir),
                })),
        );
    } else {
        checks.push(
            DiagnosticCheck::failed(
                "installation_profile",
                "installation profile has an unsupported default connection mode",
            )
            .with_details(json!({
                "state": "invalid",
                "installation_id": profile.installation_id,
                "default_connection_mode": profile.default_connection_mode,
            })),
        );
        direct_action_candidates.push(direct_action_candidate(
            DoctorDirectAction::InitializeConnection,
            "installation_profile",
            None,
        ));
    }
    if let Some(diagnostic) = inspect_command_path(
        "volicord_command",
        "volicord command",
        &PathBuf::from(&profile.volicord_command),
        checks,
        direct_action_candidates,
    ) {
        diagnostics.push(diagnostic);
    }
    let _ = inspect_command_path(
        "volicord_mcp_command",
        "MCP launch command",
        &PathBuf::from(&profile.volicord_mcp_command),
        checks,
        direct_action_candidates,
    );
    let path_env = env_var(PATH_ENV);
    inspect_command_availability(
        "volicord_command_availability",
        &volicord_binary_name(),
        &PathBuf::from(&profile.volicord_command),
        path_env.as_deref(),
        checks,
        direct_action_candidates,
    );
    inspect_command_availability(
        "volicord_mcp_command_availability",
        &mcp_binary_name(),
        &PathBuf::from(&profile.volicord_mcp_command),
        path_env.as_deref(),
        checks,
        direct_action_candidates,
    );
    inspect_path_or_shim(
        profile,
        path_env.as_deref(),
        checks,
        direct_action_candidates,
    );
    if installed_build_configuration_is_inconsistent(profile) {
        checks.push(DiagnosticCheck::failed(
            "installed_build_configuration",
            "the running Volicord build and managed installation configuration are inconsistent",
        ));
        diagnostics.push(InstallationDiagnostic::ManagedConfigurationInconsistent);
    }
    diagnostics.sort_by_key(|diagnostic| diagnostic.code());
    diagnostics.dedup();
    diagnostics
}

fn inspect_command_path(
    id: &str,
    label: &str,
    command: &Path,
    checks: &mut Vec<DiagnosticCheck>,
    direct_action_candidates: &mut Vec<DoctorActionCandidate>,
) -> Option<InstallationDiagnostic> {
    if command.is_absolute() && is_executable_file(command) {
        checks.push(
            DiagnosticCheck::passed(id, format!("{label} is executable"))
                .with_details(json!({ "path": path_text(command) })),
        );
        None
    } else {
        checks.push(
            DiagnosticCheck::failed(id, format!("{label} is missing or not executable"))
                .with_details(json!({ "path": path_text(command) })),
        );
        if id != "volicord_command" {
            direct_action_candidates.push(direct_action_candidate(
                DoctorDirectAction::RepairMcpCommand,
                id,
                None,
            ));
        }
        if id != "volicord_command" {
            None
        } else if !command.exists() {
            Some(InstallationDiagnostic::ExecutableMissing)
        } else {
            Some(InstallationDiagnostic::ExecutableNotRunnable)
        }
    }
}

fn installed_build_configuration_is_inconsistent(
    profile: &InstallationProfileInspectionRecord,
) -> bool {
    let Ok(current) = std::env::current_exe() else {
        return false;
    };
    let configured = PathBuf::from(&profile.volicord_command);
    let (Ok(current), Ok(configured)) = (fs::canonicalize(current), fs::canonicalize(configured))
    else {
        return false;
    };
    current != configured
}

fn inspect_command_availability(
    id: &str,
    command_name: &str,
    profile_command: &Path,
    path_env: Option<&OsStr>,
    checks: &mut Vec<DiagnosticCheck>,
    direct_action_candidates: &mut Vec<DoctorActionCandidate>,
) {
    let path_match = detect_command_on_path(command_name, path_env);
    let profile_command_directory_on_path = profile_command
        .parent()
        .is_some_and(|directory| path_directory_is_on_path(path_env, directory));
    let path_matches_profile = path_match
        .as_deref()
        .is_some_and(|path| paths_equivalent(path, profile_command));
    let details = json!({
        "command_name": command_name,
        "profile_command": path_text(profile_command),
        "available_on_path": path_match.is_some(),
        "path_matches_profile": path_matches_profile,
        "profile_command_directory_on_path": profile_command_directory_on_path,
        "path_match": path_match.as_deref().map(path_text),
        "agent_host_restart_or_reload_may_be_needed": !path_matches_profile,
    });

    if path_matches_profile {
        checks.push(
            DiagnosticCheck::passed(
                id,
                format!("{command_name} resolves to the installation profile command on PATH"),
            )
            .with_details(details),
        );
    } else if path_match.is_some() {
        checks.push(
            DiagnosticCheck::warning(
                id,
                format!("{command_name} resolves to a different executable on PATH"),
            )
            .with_details(details),
        );
        push_command_availability_action(direct_action_candidates, id);
    } else {
        checks.push(
            DiagnosticCheck::warning(id, format!("{command_name} is not available on PATH"))
                .with_details(details),
        );
        push_command_availability_action(direct_action_candidates, id);
    }
}

fn inspect_path_or_shim(
    profile: &InstallationProfileInspectionRecord,
    path_env: Option<&OsStr>,
    checks: &mut Vec<DiagnosticCheck>,
    direct_action_candidates: &mut Vec<DoctorActionCandidate>,
) {
    let bin_dir_on_path = path_directory_is_on_path(path_env, &profile.bin_dir);
    let volicord_link = profile.bin_dir.join(volicord_binary_name());
    let mcp_link = profile.bin_dir.join(mcp_binary_name());
    let link_ready = is_executable_file(&volicord_link) && is_executable_file(&mcp_link);

    if bin_dir_on_path && link_ready {
        checks.push(
            DiagnosticCheck::passed(
                "path_or_shim",
                "profile command directory is on PATH and contains command links",
            )
            .with_details(json!({
                "bin_dir": path_text(&profile.bin_dir),
                "volicord": path_text(&volicord_link),
                "volicord_mcp": path_text(&mcp_link),
                "agent_host_restart_or_reload_may_be_needed": false,
            })),
        );
    } else if bin_dir_on_path {
        checks.push(
            DiagnosticCheck::warning(
                "path_or_shim",
                "profile command directory is on PATH, but command links are incomplete",
            )
            .with_details(json!({
                "bin_dir": path_text(&profile.bin_dir),
                "volicord_link_ready": is_executable_file(&volicord_link),
                "volicord_mcp_link_ready": is_executable_file(&mcp_link),
                "agent_host_restart_or_reload_may_be_needed": true,
            })),
        );
        direct_action_candidates.push(direct_action_candidate(
            DoctorDirectAction::RepairCommandLinks,
            "path_or_shim",
            None,
        ));
    } else if link_ready {
        checks.push(
            DiagnosticCheck::warning(
                "path_or_shim",
                "command links exist, but the link directory is not on PATH",
            )
            .with_details(json!({
                "bin_dir": path_text(&profile.bin_dir),
                "agent_host_restart_or_reload_may_be_needed": true,
            })),
        );
        direct_action_candidates.push(direct_action_candidate(
            DoctorDirectAction::AddCommandLinksToPath,
            "path_or_shim",
            Some(format!(
                "export PATH={}:$PATH",
                doctor_shell_word(&path_text(&profile.bin_dir))
            )),
        ));
    } else {
        checks.push(
            DiagnosticCheck::warning(
                "path_or_shim",
                "no command link directory is active for this shell",
            )
            .with_details(json!({
                "bin_dir": path_text(&profile.bin_dir),
                "agent_host_restart_or_reload_may_be_needed": true,
            })),
        );
        direct_action_candidates.push(direct_action_candidate(
            DoctorDirectAction::CreateCommandLinks,
            "path_or_shim",
            None,
        ));
    }
}

fn doctor_status(checks: &[DiagnosticCheck]) -> CommandStatus {
    if checks.iter().any(|check| {
        check.status == "failed"
            && !matches!(
                check.id.as_str(),
                "runtime_home_access" | "registry" | "installation_profile"
            )
    }) {
        CommandStatus::Failed
    } else if checks.iter().any(|check| check.status == "failed") {
        CommandStatus::ActionRequired
    } else {
        CommandStatus::Complete
    }
}

fn doctor_current_timestamp() -> UtcTimestamp {
    UtcTimestamp::from_datetime(DateTime::<Utc>::from(SystemTime::now()))
}

fn doctor_installation_finding(
    diagnostic: InstallationDiagnostic,
    observed_at: UtcTimestamp,
) -> Result<DiagnosticFinding, DoctorCommandError> {
    let subject = InstallationSubject::current().map_err(DoctorCommandError::Runtime)?;
    occurrence_finding(
        OperationalDiagnostic::Installation(diagnostic),
        &subject,
        &InstallationFacts::default(),
        OperationalCheckState::Failed,
        observed_at,
    )
    .map_err(|error| DoctorCommandError::Runtime(error.to_string()))
}

fn doctor_store_finding(
    diagnostic: StoreDiagnostic,
    observed_state: &'static str,
    observed_at: UtcTimestamp,
) -> Result<DiagnosticFinding, DoctorCommandError> {
    let facts = StoreDiagnosticFacts {
        database_kind: Some("registry"),
        observed_state: Some(observed_state),
        sqlite_primary_code: None,
        sqlite_extended_code: None,
        constraint_kind: None,
        entity: None,
        field: None,
        io_error_kind: None,
    };
    store_diagnostic_finding_from_kind(
        diagnostic,
        format!("finding.doctor.{}", diagnostic.code().replace('.', "_")),
        Some("registry"),
        &facts,
        observed_at,
    )
    .map_err(|error| DoctorCommandError::Runtime(error.to_string()))
}

fn host_detection_check() -> DiagnosticCheck {
    DiagnosticCheck::skipped(
        "host_detection",
        "built-in host adapter detection is reported by init or connection verification",
    )
    .with_details(json!({
        "accepted_host_values": [HostKind::Codex.as_str()]
    }))
}

fn render_doctor_output(
    output: OutputFormat,
    report: &DoctorReport,
) -> Result<String, DoctorCommandError> {
    match output {
        OutputFormat::Json => {
            let actions_required = report
                .remediation_plan
                .required_actions()
                .collect::<Vec<_>>();
            let actions_recommended = report
                .remediation_plan
                .recommended_actions()
                .collect::<Vec<_>>();
            serde_json::to_string_pretty(&json!({
                "status": report.status.as_str(),
                "status_meaning": report.status_meaning.as_str(),
                "build": &report.build,
                "summary_card": &report.summary_card,
                "disclosure": {
                    "guarantee_class": "diagnostic_observation",
                    "non_guarantees": [
                        "NotOsSandbox",
                        "NotFullWritePrevention",
                        "NotActorAttributionProof",
                        "NotCorrectnessProof",
                    ],
                },
                "runtime_home": path_text(&report.runtime_home),
                "states": doctor_states_json(&report.runtime_home, &report.checks),
                "checks": &report.checks,
                "findings": &report.findings,
                "warning_count": report.warning_count(),
                "actions": report.remediation_plan.actions(),
                "actions_required": actions_required,
                "actions_recommended": actions_recommended,
                "primary_next_action": report.remediation_plan.primary_action(),
            }))
            .map(|text| format!("{text}\n"))
            .map_err(|error| DoctorCommandError::Runtime(error.to_string()))
        }
        OutputFormat::Compact => Ok(render_compact_doctor_text(report)),
        OutputFormat::Verbose => render_verbose_doctor_text(report),
    }
}

fn doctor_headline(report: &DoctorReport) -> String {
    match report.status {
        CommandStatus::Complete if report.warning_count() == 0 => "Volicord is ready.".to_owned(),
        CommandStatus::Complete if report.warning_count() == 1 => {
            "Volicord is ready with 1 warning.".to_owned()
        }
        CommandStatus::Complete => format!(
            "Volicord is ready with {} warnings.",
            report.warning_count()
        ),
        CommandStatus::ActionRequired | CommandStatus::Failed => {
            "Volicord needs attention.".to_owned()
        }
    }
}

fn render_compact_doctor_text(report: &DoctorReport) -> String {
    let mut body = doctor_compact_facts(report);
    let warnings = report
        .checks
        .iter()
        .filter(|check| check.status == "warning")
        .map(|check| check.summary.as_str())
        .collect::<Vec<_>>();
    if !warnings.is_empty() {
        body.push(Section::new("Warnings", vec![BulletList::new(warnings).into()]).into());
    }
    let problems = report
        .checks
        .iter()
        .filter(|check| check.status == "failed")
        .map(|check| check.summary.as_str())
        .collect::<Vec<_>>();
    if !problems.is_empty() {
        body.push(Section::new("Problems", vec![BulletList::new(problems).into()]).into());
    }
    body.extend(doctor_compact_action_elements(&report.remediation_plan));
    Document::new(doctor_headline(report), body).render()
}

fn doctor_compact_facts(report: &DoctorReport) -> Vec<Element> {
    vec![
        Field::new(
            "Runtime Home",
            HumanValue::text(path_text(&report.runtime_home)),
        )
        .into(),
        Field::new(
            "Installation profile",
            HumanValue::text(display_state_text(doctor_installation_profile_state(
                &report.checks,
            ))),
        )
        .into(),
        doctor_count_field(&report.checks, "Projects", "projects"),
        doctor_count_field(&report.checks, "Connections", "connections"),
        Field::new(
            "Selected profile",
            HumanValue::text(display_state_text(&doctor_selected_profile_from_checks(
                &report.checks,
            ))),
        )
        .into(),
        Field::new(
            "Guard state",
            HumanValue::text(display_state_text(doctor_check_state(
                &report.checks,
                GUARD_OBSERVATION_CHECK_ID,
            ))),
        )
        .into(),
        Field::new(
            "Prompt capture",
            HumanValue::text(display_state_text(&doctor_prompt_capture_status(
                &report.checks,
            ))),
        )
        .into(),
        Field::new(
            "Host reload required",
            HumanValue::YesNo(YesNo::from(doctor_host_reload_required(&report.checks))),
        )
        .into(),
        Field::new("Version", HumanValue::text(report.build.package_version)).into(),
        Field::new(
            "Source",
            HumanValue::text(doctor_build_source(&report.build)),
        )
        .into(),
    ]
}

fn doctor_count_field(checks: &[DiagnosticCheck], label: &'static str, key: &str) -> Element {
    let value = doctor_count(checks, key)
        .map(HumanValue::Count)
        .unwrap_or_else(|| HumanValue::text("unknown"));
    Field::new(label, value).into()
}

fn doctor_count(checks: &[DiagnosticCheck], key: &str) -> Option<usize> {
    checks
        .iter()
        .find(|check| check.id == "registry_counts")
        .and_then(|check| check.details.as_ref())
        .and_then(|details| details.get(key))
        .and_then(Value::as_u64)
        .and_then(|count| usize::try_from(count).ok())
}

fn doctor_build_source(build: &BuildInfo) -> String {
    let tree = match build.git_dirty {
        Some(false) => "clean",
        Some(true) => "dirty",
        None => "tree state not recorded",
    };
    format!("{} ({tree})", build.git_commit)
}

fn doctor_compact_action_elements(plan: &DoctorRemediationPlan) -> Vec<Element> {
    let Some(action) = plan.primary_action() else {
        return vec![ActionHint::new("none").into()];
    };
    let mut elements = vec![
        ActionHint::new(trimmed_sentence(action.summary())).into(),
        Field::new("Action", HumanValue::text(action.code().as_str())).into(),
        Field::new("Requirement", HumanValue::text(action.urgency().as_str())).into(),
    ];
    if let Some(command) = action.command() {
        elements.push(Field::new("Command", HumanValue::text(command)).into());
    }
    elements
}

fn doctor_verbose_action_elements(plan: &DoctorRemediationPlan) -> Vec<Element> {
    let Some(primary) = plan.primary_action() else {
        return vec![ActionHint::new("none").into()];
    };
    let mut elements =
        vec![Section::new("Primary action", vec![doctor_verbose_action_item(primary)]).into()];
    let required = plan
        .required_actions()
        .map(doctor_verbose_action_item)
        .collect::<Vec<_>>();
    if !required.is_empty() {
        elements.push(Section::new("Required actions", required).into());
    }
    let recommended = plan
        .recommended_actions()
        .map(doctor_verbose_action_item)
        .collect::<Vec<_>>();
    if !recommended.is_empty() {
        elements.push(Section::new("Recommended actions", recommended).into());
    }
    elements
}

fn doctor_verbose_action_item(action: &DoctorRemediationAction) -> Element {
    let mut fields = vec![
        Field::new("Summary", HumanValue::text(action.summary())).into(),
        Field::new("Urgency", HumanValue::text(action.urgency().as_str())).into(),
        Field::new("Priority", HumanValue::text(action.priority().as_str())).into(),
    ];
    if let Some(command) = action.command() {
        fields.push(Field::new("Command", HumanValue::text(command)).into());
    }
    Section::new(action.code().as_str(), fields).into()
}

fn render_verbose_doctor_text(report: &DoctorReport) -> Result<String, DoctorCommandError> {
    let mut body = vec![
        Section::new("Installation", doctor_compact_facts(report)).into(),
        Section::new(
            "Build provenance",
            vec![
                Field::new(
                    "Package version",
                    HumanValue::text(report.build.package_version),
                )
                .into(),
                Field::new("Commit", HumanValue::text(report.build.git_commit)).into(),
                Field::new(
                    "Tree",
                    HumanValue::text(match report.build.git_dirty {
                        Some(false) => "clean",
                        Some(true) => "dirty",
                        None => "not recorded",
                    }),
                )
                .into(),
                Field::new(
                    "Metadata source",
                    HumanValue::text(report.build.metadata_source),
                )
                .into(),
                Field::new("Target", HumanValue::text(report.build.target_triple)).into(),
                Field::new(
                    "Profile class",
                    HumanValue::text(report.build.profile_class),
                )
                .into(),
                Field::new(
                    "Profile precision",
                    HumanValue::text(report.build.profile_precision.as_str()),
                )
                .into(),
                Field::new(
                    "Exact Cargo profile",
                    report
                        .build
                        .build_profile
                        .map(HumanValue::text)
                        .unwrap_or(HumanValue::None),
                )
                .into(),
                Field::new("Optimization", HumanValue::text(report.build.opt_level)).into(),
                Field::new(
                    "Debug assertions",
                    report
                        .build
                        .debug
                        .map(|value| HumanValue::YesNo(YesNo::from(value)))
                        .unwrap_or(HumanValue::None),
                )
                .into(),
                Field::new("Build ID", HumanValue::text(&report.build.build_id)).into(),
            ],
        )
        .into(),
        Section::new(
            "Checks",
            report
                .checks
                .iter()
                .map(doctor_check_element)
                .collect::<Result<Vec<_>, _>>()?,
        )
        .into(),
    ];
    if !report.findings.is_empty() {
        body.push(
            Section::new(
                "Structured findings",
                report
                    .findings
                    .iter()
                    .map(|finding| {
                        let value = serde_json::to_value(finding)
                            .map_err(|error| DoctorCommandError::Runtime(error.to_string()))?;
                        Ok(
                            Section::new(finding.code().as_str(), json_value_elements(&value))
                                .into(),
                        )
                    })
                    .collect::<Result<Vec<_>, DoctorCommandError>>()?,
            )
            .into(),
        );
    }
    body.extend(doctor_verbose_action_elements(&report.remediation_plan));
    body.push(
        Section::new(
            "Output scope",
            vec![Field::new(
                "Disclosure",
                HumanValue::text(
                    "Local setup diagnostics are not OS enforcement, write prevention, actor attribution proof, correctness proof, test sufficiency proof, or review completion.",
                ),
            )
            .into()],
        )
        .into(),
    );
    Ok(Document::verbose(doctor_headline(report), body).render())
}

fn doctor_check_element(check: &DiagnosticCheck) -> Result<Element, DoctorCommandError> {
    let mut body = vec![
        Field::new(
            "Status",
            HumanValue::text(display_state_text(&check.status)),
        )
        .into(),
        Field::new("Summary", HumanValue::text(&check.summary)).into(),
    ];
    if let Some(details) = &check.details {
        let mut detail_elements = if check.hook_path_safety.is_some() {
            let mut without_assessment = details.clone();
            if let Some(object) = without_assessment.as_object_mut() {
                object.remove("hook_path_safety");
            }
            json_value_elements(&without_assessment)
        } else {
            json_value_elements(details)
        };
        if let Some(assessment) = &check.hook_path_safety {
            detail_elements.extend([
                Field::new(
                    "Hook path safety",
                    HumanValue::text(display_state_text(assessment.state.as_str())),
                )
                .into(),
                Field::new(
                    "Hook command CWD independence",
                    HumanValue::text(display_state_text(assessment.cwd_independence.as_str())),
                )
                .into(),
                Field::new(
                    "Hook command subdirectory safety",
                    HumanValue::text(display_state_text(assessment.subdirectory_safety.as_str())),
                )
                .into(),
            ]);
            if !assessment.evidence.is_empty() {
                detail_elements.push(
                    Section::new(
                        "Hook path-safety evidence",
                        json_value_elements(&json!(&assessment.evidence)),
                    )
                    .into(),
                );
            }
        }
        body.push(Section::new("Details", detail_elements).into());
    }
    Ok(Section::new(&check.id, body).into())
}

fn json_value_elements(value: &Value) -> Vec<Element> {
    match value {
        Value::Object(values) => values
            .iter()
            .map(|(key, value)| json_value_element(key, value))
            .collect(),
        value => vec![Field::new("Value", json_human_value(value)).into()],
    }
}

fn json_value_element(key: &str, value: &Value) -> Element {
    match value {
        Value::Object(_) => Section::new(key, json_value_elements(value)).into(),
        Value::Array(values) if values.is_empty() => Field::new(key, HumanValue::None).into(),
        Value::Array(values)
            if values
                .iter()
                .all(|value| !matches!(value, Value::Object(_) | Value::Array(_))) =>
        {
            Section::new(
                key,
                vec![BulletList::new(
                    values
                        .iter()
                        .map(|value| json_human_value(value).to_string()),
                )
                .into()],
            )
            .into()
        }
        Value::Array(values) => Section::new(
            key,
            values
                .iter()
                .enumerate()
                .map(|(index, value)| {
                    Section::new(format!("Item {}", index + 1), json_value_elements(value)).into()
                })
                .collect(),
        )
        .into(),
        value => Field::new(key, json_human_value(value)).into(),
    }
}

fn json_human_value(value: &Value) -> HumanValue {
    match value {
        Value::Null => HumanValue::None,
        Value::Bool(value) => HumanValue::YesNo(YesNo::from(*value)),
        Value::Number(value) => HumanValue::text(value.to_string()),
        Value::String(value) => HumanValue::text(value),
        Value::Array(_) | Value::Object(_) => HumanValue::text(value.to_string()),
    }
}

fn display_state_text(value: &str) -> String {
    value.replace('_', " ")
}

fn trimmed_sentence(value: &str) -> &str {
    value.trim().trim_end_matches('.')
}

fn doctor_states_json(runtime_home: &Path, checks: &[DiagnosticCheck]) -> Value {
    let mut states = json!({
        "runtime_home": doctor_runtime_home_state(runtime_home, checks),
        "installation_profile": doctor_installation_profile_state(checks),
        "command_availability": doctor_command_state(checks),
        "project_registration": doctor_count_state(checks, "projects", "registered"),
        "connection": doctor_count_state(checks, "connections", "stored"),
        "mcp_config": doctor_mcp_config_state(checks),
        "guard_installation": doctor_count_state(checks, "guard_installations", "stored"),
        "selected_profile": doctor_selected_profile_from_checks(checks),
        "control_surface": doctor_control_surface_value(checks),
        "cooperative_pre_tool_warning_available": doctor_control_surface_bool(checks, "cooperative_pre_tool_warning_available"),
        "cooperative_pre_tool_denial_available": doctor_control_surface_bool(checks, "cooperative_pre_tool_denial_available"),
        "post_tool_correlation_available": doctor_host_hook_guard_available(checks),
        "bypass_detection_active": false,
        "prompt_capture_available": doctor_prompt_capture_available(checks),
        "guard_configuration": doctor_check_state(checks, GUARD_FILES_CHECK_ID),
        "guard_observation": doctor_check_state(checks, GUARD_OBSERVATION_CHECK_ID),
        "guard_effective": doctor_check_state(checks, GUARD_OBSERVATION_CHECK_ID),
        "guard_files": doctor_check_state(checks, GUARD_FILES_CHECK_ID),
        "agents_managed_block": doctor_guard_file_kind_state(
            checks,
            GuardManagedArtifactKind::AgentsManagedBlock.as_str(),
        ),
        "volicord_policy_file": doctor_guard_file_kind_state(
            checks,
            GuardManagedArtifactKind::VolicordPolicy.as_str(),
        ),
        "rule_instruction_config": doctor_guard_file_kind_state(
            checks,
            GuardManagedArtifactKind::HostRuleInstruction.as_str(),
        ),
        "hook_config": doctor_guard_file_kind_state(
            checks,
            GuardManagedArtifactKind::HostHookConfig.as_str(),
        ),
        "required_hook_phases": doctor_required_hook_phases_state(checks),
        "missing_required_hooks": doctor_missing_required_hooks_value(checks),
        "guard_hook_observed": doctor_check_state(checks, GUARD_OBSERVATION_CHECK_ID),
        "guard_status": doctor_check_state(checks, GUARD_OBSERVATION_CHECK_ID),
        "prompt_capture": doctor_prompt_capture_health(checks),
        "prompt_capture_status": doctor_prompt_capture_status(checks),
        "host_reload_required": doctor_host_reload_required(checks),
    });
    if let Some(object) = states.as_object_mut() {
        object.insert(
            "generated_config_verified".to_owned(),
            Value::Bool(doctor_generated_config_verified_state(checks)),
        );
        object.insert(
            "hook_path_safety".to_owned(),
            json!(doctor_hook_path_safety_assessment(checks)),
        );
        object.insert(
            "direct_file_write_matcher_coverage".to_owned(),
            Value::Bool(doctor_direct_file_write_matcher_coverage(checks)),
        );
    }
    states
}

fn doctor_runtime_home_state(runtime_home: &Path, checks: &[DiagnosticCheck]) -> String {
    if !runtime_home.exists() {
        return "missing".to_owned();
    }
    match check_status(checks, "runtime_home_access") {
        Some("passed") => "ready".to_owned(),
        Some("failed") => "not_accessible".to_owned(),
        _ => "unknown".to_owned(),
    }
}

fn doctor_installation_profile_state(checks: &[DiagnosticCheck]) -> &'static str {
    let Some(check) = checks
        .iter()
        .find(|check| check.id == "installation_profile")
    else {
        return "unknown";
    };
    if let Some(state) = check
        .details
        .as_ref()
        .and_then(|details| details.get("state"))
        .and_then(Value::as_str)
    {
        return match state {
            "present" => "present",
            "missing" => "missing",
            "invalid" => "invalid",
            "unavailable" => "unavailable",
            "corrupt" => "corrupt",
            _ => "unknown",
        };
    }
    match check.status.as_str() {
        "skipped" => "not_checked",
        _ => "unknown",
    }
}

fn doctor_command_state(checks: &[DiagnosticCheck]) -> &'static str {
    if checks.iter().any(|check| {
        matches!(
            check.id.as_str(),
            "volicord_command" | "volicord_mcp_command"
        ) && check.status == "failed"
    }) {
        "not_found"
    } else if checks.iter().any(|check| {
        matches!(
            check.id.as_str(),
            "volicord_command_availability" | "volicord_mcp_command_availability" | "path_or_shim"
        ) && check.status == "warning"
    }) {
        "action_recommended"
    } else if checks.iter().any(|check| {
        matches!(
            check.id.as_str(),
            "volicord_command_availability" | "volicord_mcp_command_availability" | "path_or_shim"
        ) && check.status == "skipped"
    }) {
        "not_checked"
    } else {
        "ready"
    }
}

fn doctor_check_state(checks: &[DiagnosticCheck], id: &str) -> &'static str {
    match check_status(checks, id) {
        Some("passed") => "ready",
        Some("warning") => "action_recommended",
        Some("failed") => "failed",
        Some("skipped") => "not_checked",
        _ => "unknown",
    }
}

fn doctor_guard_file_kind_state(checks: &[DiagnosticCheck], kind: &str) -> String {
    checks
        .iter()
        .find(|check| check.id == GUARD_FILES_CHECK_ID)
        .and_then(|check| check.details.as_ref())
        .and_then(|details| details.get("file_states"))
        .and_then(|states| states.get(kind))
        .and_then(Value::as_str)
        .map(str::to_owned)
        .unwrap_or_else(|| match check_status(checks, GUARD_FILES_CHECK_ID) {
            Some("skipped") | None => "not_checked".to_owned(),
            _ => "not_configured".to_owned(),
        })
}

fn doctor_guard_file_bool_detail(checks: &[DiagnosticCheck], key: &str) -> bool {
    checks
        .iter()
        .find(|check| check.id == GUARD_FILES_CHECK_ID)
        .and_then(|check| check.details.as_ref())
        .and_then(|details| details.get(key))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn doctor_generated_config_verified_state(checks: &[DiagnosticCheck]) -> bool {
    doctor_guard_file_bool_detail(checks, "generated_config_verified")
}

fn doctor_hook_path_safety_assessment(checks: &[DiagnosticCheck]) -> HookPathSafetyAssessment {
    checks
        .iter()
        .find(|check| check.id == GUARD_FILES_CHECK_ID)
        .and_then(|check| check.hook_path_safety.clone())
        .unwrap_or_else(HookPathSafetyAssessment::not_checked)
}

fn doctor_direct_file_write_matcher_coverage(checks: &[DiagnosticCheck]) -> bool {
    doctor_guard_file_bool_detail(checks, "direct_file_write_matcher_coverage")
}

fn doctor_selected_profile_from_checks(checks: &[DiagnosticCheck]) -> String {
    checks
        .iter()
        .find(|check| check.id == "control_surface")
        .and_then(|check| check.details.as_ref())
        .and_then(|details| details.get("selected_profile"))
        .and_then(Value::as_str)
        .map(str::to_owned)
        .unwrap_or_else(|| match check_status(checks, "control_surface") {
            Some("skipped") | None => "not_checked".to_owned(),
            _ => "unknown".to_owned(),
        })
}

fn doctor_control_surface_value(checks: &[DiagnosticCheck]) -> Value {
    checks
        .iter()
        .find(|check| check.id == "control_surface")
        .and_then(|check| check.details.as_ref())
        .and_then(|details| details.get("control_surface"))
        .cloned()
        .unwrap_or_else(|| {
            json!({
                "selected_profile": doctor_selected_profile_from_checks(checks),
                "host_hooks_active": false,
                "cooperative_pre_tool_warning_available": false,
                "cooperative_pre_tool_denial_available": false,
                "unrecorded_changes_detectable": false,
                "actor_identity_provable": false,
                "os_enforced": false,
            })
        })
}

fn doctor_control_surface_bool(checks: &[DiagnosticCheck], key: &str) -> bool {
    doctor_control_surface_value(checks)
        .get(key)
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn doctor_required_hook_phases_state(checks: &[DiagnosticCheck]) -> &'static str {
    match check_status(checks, GUARD_FILES_CHECK_ID) {
        Some("passed") => "configured",
        Some("warning") | Some("failed") => "missing",
        Some("skipped") => "not_checked",
        _ => "unknown",
    }
}

fn doctor_missing_required_hooks_value(checks: &[DiagnosticCheck]) -> Vec<String> {
    checks
        .iter()
        .find(|check| check.id == GUARD_FILES_CHECK_ID)
        .and_then(|check| check.details.as_ref())
        .and_then(|details| details.get("missing_required_hooks"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}

fn doctor_prompt_capture_health(checks: &[DiagnosticCheck]) -> &'static str {
    if check_status(checks, GUARD_OBSERVATION_CHECK_ID).is_none() {
        "not_checked"
    } else {
        doctor_check_state(checks, GUARD_OBSERVATION_CHECK_ID)
    }
}

fn doctor_prompt_capture_status(checks: &[DiagnosticCheck]) -> String {
    checks
        .iter()
        .find(|check| check.id == GUARD_OBSERVATION_CHECK_ID)
        .and_then(|check| check.details.as_ref())
        .map(|details| {
            let configured = details
                .get("prompt_capture_configured")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            let observed = details
                .get("prompt_capture_observed")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            if observed > 0 {
                "available"
            } else if configured > 0 {
                "configured_unobserved"
            } else {
                "not_configured"
            }
        })
        .unwrap_or("not_checked")
        .to_owned()
}

fn doctor_host_hook_guard_available(checks: &[DiagnosticCheck]) -> bool {
    doctor_control_surface_bool(checks, "host_hooks_active")
}

fn doctor_prompt_capture_available(checks: &[DiagnosticCheck]) -> bool {
    matches!(
        doctor_prompt_capture_status(checks).as_str(),
        "available" | "configured_unobserved"
    )
}

fn doctor_count_state(checks: &[DiagnosticCheck], key: &str, suffix: &str) -> String {
    checks
        .iter()
        .find(|check| check.id == "registry_counts")
        .and_then(|check| check.details.as_ref())
        .and_then(|details| details.get(key))
        .and_then(Value::as_u64)
        .map(|count| format!("{count} {suffix}"))
        .unwrap_or_else(|| "unknown".to_owned())
}

fn doctor_mcp_config_state(checks: &[DiagnosticCheck]) -> String {
    checks
        .iter()
        .find(|check| check.id == "registry_counts")
        .and_then(|check| check.details.as_ref())
        .and_then(|details| details.get("connections"))
        .and_then(Value::as_u64)
        .map(|count| {
            if count == 0 {
                "not_configured".to_owned()
            } else {
                format!("{count} stored")
            }
        })
        .unwrap_or_else(|| "unknown".to_owned())
}

fn doctor_host_reload_required(checks: &[DiagnosticCheck]) -> bool {
    checks.iter().any(|check| {
        check
            .details
            .as_ref()
            .and_then(|details| details.get("agent_host_restart_or_reload_may_be_needed"))
            .and_then(Value::as_bool)
            .unwrap_or(false)
    })
}

fn check_status<'a>(checks: &'a [DiagnosticCheck], id: &str) -> Option<&'a str> {
    checks
        .iter()
        .find(|check| check.id == id)
        .map(|check| check.status.as_str())
}

fn doctor_summary_card(
    checks: &[DiagnosticCheck],
    remediation_plan: &DoctorRemediationPlan,
) -> SummaryCard {
    SummaryCard {
        task: "not_selected".to_owned(),
        recording: "diagnostic_observation".to_owned(),
        profile: doctor_selected_profile_from_checks(checks),
        write_ticket: "not_selected".to_owned(),
        evidence: "not_selected".to_owned(),
        user_action: "not_selected".to_owned(),
        changes: "not_selected".to_owned(),
        close_status: "not_selected".to_owned(),
        transport: "local CLI".to_owned(),
        next: match remediation_plan.primary_action() {
            Some(action) => format!("{}: {}", action.urgency().as_str(), action.summary()),
            None => "none".to_owned(),
        },
        next_action: None,
        guarantee: DIAGNOSTIC_SUMMARY_GUARANTEE.to_owned(),
    }
}

fn direct_action_candidate(
    action: DoctorDirectAction,
    check_id: impl Into<String>,
    command: Option<String>,
) -> DoctorActionCandidate {
    DoctorActionCandidate::direct(action, check_id, command)
}

fn push_command_availability_action(
    direct_action_candidates: &mut Vec<DoctorActionCandidate>,
    check_id: &str,
) {
    direct_action_candidates.push(direct_action_candidate(
        DoctorDirectAction::MakeProfileCommandsAvailable,
        check_id,
        None,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn complete_build() -> BuildInfo {
        let mut build = BuildInfo {
            package_version: "test-package-version",
            git_commit: "0123456789abcdef0123456789abcdef01234567",
            git_dirty: Some(false),
            metadata_source: "repository",
            target_triple: "test-target",
            build_profile: Some("test-profile"),
            profile_class: "test-class",
            profile_precision: volicord_mcp::BuildProfilePrecision::Exact,
            opt_level: "test-optimization",
            debug: Some(false),
            build_id: String::new(),
        };
        build.build_id = build.deterministic_build_id();
        build
    }

    fn report(
        status: CommandStatus,
        checks: Vec<DiagnosticCheck>,
        direct_action_candidates: Vec<DoctorActionCandidate>,
    ) -> DoctorReport {
        report_with(
            status,
            complete_build(),
            checks,
            Vec::new(),
            direct_action_candidates,
        )
    }

    fn report_with(
        status: CommandStatus,
        build: BuildInfo,
        mut checks: Vec<DiagnosticCheck>,
        findings: Vec<DiagnosticFinding>,
        direct_action_candidates: Vec<DoctorActionCandidate>,
    ) -> DoctorReport {
        checks.splice(
            0..0,
            [
                DiagnosticCheck::passed("installation_profile", "installation profile is present")
                    .with_details(json!({ "state": "present" })),
                DiagnosticCheck::passed("registry_counts", "registry records counted")
                    .with_details(json!({ "projects": 2, "connections": 1 })),
                DiagnosticCheck::passed("control_surface", "record profile is selected")
                    .with_details(json!({ "selected_profile": "record" })),
                DiagnosticCheck::passed(
                    GUARD_OBSERVATION_CHECK_ID,
                    "Guard observations are current",
                )
                .with_details(json!({
                    "prompt_capture_configured": 1,
                    "prompt_capture_observed": 1,
                })),
                host_detection_check(),
            ],
        );
        DoctorReport::finalize(
            status,
            PathBuf::from("/tmp/volicord doctor test/runtime home/with-a-deliberately-long-path"),
            build,
            checks,
            findings,
            direct_action_candidates,
        )
        .expect("test Doctor report should finalize")
    }

    fn report_for_build(build: BuildInfo) -> DoctorReport {
        let (check, diagnostic) = inspect_build_identity(&build);
        let findings = diagnostic
            .map(|diagnostic| {
                doctor_installation_finding(
                    diagnostic,
                    UtcTimestamp::parse("2026-07-31T00:00:00Z")
                        .expect("test observation timestamp"),
                )
                .expect("test installation finding")
            })
            .into_iter()
            .collect();
        report_with(
            CommandStatus::Complete,
            build,
            vec![check],
            findings,
            Vec::new(),
        )
    }

    fn action_codes(value: &Value, field: &str) -> BTreeSet<String> {
        value[field]
            .as_array()
            .expect("action collection")
            .iter()
            .map(|action| {
                action["code"]
                    .as_str()
                    .expect("typed action code")
                    .to_owned()
            })
            .collect()
    }

    fn json_contains_key(value: &Value, key: &str) -> bool {
        match value {
            Value::Object(object) => {
                object.contains_key(key)
                    || object.values().any(|nested| json_contains_key(nested, key))
            }
            Value::Array(values) => values.iter().any(|nested| json_contains_key(nested, key)),
            _ => false,
        }
    }

    fn assert_cross_projection_parity(report: &DoctorReport) {
        let json_text =
            render_doctor_output(OutputFormat::Json, report).expect("Doctor JSON rendering");
        let json: Value = serde_json::from_str(&json_text).expect("Doctor JSON");
        let actions = action_codes(&json, "actions");
        let required = action_codes(&json, "actions_required");
        let recommended = action_codes(&json, "actions_recommended");
        let union = required
            .union(&recommended)
            .cloned()
            .collect::<BTreeSet<_>>();
        let intersection = required
            .intersection(&recommended)
            .cloned()
            .collect::<BTreeSet<_>>();
        let finding_actions = json["findings"]
            .as_array()
            .expect("findings")
            .iter()
            .flat_map(|finding| {
                finding["actions"]
                    .as_array()
                    .expect("finding actions")
                    .iter()
            })
            .map(|action| {
                action["code"]
                    .as_str()
                    .expect("finding action code")
                    .to_owned()
            })
            .collect::<BTreeSet<_>>();

        assert_eq!(actions, union);
        assert!(intersection.is_empty());
        assert!(finding_actions.is_subset(&actions));
        assert_eq!(
            json["primary_next_action"]["code"].as_str(),
            report
                .remediation_plan
                .primary_action()
                .map(|action| action.code().as_str())
        );
    }

    #[test]
    fn candidate_input_order_does_not_change_any_report_projection() {
        let path_action = direct_action_candidate(
            DoctorDirectAction::MakeProfileCommandsAvailable,
            "path_or_shim",
            None,
        );
        let personal_files_action = direct_action_candidate(
            DoctorDirectAction::ProtectPersonalLocalFiles,
            "personal_local_git_tracking",
            None,
        );
        let checks = vec![
            DiagnosticCheck::warning("path_or_shim", "command path needs follow-up"),
            DiagnosticCheck::warning(
                "personal_local_git_tracking",
                "personal local files need protection",
            ),
        ];
        let first = report(
            CommandStatus::Complete,
            checks.clone(),
            vec![path_action.clone(), personal_files_action.clone()],
        );
        let second = report(
            CommandStatus::Complete,
            checks,
            vec![personal_files_action, path_action],
        );

        for format in [
            OutputFormat::Json,
            OutputFormat::Compact,
            OutputFormat::Verbose,
        ] {
            assert_eq!(
                render_doctor_output(format, &first).expect("first Doctor projection"),
                render_doctor_output(format, &second).expect("second Doctor projection")
            );
        }
    }

    #[test]
    fn report_finalization_propagates_remediation_conflicts() {
        let candidate = direct_action_candidate(
            DoctorDirectAction::MakeProfileCommandsAvailable,
            "path_or_shim",
            None,
        );
        let conflicting = candidate
            .clone()
            .with_summary("Perform an unrelated repair");
        let result = DoctorReport::finalize(
            CommandStatus::Complete,
            PathBuf::from("/tmp/volicord-doctor-conflict-test"),
            complete_build(),
            vec![DiagnosticCheck::warning(
                "path_or_shim",
                "command path needs follow-up",
            )],
            Vec::new(),
            vec![candidate, conflicting],
        );

        assert!(matches!(
            result,
            Err(DoctorCommandError::Runtime(detail))
                if detail.contains("conflicting summaries")
        ));
    }

    #[test]
    fn compact_doctor_projects_ready_warning_and_failure_context() {
        let ready = report(CommandStatus::Complete, Vec::new(), Vec::new());
        let ready_text = render_doctor_output(OutputFormat::Compact, &ready).unwrap();
        assert!(ready_text.starts_with("Volicord is ready.\n\n"));
        assert!(ready_text.contains("Projects: 2"));
        assert!(ready_text.contains("Connections: 1"));
        assert!(ready_text.contains("Next action: none"));
        assert!(!ready_text.contains("host_detection"));
        assert!(!ready_text.contains("not shown in this view"));

        let warning = report(
            CommandStatus::Complete,
            vec![DiagnosticCheck::warning(
                "path_or_shim",
                "the command is not currently on PATH",
            )],
            vec![direct_action_candidate(
                DoctorDirectAction::MakeProfileCommandsAvailable,
                "path_or_shim",
                None,
            )],
        );
        let warning_text = render_doctor_output(OutputFormat::Compact, &warning).unwrap();
        let warning_verbose = render_doctor_output(OutputFormat::Verbose, &warning).unwrap();
        assert_eq!(warning.remediation_plan.actions().len(), 1);
        assert!(warning_text.starts_with("Volicord is ready with 1 warning.\n\n"));
        assert!(warning_text.contains("Warnings\n  - the command is not currently on PATH"));
        assert!(warning_text.contains("Requirement: recommended"));
        assert!(warning_text.contains(
            "Make the installation-profile commands resolve on PATH and reload existing agent hosts after PATH or command-link changes"
        ));
        assert!(
            warning_text.contains("Action: action.installation.make_profile_commands_available")
        );
        assert!(warning_verbose.contains("action.installation.make_profile_commands_available"));
        assert!(warning_verbose.contains("\nRecommended actions\n"));

        let failed = report(
            CommandStatus::Failed,
            vec![DiagnosticCheck::failed(
                "project_policy_authority",
                "project policy authority is corrupt",
            )],
            vec![direct_action_candidate(
                DoctorDirectAction::RepairProjectPolicy,
                "project_policy_authority",
                None,
            )],
        );
        let failed_text = render_doctor_output(OutputFormat::Compact, &failed).unwrap();
        assert!(failed_text.starts_with("Volicord needs attention.\n\n"));
        assert!(failed_text.contains("Problems\n  - project policy authority is corrupt"));
        assert!(failed_text.contains("Requirement: required"));
    }

    #[test]
    fn doctor_modes_share_one_typed_report_without_losing_verbose_details() {
        let report = report(
            CommandStatus::Complete,
            vec![
                DiagnosticCheck::warning("build_identity", "source is dirty").with_details(json!({
                    "nested": {
                        "path": "/tmp/example path",
                        "flags": [true, false],
                    }
                })),
            ],
            Vec::new(),
        );
        let compact = render_doctor_output(OutputFormat::Compact, &report).unwrap();
        let verbose = render_doctor_output(OutputFormat::Verbose, &report).unwrap();
        let json_text = render_doctor_output(OutputFormat::Json, &report).unwrap();
        let json: Value = serde_json::from_str(&json_text).unwrap();

        assert_eq!(json["status"], report.status.as_str());
        assert_eq!(json["warning_count"], report.warning_count());
        assert_eq!(
            json["checks"].as_array().unwrap().len(),
            report.checks.len()
        );
        assert!(compact.contains("source is dirty"));
        assert!(compact.contains(&path_text(&report.runtime_home)));
        assert!(!compact.contains("nested"));
        assert!(!compact.contains("Output scope"));
        assert!(verbose.contains("build_identity"));
        assert!(verbose.contains("nested"));
        assert!(verbose.contains("path: /tmp/example path"));
        assert!(verbose.contains("Output scope"));
        for output in [&compact, &verbose, &json_text] {
            assert!(output.ends_with('\n'));
            assert!(!output.ends_with("\n\n"));
            assert!(!output.contains('\t'));
        }
    }

    #[test]
    fn doctor_projects_one_exact_hook_path_safety_assessment() {
        let verified_assessment =
            HookPathSafetyAssessment::test_state(HookPathSafetyState::Verified);
        let verified = report(
            CommandStatus::Complete,
            vec![DiagnosticCheck::passed(
                GUARD_FILES_CHECK_ID,
                "Guard files and Hook path safety are verified",
            )
            .with_details(json!({ "generated_config_verified": true }))
            .with_hook_path_safety(verified_assessment.clone())],
            Vec::new(),
        );
        let verified_json_text =
            render_doctor_output(OutputFormat::Json, &verified).expect("verified Doctor JSON");
        let verified_json: Value =
            serde_json::from_str(&verified_json_text).expect("typed verified Doctor JSON");
        let expected_verified = json!({
            "state": "verified",
            "cwd_independence": "verified",
            "subdirectory_safety": "verified",
            "evidence": [],
        });
        assert_eq!(
            verified_json["states"]["hook_path_safety"],
            expected_verified
        );
        let guard_check = verified_json["checks"]
            .as_array()
            .expect("Doctor checks")
            .iter()
            .find(|check| check["id"] == GUARD_FILES_CHECK_ID)
            .expect("guard_files check");
        assert_eq!(
            guard_check["details"]["hook_path_safety"],
            expected_verified
        );
        for removed_key in [
            "hook_commands_cwd_independent",
            "hook_commands_subdirectory_safe",
            "hook_path_safety_details",
        ] {
            assert!(!json_contains_key(&verified_json, removed_key));
        }
        let verified_verbose =
            render_doctor_output(OutputFormat::Verbose, &verified).expect("verified verbose");
        assert!(verified_verbose.contains("Hook path safety: verified"));
        assert!(verified_verbose.contains("Hook command CWD independence: verified"));
        assert!(verified_verbose.contains("Hook command subdirectory safety: verified"));
        let verified_compact =
            render_doctor_output(OutputFormat::Compact, &verified).expect("verified compact");
        assert!(!verified_compact.contains("Hook path safety"));

        let not_recorded = report(
            CommandStatus::Complete,
            vec![DiagnosticCheck::warning(
                GUARD_FILES_CHECK_ID,
                "Hook path-safety evidence is not recorded",
            )
            .with_hook_path_safety(HookPathSafetyAssessment::test_state(
                HookPathSafetyState::NotRecorded,
            ))],
            Vec::new(),
        );
        let not_recorded_verbose = render_doctor_output(OutputFormat::Verbose, &not_recorded)
            .expect("not-recorded verbose");
        assert!(not_recorded_verbose.contains("Hook path safety: not recorded"));
        assert!(not_recorded_verbose.contains("Hook command CWD independence: not recorded"));
        assert!(not_recorded_verbose.contains("Hook command subdirectory safety: not recorded"));
        for forbidden in [
            "Hook path safety: no",
            "Hook command CWD independence: no",
            "Hook command subdirectory safety: no",
        ] {
            assert!(!not_recorded_verbose
                .lines()
                .any(|line| line.trim() == forbidden));
        }

        let failed = report(
            CommandStatus::Failed,
            vec![DiagnosticCheck::failed(
                GUARD_FILES_CHECK_ID,
                "Hook path safety failed the current contract audit",
            )
            .with_hook_path_safety(HookPathSafetyAssessment::test_failed(
                crate::guard_integration::audit::HookPathSafetyEvidenceReason::PolicyHashMismatch,
            ))],
            Vec::new(),
        );
        let failed_verbose =
            render_doctor_output(OutputFormat::Verbose, &failed).expect("failed verbose");
        assert!(failed_verbose.contains("Hook path safety: failed"));
        assert!(failed_verbose.contains("Hook command CWD independence: failed"));
        assert!(failed_verbose.contains("Hook command subdirectory safety: failed"));
        assert!(failed_verbose.contains("policy_hash_mismatch"));
    }

    #[test]
    fn status_meaning_distinguishes_warnings_and_remediation_availability() {
        let complete = report(CommandStatus::Complete, Vec::new(), Vec::new());
        let warning_without_action = report(
            CommandStatus::Complete,
            vec![DiagnosticCheck::warning(
                "build_identity",
                "build source requires review",
            )],
            Vec::new(),
        );
        let warning_with_action = report(
            CommandStatus::Complete,
            vec![DiagnosticCheck::warning(
                "path_or_shim",
                "command path needs follow-up",
            )],
            vec![direct_action_candidate(
                DoctorDirectAction::MakeProfileCommandsAvailable,
                "path_or_shim",
                None,
            )],
        );
        let action_required_without_action = report(
            CommandStatus::ActionRequired,
            vec![DiagnosticCheck::failed(
                "runtime_home_access",
                "Runtime Home is unavailable",
            )],
            Vec::new(),
        );
        let action_required_with_action = report(
            CommandStatus::ActionRequired,
            vec![DiagnosticCheck::failed(
                "runtime_home_access",
                "Runtime Home is unavailable",
            )],
            vec![direct_action_candidate(
                DoctorDirectAction::InitializeConnection,
                "runtime_home_access",
                None,
            )],
        );
        let action_required_with_recommendation_only = report(
            CommandStatus::ActionRequired,
            vec![
                DiagnosticCheck::failed("runtime_home_access", "Runtime Home is unavailable"),
                DiagnosticCheck::warning("path_or_shim", "command path needs follow-up"),
            ],
            vec![direct_action_candidate(
                DoctorDirectAction::MakeProfileCommandsAvailable,
                "path_or_shim",
                None,
            )],
        );
        let failed_without_action = report(
            CommandStatus::Failed,
            vec![DiagnosticCheck::failed(
                "project_policy_authority",
                "policy authority is corrupt",
            )],
            Vec::new(),
        );
        let failed_with_action = report(
            CommandStatus::Failed,
            vec![DiagnosticCheck::failed(
                "project_policy_authority",
                "policy authority is corrupt",
            )],
            vec![direct_action_candidate(
                DoctorDirectAction::RepairProjectPolicy,
                "project_policy_authority",
                None,
            )],
        );

        assert_eq!(complete.status_meaning, DoctorStatusMeaning::CompleteReady);
        assert_eq!(
            warning_without_action.status_meaning,
            DoctorStatusMeaning::CompleteWarningsReviewOnly
        );
        assert_eq!(
            warning_with_action.status_meaning,
            DoctorStatusMeaning::CompleteWarningsWithRecommendedRemediation
        );
        assert_eq!(
            action_required_without_action.status_meaning,
            DoctorStatusMeaning::ActionRequiredNoDefinedRemediation
        );
        assert_eq!(
            action_required_with_action.status_meaning,
            DoctorStatusMeaning::ActionRequiredRemediationAvailable
        );
        assert_eq!(
            action_required_with_recommendation_only.status_meaning,
            DoctorStatusMeaning::ActionRequiredNoDefinedRemediation
        );
        assert_eq!(
            failed_without_action.status_meaning,
            DoctorStatusMeaning::FailedNoDefinedRemediation
        );
        assert_eq!(
            failed_with_action.status_meaning,
            DoctorStatusMeaning::FailedRemediationAvailable
        );
    }

    #[test]
    fn privacy_footprint_definition_has_one_ordered_category_per_claim() {
        let definition = PrivacyFootprintDefinition::current();
        let mut identities = BTreeSet::new();
        let mut texts = BTreeSet::new();

        assert_eq!(
            definition.claims.len(),
            PrivacyFootprintClaimId::ProductAssurance as usize + 1
        );
        for (index, claim) in definition.claims.iter().copied().enumerate() {
            assert_eq!(claim.id as usize, index);
            assert!(identities.insert(claim.id), "duplicate claim: {claim:?}");
            assert!(
                texts.insert(claim.text()),
                "duplicate canonical text: {}",
                claim.text()
            );
            assert!(!claim.text().is_empty());
            assert!(!claim.text().chars().any(char::is_control));
            assert_ne!(claim.text(), definition.doctor_output_scope);
        }

        let projection = definition.project("present", "registry.sqlite".to_owned(), None);
        for (kind, actual) in [
            (PrivacyFootprintClaimKind::Stores, &projection.stores),
            (
                PrivacyFootprintClaimKind::DoesNotStore,
                &projection.does_not_store,
            ),
            (
                PrivacyFootprintClaimKind::DoesNotProve,
                &projection.does_not_prove,
            ),
        ] {
            assert_eq!(*actual, definition.claim_texts(kind));
        }
        assert_eq!(
            projection.doctor_output_scope,
            definition.doctor_output_scope
        );
    }

    #[test]
    fn privacy_footprint_human_sections_and_json_are_factually_equivalent() {
        let definition = PrivacyFootprintDefinition::current();
        let report = PrivacyFootprintReport {
            status: "complete",
            runtime_home:
                "/tmp/runtime home/with spaces/and/a/deliberately/long/path/for/rendering"
                    .to_owned(),
            privacy_footprint: definition.project(
                "present",
                "/tmp/runtime home/registry.sqlite".to_owned(),
                Some(PrivacyRecordCounts {
                    projects: 3,
                    agent_connections: 2,
                    connection_projects: 4,
                    guard_installations: 1,
                    project_state_databases: 3,
                }),
            ),
        };
        let human = render_privacy_footprint_text(&report);
        let json = serde_json::to_value(&report).unwrap();

        for heading in [
            "Runtime Home",
            "Record counts",
            "Stores",
            "Does not store",
            "Does not prove",
            "Output scope",
        ] {
            assert!(human.contains(heading), "{human}");
        }
        for item in report
            .privacy_footprint
            .stores
            .iter()
            .chain(report.privacy_footprint.does_not_store.iter())
            .chain(report.privacy_footprint.does_not_prove.iter())
        {
            assert!(human.contains(item), "{item}");
        }
        assert_eq!(json["privacy_footprint"]["record_counts"]["projects"], 3);
        assert_eq!(
            json["privacy_footprint"]["doctor_output_scope"],
            report.privacy_footprint.doctor_output_scope
        );
        assert!(human.contains(&report.runtime_home));
        assert!(human.ends_with('\n'));
        assert!(!human.ends_with("\n\n"));
        assert!(!human.contains('\t'));
    }

    #[test]
    fn clean_exact_build_finalizes_with_no_remediation() {
        let report = report_for_build(complete_build());
        let compact = render_doctor_output(OutputFormat::Compact, &report).unwrap();
        let json_text = render_doctor_output(OutputFormat::Json, &report).unwrap();
        let json: Value = serde_json::from_str(&json_text).unwrap();

        assert!(report.findings.is_empty());
        assert_eq!(report.warning_count(), 0);
        assert!(report.remediation_plan.is_empty());
        assert_eq!(json["primary_next_action"], Value::Null);
        assert_eq!(json["summary_card"]["next"], "none");
        assert!(compact.contains("Next action: none"));
        assert_cross_projection_parity(&report);
    }

    #[test]
    fn class_only_profile_passes_without_finding_or_action() {
        let mut build = complete_build();
        build.build_profile = None;
        build.profile_precision = volicord_mcp::BuildProfilePrecision::ClassOnly;
        build.build_id = build.deterministic_build_id();

        let (check, finding) = inspect_build_identity(&build);
        assert_eq!(check.status, "passed");
        assert_eq!(
            check.summary,
            "build provenance identifies a clean source commit; profile precision: class only"
        );
        assert_eq!(
            check.details,
            Some(json!({
                "state": "usable_clean",
                "profile_precision": "class_only",
            }))
        );
        assert!(finding.is_none());

        let report = report_for_build(build);
        let compact = render_doctor_output(OutputFormat::Compact, &report).unwrap();
        let json_text = render_doctor_output(OutputFormat::Json, &report).unwrap();
        let json: Value = serde_json::from_str(&json_text).unwrap();
        assert!(report.findings.is_empty());
        assert_eq!(report.warning_count(), 0);
        assert!(report.remediation_plan.is_empty());
        assert_eq!(json["primary_next_action"], Value::Null);
        assert_eq!(json["summary_card"]["next"], "none");
        assert!(compact.contains("Next action: none"));
        assert_cross_projection_parity(&report);
    }

    #[test]
    fn dirty_source_has_its_own_accurate_diagnostic() {
        let mut build = complete_build();
        build.git_dirty = Some(true);
        build.build_id = build.deterministic_build_id();

        let (check, finding) = inspect_build_identity(&build);
        assert_eq!(check.status, "warning");
        assert_eq!(
            check.summary,
            "build source is dirty; the recorded commit does not identify the working-tree changes"
        );
        assert_eq!(
            finding,
            Some(InstallationDiagnostic::BuildSourceNotReproducible)
        );

        let report = report_for_build(build);
        let compact = render_doctor_output(OutputFormat::Compact, &report).unwrap();
        let verbose = render_doctor_output(OutputFormat::Verbose, &report).unwrap();
        let json_text = render_doctor_output(OutputFormat::Json, &report).unwrap();
        let json: Value = serde_json::from_str(&json_text).unwrap();
        assert_eq!(report.findings.len(), 1);
        assert!(report.remediation_plan.is_empty());
        assert!(action_codes(&json, "actions").is_empty());
        assert!(action_codes(&json, "actions_required").is_empty());
        assert!(action_codes(&json, "actions_recommended").is_empty());
        assert_eq!(json["primary_next_action"], Value::Null);
        assert_eq!(json["summary_card"]["next"], "none");
        assert_eq!(
            json["status_meaning"],
            "installation profile is usable; warnings require review and have no automatic remediation"
        );
        assert!(compact.contains("Next action: none"));
        assert!(verbose.contains("Next action: none"));
        assert!(!compact.contains("install_build_with_complete_provenance"));
        assert_cross_projection_parity(&report);
    }

    #[test]
    fn unknown_source_metadata_reports_unavailable_identity() {
        let mut build = complete_build();
        build.git_commit = "unknown";
        build.metadata_source = "unknown";
        build.build_id = build.deterministic_build_id();

        let (check, finding) = inspect_build_identity(&build);
        assert_eq!(check.status, "warning");
        assert_eq!(
            finding,
            Some(InstallationDiagnostic::BuildIdentityUnavailable)
        );

        let report = report_for_build(build);
        let compact = render_doctor_output(OutputFormat::Compact, &report).unwrap();
        let verbose = render_doctor_output(OutputFormat::Verbose, &report).unwrap();
        let json_text = render_doctor_output(OutputFormat::Json, &report).unwrap();
        let json: Value = serde_json::from_str(&json_text).unwrap();
        let expected_code = "action.installation.install_build_with_complete_provenance";
        let expected_summary = "Install a Volicord build with complete provenance metadata";

        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].actions().len(), 1);
        assert_eq!(
            report.findings[0].actions()[0].code().as_str(),
            expected_code
        );
        assert_eq!(
            action_codes(&json, "actions"),
            BTreeSet::from([expected_code.to_owned()])
        );
        assert!(action_codes(&json, "actions_required").is_empty());
        assert_eq!(
            action_codes(&json, "actions_recommended"),
            BTreeSet::from([expected_code.to_owned()])
        );
        assert_eq!(json["primary_next_action"]["code"], expected_code);
        assert_eq!(
            json["summary_card"]["next"],
            format!("recommended: {expected_summary}")
        );
        assert!(compact.contains(expected_summary));
        assert!(compact.contains(expected_code));
        assert!(verbose.contains("\nPrimary action\n"));
        assert!(verbose.contains("\nRecommended actions\n"));
        assert!(verbose.contains(expected_code));
        assert_cross_projection_parity(&report);
    }

    #[test]
    fn host_detection_reports_codex_only() {
        assert_eq!(
            host_detection_check().details,
            Some(json!({ "accepted_host_values": ["codex"] }))
        );
    }

    #[test]
    fn blocking_checks_determine_doctor_status() {
        assert_eq!(
            doctor_status(&[DiagnosticCheck::failed("installation_profile", "missing")]),
            CommandStatus::ActionRequired
        );
        assert_eq!(
            doctor_status(&[DiagnosticCheck::failed(
                "project_policy_authority",
                "invalid"
            )]),
            CommandStatus::Failed
        );
        assert_eq!(
            doctor_status(&[DiagnosticCheck::warning("path_or_shim", "not on PATH")]),
            CommandStatus::Complete
        );
    }

    #[test]
    fn installation_profile_state_does_not_collapse_missing_and_invalid() {
        let missing = DiagnosticCheck::failed("installation_profile", "missing")
            .with_details(json!({ "state": "missing" }));
        let invalid = DiagnosticCheck::failed("installation_profile", "invalid")
            .with_details(json!({ "state": "invalid" }));
        let unavailable = DiagnosticCheck::failed("installation_profile", "unavailable")
            .with_details(json!({ "state": "unavailable" }));
        let corrupt = DiagnosticCheck::failed("installation_profile", "corrupt")
            .with_details(json!({ "state": "corrupt" }));
        let unknown = DiagnosticCheck::failed("installation_profile", "unspecified");

        assert_eq!(doctor_installation_profile_state(&[missing]), "missing");
        assert_eq!(doctor_installation_profile_state(&[invalid]), "invalid");
        assert_eq!(
            doctor_installation_profile_state(&[unavailable]),
            "unavailable"
        );
        assert_eq!(doctor_installation_profile_state(&[corrupt]), "corrupt");
        assert_eq!(doctor_installation_profile_state(&[unknown]), "unknown");
    }

    #[test]
    fn policy_authority_store_failures_keep_corrupt_and_unavailable_distinct() {
        assert_eq!(
            project_policy_store_failure_route(StoreFailureRoute::PersistedDataCorrupt),
            ProjectPolicyAuthorityState::AuthorityCorrupt
        );
        assert_eq!(
            project_policy_store_failure_route(StoreFailureRoute::OperationalUnavailable),
            ProjectPolicyAuthorityState::AuthorityUnavailable
        );
    }

    #[test]
    fn truncated_policy_authority_scan_never_reports_passed() {
        assert_eq!(project_policy_authority_result(0, false), "passed");
        assert_eq!(project_policy_authority_result(0, true), "warning");
        assert_eq!(project_policy_authority_result(1, false), "failed");
        assert_eq!(project_policy_authority_result(1, true), "failed");
    }

    #[test]
    fn default_control_surface_is_fail_closed() {
        assert_eq!(
            doctor_control_surface_value(&[]),
            json!({
                "selected_profile": "not_checked",
                "host_hooks_active": false,
                "cooperative_pre_tool_warning_available": false,
                "cooperative_pre_tool_denial_available": false,
                "unrecorded_changes_detectable": false,
                "actor_identity_provable": false,
                "os_enforced": false,
            })
        );
    }
}
