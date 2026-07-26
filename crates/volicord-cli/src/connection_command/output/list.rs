use std::{cmp::Ordering, collections::BTreeSet};

use serde::Serialize;
use volicord_types::ids::{AgentConnectionId, ProjectId};

use super::*;

const METADATA_CORRUPT_SUMMARY: &str =
    "Persisted Agent Connection registration metadata is corrupt.";
const VERIFICATION_REPORT_CORRUPT_SUMMARY: &str =
    "Persisted Agent Connection verification report is corrupt.";

#[derive(Debug, Serialize)]
struct ConnectionListReport {
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
    connected_projects: Vec<ProjectId>,
    connected_repositories: Vec<String>,
    verification_report: Option<ConnectionVerificationReport>,
    issues: Vec<ConnectionListIssue>,
    server_name: String,
    config_target: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct ConnectionListIssue {
    kind: ConnectionListIssueKind,
    summary: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum ConnectionListIssueKind {
    MetadataCorrupt,
    VerificationReportCorrupt,
}

impl ConnectionListIssueKind {
    const fn as_str(self) -> &'static str {
        match self {
            Self::MetadataCorrupt => "metadata_corrupt",
            Self::VerificationReportCorrupt => "verification_report_corrupt",
        }
    }

    const fn summary(self) -> &'static str {
        match self {
            Self::MetadataCorrupt => METADATA_CORRUPT_SUMMARY,
            Self::VerificationReportCorrupt => VERIFICATION_REPORT_CORRUPT_SUMMARY,
        }
    }
}

impl Ord for ConnectionListIssueKind {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl PartialOrd for ConnectionListIssueKind {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl From<ConnectionListIssueKind> for ConnectionListIssue {
    fn from(kind: ConnectionListIssueKind) -> Self {
        Self {
            kind,
            summary: kind.summary().to_owned(),
        }
    }
}

pub(in crate::connection_command) fn render_connections_output(
    format: OutputFormat,
    rows: &[(AgentConnectionRecord, Vec<ConnectionProjectRecord>)],
) -> Result<String, ConnectionCommandError> {
    let report = ConnectionListReport {
        connections: rows
            .iter()
            .map(|(connection, projects)| connection_entry(connection, projects))
            .collect::<Result<Vec<_>, _>>()?,
        limits: cooperative_assurance_limits(),
    };
    match format {
        OutputFormat::Human(_) => Ok(render_connections_text(&report.connections)),
        OutputFormat::Json => serde_json::to_string_pretty(&report)
            .map(|text| format!("{text}\n"))
            .map_err(|error| ConnectionCommandError::runtime(error.to_string())),
    }
}

fn connection_entry(
    connection: &AgentConnectionRecord,
    projects: &[ConnectionProjectRecord],
) -> Result<ConnectionListEntry, ConnectionCommandError> {
    let mut issue_kinds = BTreeSet::new();
    if decode_persisted_object(&connection.metadata_json).is_none() {
        issue_kinds.insert(ConnectionListIssueKind::MetadataCorrupt);
    }
    let verification_report = match connection.verification_report() {
        Ok(Some(report)) => Some(report),
        Ok(None) => Some(effective_connection_report(connection)?),
        Err(_) => {
            issue_kinds.insert(ConnectionListIssueKind::VerificationReportCorrupt);
            None
        }
    };

    Ok(ConnectionListEntry {
        connection_id: AgentConnectionId::new(connection.connection_internal_id.clone()),
        host_kind: connection.host_kind.clone(),
        connection_intent: connection.intent.clone(),
        host_scope: connection.host_scope.clone(),
        mode: connection.mode.clone(),
        enabled: connection.enabled,
        connected_projects: projects
            .iter()
            .map(|project| ProjectId::new(project.project_id.clone()))
            .collect(),
        connected_repositories: projects
            .iter()
            .map(|project| path_text(&project.project.repo_root))
            .collect(),
        verification_report,
        issues: issue_kinds.into_iter().map(Into::into).collect(),
        server_name: connection.server_name.clone(),
        config_target: connection.config_target.clone(),
    })
}

fn render_connections_text(connections: &[ConnectionListEntry]) -> String {
    let mut output = String::from(
        "host\tintent\tmode\tenabled\tconnected_repositories\tverification_status\tissues\ttarget\n",
    );
    for connection in connections {
        output.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            public_host_name_text(&connection.host_kind),
            connection.connection_intent,
            public_mode_text(&connection.mode),
            connection.enabled,
            connection.connected_repositories.join(","),
            connection
                .verification_report
                .as_ref()
                .map_or("-", |report| report.status().as_str()),
            display_issues(&connection.issues),
            connection.config_target
        ));
    }
    output
}

fn display_issues(issues: &[ConnectionListIssue]) -> String {
    if issues.is_empty() {
        "-".to_owned()
    } else {
        issues
            .iter()
            .map(|issue| issue.kind.as_str())
            .collect::<Vec<_>>()
            .join(",")
    }
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
