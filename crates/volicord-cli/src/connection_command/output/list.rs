use serde::Serialize;
use volicord_types::{
    connection_verification::{
        ActivationStep, ActivationStepId, ConnectionCheckStatus, ConnectionStatus,
        HookActivationState, IntegrationActivationState,
    },
    ids::{AgentConnectionId, ProjectId},
    integration_revision::IntegrationRevision,
    values::UtcTimestamp,
};

use crate::presentation::{
    ActionHint, BulletList, Document, Element, Field, HumanValue, Section, YesNo,
};

use super::*;
use crate::connection_command::args::HumanOutputDetail;

const REGISTRATION_METADATA_CORRUPT_SUMMARY: &str =
    "Persisted Agent Connection registration metadata is corrupt.";

#[derive(Debug)]
pub(in crate::connection_command) struct EvaluatedConnectionListEntry {
    pub(in crate::connection_command) connection: AgentConnectionRecord,
    pub(in crate::connection_command) memberships: Vec<EvaluatedConnectionMembership>,
}

#[derive(Debug)]
pub(in crate::connection_command) struct EvaluatedConnectionMembership {
    pub(in crate::connection_command) project: ConnectionProjectRecord,
    pub(in crate::connection_command) evaluation:
        Result<VerificationReport, CurrentConnectionEvaluationUnavailable>,
}

#[derive(Debug, Serialize)]
struct ConnectionListReport {
    generated_at: UtcTimestamp,
    connections: Vec<ConnectionListEntry>,
    limits: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ConnectionListEntry {
    connection_id: AgentConnectionId,
    host_kind: String,
    connection_intent: String,
    host_scope: String,
    mode: String,
    enabled: bool,
    server_name: String,
    config_target: String,
    memberships: Vec<ConnectionMembershipEntry>,
    issues: Vec<ConnectionListIssue>,
}

#[derive(Debug, Serialize)]
struct ConnectionMembershipEntry {
    project_id: ProjectId,
    project_name: String,
    repository: String,
    current_state: ConnectionMembershipCurrentState,
}

#[derive(Debug, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
enum ConnectionMembershipCurrentState {
    Available {
        status: ConnectionStatus,
        activation: IntegrationActivationState,
        hook_activation: HookActivationState,
        check_counts: ConnectionCheckCounts,
        required_step_count: usize,
        primary_required_step: Option<ConnectionRequiredStep>,
        required_steps: Vec<ConnectionRequiredStep>,
        integration_revision: IntegrationRevision,
        evaluated_at: UtcTimestamp,
    },
    Unavailable {
        reason: CurrentStateUnavailableReason,
        summary: String,
        issues: Vec<CurrentStateIssue>,
    },
}

#[derive(Debug, Clone, Copy, Default, Serialize)]
struct ConnectionCheckCounts {
    passed: usize,
    blocked: usize,
    pending: usize,
    failed: usize,
    not_applicable: usize,
}

#[derive(Debug, Clone, Serialize)]
struct ConnectionRequiredStep {
    id: ActivationStepId,
    instruction: String,
}

#[derive(Debug, Serialize)]
struct CurrentStateIssue {
    summary: String,
    detail: String,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
enum CurrentStateUnavailableReason {
    RegistrationMetadataCorrupt,
    PersistedActiveVerificationEvidenceCorrupt,
    ManagedConfigurationUnavailable,
    ProjectMembershipUnavailable,
    ProjectStoreUnavailable,
    GuardStateUnavailable,
    RuntimeSessionStateUnavailable,
    DiagnosticStateUnavailable,
    CurrentEvaluationInconsistent,
}

impl CurrentStateUnavailableReason {
    const fn summary(self) -> &'static str {
        match self {
            Self::RegistrationMetadataCorrupt => REGISTRATION_METADATA_CORRUPT_SUMMARY,
            Self::PersistedActiveVerificationEvidenceCorrupt => {
                "Persisted active verification evidence cannot be used safely."
            }
            Self::ManagedConfigurationUnavailable => {
                "Current managed configuration cannot be read or validated."
            }
            Self::ProjectMembershipUnavailable => {
                "The current Connection membership cannot be read consistently."
            }
            Self::ProjectStoreUnavailable => {
                "The Product Repository Store is unavailable for current evaluation."
            }
            Self::GuardStateUnavailable => {
                "Current Guard state is unavailable for this membership."
            }
            Self::RuntimeSessionStateUnavailable => {
                "Current managed runtime-session state is unavailable."
            }
            Self::DiagnosticStateUnavailable => "Current diagnostic state is unavailable.",
            Self::CurrentEvaluationInconsistent => {
                "Current Connection evaluation could not produce a consistent result."
            }
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::RegistrationMetadataCorrupt => "registration_metadata_corrupt",
            Self::PersistedActiveVerificationEvidenceCorrupt => {
                "persisted_active_verification_evidence_corrupt"
            }
            Self::ManagedConfigurationUnavailable => "managed_configuration_unavailable",
            Self::ProjectMembershipUnavailable => "project_membership_unavailable",
            Self::ProjectStoreUnavailable => "project_store_unavailable",
            Self::GuardStateUnavailable => "guard_state_unavailable",
            Self::RuntimeSessionStateUnavailable => "runtime_session_state_unavailable",
            Self::DiagnosticStateUnavailable => "diagnostic_state_unavailable",
            Self::CurrentEvaluationInconsistent => "current_evaluation_inconsistent",
        }
    }
}

impl From<CurrentConnectionEvaluationUnavailableCause> for CurrentStateUnavailableReason {
    fn from(cause: CurrentConnectionEvaluationUnavailableCause) -> Self {
        match cause {
            CurrentConnectionEvaluationUnavailableCause::RegistrationMetadataCorrupt => {
                Self::RegistrationMetadataCorrupt
            }
            CurrentConnectionEvaluationUnavailableCause::PersistedActiveVerificationEvidenceCorrupt => {
                Self::PersistedActiveVerificationEvidenceCorrupt
            }
            CurrentConnectionEvaluationUnavailableCause::ManagedConfigurationUnreadableOrInvalid => {
                Self::ManagedConfigurationUnavailable
            }
            CurrentConnectionEvaluationUnavailableCause::ProjectMembershipUnavailable => {
                Self::ProjectMembershipUnavailable
            }
            CurrentConnectionEvaluationUnavailableCause::ProjectStoreUnavailable => {
                Self::ProjectStoreUnavailable
            }
            CurrentConnectionEvaluationUnavailableCause::GuardStateUnavailable => {
                Self::GuardStateUnavailable
            }
            CurrentConnectionEvaluationUnavailableCause::RuntimeSessionStateUnavailable => {
                Self::RuntimeSessionStateUnavailable
            }
            CurrentConnectionEvaluationUnavailableCause::DiagnosticStateUnavailable => {
                Self::DiagnosticStateUnavailable
            }
            CurrentConnectionEvaluationUnavailableCause::IntegrationRevisionUnavailableOrInconsistent
            | CurrentConnectionEvaluationUnavailableCause::EvaluationAssemblyUnavailable => {
                Self::CurrentEvaluationInconsistent
            }
        }
    }
}

#[derive(Debug, Serialize)]
struct ConnectionListIssue {
    kind: ConnectionListIssueKind,
    summary: String,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
enum ConnectionListIssueKind {
    RegistrationMetadataCorrupt,
}

pub(in crate::connection_command) fn render_connections_output(
    format: OutputFormat,
    generated_at: UtcTimestamp,
    rows: &[EvaluatedConnectionListEntry],
) -> Result<String, ConnectionCommandError> {
    let report = ConnectionListReport {
        generated_at,
        connections: rows.iter().map(connection_entry).collect(),
        limits: cooperative_assurance_limits(),
    };
    match format {
        OutputFormat::Human(detail) => Ok(render_connections_text(&report, detail)),
        OutputFormat::Json => serde_json::to_string_pretty(&report)
            .map(|text| format!("{text}\n"))
            .map_err(|error| ConnectionCommandError::runtime(error.to_string())),
    }
}

fn connection_entry(row: &EvaluatedConnectionListEntry) -> ConnectionListEntry {
    let registration_metadata_corrupt = decode_persisted_object(&row.connection.metadata_json)
        .is_none()
        || row.memberships.iter().any(|membership| {
            membership.evaluation.as_ref().is_err_and(|error| {
                error.cause()
                    == CurrentConnectionEvaluationUnavailableCause::RegistrationMetadataCorrupt
            })
        });
    ConnectionListEntry {
        connection_id: AgentConnectionId::new(row.connection.connection_internal_id.clone()),
        host_kind: row.connection.host_kind.clone(),
        connection_intent: row.connection.intent.clone(),
        host_scope: row.connection.host_scope.clone(),
        mode: row.connection.mode.clone(),
        enabled: row.connection.enabled,
        server_name: row.connection.server_name.clone(),
        config_target: row.connection.config_target.clone(),
        memberships: row.memberships.iter().map(membership_entry).collect(),
        issues: registration_metadata_corrupt
            .then(|| ConnectionListIssue {
                kind: ConnectionListIssueKind::RegistrationMetadataCorrupt,
                summary: REGISTRATION_METADATA_CORRUPT_SUMMARY.to_owned(),
            })
            .into_iter()
            .collect(),
    }
}

fn membership_entry(row: &EvaluatedConnectionMembership) -> ConnectionMembershipEntry {
    ConnectionMembershipEntry {
        project_id: ProjectId::new(row.project.project_id.clone()),
        project_name: row.project.project.project_name.clone(),
        repository: path_text(&row.project.project.repo_root),
        current_state: match &row.evaluation {
            Ok(evaluation) => {
                let report = &evaluation.report;
                let required_steps = report
                    .activation_plan()
                    .required_steps()
                    .iter()
                    .map(required_step)
                    .collect::<Vec<_>>();
                ConnectionMembershipCurrentState::Available {
                    status: report.status(),
                    activation: report.activation_state(),
                    hook_activation: report.hook_activation_state(),
                    check_counts: check_counts(report),
                    required_step_count: required_steps.len(),
                    primary_required_step: required_steps.first().cloned(),
                    required_steps,
                    integration_revision: evaluation.integration_revision.clone(),
                    evaluated_at: report.checked_at().clone(),
                }
            }
            Err(error) => {
                let reason = CurrentStateUnavailableReason::from(error.cause());
                ConnectionMembershipCurrentState::Unavailable {
                    reason,
                    summary: reason.summary().to_owned(),
                    issues: vec![CurrentStateIssue {
                        summary: reason.summary().to_owned(),
                        detail: error.bounded_detail(),
                    }],
                }
            }
        },
    }
}

fn required_step(step: &ActivationStep) -> ConnectionRequiredStep {
    ConnectionRequiredStep {
        id: step.id(),
        instruction: step.instruction().to_owned(),
    }
}

fn check_counts(report: &ConnectionVerificationReport) -> ConnectionCheckCounts {
    report
        .checks()
        .iter()
        .fold(ConnectionCheckCounts::default(), |mut counts, check| {
            match check.status() {
                ConnectionCheckStatus::Passed => counts.passed += 1,
                ConnectionCheckStatus::Blocked => counts.blocked += 1,
                ConnectionCheckStatus::Pending => counts.pending += 1,
                ConnectionCheckStatus::Failed => counts.failed += 1,
                ConnectionCheckStatus::NotApplicable => counts.not_applicable += 1,
            }
            counts
        })
}

fn render_connections_text(report: &ConnectionListReport, detail: HumanOutputDetail) -> String {
    let body = report
        .connections
        .iter()
        .map(|connection| connection_section(connection, detail))
        .map(Element::from)
        .collect();
    let headline = format!("Connections ({})", report.connections.len());
    match detail {
        HumanOutputDetail::Concise => Document::new(headline, body).render(),
        HumanOutputDetail::Verbose => Document::verbose(headline, body).render(),
    }
}

fn connection_section(connection: &ConnectionListEntry, detail: HumanOutputDetail) -> Section {
    let mut body = vec![
        Field::new(
            "Intent",
            HumanValue::text(human_label(&connection.connection_intent)),
        )
        .into(),
        Field::new(
            "Scope",
            HumanValue::text(human_label(&connection.host_scope)),
        )
        .into(),
        Field::new("Mode", HumanValue::text(public_mode_text(&connection.mode))).into(),
        Field::new(
            "Enabled",
            HumanValue::YesNo(YesNo::from(connection.enabled)),
        )
        .into(),
        Field::verbose(
            "Connection ID",
            HumanValue::text(connection.connection_id.as_str()),
        )
        .into(),
        Field::verbose("Server name", HumanValue::text(&connection.server_name)).into(),
        Field::verbose(
            "Configuration target",
            HumanValue::text(&connection.config_target),
        )
        .into(),
    ];
    for issue in &connection.issues {
        body.push(Field::new("Registration issue", HumanValue::text(&issue.summary)).into());
    }
    body.extend(
        connection
            .memberships
            .iter()
            .map(|membership| membership_section(membership, detail))
            .map(Element::from),
    );
    Section::new(public_host_name_text(&connection.host_kind), body)
}

fn membership_section(
    membership: &ConnectionMembershipEntry,
    detail: HumanOutputDetail,
) -> Section {
    let mut body = vec![
        Field::new("Repository", HumanValue::text(&membership.repository)).into(),
        Field::verbose(
            "Project ID",
            HumanValue::text(membership.project_id.as_str()),
        )
        .into(),
    ];
    match &membership.current_state {
        ConnectionMembershipCurrentState::Available {
            status,
            activation,
            hook_activation,
            check_counts,
            primary_required_step,
            required_steps,
            integration_revision,
            evaluated_at,
            ..
        } => {
            body.extend([
                Field::new("Status", HumanValue::text(human_label(status.as_str()))).into(),
                Field::new(
                    "Activation",
                    HumanValue::text(human_label(activation.as_str())),
                )
                .into(),
                Field::new(
                    "Hook activation",
                    HumanValue::text(human_label(hook_activation.as_str())),
                )
                .into(),
                Field::new(
                    "Checks",
                    HumanValue::text(format!(
                        "{} passed, {} blocked, {} pending, {} failed",
                        check_counts.passed,
                        check_counts.blocked,
                        check_counts.pending,
                        check_counts.failed
                    )),
                )
                .into(),
                Field::verbose(
                    "Not applicable checks",
                    HumanValue::Count(check_counts.not_applicable),
                )
                .into(),
                Field::verbose(
                    "Integration revision",
                    HumanValue::text(integration_revision.as_str()),
                )
                .into(),
                Field::verbose("Evaluated at", HumanValue::text(evaluated_at.to_string())).into(),
            ]);
            if let Some(primary) = primary_required_step {
                body.push(ActionHint::new(&primary.instruction).into());
            }
            if detail == HumanOutputDetail::Verbose && !required_steps.is_empty() {
                body.push(
                    Section::new(
                        "Required actions",
                        vec![BulletList::new(
                            required_steps
                                .iter()
                                .map(|step| format!("{}: {}", step.id.as_str(), step.instruction)),
                        )
                        .into()],
                    )
                    .into(),
                );
            }
        }
        ConnectionMembershipCurrentState::Unavailable {
            reason,
            summary,
            issues,
        } => {
            body.extend([
                Field::new("Current state", HumanValue::text("unavailable")).into(),
                Field::new("Reason", HumanValue::text(human_label(reason.as_str()))).into(),
                Field::new("Summary", HumanValue::text(summary)).into(),
            ]);
            for issue in issues {
                body.push(
                    Field::verbose("Current-state issue", HumanValue::text(&issue.detail)).into(),
                );
            }
        }
    }
    Section::new(&membership.project_name, body)
}

fn human_label(value: &str) -> String {
    value.replace('_', " ")
}

pub(in crate::connection_command) fn display_project_roots(
    projects: &[ConnectionProjectRecord],
) -> String {
    projects
        .iter()
        .map(|project| path_text(&project.project.repo_root))
        .collect::<Vec<_>>()
        .join(",")
}
