use super::*;

pub(super) fn render_compact_connection_text(
    data: &ConnectionOutput<'_>,
    mcp_config_state: &str,
    primary_next_action: Option<&PrimaryNextAction>,
) -> Result<String, ConnectionCommandError> {
    let host = public_host_display_name(data.host_kind);
    if data.action == "removed" {
        return Ok(render_compact_remove_text(data, host));
    }
    let title = compact_connection_title(data.action, host);
    let mut output = format!("{title}\n\nStatus:\n");
    match data.action {
        "verified" => {
            output.push_str(&format!(
                "  Verification: {}\n  Connection: {}\n  Mode: {}\n",
                compact_agent_status_text(data.status),
                enabled_text(data.connection.enabled),
                public_mode_text(&data.connection.mode)
            ));
        }
        "status" | "mode_updated" => {
            output.push_str(&format!(
                "  Connection: {}\n  Mode: {}\n  Last verification: {}\n",
                enabled_text(data.connection.enabled),
                public_mode_text(&data.connection.mode),
                compact_agent_status_text(data.status)
            ));
        }
        "connected" => {
            output.push_str(&format!(
                "  Connection: {}\n  Verification: {}\n  Mode: {}\n",
                enabled_text(data.connection.enabled),
                compact_agent_status_text(data.status),
                public_mode_text(&data.connection.mode)
            ));
        }
        _ => {
            output.push_str(&format!(
                "  Connection: {}\n  Mode: {}\n  Last verification: {}\n",
                enabled_text(data.connection.enabled),
                public_mode_text(&data.connection.mode),
                compact_agent_status_text(data.status)
            ));
        }
    }

    output.push_str(&format!(
        "\nProfile:\n  {}\n\n",
        data.guard_state.selected_profile()
    ));
    if let Some(repo_root) = data.affected_repo_root {
        append_compact_repository(&mut output, repo_root);
    } else {
        append_compact_repositories(&mut output, data.projects);
    }
    if data.action == "connected" {
        let repo_root = data.affected_repo_root.or_else(|| {
            data.projects
                .first()
                .map(|project| project.project.repo_root.as_path())
        });
        append_compact_host_configuration(&mut output, data.plan, repo_root, data.status);
    }
    output.push_str("\nChecks:\n");
    for (label, value) in compact_connection_checks(data, mcp_config_state, primary_next_action) {
        output.push_str(&format!("  {label}: {value}\n"));
    }
    output.push_str("\nNext:\n");
    append_compact_next_steps(&mut output, data, host, primary_next_action);
    output.push_str(&format!(
        "\nLimits:\n{}\n\nDiagnostics:\n  Run:\n    {}\n",
        connection_limits_text(data.guard_state.selected_profile()),
        connection_diagnostics_command(data.connection, data.projects)
    ));
    Ok(output)
}

fn compact_connection_title(action: &str, host: &str) -> String {
    match action {
        "connected" => format!("Agent Connection configured for {host}"),
        "verified" => format!("Agent Connection checked for {host}"),
        "status" => format!("Agent Connection status for {host}"),
        "mode_updated" => format!("Agent Connection mode updated for {host}"),
        other => format!("Agent Connection {other} for {host}"),
    }
}

fn append_compact_repository(output: &mut String, repo_root: &Path) {
    output.push_str(&format!("Repository:\n  {}\n", repo_root.display()));
}

fn append_compact_host_configuration(
    output: &mut String,
    plan: Option<&HostPlan>,
    repo_root: Option<&Path>,
    status: AgentResultStatus,
) {
    let Some(plan) = plan else {
        return;
    };
    output.push('\n');
    if let Some(repo_root) = repo_root {
        if let Some(path) = repo_relative_host_target_path(plan, repo_root) {
            output.push_str("Repo file changes:\n");
            if let Some(status) = repo_file_change_from_host_plan(plan.change, status) {
                output.push_str(&format!("  {} {}\n", status.text_verb(), path));
            } else {
                output.push_str("  none\n");
            }
            return;
        }
    }
    output.push_str(&format!(
        "Host configuration:\n  Target: {}\n  Change: {}\n",
        host_target_text(&plan.target),
        planned_change_text(plan.change)
    ));
}

fn render_compact_remove_text(data: &ConnectionOutput<'_>, host: &str) -> String {
    let remaining = data.projects.len();
    let mut output = format!(
        "Agent Connection removed for {host}\n\nStatus:\n  Connection: removed from selected repository\n  Mode: {}\n  Remaining repositories: {}\n\n",
        public_mode_text(&data.connection.mode),
        remaining
    );
    if let Some(repo_root) = data.affected_repo_root {
        append_compact_repository(&mut output, repo_root);
    }
    if !data.projects.is_empty() {
        output.push_str("\nRemaining repositories:\n");
        for project in data.projects {
            output.push_str(&format!("  {}\n", project.project.repo_root.display()));
        }
    }
    output.push_str("\nRemoved:\n  Selected repository membership\n");
    if data.plan.is_some() && remaining == 0 {
        output.push_str(
            "  Matching managed host configuration\n  Running host processes may keep cached configuration until they reload.\n",
        );
    } else {
        output.push_str("  Host configuration kept for remaining connected repositories\n");
    }
    output.push_str("\nNext:\n");
    if data.plan.is_some() && remaining == 0 {
        output.push_str(&format!(
            "  1. Restart or reload {host} if a running host still shows cached Volicord tools.\n"
        ));
    } else {
        output.push_str("  none\n");
    }
    output.push_str(&format!(
        "\nDiagnostics:\n  Run:\n    {}\n",
        connection_diagnostics_command(data.connection, data.projects)
    ));
    output
}

fn append_compact_repositories(output: &mut String, projects: &[ConnectionProjectRecord]) {
    if projects.len() == 1 {
        output.push_str(&format!(
            "Repository:\n  {}\n",
            projects[0].project.repo_root.display()
        ));
        return;
    }
    output.push_str("Repositories:\n");
    if projects.is_empty() {
        output.push_str("  none\n");
    } else {
        for project in projects {
            output.push_str(&format!("  {}\n", project.project.repo_root.display()));
        }
    }
}

fn compact_connection_checks(
    data: &ConnectionOutput<'_>,
    _mcp_config_state: &str,
    primary_next_action: Option<&PrimaryNextAction>,
) -> Vec<(&'static str, String)> {
    let report = data
        .verification
        .map(|verification| verification.report.clone())
        .or_else(|| data.current_report.clone())
        .or_else(|| effective_connection_report(data.connection).ok());
    let mut checks = report
        .map(|report| {
            report
                .checks()
                .iter()
                .map(|check| {
                    let value = check
                        .code()
                        .map(|code| format!("{} ({code})", check.status().as_str()))
                        .unwrap_or_else(|| check.status().as_str().to_owned());
                    (canonical_check_label(check.id().as_str()), value)
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    checks.push((
        "Host follow-up",
        host_follow_up_text(data.status, primary_next_action).to_owned(),
    ));
    checks
}

fn canonical_check_label(id: &str) -> &'static str {
    match id {
        "managed_config" => "Managed MCP configuration",
        "host_executable" => "Codex executable",
        "mcp_server" => "Volicord MCP server",
        "host_session" => "Managed host session",
        "required_tools" => "Required tools",
        "tool_round_trip" => "Tool round trip",
        "project_trust" => "Project trust",
        "guard_files" => "Guard files",
        "guard_observation" => "Guard observation",
        "verification_not_run" => "Verification",
        _ => "Connection check",
    }
}

fn append_compact_next_steps(
    output: &mut String,
    data: &ConnectionOutput<'_>,
    host: &str,
    primary_next_action: Option<&PrimaryNextAction>,
) {
    let Some(action) = primary_next_action else {
        output.push_str("  none\n");
        return;
    };
    let command = action
        .command
        .clone()
        .or_else(|| connection_verify_command(Some(data.connection), data.projects));
    let mut index = 1;
    match action.id.as_str() {
        "reload_required" => {
            push_numbered_text(
                output,
                &mut index,
                format!("Open, restart, or reload {host} in this repository."),
            );
            if init_actions_include_trust_or_approval(&data.user_actions) {
                push_numbered_text(
                    output,
                    &mut index,
                    format!("Trust or approve the project configuration if {host} asks."),
                );
            }
            push_optional_numbered_command(output, &mut index, "Run", command.as_deref());
        }
        "host_trust_required" | "project_approval_required" => {
            push_numbered_text(
                output,
                &mut index,
                format!("Trust or approve the project configuration if {host} asks."),
            );
            push_optional_numbered_command(output, &mut index, "Run", command.as_deref());
        }
        "managed_host_startup_not_observed" => {
            push_numbered_text(
                output,
                &mut index,
                "Restart, reload, resume, or start a new Codex session in this repository.",
            );
            push_numbered_text(
                output,
                &mut index,
                "Confirm that Volicord tools are exposed in the active Codex session.",
            );
            push_numbered_text(
                output,
                &mut index,
                "If tools are not exposed, check Codex MCP startup/tool-list logs.",
            );
            push_optional_numbered_command(output, &mut index, "Run", command.as_deref());
        }
        "managed_host_tools_list_not_observed" => {
            push_numbered_text(
                output,
                &mut index,
                "Check Codex MCP startup/tool-list logs.",
            );
            push_numbered_text(
                output,
                &mut index,
                "Restart, reload, resume, or start a new Codex session in this repository if the managed tools/list snapshot is still absent.",
            );
            push_optional_numbered_command(output, &mut index, "Run", command.as_deref());
        }
        "active_tool_exposure_unconfirmed" => {
            push_numbered_text(
                output,
                &mut index,
                "Confirm that Volicord tools are exposed in the active Codex session.",
            );
            push_numbered_text(
                output,
                &mut index,
                "Invoke a read-only Volicord tool from the active Codex session.",
            );
            push_optional_numbered_command(output, &mut index, "Run", command.as_deref());
        }
        "managed_host_storage_degraded" => {
            push_numbered_text(
                output,
                &mut index,
                "Repair managed Codex host storage read/write capability or switch to a compatible read-only mode.",
            );
            push_optional_numbered_command(output, &mut index, "Run", command.as_deref());
        }
        "mcp_config_missing" => {
            push_numbered_text(
                output,
                &mut index,
                "Reinstall the missing MCP configuration.",
            );
            push_optional_numbered_command(output, &mut index, "Run", command.as_deref());
        }
        "mcp_config_changed" => {
            push_numbered_text(output, &mut index, "Review the changed MCP configuration.");
            push_optional_numbered_command(
                output,
                &mut index,
                "If Volicord should manage it, run",
                command.as_deref(),
            );
        }
        "mcp_config_malformed" => {
            push_numbered_text(
                output,
                &mut index,
                "Repair the malformed MCP configuration.",
            );
            push_optional_numbered_command(output, &mut index, "Run", command.as_deref());
        }
        "guard_files_missing" => {
            push_numbered_text(
                output,
                &mut index,
                "Reinstall missing Codex Record Guard files.",
            );
            push_optional_numbered_command(output, &mut index, "Run", command.as_deref());
        }
        "guard_files_stale" => {
            push_numbered_text(
                output,
                &mut index,
                "Refresh stale Codex Record Guard files.",
            );
            push_optional_numbered_command(output, &mut index, "Run", command.as_deref());
        }
        "guard_files_broken" => {
            push_numbered_text(
                output,
                &mut index,
                "Repair broken Codex Record Guard files.",
            );
            push_optional_numbered_command(output, &mut index, "Run", command.as_deref());
        }
        "guard_capability_degraded" => {
            push_numbered_text(
                output,
                &mut index,
                "Repair the required Codex Record Guard hook configuration.",
            );
            push_optional_numbered_command(output, &mut index, "Run", command.as_deref());
        }
        _ => {
            push_numbered_text(output, &mut index, action.instruction.trim_end_matches('.'));
            push_optional_numbered_command(output, &mut index, "Run", command.as_deref());
        }
    }
}

fn push_numbered_text(output: &mut String, index: &mut usize, text: impl AsRef<str>) {
    output.push_str(&format!("  {}. {}\n", *index, text.as_ref()));
    *index += 1;
}

fn push_optional_numbered_command(
    output: &mut String,
    index: &mut usize,
    label: &str,
    command: Option<&str>,
) {
    if let Some(command) = command {
        output.push_str(&format!("  {}. {label}:\n     {command}\n", *index));
        *index += 1;
    }
}

fn compact_agent_status_text(status: AgentResultStatus) -> &'static str {
    match status {
        AgentResultStatus::Complete => "complete",
        AgentResultStatus::ActionRequired => "action required",
        AgentResultStatus::Failed => "failed",
        AgentResultStatus::DryRun => "dry run",
    }
}

fn host_follow_up_text(
    status: AgentResultStatus,
    primary_next_action: Option<&PrimaryNextAction>,
) -> &'static str {
    if primary_next_action.is_some() {
        return "action required";
    }
    match status {
        AgentResultStatus::Complete => "ready",
        AgentResultStatus::ActionRequired => "action required",
        AgentResultStatus::Failed => "failed",
        AgentResultStatus::DryRun => "skipped",
    }
}

fn enabled_text(enabled: bool) -> &'static str {
    if enabled {
        "enabled"
    } else {
        "disabled"
    }
}

fn connection_limits_text(profile: &str) -> &'static str {
    let _ = profile;
    init_limits_text(InitMode::Record)
}

fn connection_status_diagnostics_command(
    connection: &AgentConnectionRecord,
    projects: &[ConnectionProjectRecord],
) -> Option<String> {
    let project = projects.first()?;
    let intent = parse_connection_intent(&connection.intent).ok()?;
    Some(format!(
        "volicord connection status {}{} --repo {} --json",
        public_host_name_text(&connection.host_kind),
        intent_flag_suffix(intent),
        project.project.repo_root.display()
    ))
}

fn connection_diagnostics_command(
    connection: &AgentConnectionRecord,
    projects: &[ConnectionProjectRecord],
) -> String {
    connection_status_diagnostics_command(connection, projects)
        .unwrap_or_else(|| "volicord connection list --json".to_owned())
}

pub(super) fn render_compact_plan_text(data: &ConnectionPlanOutput<'_>) -> String {
    let host = public_host_display_name(data.host_kind);
    let mut output = format!(
        "Agent Connection plan for {host}\n\nStatus:\n  Plan: dry run\n  Mode: {}\n  Intent: {}\n",
        public_mode_text(data.mode),
        data.intent.as_str()
    );
    if let Some(repo_root) = data.repo_root {
        output.push('\n');
        append_compact_repository(&mut output, repo_root);
    }
    output.push_str("\nPlanned changes:\n");
    append_compact_plan_changes(&mut output, data);
    output.push_str("\nNext:\n");
    append_compact_plan_next_steps(&mut output, data, host);
    output.push_str(&format!(
        "\nDiagnostics:\n  Run:\n    {}\n",
        connection_plan_diagnostics_command(data)
    ));
    output
}

fn append_compact_plan_changes(output: &mut String, data: &ConnectionPlanOutput<'_>) {
    if data.action == "remove" {
        output.push_str("  remove selected repository membership\n");
    }
    if let Some(repo_root) = data.repo_root {
        if let Some(path) = repo_relative_host_target_path(data.plan, repo_root) {
            match data.plan.change {
                PlannedChange::Create | PlannedChange::Update => {
                    if let Some(status) =
                        repo_file_change_from_host_plan(data.plan.change, data.status)
                    {
                        output.push_str(&format!("  {} {}\n", status.text_verb(), path));
                    }
                }
                PlannedChange::Remove => {
                    output.push_str(&format!("  would remove {path}\n"));
                }
                PlannedChange::Noop => {
                    output.push_str("  no host configuration file change\n");
                }
                PlannedChange::ExternalCommand => {
                    output.push_str(&format!(
                        "  would run external host configuration command for {}\n",
                        host_target_text(&data.plan.target)
                    ));
                }
            }
        } else {
            output.push_str(&format!(
                "  host configuration {}: {}\n",
                planned_change_text(data.plan.change),
                host_target_text(&data.plan.target)
            ));
        }
    } else {
        output.push_str(&format!(
            "  host configuration {}: {}\n",
            planned_change_text(data.plan.change),
            host_target_text(&data.plan.target)
        ));
    }
    if let Some(remaining) = data.projects_remaining {
        if remaining == 0 {
            output.push_str(
                "  remove matching managed host configuration\n  running host processes may keep cached configuration until they reload\n",
            );
        } else {
            output.push_str(&format!(
                "  keep host configuration for {} {}\n",
                remaining,
                connected_repository_phrase(remaining)
            ));
        }
    }
}

fn append_compact_plan_next_steps(
    output: &mut String,
    data: &ConnectionPlanOutput<'_>,
    host: &str,
) {
    let mut index = 1;
    if let Some(command) = connection_plan_apply_command(data) {
        push_optional_numbered_command(output, &mut index, "Run", Some(&command));
    }
    if data.action == "connection_add" {
        push_numbered_text(
            output,
            &mut index,
            format!("After applying, open, restart, or reload {host} in this repository."),
        );
        if init_actions_include_trust_or_approval(&data.user_actions) {
            push_numbered_text(
                output,
                &mut index,
                format!("Trust or approve the project configuration if {host} asks."),
            );
        }
        if let Some(repo_root) = data.repo_root {
            let command = connection_plan_verify_command(data.host_kind, data.intent, repo_root);
            push_optional_numbered_command(
                output,
                &mut index,
                "After applying, run",
                Some(&command),
            );
        }
    } else if data.action == "remove" && data.projects_remaining == Some(0) {
        push_numbered_text(
            output,
            &mut index,
            format!(
                "After applying, restart or reload {host} if it still shows cached Volicord tools."
            ),
        );
    }
    if index == 1 {
        output.push_str("  none\n");
    }
}

fn connection_plan_apply_command(data: &ConnectionPlanOutput<'_>) -> Option<String> {
    let repo_root = data.repo_root?;
    match data.action {
        "connection_add" => Some(connection_add_command(
            data.host_kind,
            data.intent,
            data.mode,
            repo_root,
            false,
            false,
        )),
        "remove" => Some(connection_remove_command(
            data.host_kind,
            data.intent,
            repo_root,
            false,
            false,
        )),
        _ => None,
    }
}

fn connection_plan_diagnostics_command(data: &ConnectionPlanOutput<'_>) -> String {
    let Some(repo_root) = data.repo_root else {
        return "volicord connection list --json".to_owned();
    };
    match data.action {
        "connection_add" => connection_add_command(
            data.host_kind,
            data.intent,
            data.mode,
            repo_root,
            true,
            true,
        ),
        "remove" => connection_remove_command(data.host_kind, data.intent, repo_root, true, true),
        _ => "volicord connection list --json".to_owned(),
    }
}

fn connection_plan_verify_command(
    host_kind: HostKind,
    intent: ConnectionIntent,
    repo_root: &Path,
) -> String {
    format!(
        "volicord connection verify {}{} --repo {}",
        public_host_label(host_kind),
        intent_flag_suffix(intent),
        repo_root.display()
    )
}

fn connection_add_command(
    host_kind: HostKind,
    intent: ConnectionIntent,
    mode: &str,
    repo_root: &Path,
    dry_run: bool,
    json: bool,
) -> String {
    let read_only_flag = if mode == CONNECTION_MODE_READ_ONLY {
        " --read-only"
    } else {
        ""
    };
    format!(
        "volicord connection add {}{}{} --repo {}{}{}",
        public_host_label(host_kind),
        intent_flag_suffix(intent),
        read_only_flag,
        repo_root.display(),
        if dry_run { " --dry-run" } else { "" },
        if json { " --json" } else { "" }
    )
}

fn connection_remove_command(
    host_kind: HostKind,
    intent: ConnectionIntent,
    repo_root: &Path,
    dry_run: bool,
    json: bool,
) -> String {
    format!(
        "volicord connection remove {}{} --repo {}{}{}",
        public_host_label(host_kind),
        intent_flag_suffix(intent),
        repo_root.display(),
        if dry_run { " --dry-run" } else { "" },
        if json { " --json" } else { "" }
    )
}

fn connected_repository_phrase(count: usize) -> &'static str {
    if count == 1 {
        "remaining connected repository"
    } else {
        "remaining connected repositories"
    }
}

pub(super) fn render_init_text_output(
    data: &InitOutput<'_>,
    actions: &[UserAction],
    repo_file_changes: &[RepoFileChange],
) -> String {
    let file_section_label = if data.status == AgentResultStatus::DryRun {
        "Planned repo file changes"
    } else {
        "Repo file changes"
    };
    let mut output = format!(
        "{}\n\nProfile:\n  {}\n\nConnection:\n  intent: {}\n  host scope: {}\n\nRepository:\n  {}\n\n{}:\n",
        init_text_title(data.status, data.host_kind),
        data.init_mode.profile_value(),
        data.intent.as_str(),
        data.host_scope.as_str(),
        data.repo_root.display(),
        file_section_label,
    );
    if repo_file_changes.is_empty() {
        output.push_str("  none\n");
    } else {
        for change in repo_file_changes {
            output.push_str(&format!(
                "  {} {}\n",
                change.status.text_verb(),
                change.path
            ));
        }
    }
    output.push_str(&format!(
        "\nStored local Volicord state:\n  {}\n\nNext:\n",
        data.runtime_home.display()
    ));
    for (index, step) in init_next_steps(data, actions).iter().enumerate() {
        match step {
            InitNextStep::Text(text) => {
                output.push_str(&format!("  {}. {}\n", index + 1, text));
            }
            InitNextStep::Command { label, command } => {
                output.push_str(&format!("  {}. {}:\n     {}\n", index + 1, label, command));
            }
        }
    }
    output.push_str(&format!(
        "\nLimits:\n{}\n\nDiagnostics:\n  Run:\n    {}\n",
        init_limits_text(data.init_mode),
        init_diagnostics_command(data),
    ));
    output
}

fn init_text_title(status: AgentResultStatus, host_kind: HostKind) -> String {
    let host = public_host_display_name(host_kind);
    match status {
        AgentResultStatus::Complete | AgentResultStatus::ActionRequired => {
            format!("Volicord initialized for {host}")
        }
        AgentResultStatus::DryRun => format!("Volicord init plan for {host}"),
        AgentResultStatus::Failed => format!("Volicord init failed for {host}"),
    }
}

enum InitNextStep {
    Text(String),
    Command {
        label: &'static str,
        command: String,
    },
}

fn init_next_steps(data: &InitOutput<'_>, actions: &[UserAction]) -> Vec<InitNextStep> {
    let host = public_host_display_name(data.host_kind);
    let verify_command = init_verify_command(data.host_kind, data.intent, data.repo_root);
    if data.status == AgentResultStatus::DryRun {
        let mut steps = vec![
            InitNextStep::Text(
                "Run the same init command without --dry-run to apply the planned repo file changes."
                    .to_owned(),
            ),
            InitNextStep::Text(format!(
                "After applying, open, restart, or reload {host} in this repository."
            )),
        ];
        if init_actions_include_trust_or_approval(actions) {
            steps.push(InitNextStep::Text(format!(
                "Trust or approve the project configuration if {host} asks."
            )));
        }
        steps.push(InitNextStep::Command {
            label: "After applying, run",
            command: verify_command,
        });
        return steps;
    }
    if data.status == AgentResultStatus::Failed {
        return vec![
            InitNextStep::Command {
                label: "Review detailed diagnostics",
                command: init_diagnostics_command(data),
            },
            InitNextStep::Text(format!(
                "Fix the reported issue, then rerun init for {host}."
            )),
        ];
    }
    let mut steps = vec![InitNextStep::Text(format!(
        "Open, restart, or reload {host} in this repository."
    ))];
    if init_actions_include_trust_or_approval(actions) {
        steps.push(InitNextStep::Text(format!(
            "Trust or approve the project configuration if {host} asks."
        )));
    }
    steps.push(InitNextStep::Command {
        label: "Run",
        command: verify_command,
    });
    steps
}

fn init_actions_include_trust_or_approval(actions: &[UserAction]) -> bool {
    actions
        .iter()
        .any(|action| matches!(action.kind, UserActionKind::HostTrustRequired))
}

fn init_verify_command(host_kind: HostKind, intent: ConnectionIntent, repo_root: &Path) -> String {
    format!(
        "volicord connection verify {}{} --repo {}",
        public_host_label(host_kind),
        intent_flag_suffix(intent),
        repo_root.display()
    )
}

fn init_status_command(host_kind: HostKind, intent: ConnectionIntent, repo_root: &Path) -> String {
    format!(
        "volicord connection status {}{} --repo {} --json",
        public_host_label(host_kind),
        intent_flag_suffix(intent),
        repo_root.display()
    )
}

fn init_diagnostics_command(data: &InitOutput<'_>) -> String {
    if data.status == AgentResultStatus::DryRun {
        return format!(
            "volicord init --host {}{} --repo {} --profile {} --dry-run --json",
            public_host_label(data.host_kind),
            intent_flag_suffix(data.intent),
            data.repo_root.display(),
            data.init_mode.profile_value()
        );
    }
    init_status_command(data.host_kind, data.intent, data.repo_root)
}

fn init_limits_text(init_mode: InitMode) -> &'static str {
    match init_mode {
        InitMode::Record => {
            "  The record profile supports cooperative Volicord workflow recording through MCP.\n  It does not provide OS sandboxing, network isolation, malware defense, full write prevention, actor identity proof, correctness proof, test sufficiency proof, or human review completion."
        }
    }
}

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
                .map(|(connection, projects)| {
                    let project_ids = projects
                        .iter()
                        .map(|project| project.project_id.clone())
                        .collect::<Vec<_>>();
                    let mut value = connection_json(connection, &project_ids, None);
                    if let Some(object) = value.as_object_mut() {
                        object.insert(
                            "connected_repositories".to_owned(),
                            Value::Array(
                                projects
                                    .iter()
                                    .map(|project| {
                                        Value::String(path_text(&project.project.repo_root))
                                    })
                                    .collect(),
                            ),
                        );
                    }
                    value
                })
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

pub(in crate::connection_command) fn render_connection_remove_dry_run_output(
    format: OutputFormat,
    runtime_home: &Path,
    connection: &AgentConnectionRecord,
    projects: &[ConnectionProjectRecord],
    selected_project: &ConnectionProjectRecord,
    plan: ConnectionRemovePlan<'_>,
    remaining_count: usize,
) -> Result<String, ConnectionCommandError> {
    match plan {
        ConnectionRemovePlan::Host(host_plan) => {
            render_connection_plan_output(ConnectionPlanOutput {
                format,
                action: "remove",
                status: AgentResultStatus::DryRun,
                runtime_home,
                connection_id: &connection.connection_internal_id,
                host_kind: parse_host_kind(&connection.host_kind)?,
                intent: parse_connection_intent(&connection.intent)?,
                host_scope: parse_host_scope(&connection.host_scope)?,
                mode: &connection.mode,
                enabled: connection.enabled,
                repo_root: Some(&selected_project.project.repo_root),
                plan: host_plan,
                projects_remaining: Some(remaining_count),
                user_actions: Vec::new(),
            })
        }
        ConnectionRemovePlan::MembershipOnly => match format {
            OutputFormat::Text => render_compact_membership_remove_plan_text(
                connection,
                selected_project,
                remaining_count,
            ),
            OutputFormat::Json => {
                let project_ids = projects
                    .iter()
                    .map(|project| project.project_id.clone())
                    .collect::<Vec<_>>();
                serde_json::to_string_pretty(&json!({
                    "action": "remove",
                    "status": AgentResultStatus::DryRun.as_str(),
                    "disclosure": cooperative_host_decision_disclosure_json(),
                    "runtime_home": path_text(runtime_home),
                    "states": {
                        "runtime_home": "ready",
                        "connection": AgentResultStatus::DryRun.as_str(),
                        "project_registration": project_registration_state(projects),
                        "mcp_config": "membership",
                        "selected_profile": "not_checked",
                        "guard_installation": "not_checked",
                        "guard_files": "not_checked",
                        "guard_hook_observed": "not_checked",
                        "last_guard_event_at": Value::Null,
                        "prompt_capture": "not_checked",
                        "host_reload_required": false,
                        "guard_blockers": [],
                    },
                    "connection": connection_json(connection, &project_ids, None),
                    "target": connection.config_target,
                    "planned_change": "membership",
                    "remaining_connected_projects": remaining_count,
                    "checks": [{
                        "id": "connection_membership",
                        "status": "passed",
                        "summary": "selected repository membership can be removed"
                    }],
                    "actions": [],
                    "primary_next_action": Value::Null,
                }))
                .map(|text| format!("{text}\n"))
                .map_err(|error| ConnectionCommandError::runtime(error.to_string()))
            }
        },
    }
}

fn render_compact_membership_remove_plan_text(
    connection: &AgentConnectionRecord,
    selected_project: &ConnectionProjectRecord,
    remaining_count: usize,
) -> Result<String, ConnectionCommandError> {
    let host_kind = parse_host_kind(&connection.host_kind)?;
    let intent = parse_connection_intent(&connection.intent)?;
    let host = public_host_display_name(host_kind);
    let repo_root = &selected_project.project.repo_root;
    let apply_command = connection_remove_command(host_kind, intent, repo_root, false, false);
    let diagnostics_command = connection_remove_command(host_kind, intent, repo_root, true, true);
    Ok(format!(
        "Agent Connection plan for {host}\n\nStatus:\n  Plan: dry run\n  Mode: {}\n  Intent: {}\n\nRepository:\n  {}\n\nPlanned changes:\n  remove selected repository membership\n  keep host configuration for {} {}\n\nNext:\n  1. Run:\n     {}\n\nDiagnostics:\n  Run:\n    {}\n",
        public_mode_text(&connection.mode),
        connection.intent,
        repo_root.display(),
        remaining_count,
        connected_repository_phrase(remaining_count),
        apply_command,
        diagnostics_command
    ))
}
