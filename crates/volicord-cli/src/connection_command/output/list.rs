use super::*;

use crate::disclosure::cooperative_host_decision_disclosure_json;

pub(in crate::connection_command) fn render_connections_output(
    format: OutputFormat,
    rows: &[(AgentConnectionRecord, Vec<ConnectionProjectRecord>)],
) -> Result<String, ConnectionCommandError> {
    match format {
        OutputFormat::Text => {
            let mut output = String::from(
                "host\tintent\tmode\tenabled\tconnected_repositories\tverification_status\tmetadata_state\ttarget\n",
            );
            for (connection, projects) in rows {
                let report = effective_connection_report(connection)?;
                output.push_str(&format!(
                    "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                    public_host_name_text(&connection.host_kind),
                    connection.intent,
                    public_mode_text(&connection.mode),
                    connection.enabled,
                    display_project_roots(projects),
                    report.status().as_str(),
                    if decode_persisted_object(&connection.metadata_json).is_some() {
                        "current"
                    } else {
                        PERSISTED_CONNECTION_METADATA_CORRUPT_REASON
                    },
                    connection.config_target
                ));
            }
            Ok(output)
        }
        OutputFormat::Json => {
            let degraded = rows.iter().any(|(connection, _)| {
                connection.verification_report().is_err()
                    || decode_persisted_object(&connection.metadata_json).is_none()
            });
            let values = rows
                .iter()
                .map(|(connection, projects)| connection_json(connection, projects))
                .collect::<Vec<_>>();
            serde_json::to_string_pretty(&json!({
                "status": if degraded { "degraded" } else { "complete" },
                "disclosure": cooperative_host_decision_disclosure_json(),
                "connections": values,
                "checks": [],
                "actions": [],
            }))
            .map(|text| format!("{text}\n"))
            .map_err(|error| ConnectionCommandError::runtime(error.to_string()))
        }
    }
}

fn connection_json(
    connection: &AgentConnectionRecord,
    projects: &[ConnectionProjectRecord],
) -> Value {
    let verification_report = effective_connection_report(connection)
        .and_then(|report| {
            serde_json::to_value(report)
                .map_err(|error| ConnectionCommandError::runtime(error.to_string()))
        })
        .unwrap_or(Value::Null);
    json!({
        "connection_id": connection.connection_internal_id,
        "host_kind": connection.host_kind,
        "connection_intent": connection.intent,
        "host_scope": connection.host_scope,
        "mode": connection.mode,
        "enabled": connection.enabled,
        "connected_projects": projects
            .iter()
            .map(|project| project.project_id.clone())
            .collect::<Vec<_>>(),
        "connected_repositories": projects
            .iter()
            .map(|project| path_text(&project.project.repo_root))
            .collect::<Vec<_>>(),
        "verification_report": verification_report,
        "metadata_state": persisted_object_state_json(
            &connection.metadata_json,
            PERSISTED_CONNECTION_METADATA_CORRUPT_REASON,
            "recreate_or_repair_the_agent_connection_registration",
        ),
        "server_name": connection.server_name,
        "config_target": connection.config_target,
    })
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
