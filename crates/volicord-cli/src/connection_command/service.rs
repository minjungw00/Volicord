use std::path::{Path, PathBuf};

use super::*;

pub(super) struct InitProvisioningRequest<'a> {
    pub(super) parsed: &'a ParsedInitOptions,
    pub(super) current_dir: &'a Path,
}

pub(super) struct InitProvisioningOutcome {
    pub(super) status: AgentResultStatus,
    pub(super) host_kind: HostKind,
    pub(super) init_mode: InitMode,
    pub(super) intent: ConnectionIntent,
    pub(super) host_scope: HostScope,
    pub(super) runtime_home: PathBuf,
    pub(super) repo_root: PathBuf,
    pub(super) connection_id: String,
    pub(super) project_id: Option<String>,
    pub(super) host_plan: HostPlan,
    pub(super) verification: Option<VerificationReport>,
    pub(super) integration: GuardIntegrationPlan,
    pub(super) guard_installation: Option<GuardInstallationRecord>,
    pub(super) profile_action: &'static str,
}

pub(super) struct ProvisionConnectionRequest<'a> {
    pub(super) parsed: &'a ParsedConnectionOptions,
    pub(super) current_dir: &'a Path,
}

pub(super) enum ConnectionProvisioningOutcome {
    DryRun(Box<ConnectionProvisioningPlan>),
    Applied(Box<ConnectionProvisioningResult>),
}

pub(super) struct ConnectionProvisioningPlan {
    pub(super) runtime_home: PathBuf,
    pub(super) connection_id: String,
    pub(super) host_kind: HostKind,
    pub(super) intent: ConnectionIntent,
    pub(super) host_scope: HostScope,
    pub(super) mode: String,
    pub(super) repo_root: PathBuf,
    pub(super) host_plan: HostPlan,
    installation_profile: InstallationProfileRecord,
    target_hint: String,
    server_name: String,
}

pub(super) struct ConnectionProvisioningResult {
    pub(super) runtime_home: PathBuf,
    pub(super) connection: AgentConnectionRecord,
    pub(super) projects: Vec<ConnectionProjectRecord>,
    pub(super) affected_repo_root: PathBuf,
    pub(super) verification: VerificationReport,
    pub(super) host_plan: HostPlan,
    pub(super) guard_state: GuardOperationalState,
}

struct InitProvisioningPlan {
    host_kind: HostKind,
    init_mode: InitMode,
    intent: ConnectionIntent,
    host_scope: HostScope,
    runtime_home: PathBuf,
    repo_root: PathBuf,
    connection_id: String,
    project_id: Option<String>,
    host_plan: HostPlan,
    integration: GuardIntegrationPlan,
    profile_plan: InitProfilePlan,
    profile_exists: bool,
    target_hint: String,
    guard_installation_id: String,
    server_name: String,
}

struct OppositeIntegration {
    connection: AgentConnectionRecord,
    selected_project: ConnectionProjectRecord,
    host_plan: Option<HostPlan>,
}

pub(super) fn provision_init(
    request: InitProvisioningRequest<'_>,
    process: &mut impl ConnectionProcess,
) -> Result<InitProvisioningOutcome, ConnectionCommandError> {
    let dry_run = request.parsed.dry_run;
    let plan = plan_init_provisioning(request, process)?;
    if dry_run {
        return Ok(InitProvisioningOutcome {
            status: AgentResultStatus::DryRun,
            host_kind: plan.host_kind,
            init_mode: plan.init_mode,
            intent: plan.intent,
            host_scope: plan.host_scope,
            runtime_home: plan.runtime_home,
            repo_root: plan.repo_root,
            connection_id: plan.connection_id,
            project_id: plan.project_id,
            host_plan: plan.host_plan,
            verification: None,
            integration: plan.integration,
            guard_installation: None,
            profile_action: if plan.profile_exists {
                "reused"
            } else {
                "planned"
            },
        });
    }

    apply_init_provisioning(plan, process)
}

fn plan_init_provisioning(
    request: InitProvisioningRequest<'_>,
    process: &impl ConnectionProcess,
) -> Result<InitProvisioningPlan, ConnectionCommandError> {
    let parsed = request.parsed;
    let host_kind = parsed
        .host_kind
        .ok_or_else(|| ConnectionCommandError::usage("--host is required"))?;
    let repo = parsed
        .repo
        .as_deref()
        .ok_or_else(|| ConnectionCommandError::usage("--repo is required"))?;
    let repo_root = resolve_init_repo_root(request.current_dir, repo, host_kind, parsed.mode)?;
    let runtime_home = init_runtime_home_path(parsed, request.current_dir, process)?;
    let existing_profile = installation_profile(&runtime_home)?;
    let profile_plan =
        init_profile_plan(parsed, &runtime_home, existing_profile.as_ref(), process)?;
    let intent = if parsed.shared {
        ConnectionIntent::Shared
    } else {
        ConnectionIntent::Personal
    };
    let host_scope = host_scope_for_intent(host_kind, intent)?;
    let mode = CONNECTION_MODE_WORKFLOW;
    let server_name = DEFAULT_SERVER_NAME.to_owned();
    let target_hint = connection_target_hint(host_kind, host_scope, Some(&repo_root), process)?;
    let existing = connection_for_host_target(
        &runtime_home,
        host_kind,
        intent,
        host_scope,
        &target_hint,
        &server_name,
    )?;
    let connection_id = existing
        .as_ref()
        .map(|connection| connection.connection_internal_id.clone())
        .unwrap_or_else(|| {
            deterministic_connection_id(
                host_kind,
                host_scope,
                Some(&path_text(&repo_root)),
                &target_hint,
                &server_name,
            )
        });
    let project_hint = project_record_by_repo_root(&runtime_home, &repo_root)
        .ok()
        .flatten();
    let expected_fingerprint = existing
        .as_ref()
        .map(|connection| connection.managed_fingerprint.as_str());
    let installation_context = InstallationProfile {
        runtime_home: &runtime_home,
        volicord_command: &profile_plan.volicord_command,
        volicord_mcp_command: &profile_plan.volicord_mcp_command,
        default_connection_mode: CONNECTION_MODE_WORKFLOW,
    };
    let host_plan = build_host_plan(
        BuildHostPlanRequest {
            host_kind,
            connection_intent: intent,
            connection_id: &connection_id,
            repo_root: Some(&repo_root),
            project_id: project_hint
                .as_ref()
                .map(|project| project.project_id.as_str())
                .or(Some("planned_project")),
            project_name: project_hint
                .as_ref()
                .map(|project| project.project_name.as_str())
                .or(Some("planned project")),
            installation_profile: installation_context,
            mode,
            expected_fingerprint,
        },
        process,
    )?;
    ensure_host_plan_has_no_conflict(&host_plan)?;
    let repo_root_key = path_text(&repo_root);
    let guard_installation_id = stable_id(
        "guard_installation",
        &[&connection_id, &repo_root_key, parsed.mode.guard_value()],
    );
    let integration = plan_guard_integration(
        host_kind,
        parsed.mode.integration_profile(),
        &runtime_home,
        &repo_root,
        &connection_id,
        &guard_installation_id,
        &host_plan.entry,
        intent,
    )?;

    Ok(InitProvisioningPlan {
        host_kind,
        init_mode: parsed.mode,
        intent,
        host_scope,
        runtime_home,
        repo_root,
        connection_id,
        project_id: project_hint.map(|project| project.project_id),
        host_plan,
        integration,
        profile_plan,
        profile_exists: existing_profile.is_some(),
        target_hint,
        guard_installation_id,
        server_name,
    })
}

fn apply_init_provisioning(
    plan: InitProvisioningPlan,
    process: &mut impl ConnectionProcess,
) -> Result<InitProvisioningOutcome, ConnectionCommandError> {
    let runtime_home_id = runtime_home_id_for_path(&plan.runtime_home)
        .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?;
    initialize_runtime_home(&plan.runtime_home, &runtime_home_id, ADMIN_METADATA_JSON)?;
    let profile = ensure_init_installation_profile(&plan.runtime_home, &plan.profile_plan)?;
    let project = ensure_project_for_repo(
        &plan.runtime_home,
        RepoProjectRegistration {
            project_name: None,
            project_alias: None,
            repo_root: plan.repo_root,
            project_home: None,
            status: ACTIVE_PROJECT_STATUS.to_owned(),
            metadata_json: metadata_json_base()?,
        },
    )?;
    let mode = CONNECTION_MODE_WORKFLOW;
    let existing = connection_for_host_target(
        &plan.runtime_home,
        plan.host_kind,
        plan.intent,
        plan.host_scope,
        &plan.target_hint,
        &plan.server_name,
    )?;
    let expected_fingerprint = existing
        .as_ref()
        .map(|connection| connection.managed_fingerprint.as_str());
    let host_plan = build_host_plan(
        BuildHostPlanRequest {
            host_kind: plan.host_kind,
            connection_intent: plan.intent,
            connection_id: &plan.connection_id,
            repo_root: Some(&project.repo_root),
            project_id: Some(&project.project_id),
            project_name: Some(&project.project_name),
            installation_profile: installation_profile_context(&plan.runtime_home, &profile),
            mode,
            expected_fingerprint,
        },
        process,
    )?;
    ensure_host_plan_has_no_conflict(&host_plan)?;
    let mut integration = plan_guard_integration(
        plan.host_kind,
        plan.init_mode.integration_profile(),
        &plan.runtime_home,
        &project.repo_root,
        &plan.connection_id,
        &plan.guard_installation_id,
        &host_plan.entry,
        plan.intent,
    )?;
    apply_guard_migration_protection(&mut integration)?;
    let opposite_integrations = opposite_integrations_for_project(
        &plan.runtime_home,
        plan.host_kind,
        plan.intent,
        &project.repo_root,
        process,
    )?;
    let mcp_command = PathBuf::from(&host_plan.entry.command);
    let metadata_json = connection_metadata_json(&host_plan, &mcp_command, &plan.runtime_home)?;
    let mut connection = ensure_agent_connection(
        &plan.runtime_home,
        AgentConnectionRegistration {
            connection_internal_id: plan.connection_id.clone(),
            host_kind: plan.host_kind.as_str().to_owned(),
            intent: plan.intent.as_str().to_owned(),
            host_scope: plan.host_scope.as_str().to_owned(),
            server_name: host_plan.server_name.clone(),
            config_target: host_target_text(&host_plan.target),
            mode: mode.to_owned(),
            enabled: true,
            managed_fingerprint: host_plan.fingerprint.clone(),
            last_verification_status: existing
                .as_ref()
                .map(|record| record.last_verification_status.clone())
                .unwrap_or_else(|| VERIFIED_STATUS_NOT_VERIFIED.to_owned()),
            last_verification_report_json: existing
                .as_ref()
                .map(|record| record.last_verification_report_json.clone())
                .unwrap_or_else(|| "{}".to_owned()),
            last_user_actions_json: user_actions_json(&host_plan.user_actions)?,
            metadata_json,
        },
    )?;
    enforce_single_project_scope(&plan.runtime_home, &connection, &project.project_id)?;
    add_connection_project(
        &plan.runtime_home,
        ConnectionProjectRegistration {
            connection_internal_id: connection.connection_internal_id.clone(),
            project_id: project.project_id.clone(),
        },
    )?;
    apply_host_plan(plan.host_kind, &host_plan, process)?;
    retire_opposite_host_configuration(&opposite_integrations, process)?;
    // Host setup may create repository-local parent directories. Replan after
    // those mutations so every managed-file snapshot is anchored to the
    // current filesystem state. The protective union exclude was already
    // applied above and remains in force while the migration completes.
    let mut integration = plan_guard_integration(
        plan.host_kind,
        plan.init_mode.integration_profile(),
        &plan.runtime_home,
        &project.repo_root,
        &plan.connection_id,
        &plan.guard_installation_id,
        &host_plan.entry,
        plan.intent,
    )?;
    integration.migration_protection_applied = true;
    let integration = apply_guard_integration(integration)?;
    retire_opposite_connection_inventory(&plan.runtime_home, &opposite_integrations)?;
    let integration_profile = plan.init_mode.integration_profile();
    let installation_status =
        initial_guard_installation_status(integration_profile, &host_plan, &integration);
    let guard_installation = record_guard_installation(
        &plan.runtime_home,
        plan.host_kind,
        integration_profile,
        installation_status,
        &connection.connection_internal_id,
        &project.project_id,
        &integration,
    )?;
    let launch = mcp_launch_from_host_plan(&host_plan, Some(&project.repo_root));
    let verification = verify_connection(
        &plan.runtime_home,
        &connection,
        &host_plan,
        &launch,
        Some(&project.project_id),
        process,
    )?;
    let user_actions = init_first_run_user_actions(
        &verification.host.user_actions,
        plan.host_kind,
        plan.init_mode,
    );
    connection = update_agent_connection_verification_report(
        &plan.runtime_home,
        &connection.connection_internal_id,
        verification.status.store_status(),
        &host_plan.fingerprint,
        &detailed_verification_report_json(&verification)?,
        &user_actions_json(&user_actions)?,
    )?;
    let status = if verification.status == AgentResultStatus::Complete && user_actions.is_empty() {
        AgentResultStatus::Complete
    } else if verification.status == AgentResultStatus::Failed {
        AgentResultStatus::Failed
    } else {
        AgentResultStatus::ActionRequired
    };
    let _ = connection;

    Ok(InitProvisioningOutcome {
        status,
        host_kind: plan.host_kind,
        init_mode: plan.init_mode,
        intent: plan.intent,
        host_scope: plan.host_scope,
        runtime_home: plan.runtime_home,
        repo_root: project.repo_root,
        connection_id: plan.connection_id,
        project_id: Some(project.project_id),
        host_plan,
        verification: Some(verification),
        integration,
        guard_installation: Some(guard_installation),
        profile_action: if plan.profile_exists {
            "reused"
        } else {
            "created"
        },
    })
}

fn opposite_integrations_for_project(
    runtime_home: &Path,
    host_kind: HostKind,
    requested_intent: ConnectionIntent,
    repo_root: &Path,
    process: &impl ConnectionProcess,
) -> Result<Vec<OppositeIntegration>, ConnectionCommandError> {
    let mut integrations = Vec::new();
    for connection in list_agent_connections(runtime_home)? {
        if connection.host_kind != host_kind.as_str()
            || connection.intent == requested_intent.as_str()
            || !matches!(connection.intent.as_str(), "personal" | "shared")
        {
            continue;
        }
        let projects = list_connection_projects(runtime_home, &connection.connection_internal_id)?;
        let Some(selected_project) = projects
            .iter()
            .find(|project| project.project.repo_root == repo_root)
            .cloned()
        else {
            continue;
        };
        let host_plan = if projects.len() == 1 {
            Some(existing_host_plan(
                &connection,
                runtime_home,
                process,
                Some(&selected_project),
            )?)
        } else {
            None
        };
        integrations.push(OppositeIntegration {
            connection,
            selected_project,
            host_plan,
        });
    }
    Ok(integrations)
}

fn retire_opposite_host_configuration(
    integrations: &[OppositeIntegration],
    process: &impl ConnectionProcess,
) -> Result<(), ConnectionCommandError> {
    for integration in integrations {
        if let Some(host_plan) = &integration.host_plan {
            remove_host_configuration(host_plan, &integration.connection, process)?;
        }
    }
    Ok(())
}

fn retire_opposite_connection_inventory(
    runtime_home: &Path,
    integrations: &[OppositeIntegration],
) -> Result<(), ConnectionCommandError> {
    for integration in integrations {
        remove_connection_project(
            runtime_home,
            &integration.connection.connection_internal_id,
            &integration.selected_project.project_id,
        )?;
        if list_connection_projects(runtime_home, &integration.connection.connection_internal_id)?
            .is_empty()
        {
            set_connection_enabled(
                runtime_home,
                &integration.connection.connection_internal_id,
                false,
            )?;
        }
    }
    Ok(())
}

pub(super) fn provision_connection(
    request: ProvisionConnectionRequest<'_>,
    process: &mut impl ConnectionProcess,
) -> Result<ConnectionProvisioningOutcome, ConnectionCommandError> {
    let dry_run = request.parsed.dry_run;
    let plan = plan_connection_provisioning(request, process)?;
    if dry_run {
        return Ok(ConnectionProvisioningOutcome::DryRun(Box::new(plan)));
    }

    apply_connection_provisioning(plan, process)
        .map(Box::new)
        .map(ConnectionProvisioningOutcome::Applied)
}

fn plan_connection_provisioning(
    request: ProvisionConnectionRequest<'_>,
    process: &impl ConnectionProcess,
) -> Result<ConnectionProvisioningPlan, ConnectionCommandError> {
    let parsed = request.parsed;
    let host_kind = resolve_connection_host(parsed.host_kind, process)?;
    let intent = connection_intent_from_flags(parsed)?;
    let host_scope = host_scope_for_intent(host_kind, intent)?;
    let mode = if parsed.read_only {
        CONNECTION_MODE_READ_ONLY
    } else {
        CONNECTION_MODE_WORKFLOW
    };
    let runtime_home = resolve_runtime_home(|name| process.env_var(name), request.current_dir)?;
    let installation_profile = required_installation_profile(&runtime_home)?;
    let repo_root = resolve_connection_repo_root(request.current_dir, parsed.repo.as_deref())?;
    let server_name = DEFAULT_SERVER_NAME.to_owned();
    let target_hint = connection_target_hint(host_kind, host_scope, Some(&repo_root), process)?;
    let existing = connection_for_host_target(
        &runtime_home,
        host_kind,
        intent,
        host_scope,
        &target_hint,
        &server_name,
    )?;
    let connection_id = existing
        .as_ref()
        .map(|connection| connection.connection_internal_id.clone())
        .unwrap_or_else(|| {
            deterministic_connection_id(
                host_kind,
                host_scope,
                Some(&path_text(&repo_root)),
                &target_hint,
                &server_name,
            )
        });
    let project_hint = project_record_by_repo_root(&runtime_home, &repo_root)
        .ok()
        .flatten();
    let expected_fingerprint = existing
        .as_ref()
        .map(|connection| connection.managed_fingerprint.as_str());
    let host_plan = build_host_plan(
        BuildHostPlanRequest {
            host_kind,
            connection_intent: intent,
            connection_id: &connection_id,
            repo_root: Some(&repo_root),
            project_id: project_hint
                .as_ref()
                .map(|project| project.project_id.as_str())
                .or(Some("planned_project")),
            project_name: project_hint
                .as_ref()
                .map(|project| project.project_name.as_str())
                .or(Some("planned project")),
            installation_profile: installation_profile_context(
                &runtime_home,
                &installation_profile,
            ),
            mode,
            expected_fingerprint,
        },
        process,
    )?;
    ensure_host_plan_has_no_conflict(&host_plan)?;

    Ok(ConnectionProvisioningPlan {
        runtime_home,
        connection_id,
        host_kind,
        intent,
        host_scope,
        mode: mode.to_owned(),
        repo_root,
        host_plan,
        installation_profile,
        target_hint,
        server_name,
    })
}

fn apply_connection_provisioning(
    plan: ConnectionProvisioningPlan,
    process: &mut impl ConnectionProcess,
) -> Result<ConnectionProvisioningResult, ConnectionCommandError> {
    initialize_runtime_home(
        &plan.runtime_home,
        AGENT_RUNTIME_HOME_ID,
        metadata_json_base()?.as_str(),
    )?;
    let project = ensure_project_for_repo(
        &plan.runtime_home,
        RepoProjectRegistration {
            project_name: None,
            project_alias: None,
            repo_root: plan.repo_root,
            project_home: None,
            status: ACTIVE_PROJECT_STATUS.to_owned(),
            metadata_json: metadata_json_base()?,
        },
    )?;
    let existing = connection_for_host_target(
        &plan.runtime_home,
        plan.host_kind,
        plan.intent,
        plan.host_scope,
        &plan.target_hint,
        &plan.server_name,
    )?;
    let expected_fingerprint = existing
        .as_ref()
        .map(|connection| connection.managed_fingerprint.as_str());
    let host_plan = build_host_plan(
        BuildHostPlanRequest {
            host_kind: plan.host_kind,
            connection_intent: plan.intent,
            connection_id: &plan.connection_id,
            repo_root: Some(&project.repo_root),
            project_id: Some(&project.project_id),
            project_name: Some(&project.project_name),
            installation_profile: installation_profile_context(
                &plan.runtime_home,
                &plan.installation_profile,
            ),
            mode: &plan.mode,
            expected_fingerprint,
        },
        process,
    )?;
    ensure_host_plan_has_no_conflict(&host_plan)?;
    let mcp_command = PathBuf::from(&host_plan.entry.command);
    let metadata_json = connection_metadata_json(&host_plan, &mcp_command, &plan.runtime_home)?;
    let mut connection = ensure_agent_connection(
        &plan.runtime_home,
        AgentConnectionRegistration {
            connection_internal_id: plan.connection_id,
            host_kind: plan.host_kind.as_str().to_owned(),
            intent: plan.intent.as_str().to_owned(),
            host_scope: plan.host_scope.as_str().to_owned(),
            server_name: host_plan.server_name.clone(),
            config_target: host_target_text(&host_plan.target),
            mode: plan.mode.clone(),
            enabled: true,
            managed_fingerprint: host_plan.fingerprint.clone(),
            last_verification_status: existing
                .as_ref()
                .map(|record| record.last_verification_status.clone())
                .unwrap_or_else(|| VERIFIED_STATUS_NOT_VERIFIED.to_owned()),
            last_verification_report_json: existing
                .as_ref()
                .map(|record| record.last_verification_report_json.clone())
                .unwrap_or_else(|| "{}".to_owned()),
            last_user_actions_json: user_actions_json(&host_plan.user_actions)?,
            metadata_json,
        },
    )?;
    enforce_single_project_scope(&plan.runtime_home, &connection, &project.project_id)?;
    add_connection_project(
        &plan.runtime_home,
        ConnectionProjectRegistration {
            connection_internal_id: connection.connection_internal_id.clone(),
            project_id: project.project_id.clone(),
        },
    )?;
    apply_host_plan(plan.host_kind, &host_plan, process)?;
    let launch = mcp_launch_from_host_plan(&host_plan, Some(&project.repo_root));
    let verification = verify_connection(
        &plan.runtime_home,
        &connection,
        &host_plan,
        &launch,
        Some(&project.project_id),
        process,
    )?;
    connection = update_agent_connection_verification_report(
        &plan.runtime_home,
        &connection.connection_internal_id,
        verification.status.store_status(),
        &host_plan.fingerprint,
        &detailed_verification_report_json(&verification)?,
        &user_actions_json(&verification.host.user_actions)?,
    )?;
    let projects =
        list_connection_projects(&plan.runtime_home, &connection.connection_internal_id)?;
    let guard_state = guard_state_for_connection(
        &plan.runtime_home,
        &connection.connection_internal_id,
        &projects,
    )?;

    Ok(ConnectionProvisioningResult {
        runtime_home: plan.runtime_home,
        connection,
        projects,
        affected_repo_root: project.repo_root,
        verification,
        host_plan,
        guard_state,
    })
}

fn ensure_host_plan_has_no_conflict(plan: &HostPlan) -> Result<(), ConnectionCommandError> {
    if let Some(conflict) = plan.conflicts.first() {
        Err(ConnectionCommandError::runtime(conflict.message.clone()))
    } else {
        Ok(())
    }
}
