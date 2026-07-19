use std::path::{Path, PathBuf};

use super::*;

pub(super) struct InitProvisioningRequest<'a> {
    pub(super) parsed: &'a ParsedInitOptions,
    pub(super) current_dir: &'a Path,
}

pub(super) struct InitProvisioningOutcome {
    pub(super) dry_run: bool,
    pub(super) host_kind: HostKind,
    pub(super) host_scope: HostScope,
    pub(super) runtime_home: PathBuf,
    pub(super) repo_root: PathBuf,
    pub(super) connection_id: String,
    pub(super) mode: String,
    pub(super) host_plan: HostPlan,
    pub(super) verification: Option<VerificationReport>,
    pub(super) current_report: Option<volicord_types::ConnectionVerificationReport>,
    pub(super) planned_changes: Vec<PlannedConnectionChange>,
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
    output_format: OutputFormat,
    migration_id: String,
    host_kind: HostKind,
    init_mode: InitMode,
    intent: ConnectionIntent,
    host_scope: HostScope,
    runtime_home: PathBuf,
    repo_root: PathBuf,
    connection_id: String,
    effective_mode: String,
    expected_connection: Option<InitConnectionExpectation>,
    current_report: Option<volicord_types::ConnectionVerificationReport>,
    project_id: Option<String>,
    host_plan: HostPlan,
    integration: GuardIntegrationPlan,
    profile_plan: InitProfilePlan,
    profile_exists: bool,
    target_hint: String,
    guard_installation_id: String,
    server_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct InitConnectionExpectation {
    connection_internal_id: String,
    mode: String,
    managed_fingerprint: String,
}

struct SupersededIntegration {
    connection: AgentConnectionRecord,
    selected_project: ConnectionProjectRecord,
}

#[derive(Clone, Copy)]
enum MigrationRegistryPhase {
    Pending,
    Attempted,
    AppliedCleanupPending,
    AppliedCleanupUnknown,
    Applied,
}

impl MigrationRegistryPhase {
    fn registry_transition(self, connection_migration: bool) -> &'static str {
        if !connection_migration {
            return "not_required";
        }
        match self {
            Self::Pending => "not_applied",
            Self::Attempted => "unknown",
            Self::AppliedCleanupPending | Self::AppliedCleanupUnknown | Self::Applied => "applied",
        }
    }

    fn prior_connection_inventory(self, connection_migration: bool) -> &'static str {
        if !connection_migration {
            return "unchanged";
        }
        match self {
            Self::Pending => "unchanged",
            Self::Attempted | Self::AppliedCleanupUnknown => "unknown",
            Self::AppliedCleanupPending => "disabled_pending_host_cleanup",
            Self::Applied => "retired_for_project",
        }
    }

    fn host_projection(self) -> &'static str {
        match self {
            Self::Pending => "partially_applied_or_pending_verification",
            Self::Attempted => "applied_registry_transition_unknown",
            Self::AppliedCleanupPending => "partially_applied_after_registry_transition",
            Self::AppliedCleanupUnknown => "cleanup_inventory_changed_after_registry_transition",
            Self::Applied => "applied_pending_verification",
        }
    }
}

pub(super) fn provision_init(
    request: InitProvisioningRequest<'_>,
    process: &mut impl ConnectionProcess,
) -> Result<InitProvisioningOutcome, ConnectionCommandError> {
    let dry_run = request.parsed.dry_run;
    let plan = plan_init_provisioning(request, process)?;
    if dry_run {
        let planned_changes = plan_init_changes(InitPlannedChanges {
            runtime_home: &plan.runtime_home,
            repo_root: &plan.repo_root,
            profile_exists: plan.profile_exists,
            project_exists: plan.project_id.is_some(),
            host_plan: &plan.host_plan,
            integration: &plan.integration,
        });
        return Ok(InitProvisioningOutcome {
            dry_run: true,
            host_kind: plan.host_kind,
            host_scope: plan.host_scope,
            runtime_home: plan.runtime_home,
            repo_root: plan.repo_root.clone(),
            connection_id: plan.connection_id,
            mode: plan.effective_mode,
            host_plan: plan.host_plan,
            verification: None,
            current_report: plan.current_report,
            planned_changes,
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
    let effective_mode = effective_init_connection_mode(existing.as_ref())?;
    let expected_connection = existing.as_ref().map(init_connection_expectation);
    let current_report = existing
        .as_ref()
        .map(effective_connection_report)
        .transpose()?;
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
            mode: &effective_mode,
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
    let migration_id = stable_id(
        "migration",
        &[
            &connection_id,
            &repo_root_key,
            intent.as_str(),
            parsed.mode.profile_value(),
        ],
    );
    let integration = plan_guard_integration(GuardIntegrationPlanRequest {
        host_kind,
        profile: parsed.mode.integration_profile(),
        runtime_home: &runtime_home,
        volicord_command: &profile_plan.volicord_command,
        repo_root: &repo_root,
        connection_id: &connection_id,
        guard_installation_id: &guard_installation_id,
        mcp_entry: &host_plan.entry,
        connection_intent: intent,
    })?;

    Ok(InitProvisioningPlan {
        output_format: init_output_format(parsed),
        migration_id,
        host_kind,
        init_mode: parsed.mode,
        intent,
        host_scope,
        runtime_home,
        repo_root,
        connection_id,
        effective_mode,
        expected_connection,
        current_report,
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
    mut plan: InitProvisioningPlan,
    process: &mut impl ConnectionProcess,
) -> Result<InitProvisioningOutcome, ConnectionCommandError> {
    validate_init_connection_expectation(&plan)?;
    let runtime_home_id = runtime_home_id_for_path(&plan.runtime_home)
        .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?;
    initialize_runtime_home(&plan.runtime_home, &runtime_home_id, ADMIN_METADATA_JSON)?;
    let profile = ensure_init_installation_profile(&plan.runtime_home, &plan.profile_plan)?;
    let project = ensure_project_for_repo(
        &plan.runtime_home,
        RepoProjectRegistration {
            project_name: None,
            project_alias: None,
            repo_root: plan.repo_root.clone(),
            project_home: None,
            status: ACTIVE_PROJECT_STATUS.to_owned(),
            metadata_json: metadata_json_base()?,
        },
    )?;
    plan.project_id = Some(project.project_id.clone());
    let existing = validate_init_connection_expectation(&plan)?;
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
            mode: &plan.effective_mode,
            expected_fingerprint,
        },
        process,
    )?;
    ensure_host_plan_has_no_conflict(&host_plan)?;
    let mut integration = plan_guard_integration(GuardIntegrationPlanRequest {
        host_kind: plan.host_kind,
        profile: plan.init_mode.integration_profile(),
        runtime_home: &plan.runtime_home,
        volicord_command: Path::new(&profile.volicord_command),
        repo_root: &project.repo_root,
        connection_id: &plan.connection_id,
        guard_installation_id: &plan.guard_installation_id,
        mcp_entry: &host_plan.entry,
        connection_intent: plan.intent,
    })?;
    let superseded_integrations = superseded_integrations_for_project(
        &plan.runtime_home,
        &plan.connection_id,
        integration.prior_connection_id.as_deref(),
        &project.repo_root,
    )?;
    let is_connection_migration = !superseded_integrations.is_empty();
    let is_integration_migration = integration.migration_required || is_connection_migration;
    let mcp_command = PathBuf::from(&host_plan.entry.command);
    let metadata_json = connection_metadata_json(&host_plan, &mcp_command, &plan.runtime_home)?;
    let desired_connection_registration = AgentConnectionRegistration {
        connection_internal_id: plan.connection_id.clone(),
        host_kind: plan.host_kind.as_str().to_owned(),
        intent: plan.intent.as_str().to_owned(),
        host_scope: plan.host_scope.as_str().to_owned(),
        server_name: host_plan.server_name.clone(),
        config_target: host_target_text(&host_plan.target),
        mode: plan.effective_mode.clone(),
        enabled: !is_connection_migration,
        managed_fingerprint: host_plan.fingerprint.clone(),
        metadata_json,
    };
    let superseded_bindings = superseded_integrations
        .iter()
        .map(|integration| SupersededConnectionProject {
            connection_internal_id: integration.connection.connection_internal_id.clone(),
            project_id: integration.selected_project.project_id.clone(),
        })
        .collect::<Vec<_>>();
    let mut cleanup_resume = false;
    let mut cleanup_resume_pending = Vec::new();
    if is_connection_migration && existing.is_some() {
        let (_, migration_state) = migration_step(
            &plan,
            &superseded_integrations,
            is_integration_migration,
            staged_connection_migration_state(
                &plan.runtime_home,
                &desired_connection_registration.connection_internal_id,
                &project.project_id,
                &superseded_bindings,
            )
            .map_err(ConnectionCommandError::from),
        )?;
        if let StagedConnectionMigrationState::CleanupResume {
            pending_connection_ids,
        } = migration_state
        {
            cleanup_resume = true;
            cleanup_resume_pending = pending_connection_ids;
        }
    }
    if plan.expected_connection.is_some() {
        validate_init_connection_expectation(&plan)?;
    }
    migration_before_cleanup_step(
        &plan,
        &superseded_integrations,
        is_integration_migration,
        cleanup_resume,
        apply_guard_migration_protection(&mut integration).map_err(ConnectionCommandError::from),
    )?;
    migration_before_cleanup_step(
        &plan,
        &superseded_integrations,
        is_integration_migration,
        cleanup_resume,
        apply_host_plan(plan.host_kind, &host_plan, process),
    )?;
    let registered_connection = if is_connection_migration {
        ensure_staged_agent_connection(&plan.runtime_home, desired_connection_registration)
    } else {
        ensure_agent_connection(&plan.runtime_home, desired_connection_registration)
    };
    let mut connection = migration_before_cleanup_step(
        &plan,
        &superseded_integrations,
        is_integration_migration,
        cleanup_resume,
        registered_connection.map_err(ConnectionCommandError::from),
    )?;
    if is_connection_migration && !cleanup_resume {
        let (current_connection, migration_state) = migration_before_cleanup_step(
            &plan,
            &superseded_integrations,
            is_integration_migration,
            cleanup_resume,
            staged_connection_migration_state(
                &plan.runtime_home,
                &connection.connection_internal_id,
                &project.project_id,
                &superseded_bindings,
            )
            .map_err(ConnectionCommandError::from),
        )?;
        connection = current_connection;
        if let StagedConnectionMigrationState::CleanupResume {
            pending_connection_ids,
        } = migration_state
        {
            cleanup_resume = true;
            cleanup_resume_pending = pending_connection_ids;
        }
    } else if !is_connection_migration {
        migration_step(
            &plan,
            &superseded_integrations,
            is_integration_migration,
            add_connection_project(
                &plan.runtime_home,
                ConnectionProjectRegistration {
                    connection_internal_id: connection.connection_internal_id.clone(),
                    project_id: project.project_id.clone(),
                },
            )
            .map(|_| ())
            .map_err(ConnectionCommandError::from),
        )?;
    }
    migration_before_cleanup_step(
        &plan,
        &superseded_integrations,
        is_integration_migration,
        cleanup_resume,
        enforce_single_project_scope(&plan.runtime_home, &connection, &project.project_id),
    )?;
    // Host setup may create repository-local parent directories. Replan after
    // those mutations so every managed-file snapshot is anchored to the
    // current filesystem state. The protective union exclude was already
    // applied above and remains in force while the migration completes.
    let mut integration = migration_before_cleanup_step(
        &plan,
        &superseded_integrations,
        is_integration_migration,
        cleanup_resume,
        plan_guard_integration(GuardIntegrationPlanRequest {
            host_kind: plan.host_kind,
            profile: plan.init_mode.integration_profile(),
            runtime_home: &plan.runtime_home,
            volicord_command: Path::new(&profile.volicord_command),
            repo_root: &project.repo_root,
            connection_id: &plan.connection_id,
            guard_installation_id: &plan.guard_installation_id,
            mcp_entry: &host_plan.entry,
            connection_intent: plan.intent,
        })
        .map_err(ConnectionCommandError::from),
    )?;
    integration.migration_protection_applied = true;
    migration_before_cleanup_step(
        &plan,
        &superseded_integrations,
        is_integration_migration,
        cleanup_resume,
        record_authoritative_workflow_policy(
            &plan.runtime_home,
            &project.project_id,
            &integration.policy,
        ),
    )?;
    let integration = migration_before_cleanup_step(
        &plan,
        &superseded_integrations,
        is_integration_migration,
        cleanup_resume,
        apply_guard_integration(integration).map_err(ConnectionCommandError::from),
    )?;
    let (_guard_installation, pending_host_cleanup_connections) = if cleanup_resume {
        let guard_installation = migration_cleanup_step(
            &plan,
            &superseded_integrations,
            is_integration_migration,
            record_guard_installation(
                &plan.runtime_home,
                &connection,
                &project.project_id,
                &integration,
            )
            .map_err(ConnectionCommandError::from),
        )?;
        (guard_installation, cleanup_resume_pending)
    } else if is_connection_migration {
        let guard_upsert = migration_step(
            &plan,
            &superseded_integrations,
            is_integration_migration,
            guard_installation_upsert(&connection, &project.project_id, &integration)
                .map_err(ConnectionCommandError::from),
        )?;
        let (activated_connection, guard_installation, pending) = migration_transition_step(
            &plan,
            &superseded_integrations,
            is_integration_migration,
            activate_staged_connection(
                &plan.runtime_home,
                &connection.connection_internal_id,
                &project.project_id,
                &superseded_bindings,
                guard_upsert,
            )
            .map_err(ConnectionCommandError::from),
        )?;
        connection = activated_connection;
        (guard_installation, pending)
    } else {
        (
            migration_step(
                &plan,
                &superseded_integrations,
                is_integration_migration,
                record_guard_installation(
                    &plan.runtime_home,
                    &connection,
                    &project.project_id,
                    &integration,
                )
                .map_err(ConnectionCommandError::from),
            )?,
            Vec::new(),
        )
    };
    if !pending_host_cleanup_connections.is_empty() {
        let cleanup = complete_pending_host_cleanup(
            &plan.runtime_home,
            &project.project_id,
            &connection.connection_internal_id,
            &pending_host_cleanup_connections,
            |pending_connection_ids| {
                retire_superseded_host_configuration(
                    &plan.runtime_home,
                    &superseded_integrations,
                    pending_connection_ids,
                    process,
                )
            },
        );
        match cleanup {
            Ok(()) => {}
            Err(PendingHostCleanupError::Host(error)) => migration_cleanup_step(
                &plan,
                &superseded_integrations,
                is_integration_migration,
                Err(error),
            )?,
            Err(PendingHostCleanupError::Store(error)) => migration_cleanup_unknown_step(
                &plan,
                &superseded_integrations,
                is_integration_migration,
                Err(ConnectionCommandError::from(error)),
            )?,
        }
    }
    let expected_integration_revision = connection_integration_revision(&connection)?;
    let launch = mcp_launch_from_host_plan(&host_plan, Some(&project.repo_root));
    let verification = migration_post_transition_step(
        &plan,
        &superseded_integrations,
        is_integration_migration,
        verify_connection(
            &plan.runtime_home,
            &connection,
            &host_plan,
            &launch,
            Some(&project.project_id),
            process,
        ),
    )?;
    connection = migration_post_transition_step(
        &plan,
        &superseded_integrations,
        is_integration_migration,
        persist_connection_verification_report(
            &plan.runtime_home,
            &connection.connection_internal_id,
            &expected_integration_revision,
            Some(&verification.report),
        ),
    )?;
    let _ = connection;

    Ok(InitProvisioningOutcome {
        dry_run: false,
        host_kind: plan.host_kind,
        host_scope: plan.host_scope,
        runtime_home: plan.runtime_home,
        repo_root: project.repo_root,
        connection_id: plan.connection_id,
        mode: plan.effective_mode,
        host_plan,
        verification: Some(verification),
        current_report: None,
        planned_changes: Vec::new(),
    })
}

fn effective_init_connection_mode(
    existing: Option<&AgentConnectionRecord>,
) -> Result<String, ConnectionCommandError> {
    match existing.map(|connection| connection.mode.as_str()) {
        None => Ok(CONNECTION_MODE_WORKFLOW.to_owned()),
        Some(mode @ (CONNECTION_MODE_WORKFLOW | CONNECTION_MODE_READ_ONLY)) => Ok(mode.to_owned()),
        Some(mode) => Err(ConnectionCommandError::runtime(format!(
            "stored Agent Connection has invalid mode {mode}"
        ))),
    }
}

fn init_connection_expectation(connection: &AgentConnectionRecord) -> InitConnectionExpectation {
    InitConnectionExpectation {
        connection_internal_id: connection.connection_internal_id.clone(),
        mode: connection.mode.clone(),
        managed_fingerprint: connection.managed_fingerprint.clone(),
    }
}

fn validate_init_connection_expectation(
    plan: &InitProvisioningPlan,
) -> Result<Option<AgentConnectionRecord>, ConnectionCommandError> {
    let current = connection_for_host_target(
        &plan.runtime_home,
        plan.host_kind,
        plan.intent,
        plan.host_scope,
        &plan.target_hint,
        &plan.server_name,
    )?;
    match (&plan.expected_connection, current) {
        (None, None) => Ok(None),
        (None, Some(_)) => Err(init_connection_changed_error(
            "a matching Agent Connection appeared after init planning",
        )),
        (Some(_), None) => Err(init_connection_changed_error(
            "the planned Agent Connection no longer matches the selected host target",
        )),
        (Some(expected), Some(current)) => {
            if current.connection_internal_id != expected.connection_internal_id {
                return Err(init_connection_changed_error(
                    "the selected host target now resolves to a different Agent Connection",
                ));
            }
            if current.mode != expected.mode {
                return Err(init_connection_changed_error(&format!(
                    "Agent Connection mode changed from {} to {} after init planning",
                    expected.mode, current.mode
                )));
            }
            if current.managed_fingerprint != expected.managed_fingerprint {
                return Err(init_connection_changed_error(
                    "the Agent Connection managed configuration fingerprint changed after init planning",
                ));
            }
            Ok(Some(current))
        }
    }
}

fn init_connection_changed_error(detail: &str) -> ConnectionCommandError {
    ConnectionCommandError::runtime(format!(
        "INIT_CONNECTION_CHANGED: {detail}; rerun `volicord init` against the current state"
    ))
}

pub(super) fn record_authoritative_workflow_policy(
    runtime_home: &Path,
    project_id: &str,
    policy: &Value,
) -> Result<(), ConnectionCommandError> {
    let canonical_json = canonical_json_string(policy)
        .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?;
    let fingerprint = canonical_json_sha256(policy)
        .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?
        .into_inner();
    let mut store = CoreProjectStore::open(runtime_home, &ProjectId::new(project_id))?;
    let prior = store.project_workflow_policy()?;
    if prior
        .as_ref()
        .is_some_and(|record| record.policy_fingerprint == fingerprint)
    {
        return Ok(());
    }
    let policy_version = prior.as_ref().map_or(Ok(1), |record| {
        record.policy_version.checked_add(1).ok_or_else(|| {
            ConnectionCommandError::runtime("project workflow policy version is exhausted")
        })
    })?;
    store.apply_project_workflow_policy_authority(ProjectWorkflowPolicyAuthorityApply {
        policy_version,
        policy_json: canonical_json,
        policy_fingerprint: fingerprint,
        source: "project_database".to_owned(),
        expected_prior_fingerprint: prior
            .as_ref()
            .map(|record| record.policy_fingerprint.clone()),
    })?;
    Ok(())
}

fn superseded_integrations_for_project(
    runtime_home: &Path,
    requested_connection_id: &str,
    prior_policy_connection_id: Option<&str>,
    repo_root: &Path,
) -> Result<Vec<SupersededIntegration>, ConnectionCommandError> {
    let mut integrations = Vec::new();
    for connection in list_agent_connections(runtime_home)? {
        if connection.host_kind != "codex"
            || !matches!(connection.intent.as_str(), "personal" | "shared")
            || connection.connection_internal_id == requested_connection_id
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
        let selected_by_prior_policy =
            prior_policy_connection_id == Some(connection.connection_internal_id.as_str());
        if !connection.enabled
            && !selected_by_prior_policy
            && !connection_metadata_has_pending_host_cleanup_for_project(
                &connection.metadata_json,
                &selected_project.project_id,
            )
        {
            continue;
        }
        integrations.push(SupersededIntegration {
            connection,
            selected_project,
        });
    }
    Ok(integrations)
}

fn retire_superseded_host_configuration(
    runtime_home: &Path,
    integrations: &[SupersededIntegration],
    disabled_connection_ids: &[String],
    process: &impl ConnectionProcess,
) -> Result<(), ConnectionCommandError> {
    for integration in integrations {
        if disabled_connection_ids
            .iter()
            .any(|connection_id| connection_id == &integration.connection.connection_internal_id)
        {
            let host_plan = existing_host_plan(
                &integration.connection,
                runtime_home,
                process,
                Some(&integration.selected_project),
            )?;
            remove_host_configuration(&host_plan, &integration.connection, process)?;
        }
    }
    Ok(())
}

fn migration_step<T>(
    plan: &InitProvisioningPlan,
    integrations: &[SupersededIntegration],
    migration_required: bool,
    result: Result<T, ConnectionCommandError>,
) -> Result<T, ConnectionCommandError> {
    if !migration_required {
        return result;
    }
    result.map_err(|error| {
        migration_partial_application(plan, integrations, MigrationRegistryPhase::Pending, &error)
    })
}

fn migration_transition_step<T>(
    plan: &InitProvisioningPlan,
    integrations: &[SupersededIntegration],
    migration_required: bool,
    result: Result<T, ConnectionCommandError>,
) -> Result<T, ConnectionCommandError> {
    if !migration_required {
        return result;
    }
    result.map_err(|error| {
        migration_partial_application(
            plan,
            integrations,
            MigrationRegistryPhase::Attempted,
            &error,
        )
    })
}

fn migration_before_cleanup_step<T>(
    plan: &InitProvisioningPlan,
    integrations: &[SupersededIntegration],
    migration_required: bool,
    cleanup_resume: bool,
    result: Result<T, ConnectionCommandError>,
) -> Result<T, ConnectionCommandError> {
    if cleanup_resume {
        migration_cleanup_step(plan, integrations, migration_required, result)
    } else {
        migration_step(plan, integrations, migration_required, result)
    }
}

fn migration_cleanup_step<T>(
    plan: &InitProvisioningPlan,
    integrations: &[SupersededIntegration],
    migration_required: bool,
    result: Result<T, ConnectionCommandError>,
) -> Result<T, ConnectionCommandError> {
    if !migration_required {
        return result;
    }
    result.map_err(|error| {
        migration_partial_application(
            plan,
            integrations,
            MigrationRegistryPhase::AppliedCleanupPending,
            &error,
        )
    })
}

fn migration_cleanup_unknown_step<T>(
    plan: &InitProvisioningPlan,
    integrations: &[SupersededIntegration],
    migration_required: bool,
    result: Result<T, ConnectionCommandError>,
) -> Result<T, ConnectionCommandError> {
    if !migration_required {
        return result;
    }
    result.map_err(|error| {
        migration_partial_application(
            plan,
            integrations,
            MigrationRegistryPhase::AppliedCleanupUnknown,
            &error,
        )
    })
}

fn migration_post_transition_step<T>(
    plan: &InitProvisioningPlan,
    integrations: &[SupersededIntegration],
    migration_required: bool,
    result: Result<T, ConnectionCommandError>,
) -> Result<T, ConnectionCommandError> {
    if !migration_required {
        return result;
    }
    result.map_err(|error| {
        migration_partial_application(plan, integrations, MigrationRegistryPhase::Applied, &error)
    })
}

fn migration_prior_connection_state(
    plan: &InitProvisioningPlan,
    integration: &SupersededIntegration,
) -> String {
    let connection_id = &integration.connection.connection_internal_id;
    let project_id = &integration.selected_project.project_id;
    let connection = match agent_connection_record(&plan.runtime_home, connection_id) {
        Ok(Some(connection)) => connection,
        Ok(None) => return "retired_for_project".to_owned(),
        Err(_) => return "unknown".to_owned(),
    };
    let membership_active = match list_connection_projects(&plan.runtime_home, connection_id) {
        Ok(memberships) => memberships
            .iter()
            .any(|membership| membership.project_id == *project_id),
        Err(_) => return "unknown".to_owned(),
    };
    if !membership_active {
        return "retired_for_project".to_owned();
    }
    if connection.enabled {
        return "unchanged".to_owned();
    }
    if connection_metadata_has_pending_host_cleanup(
        &connection.metadata_json,
        project_id,
        &plan.connection_id,
    ) {
        "disabled_pending_host_cleanup".to_owned()
    } else {
        "disabled_for_project".to_owned()
    }
}

fn aggregate_prior_connection_inventory(
    prior_connection_states: &[(String, String)],
    fallback: &'static str,
) -> String {
    if prior_connection_states.is_empty() || matches!(fallback, "unchanged" | "unknown") {
        return fallback.to_owned();
    }
    let states = prior_connection_states
        .iter()
        .map(|(_, state)| state.as_str())
        .collect::<BTreeSet<_>>();
    match states.into_iter().collect::<Vec<_>>().as_slice() {
        [state] => (*state).to_owned(),
        _ => "mixed".to_owned(),
    }
}

fn migration_partial_application(
    plan: &InitProvisioningPlan,
    integrations: &[SupersededIntegration],
    registry_phase: MigrationRegistryPhase,
    error: &ConnectionCommandError,
) -> ConnectionCommandError {
    let prior_connection_ids = integrations
        .iter()
        .map(|integration| integration.connection.connection_internal_id.clone())
        .collect::<Vec<_>>();
    let connection_migration = !integrations.is_empty();
    let prior_connection_states = integrations
        .iter()
        .map(|integration| {
            (
                integration.connection.connection_internal_id.clone(),
                migration_prior_connection_state(plan, integration),
            )
        })
        .collect::<Vec<_>>();
    let requested_connection_enabled =
        agent_connection_record(&plan.runtime_home, &plan.connection_id)
            .ok()
            .flatten()
            .map(|connection| connection.enabled);
    let requested_project_membership_active = plan.project_id.as_deref().and_then(|project_id| {
        list_connection_projects(&plan.runtime_home, &plan.connection_id)
            .ok()
            .map(|memberships| {
                memberships
                    .iter()
                    .any(|membership| membership.project_id == project_id)
            })
    });
    let registry_transition = registry_phase.registry_transition(connection_migration);
    let prior_connection_inventory = aggregate_prior_connection_inventory(
        &prior_connection_states,
        registry_phase.prior_connection_inventory(connection_migration),
    );
    let host_projection = registry_phase.host_projection();
    let mut retry_arguments = vec![
        "volicord".to_owned(),
        "init".to_owned(),
        "--home".to_owned(),
        path_text(&plan.runtime_home),
        "--host".to_owned(),
        public_host_label(plan.host_kind).to_owned(),
    ];
    if plan.intent == ConnectionIntent::Shared {
        retry_arguments.push("--shared".to_owned());
    }
    retry_arguments.extend([
        "--repo".to_owned(),
        path_text(&plan.repo_root),
        "--profile".to_owned(),
        plan.init_mode.profile_value().to_owned(),
    ]);
    let explanation = error.to_string();
    let output = match plan.output_format {
        OutputFormat::Text => format!(
            "Result: failed\nMigration state: partial_application\nMigration ID: {}\nRequested connection: {} ({})\nRequested project membership: {}\nPrior connection inventory: {} ({})\nPrior connection states: {}\nRegistry transition: {}\nHost projection: {}\nWhy: {}\nNext: resolve the reported conflict, then rerun with arguments {}\n",
            plan.migration_id,
            match requested_connection_enabled {
                Some(true) => "enabled",
                Some(false) => "disabled",
                None => "unknown",
            },
            plan.connection_id,
            match requested_project_membership_active {
                Some(true) => "active",
                Some(false) => "inactive",
                None => "unknown",
            },
            prior_connection_inventory,
            prior_connection_ids.join(", "),
            prior_connection_states
                .iter()
                .map(|(connection_id, state)| format!("{connection_id}={state}"))
                .collect::<Vec<_>>()
                .join(", "),
            registry_transition,
            host_projection,
            explanation,
            serde_json::to_string(&retry_arguments).unwrap_or_else(|_| "[]".to_owned()),
        ),
        OutputFormat::Json => serde_json::to_string_pretty(&json!({
            "action": "init",
            "status": "failed",
            "migration": {
                "migration_id": plan.migration_id,
                "state": "partial_application",
                "requested_connection_id": plan.connection_id,
                "requested_connection_enabled": requested_connection_enabled,
                "requested_project_membership_active": requested_project_membership_active,
                "prior_connection_ids": prior_connection_ids,
                "prior_connection_inventory": prior_connection_inventory,
                "prior_connection_states": prior_connection_states
                    .iter()
                    .map(|(connection_id, state)| json!({
                        "connection_id": connection_id,
                        "state": state
                    }))
                    .collect::<Vec<_>>(),
                "registry_transition": registry_transition,
                "host_projection": host_projection
            },
            "error": explanation,
            "retryable": true,
            "retry_arguments": retry_arguments,
            "next": "Resolve the reported conflict, then rerun the same init migration."
        }))
        .map(|text| format!("{text}\n"))
        .unwrap_or_else(|_| {
            format!(
                "Result: failed\nMigration state: partial_application\nMigration ID: {}\nWhy: {}\n",
                plan.migration_id, explanation
            )
        }),
    };
    ConnectionCommandError::FailureOutput(output)
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
    let desired_connection_registration = AgentConnectionRegistration {
        connection_internal_id: plan.connection_id,
        host_kind: plan.host_kind.as_str().to_owned(),
        intent: plan.intent.as_str().to_owned(),
        host_scope: plan.host_scope.as_str().to_owned(),
        server_name: host_plan.server_name.clone(),
        config_target: host_target_text(&host_plan.target),
        mode: plan.mode.clone(),
        enabled: true,
        managed_fingerprint: host_plan.fingerprint.clone(),
        metadata_json,
    };
    apply_host_plan(plan.host_kind, &host_plan, process)?;
    let mut connection =
        ensure_agent_connection(&plan.runtime_home, desired_connection_registration)?;
    enforce_single_project_scope(&plan.runtime_home, &connection, &project.project_id)?;
    add_connection_project(
        &plan.runtime_home,
        ConnectionProjectRegistration {
            connection_internal_id: connection.connection_internal_id.clone(),
            project_id: project.project_id.clone(),
        },
    )?;
    let expected_integration_revision = connection_integration_revision(&connection)?;
    let launch = mcp_launch_from_host_plan(&host_plan, Some(&project.repo_root));
    let verification = verify_connection(
        &plan.runtime_home,
        &connection,
        &host_plan,
        &launch,
        Some(&project.project_id),
        process,
    )?;
    connection = persist_connection_verification_report(
        &plan.runtime_home,
        &connection.connection_internal_id,
        &expected_integration_revision,
        Some(&verification.report),
    )?;
    let projects =
        list_connection_projects(&plan.runtime_home, &connection.connection_internal_id)?;
    let guard_state = guard_state_for_connection(&plan.runtime_home, &connection, &projects)?;

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

#[cfg(test)]
mod init_planning_tests {
    use std::{ffi::OsString, fs, path::PathBuf};

    use volicord_store::bootstrap::runtime_home_record_read_only;
    use volicord_test_support::TempRuntimeHome;

    use super::*;

    struct PlanningProcess {
        current_exe: PathBuf,
    }

    impl PlanningProcess {
        fn new() -> Result<Self, std::io::Error> {
            Ok(Self {
                current_exe: std::env::current_exe()?,
            })
        }
    }

    impl ConnectionProcess for PlanningProcess {
        fn env_var(&self, _name: &str) -> Option<OsString> {
            None
        }

        fn current_exe(&self) -> Result<PathBuf, String> {
            Ok(self.current_exe.clone())
        }

        fn run_preflight(
            &mut self,
            _launch: &McpLaunch,
            _runtime_home: &Path,
            _connection_id: &str,
            _project_id: Option<&str>,
        ) -> Result<ConnectionProcessOutput, String> {
            Err("init planning must not run MCP preflight".to_owned())
        }

        fn verify_mcp_stdio(
            &mut self,
            _launch: &McpLaunch,
            _runtime_home: &Path,
            _connection_id: &str,
            _mode: &str,
        ) -> Result<McpVerification, String> {
            Err("init planning must not run MCP verification".to_owned())
        }
    }

    fn parsed_init(runtime_home: &Path, repo_root: &Path, dry_run: bool) -> ParsedInitOptions {
        ParsedInitOptions {
            host_kind: Some(HostKind::Codex),
            repo: Some(repo_root.to_path_buf()),
            runtime_home: Some(runtime_home.to_path_buf()),
            mcp_command: None,
            mode: InitMode::Record,
            shared: true,
            dry_run,
            json: true,
        }
    }

    fn directory_is_empty(path: &Path) -> Result<bool, std::io::Error> {
        Ok(fs::read_dir(path)?.next().is_none())
    }

    fn create_empty_product_repository(
        fixture: &TempRuntimeHome,
    ) -> Result<PathBuf, std::io::Error> {
        let repo_root = fixture.create_product_repo("empty-repo")?;
        fs::create_dir(repo_root.join(".git"))?;
        Ok(repo_root)
    }

    fn assert_empty_product_repository_untouched(repo_root: &Path) -> Result<(), std::io::Error> {
        let entries = fs::read_dir(repo_root)?.collect::<Result<Vec<_>, _>>()?;
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].file_name(), ".git");
        assert!(directory_is_empty(&repo_root.join(".git"))?);
        Ok(())
    }

    fn assert_no_planned_files_exist(plan: &InitProvisioningPlan) {
        for file in &plan.integration.generated_files {
            assert!(
                !file.path.exists(),
                "planning unexpectedly created {}",
                file.path.display()
            );
        }
    }

    #[test]
    fn normal_init_planning_validates_command_projection_before_apply(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("init-capability-preflight")?;
        let repo_root = create_empty_product_repository(&fixture)?;
        let parsed = parsed_init(fixture.path(), &repo_root, false);
        let process = PlanningProcess::new()?;

        let plan = plan_init_provisioning(
            InitProvisioningRequest {
                parsed: &parsed,
                current_dir: &repo_root,
            },
            &process,
        )?;
        for phase in volicord_types::GuardHookPhase::REQUIRED {
            let policy = plan.integration.policy_commands.get(phase);
            let runtime = plan.integration.runtime_commands.get(phase);
            assert_eq!(policy.args.len(), 14);
            assert_eq!(runtime.args.len(), 16);
            assert_eq!(&runtime.args[..12], &policy.args[..12]);
            assert_eq!(&runtime.args[14..], &policy.args[12..]);
        }
        assert!(runtime_home_record_read_only(fixture.path())?.is_none());
        assert!(!fixture.registry_db_path().exists());
        assert_no_planned_files_exist(&plan);
        assert_empty_product_repository_untouched(&repo_root)?;
        Ok(())
    }

    #[test]
    fn init_dry_run_does_not_write_runtime_or_repo_files() -> Result<(), Box<dyn std::error::Error>>
    {
        let fixture = TempRuntimeHome::new("init-dry-run-empty-repo")?;
        let repo_root = create_empty_product_repository(&fixture)?;
        let parsed = parsed_init(fixture.path(), &repo_root, true);
        let mut process = PlanningProcess::new()?;

        let outcome = provision_init(
            InitProvisioningRequest {
                parsed: &parsed,
                current_dir: &repo_root,
            },
            &mut process,
        )?;
        assert!(outcome.dry_run);
        assert_eq!(outcome.mode, CONNECTION_MODE_WORKFLOW);
        assert!(runtime_home_record_read_only(fixture.path())?.is_none());
        assert!(!fixture.registry_db_path().exists());
        assert!(directory_is_empty(fixture.path())?);
        assert_empty_product_repository_untouched(&repo_root)?;
        Ok(())
    }

    #[test]
    fn mode_change_after_init_planning_fails_before_host_or_guard_mutation(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("init-mode-planning-conflict")?;
        let repo_root = create_empty_product_repository(&fixture)?;
        let parsed = parsed_init(fixture.path(), &repo_root, false);
        let mut process = PlanningProcess::new()?;

        let initial = provision_init(
            InitProvisioningRequest {
                parsed: &parsed,
                current_dir: &repo_root,
            },
            &mut process,
        )?;
        assert_eq!(initial.mode, CONNECTION_MODE_WORKFLOW);
        let project_id = project_record_by_repo_root(fixture.path(), &repo_root)?
            .expect("initialized project")
            .project_id;
        let connection_id = initial.connection_id.as_str();
        let plan = plan_init_provisioning(
            InitProvisioningRequest {
                parsed: &parsed,
                current_dir: &repo_root,
            },
            &process,
        )?;
        assert_eq!(plan.effective_mode, CONNECTION_MODE_WORKFLOW);

        let host_target = PathBuf::from(&plan.target_hint);
        let host_before = fs::read(&host_target)?;
        let repository_before = directory_contents(&repo_root)?;
        let guard_manifest_before =
            list_guard_installations(fixture.path(), connection_id, Some(&project_id))?
                .into_iter()
                .next()
                .expect("initial Guard Installation")
                .manifest_json;
        let registry = rusqlite::Connection::open(fixture.registry_db_path())?;
        let changed = registry.execute(
            "UPDATE agent_connections
                SET mode = ?2,
                    integration_generation = integration_generation + 1
              WHERE connection_internal_id = ?1",
            [plan.connection_id.as_str(), CONNECTION_MODE_READ_ONLY],
        )?;
        assert_eq!(changed, 1);
        drop(registry);

        let error = match apply_init_provisioning(plan, &mut process) {
            Err(error) => error,
            Ok(_) => panic!("mode conflict unexpectedly applied"),
        };
        let message = error.to_string();
        assert!(message.contains("INIT_CONNECTION_CHANGED"));
        assert!(message.contains("mode changed from workflow to read_only"));
        assert!(message.contains("rerun `volicord init`"));
        assert_eq!(fs::read(host_target)?, host_before);
        assert_eq!(directory_contents(&repo_root)?, repository_before);
        assert_eq!(
            list_guard_installations(fixture.path(), connection_id, Some(&project_id))?
                .into_iter()
                .next()
                .expect("Guard Installation remains")
                .manifest_json,
            guard_manifest_before
        );
        Ok(())
    }

    fn directory_contents(
        root: &Path,
    ) -> Result<std::collections::BTreeMap<PathBuf, Vec<u8>>, std::io::Error> {
        fn visit(
            root: &Path,
            current: &Path,
            output: &mut std::collections::BTreeMap<PathBuf, Vec<u8>>,
        ) -> Result<(), std::io::Error> {
            for entry in fs::read_dir(current)? {
                let entry = entry?;
                let path = entry.path();
                if entry.file_type()?.is_dir() {
                    visit(root, &path, output)?;
                } else {
                    output.insert(
                        path.strip_prefix(root).unwrap().to_path_buf(),
                        fs::read(path)?,
                    );
                }
            }
            Ok(())
        }

        let mut output = std::collections::BTreeMap::new();
        visit(root, root, &mut output)?;
        Ok(output)
    }
}

#[cfg(test)]
mod migration_state_tests {
    use super::*;

    #[test]
    fn cleanup_revalidation_failure_reports_unknown_inventory() {
        assert_eq!(
            MigrationRegistryPhase::AppliedCleanupUnknown.registry_transition(true),
            "applied"
        );
        assert_eq!(
            MigrationRegistryPhase::AppliedCleanupUnknown.prior_connection_inventory(true),
            "unknown"
        );
        assert_eq!(
            MigrationRegistryPhase::AppliedCleanupUnknown.host_projection(),
            "cleanup_inventory_changed_after_registry_transition"
        );
    }

    #[test]
    fn mixed_prior_inventory_is_not_misreported_as_all_pending_cleanup() {
        let states = vec![
            (
                "conn_shared_elsewhere".to_owned(),
                "retired_for_project".to_owned(),
            ),
            (
                "conn_last_project".to_owned(),
                "disabled_pending_host_cleanup".to_owned(),
            ),
        ];

        assert_eq!(
            aggregate_prior_connection_inventory(
                &states,
                MigrationRegistryPhase::AppliedCleanupPending.prior_connection_inventory(true),
            ),
            "mixed"
        );
    }
}
