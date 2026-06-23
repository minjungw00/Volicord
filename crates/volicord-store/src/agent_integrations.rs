use std::path::{Path, PathBuf};

use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;

use crate::{
    bootstrap::{validate_current_project_registration, validate_project_id, ProjectRecord},
    sqlite::{begin_immediate_transaction, open_registry_database, registry_db_path},
    StoreError, StoreResult,
};

/// Baseline-valid Agent Integration Profile role.
pub const AGENT_INTERACTION_ROLE: &str = "agent";

/// Baseline-valid Codex host kind.
pub const HOST_KIND_CODEX: &str = "codex";
/// Baseline-valid Claude Code host kind.
pub const HOST_KIND_CLAUDE_CODE: &str = "claude_code";
/// Baseline-valid exported generic host kind.
pub const HOST_KIND_GENERIC: &str = "generic";

/// Baseline-valid user-scoped host configuration.
pub const HOST_SCOPE_USER: &str = "user";
/// Baseline-valid project-scoped host configuration.
pub const HOST_SCOPE_PROJECT: &str = "project";
/// Baseline-valid local host configuration.
pub const HOST_SCOPE_LOCAL: &str = "local";
/// Baseline-valid exported host configuration.
pub const HOST_SCOPE_EXPORT: &str = "export";

/// Host installation has not been checked.
pub const VERIFIED_STATUS_NOT_VERIFIED: &str = "not_verified";
/// Host installation has been verified complete.
pub const VERIFIED_STATUS_COMPLETE: &str = "complete";
/// Host installation needs a host-controlled action.
pub const VERIFIED_STATUS_ACTION_REQUIRED: &str = "action_required";
/// Host installation partly succeeded or partly failed verification.
pub const VERIFIED_STATUS_PARTIAL_FAILURE: &str = "partial_failure";
/// Host installation verification failed.
pub const VERIFIED_STATUS_FAILED: &str = "failed";

/// Agent Integration Profile creation input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentIntegrationRegistration {
    pub integration_id: String,
    pub interaction_role: String,
    pub surface_id: String,
    pub surface_instance_id: String,
    pub metadata_json: String,
}

/// Agent Integration Profile row stored in `registry.sqlite`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentIntegrationRecord {
    pub integration_id: String,
    pub interaction_role: String,
    pub surface_id: String,
    pub surface_instance_id: String,
    pub default_project_id: Option<String>,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
    pub metadata_json: String,
}

/// Explicit project allowlist row creation input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationProjectRegistration {
    pub integration_id: String,
    pub project_id: String,
}

/// Explicit project allowlist row with its current project registration facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationProjectRecord {
    pub integration_id: String,
    pub project_id: String,
    pub created_at: String,
    pub is_default: bool,
    pub project: ProjectRecord,
}

/// Current dynamic project-access facts for one integration/project pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentIntegrationProjectAccess {
    pub integration_id: String,
    pub project_id: String,
    pub integration_enabled: bool,
    pub project_allowed: bool,
    pub is_default: bool,
    pub project: Option<ProjectRecord>,
}

/// Host Installation creation or compatible update input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostInstallationRegistration {
    pub installation_id: String,
    pub integration_id: String,
    pub host_kind: String,
    pub host_scope: String,
    pub server_name: String,
    pub config_target: String,
    pub managed_fingerprint: String,
    pub last_verified_status: String,
    pub metadata_json: String,
}

/// Host Installation row stored in `registry.sqlite`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostInstallationRecord {
    pub installation_id: String,
    pub integration_id: String,
    pub host_kind: String,
    pub host_scope: String,
    pub server_name: String,
    pub config_target: String,
    pub managed_fingerprint: String,
    pub last_verified_status: String,
    pub created_at: String,
    pub updated_at: String,
    pub metadata_json: String,
}

/// Registers one Agent Integration Profile or returns the compatible existing row.
pub fn register_agent_integration(
    runtime_home: impl AsRef<Path>,
    registration: AgentIntegrationRegistration,
) -> StoreResult<AgentIntegrationRecord> {
    validate_agent_integration_registration(&registration)?;

    let registry_path = registry_db_path(runtime_home);
    let mut conn = open_registry_database(&registry_path)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    require_runtime_home(&tx, &registry_path)?;

    if let Some(existing) = agent_integration_record_from_conn(&tx, &registration.integration_id)? {
        if integration_registration_is_compatible(&existing, &registration) {
            tx.commit()?;
            return Ok(existing);
        }
        return Err(conflict(
            "agent_integration",
            &registration.integration_id,
            "integration_id is already bound to different integration facts",
        ));
    }

    tx.execute(
        "INSERT INTO agent_integrations (
            integration_id,
            interaction_role,
            surface_id,
            surface_instance_id,
            enabled,
            created_at,
            updated_at,
            metadata_json
        )
        VALUES (
            ?1,
            ?2,
            ?3,
            ?4,
            1,
            strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
            strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
            ?5
        )",
        params![
            registration.integration_id,
            registration.interaction_role,
            registration.surface_id,
            registration.surface_instance_id,
            registration.metadata_json
        ],
    )?;
    tx.commit()?;

    agent_integration_record_from_conn(&conn, &registration.integration_id)?.ok_or_else(|| {
        StoreError::NotFound {
            entity: "agent_integration",
            id: registration.integration_id,
        }
    })
}

/// Reads one Agent Integration Profile.
pub fn agent_integration_record(
    runtime_home: impl AsRef<Path>,
    integration_id: &str,
) -> StoreResult<Option<AgentIntegrationRecord>> {
    validate_identifier("integration_id", integration_id)?;
    let registry_path = registry_db_path(runtime_home);
    if !registry_path.exists() {
        return Ok(None);
    }

    let conn = open_registry_database(registry_path)?;
    agent_integration_record_from_conn(&conn, integration_id)
}

/// Lists Agent Integration Profiles in deterministic order.
pub fn list_agent_integrations(
    runtime_home: impl AsRef<Path>,
) -> StoreResult<Vec<AgentIntegrationRecord>> {
    let registry_path = registry_db_path(runtime_home);
    if !registry_path.exists() {
        return Ok(Vec::new());
    }

    let conn = open_registry_database(registry_path)?;
    let mut stmt = conn.prepare(
        "SELECT
            integration_id,
            interaction_role,
            surface_id,
            surface_instance_id,
            default_project_id,
            enabled,
            created_at,
            updated_at,
            metadata_json
         FROM agent_integrations
         ORDER BY integration_id",
    )?;
    let mut rows = stmt.query([])?;
    let mut integrations = Vec::new();
    while let Some(row) = rows.next()? {
        integrations.push(agent_integration_record_from_row(row)?);
    }
    Ok(integrations)
}

/// Enables or disables an Agent Integration Profile without changing host inventory.
pub fn set_agent_integration_enabled(
    runtime_home: impl AsRef<Path>,
    integration_id: &str,
    enabled: bool,
) -> StoreResult<AgentIntegrationRecord> {
    validate_identifier("integration_id", integration_id)?;
    let registry_path = registry_db_path(runtime_home);
    let mut conn = open_registry_database(&registry_path)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    require_runtime_home(&tx, &registry_path)?;
    let changed = tx.execute(
        "UPDATE agent_integrations
            SET enabled = ?2,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
          WHERE integration_id = ?1",
        params![integration_id, enabled_as_i64(enabled)],
    )?;
    if changed == 0 {
        return Err(StoreError::NotFound {
            entity: "agent_integration",
            id: integration_id.to_owned(),
        });
    }
    tx.commit()?;

    agent_integration_record_from_conn(&conn, integration_id)?.ok_or_else(|| StoreError::NotFound {
        entity: "agent_integration",
        id: integration_id.to_owned(),
    })
}

/// Removes an Agent Integration Profile only when no dependent registry rows remain.
pub fn remove_agent_integration_if_unused(
    runtime_home: impl AsRef<Path>,
    integration_id: &str,
) -> StoreResult<bool> {
    validate_identifier("integration_id", integration_id)?;
    let registry_path = registry_db_path(runtime_home);
    let mut conn = open_registry_database(&registry_path)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    require_runtime_home(&tx, &registry_path)?;
    require_agent_integration(&tx, integration_id)?;

    let membership_count: i64 = tx.query_row(
        "SELECT COUNT(*)
           FROM integration_projects
          WHERE integration_id = ?1",
        [integration_id],
        |row| row.get(0),
    )?;
    let installation_count: i64 = tx.query_row(
        "SELECT COUNT(*)
           FROM host_installations
          WHERE integration_id = ?1",
        [integration_id],
        |row| row.get(0),
    )?;
    if membership_count != 0 || installation_count != 0 {
        tx.commit()?;
        return Ok(false);
    }

    tx.execute(
        "UPDATE agent_integrations
            SET default_project_id = NULL,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
          WHERE integration_id = ?1",
        [integration_id],
    )?;
    let changed = tx.execute(
        "DELETE FROM agent_integrations WHERE integration_id = ?1",
        [integration_id],
    )?;
    tx.commit()?;
    Ok(changed > 0)
}

/// Adds a registered project to an integration allowlist.
pub fn add_integration_project(
    runtime_home: impl AsRef<Path>,
    registration: IntegrationProjectRegistration,
) -> StoreResult<IntegrationProjectRecord> {
    validate_integration_project_registration(&registration)?;
    let runtime_home = runtime_home.as_ref().to_path_buf();
    let registry_path = registry_db_path(&runtime_home);
    let mut conn = open_registry_database(&registry_path)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    require_runtime_home(&tx, &registry_path)?;
    require_agent_integration(&tx, &registration.integration_id)?;
    require_current_project_registration(&tx, &runtime_home, &registration.project_id)?;
    tx.execute(
        "INSERT OR IGNORE INTO integration_projects (
            integration_id,
            project_id,
            created_at
        )
        VALUES (
            ?1,
            ?2,
            strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
        )",
        params![registration.integration_id, registration.project_id],
    )?;
    tx.commit()?;

    integration_project_record_from_conn(
        &conn,
        &runtime_home,
        &registration.integration_id,
        &registration.project_id,
    )?
    .ok_or_else(|| StoreError::NotFound {
        entity: "integration_project",
        id: format!(
            "{}/{}",
            registration.integration_id, registration.project_id
        ),
    })
}

/// Removes one project from an integration allowlist unless it is the current default.
pub fn remove_integration_project(
    runtime_home: impl AsRef<Path>,
    integration_id: &str,
    project_id: &str,
) -> StoreResult<bool> {
    validate_identifier("integration_id", integration_id)?;
    validate_project_id(project_id)?;
    let registry_path = registry_db_path(runtime_home);
    let mut conn = open_registry_database(&registry_path)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    let integration = require_agent_integration(&tx, integration_id)?;
    if integration.default_project_id.as_deref() == Some(project_id) {
        return Err(conflict(
            "integration_project",
            &format!("{integration_id}/{project_id}"),
            "project is the integration default and must be cleared or changed first",
        ));
    }
    let changed = tx.execute(
        "DELETE FROM integration_projects
          WHERE integration_id = ?1
            AND project_id = ?2",
        params![integration_id, project_id],
    )?;
    tx.commit()?;
    Ok(changed > 0)
}

/// Lists the explicitly allowed projects for one integration.
pub fn list_integration_projects(
    runtime_home: impl AsRef<Path>,
    integration_id: &str,
) -> StoreResult<Vec<IntegrationProjectRecord>> {
    validate_identifier("integration_id", integration_id)?;
    let runtime_home = runtime_home.as_ref().to_path_buf();
    let registry_path = registry_db_path(&runtime_home);
    if !registry_path.exists() {
        return Err(StoreError::NotFound {
            entity: "agent_integration",
            id: integration_id.to_owned(),
        });
    }

    let conn = open_registry_database(registry_path)?;
    require_agent_integration(&conn, integration_id)?;
    let mut stmt = conn.prepare(
        "SELECT
            ip.integration_id,
            ip.project_id,
            ip.created_at,
            CASE WHEN ai.default_project_id = ip.project_id THEN 1 ELSE 0 END AS is_default,
            p.runtime_home_id,
            p.repo_root,
            p.project_home,
            p.state_db_path,
            p.status,
            p.metadata_json
         FROM integration_projects AS ip
         JOIN agent_integrations AS ai
           ON ai.integration_id = ip.integration_id
         JOIN projects AS p
           ON p.project_id = ip.project_id
        WHERE ip.integration_id = ?1
        ORDER BY ip.project_id",
    )?;
    let mut rows = stmt.query([integration_id])?;
    let mut projects = Vec::new();
    while let Some(row) = rows.next()? {
        let project = integration_project_record_from_row(row)?;
        projects.push(validate_integration_project_record(&runtime_home, project)?);
    }
    Ok(projects)
}

/// Returns current access facts for an integration/project pair.
pub fn agent_integration_project_access(
    runtime_home: impl AsRef<Path>,
    integration_id: &str,
    project_id: &str,
) -> StoreResult<Option<AgentIntegrationProjectAccess>> {
    validate_identifier("integration_id", integration_id)?;
    validate_project_id(project_id)?;
    let runtime_home = runtime_home.as_ref().to_path_buf();
    let registry_path = registry_db_path(&runtime_home);
    if !registry_path.exists() {
        return Ok(None);
    }

    let conn = open_registry_database(registry_path)?;
    let access = conn
        .query_row(
            "SELECT
            ai.enabled,
            CASE WHEN ip.project_id IS NULL THEN 0 ELSE 1 END AS project_allowed,
            CASE WHEN ai.default_project_id = ?2 THEN 1 ELSE 0 END AS is_default,
            p.project_id,
            p.runtime_home_id,
            p.repo_root,
            p.project_home,
            p.state_db_path,
            p.status,
            p.metadata_json
         FROM agent_integrations AS ai
         LEFT JOIN integration_projects AS ip
           ON ip.integration_id = ai.integration_id
          AND ip.project_id = ?2
         LEFT JOIN projects AS p
           ON p.project_id = ?2
        WHERE ai.integration_id = ?1",
            params![integration_id, project_id],
            |row| {
                let project_id_value = row.get::<_, Option<String>>(3)?;
                let project = if let Some(project_id_value) = project_id_value {
                    Some(ProjectRecord {
                        project_id: project_id_value,
                        runtime_home_id: row.get(4)?,
                        repo_root: row.get::<_, String>(5)?.into(),
                        project_home: row.get::<_, String>(6)?.into(),
                        state_db_path: row.get::<_, String>(7)?.into(),
                        status: row.get(8)?,
                        metadata_json: row.get(9)?,
                    })
                } else {
                    None
                };
                Ok(AgentIntegrationProjectAccess {
                    integration_id: integration_id.to_owned(),
                    project_id: project_id.to_owned(),
                    integration_enabled: row.get::<_, i64>(0)? == 1,
                    project_allowed: row.get::<_, i64>(1)? == 1,
                    is_default: row.get::<_, i64>(2)? == 1,
                    project,
                })
            },
        )
        .optional()
        .map_err(StoreError::from)?;
    access
        .map(|mut access| {
            if let Some(project) = access.project.take() {
                access.project = Some(validate_current_project_registration(
                    &runtime_home,
                    &project,
                )?);
            }
            Ok(access)
        })
        .transpose()
}

/// Returns whether the integration is currently enabled and the project is allowlisted.
pub fn is_agent_integration_project_allowed(
    runtime_home: impl AsRef<Path>,
    integration_id: &str,
    project_id: &str,
) -> StoreResult<bool> {
    Ok(
        agent_integration_project_access(runtime_home, integration_id, project_id)?
            .is_some_and(|access| access.integration_enabled && access.project_allowed),
    )
}

/// Sets a default project after verifying it is already allowlisted.
pub fn set_agent_integration_default_project(
    runtime_home: impl AsRef<Path>,
    integration_id: &str,
    project_id: &str,
) -> StoreResult<AgentIntegrationRecord> {
    validate_identifier("integration_id", integration_id)?;
    validate_project_id(project_id)?;
    let runtime_home = runtime_home.as_ref().to_path_buf();
    let registry_path = registry_db_path(&runtime_home);
    let mut conn = open_registry_database(&registry_path)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    require_runtime_home(&tx, &registry_path)?;
    require_agent_integration(&tx, integration_id)?;
    require_current_project_registration(&tx, &runtime_home, project_id)?;
    if !integration_project_exists(&tx, integration_id, project_id)? {
        return Err(StoreError::InvalidInput {
            detail: "default project must be an allowed integration project".to_owned(),
        });
    }
    tx.execute(
        "UPDATE agent_integrations
            SET default_project_id = ?2,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
          WHERE integration_id = ?1",
        params![integration_id, project_id],
    )?;
    tx.commit()?;

    agent_integration_record_from_conn(&conn, integration_id)?.ok_or_else(|| StoreError::NotFound {
        entity: "agent_integration",
        id: integration_id.to_owned(),
    })
}

/// Clears the default project for an integration.
pub fn clear_agent_integration_default_project(
    runtime_home: impl AsRef<Path>,
    integration_id: &str,
) -> StoreResult<AgentIntegrationRecord> {
    validate_identifier("integration_id", integration_id)?;
    let registry_path = registry_db_path(runtime_home);
    let mut conn = open_registry_database(&registry_path)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    require_runtime_home(&tx, &registry_path)?;
    let changed = tx.execute(
        "UPDATE agent_integrations
            SET default_project_id = NULL,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
          WHERE integration_id = ?1",
        [integration_id],
    )?;
    if changed == 0 {
        return Err(StoreError::NotFound {
            entity: "agent_integration",
            id: integration_id.to_owned(),
        });
    }
    tx.commit()?;

    agent_integration_record_from_conn(&conn, integration_id)?.ok_or_else(|| StoreError::NotFound {
        entity: "agent_integration",
        id: integration_id.to_owned(),
    })
}

/// Registers or compatibly updates one Host Installation inventory record.
pub fn register_host_installation(
    runtime_home: impl AsRef<Path>,
    registration: HostInstallationRegistration,
) -> StoreResult<HostInstallationRecord> {
    validate_host_installation_registration(&registration)?;

    let registry_path = registry_db_path(runtime_home);
    let mut conn = open_registry_database(&registry_path)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    require_runtime_home(&tx, &registry_path)?;
    require_agent_integration(&tx, &registration.integration_id)?;

    if let Some(existing_target_id) = host_installation_id_for_target(&tx, &registration)? {
        if existing_target_id != registration.installation_id {
            return Err(conflict(
                "host_installation",
                &registration.installation_id,
                "host target is already managed by another installation_id",
            ));
        }
    }

    if let Some(existing) = host_installation_record_from_conn(&tx, &registration.installation_id)?
    {
        if !host_installation_registration_is_compatible(&existing, &registration) {
            return Err(conflict(
                "host_installation",
                &registration.installation_id,
                "installation_id is already bound to a different managed host target",
            ));
        }
        tx.execute(
            "UPDATE host_installations
                SET managed_fingerprint = ?2,
                    last_verified_status = ?3,
                    updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                    metadata_json = ?4
              WHERE installation_id = ?1",
            params![
                registration.installation_id,
                registration.managed_fingerprint,
                registration.last_verified_status,
                registration.metadata_json
            ],
        )?;
    } else {
        tx.execute(
            "INSERT INTO host_installations (
                installation_id,
                integration_id,
                host_kind,
                host_scope,
                server_name,
                config_target,
                managed_fingerprint,
                last_verified_status,
                created_at,
                updated_at,
                metadata_json
            )
            VALUES (
                ?1,
                ?2,
                ?3,
                ?4,
                ?5,
                ?6,
                ?7,
                ?8,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                ?9
            )",
            params![
                registration.installation_id,
                registration.integration_id,
                registration.host_kind,
                registration.host_scope,
                registration.server_name,
                registration.config_target,
                registration.managed_fingerprint,
                registration.last_verified_status,
                registration.metadata_json
            ],
        )?;
    }
    tx.commit()?;

    host_installation_record_from_conn(&conn, &registration.installation_id)?.ok_or_else(|| {
        StoreError::NotFound {
            entity: "host_installation",
            id: registration.installation_id,
        }
    })
}

/// Reads one Host Installation inventory record.
pub fn host_installation_record(
    runtime_home: impl AsRef<Path>,
    installation_id: &str,
) -> StoreResult<Option<HostInstallationRecord>> {
    validate_identifier("installation_id", installation_id)?;
    let registry_path = registry_db_path(runtime_home);
    if !registry_path.exists() {
        return Ok(None);
    }

    let conn = open_registry_database(registry_path)?;
    host_installation_record_from_conn(&conn, installation_id)
}

/// Lists Host Installation inventory records in deterministic order.
pub fn list_host_installations(
    runtime_home: impl AsRef<Path>,
) -> StoreResult<Vec<HostInstallationRecord>> {
    let registry_path = registry_db_path(runtime_home);
    if !registry_path.exists() {
        return Ok(Vec::new());
    }

    let conn = open_registry_database(registry_path)?;
    list_host_installations_from_conn(&conn, None)
}

/// Lists Host Installation inventory records for one integration.
pub fn list_host_installations_for_integration(
    runtime_home: impl AsRef<Path>,
    integration_id: &str,
) -> StoreResult<Vec<HostInstallationRecord>> {
    validate_identifier("integration_id", integration_id)?;
    let registry_path = registry_db_path(runtime_home);
    if !registry_path.exists() {
        return Err(StoreError::NotFound {
            entity: "agent_integration",
            id: integration_id.to_owned(),
        });
    }

    let conn = open_registry_database(registry_path)?;
    require_agent_integration(&conn, integration_id)?;
    list_host_installations_from_conn(&conn, Some(integration_id))
}

/// Updates last-known Host Installation verification state and fingerprint.
pub fn update_host_installation_verification(
    runtime_home: impl AsRef<Path>,
    installation_id: &str,
    last_verified_status: &str,
    managed_fingerprint: &str,
) -> StoreResult<HostInstallationRecord> {
    validate_identifier("installation_id", installation_id)?;
    validate_verification_status(last_verified_status)?;
    validate_nonempty("managed_fingerprint", managed_fingerprint)?;
    let registry_path = registry_db_path(runtime_home);
    let mut conn = open_registry_database(&registry_path)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    require_runtime_home(&tx, &registry_path)?;
    let changed = tx.execute(
        "UPDATE host_installations
            SET managed_fingerprint = ?2,
                last_verified_status = ?3,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
          WHERE installation_id = ?1",
        params![installation_id, managed_fingerprint, last_verified_status],
    )?;
    if changed == 0 {
        return Err(StoreError::NotFound {
            entity: "host_installation",
            id: installation_id.to_owned(),
        });
    }
    tx.commit()?;

    host_installation_record_from_conn(&conn, installation_id)?.ok_or_else(|| {
        StoreError::NotFound {
            entity: "host_installation",
            id: installation_id.to_owned(),
        }
    })
}

/// Removes one Host Installation inventory record.
pub fn remove_host_installation(
    runtime_home: impl AsRef<Path>,
    installation_id: &str,
) -> StoreResult<bool> {
    validate_identifier("installation_id", installation_id)?;
    let registry_path = registry_db_path(runtime_home);
    let mut conn = open_registry_database(&registry_path)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    require_runtime_home(&tx, &registry_path)?;
    let changed = tx.execute(
        "DELETE FROM host_installations WHERE installation_id = ?1",
        [installation_id],
    )?;
    tx.commit()?;
    Ok(changed > 0)
}

fn validate_agent_integration_registration(
    registration: &AgentIntegrationRegistration,
) -> StoreResult<()> {
    validate_identifier("integration_id", &registration.integration_id)?;
    validate_interaction_role(&registration.interaction_role)?;
    validate_identifier("surface_id", &registration.surface_id)?;
    validate_identifier("surface_instance_id", &registration.surface_instance_id)?;
    validate_json_object(
        "agent_integrations.metadata_json",
        &registration.metadata_json,
    )
}

fn validate_integration_project_registration(
    registration: &IntegrationProjectRegistration,
) -> StoreResult<()> {
    validate_identifier("integration_id", &registration.integration_id)?;
    validate_project_id(&registration.project_id)
}

fn validate_host_installation_registration(
    registration: &HostInstallationRegistration,
) -> StoreResult<()> {
    validate_identifier("installation_id", &registration.installation_id)?;
    validate_identifier("integration_id", &registration.integration_id)?;
    validate_host_kind_scope(&registration.host_kind, &registration.host_scope)?;
    validate_nonempty("server_name", &registration.server_name)?;
    validate_nonempty("config_target", &registration.config_target)?;
    validate_nonempty("managed_fingerprint", &registration.managed_fingerprint)?;
    validate_verification_status(&registration.last_verified_status)?;
    validate_json_object(
        "host_installations.metadata_json",
        &registration.metadata_json,
    )
}

fn validate_identifier(field: &'static str, value: &str) -> StoreResult<()> {
    validate_nonempty(field, value)?;
    if value.contains('\0') {
        Err(StoreError::InvalidInput {
            detail: format!("{field} must not contain NUL bytes"),
        })
    } else {
        Ok(())
    }
}

fn validate_nonempty(field: &'static str, value: &str) -> StoreResult<()> {
    if value.trim().is_empty() {
        Err(StoreError::InvalidInput {
            detail: format!("{field} must not be empty"),
        })
    } else {
        Ok(())
    }
}

fn validate_interaction_role(role: &str) -> StoreResult<()> {
    if role == AGENT_INTERACTION_ROLE {
        Ok(())
    } else {
        Err(StoreError::InvalidInput {
            detail: format!("interaction_role must be {AGENT_INTERACTION_ROLE}"),
        })
    }
}

fn validate_host_kind_scope(host_kind: &str, host_scope: &str) -> StoreResult<()> {
    let valid = matches!(
        (host_kind, host_scope),
        (HOST_KIND_CODEX, HOST_SCOPE_USER)
            | (HOST_KIND_CODEX, HOST_SCOPE_PROJECT)
            | (HOST_KIND_CLAUDE_CODE, HOST_SCOPE_LOCAL)
            | (HOST_KIND_CLAUDE_CODE, HOST_SCOPE_PROJECT)
            | (HOST_KIND_CLAUDE_CODE, HOST_SCOPE_USER)
            | (HOST_KIND_GENERIC, HOST_SCOPE_EXPORT)
    );
    if valid {
        Ok(())
    } else {
        Err(StoreError::InvalidInput {
            detail: "host_kind and host_scope must match the supported matrix".to_owned(),
        })
    }
}

fn validate_verification_status(status: &str) -> StoreResult<()> {
    if matches!(
        status,
        VERIFIED_STATUS_NOT_VERIFIED
            | VERIFIED_STATUS_COMPLETE
            | VERIFIED_STATUS_ACTION_REQUIRED
            | VERIFIED_STATUS_PARTIAL_FAILURE
            | VERIFIED_STATUS_FAILED
    ) {
        Ok(())
    } else {
        Err(StoreError::InvalidInput {
            detail: "last_verified_status is not supported".to_owned(),
        })
    }
}

fn validate_json_object(field: &'static str, text: &str) -> StoreResult<()> {
    let value = serde_json::from_str::<Value>(text).map_err(|error| StoreError::InvalidInput {
        detail: format!("{field} must be JSON object text: {error}"),
    })?;
    if value.is_object() {
        Ok(())
    } else {
        Err(StoreError::InvalidInput {
            detail: format!("{field} must be a JSON object"),
        })
    }
}

fn enabled_as_i64(enabled: bool) -> i64 {
    if enabled {
        1
    } else {
        0
    }
}

fn require_runtime_home(conn: &Connection, registry_path: &Path) -> StoreResult<()> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM runtime_home", [], |row| row.get(0))?;
    if count == 1 {
        Ok(())
    } else {
        Err(StoreError::NotFound {
            entity: "runtime_home",
            id: registry_path.display().to_string(),
        })
    }
}

fn require_agent_integration(
    conn: &Connection,
    integration_id: &str,
) -> StoreResult<AgentIntegrationRecord> {
    agent_integration_record_from_conn(conn, integration_id)?.ok_or_else(|| StoreError::NotFound {
        entity: "agent_integration",
        id: integration_id.to_owned(),
    })
}

fn require_current_project_registration(
    conn: &Connection,
    runtime_home: &Path,
    project_id: &str,
) -> StoreResult<ProjectRecord> {
    project_record_from_conn(conn, runtime_home, project_id)?.ok_or_else(|| StoreError::NotFound {
        entity: "project",
        id: project_id.to_owned(),
    })
}

fn integration_project_exists(
    conn: &Connection,
    integration_id: &str,
    project_id: &str,
) -> StoreResult<bool> {
    conn.query_row(
        "SELECT COUNT(*)
           FROM integration_projects
          WHERE integration_id = ?1
            AND project_id = ?2",
        params![integration_id, project_id],
        |row| Ok(row.get::<_, i64>(0)? == 1),
    )
    .map_err(StoreError::from)
}

fn agent_integration_record_from_conn(
    conn: &Connection,
    integration_id: &str,
) -> StoreResult<Option<AgentIntegrationRecord>> {
    conn.query_row(
        "SELECT
            integration_id,
            interaction_role,
            surface_id,
            surface_instance_id,
            default_project_id,
            enabled,
            created_at,
            updated_at,
            metadata_json
         FROM agent_integrations
         WHERE integration_id = ?1",
        [integration_id],
        agent_integration_record_from_row,
    )
    .optional()
    .map_err(StoreError::from)
}

fn agent_integration_record_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<AgentIntegrationRecord> {
    Ok(AgentIntegrationRecord {
        integration_id: row.get(0)?,
        interaction_role: row.get(1)?,
        surface_id: row.get(2)?,
        surface_instance_id: row.get(3)?,
        default_project_id: row.get(4)?,
        enabled: row.get::<_, i64>(5)? == 1,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
        metadata_json: row.get(8)?,
    })
}

fn integration_project_record_from_conn(
    conn: &Connection,
    runtime_home: &Path,
    integration_id: &str,
    project_id: &str,
) -> StoreResult<Option<IntegrationProjectRecord>> {
    let record = conn
        .query_row(
            "SELECT
            ip.integration_id,
            ip.project_id,
            ip.created_at,
            CASE WHEN ai.default_project_id = ip.project_id THEN 1 ELSE 0 END AS is_default,
            p.runtime_home_id,
            p.repo_root,
            p.project_home,
            p.state_db_path,
            p.status,
            p.metadata_json
         FROM integration_projects AS ip
         JOIN agent_integrations AS ai
           ON ai.integration_id = ip.integration_id
         JOIN projects AS p
           ON p.project_id = ip.project_id
        WHERE ip.integration_id = ?1
          AND ip.project_id = ?2",
            params![integration_id, project_id],
            integration_project_record_from_row,
        )
        .optional()
        .map_err(StoreError::from)?;
    record
        .map(|record| validate_integration_project_record(runtime_home, record))
        .transpose()
}

fn project_record_from_conn(
    conn: &Connection,
    runtime_home: &Path,
    project_id: &str,
) -> StoreResult<Option<ProjectRecord>> {
    let project = conn
        .query_row(
            "SELECT
                project_id,
                runtime_home_id,
                repo_root,
                project_home,
                state_db_path,
                status,
                metadata_json
             FROM projects
             WHERE project_id = ?1",
            [project_id],
            |row| {
                Ok(ProjectRecord {
                    project_id: row.get(0)?,
                    runtime_home_id: row.get(1)?,
                    repo_root: PathBuf::from(row.get::<_, String>(2)?),
                    project_home: PathBuf::from(row.get::<_, String>(3)?),
                    state_db_path: PathBuf::from(row.get::<_, String>(4)?),
                    status: row.get(5)?,
                    metadata_json: row.get(6)?,
                })
            },
        )
        .optional()
        .map_err(StoreError::from)?;
    project
        .map(|project| validate_current_project_registration(runtime_home, &project))
        .transpose()
}

fn validate_integration_project_record(
    runtime_home: &Path,
    mut record: IntegrationProjectRecord,
) -> StoreResult<IntegrationProjectRecord> {
    record.project = validate_current_project_registration(runtime_home, &record.project)?;
    Ok(record)
}

fn integration_project_record_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<IntegrationProjectRecord> {
    let project_id: String = row.get(1)?;
    Ok(IntegrationProjectRecord {
        integration_id: row.get(0)?,
        project_id: project_id.clone(),
        created_at: row.get(2)?,
        is_default: row.get::<_, i64>(3)? == 1,
        project: ProjectRecord {
            project_id,
            runtime_home_id: row.get(4)?,
            repo_root: row.get::<_, String>(5)?.into(),
            project_home: row.get::<_, String>(6)?.into(),
            state_db_path: row.get::<_, String>(7)?.into(),
            status: row.get(8)?,
            metadata_json: row.get(9)?,
        },
    })
}

fn host_installation_record_from_conn(
    conn: &Connection,
    installation_id: &str,
) -> StoreResult<Option<HostInstallationRecord>> {
    conn.query_row(
        "SELECT
            installation_id,
            integration_id,
            host_kind,
            host_scope,
            server_name,
            config_target,
            managed_fingerprint,
            last_verified_status,
            created_at,
            updated_at,
            metadata_json
         FROM host_installations
         WHERE installation_id = ?1",
        [installation_id],
        host_installation_record_from_row,
    )
    .optional()
    .map_err(StoreError::from)
}

fn host_installation_record_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<HostInstallationRecord> {
    Ok(HostInstallationRecord {
        installation_id: row.get(0)?,
        integration_id: row.get(1)?,
        host_kind: row.get(2)?,
        host_scope: row.get(3)?,
        server_name: row.get(4)?,
        config_target: row.get(5)?,
        managed_fingerprint: row.get(6)?,
        last_verified_status: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
        metadata_json: row.get(10)?,
    })
}

fn list_host_installations_from_conn(
    conn: &Connection,
    integration_id: Option<&str>,
) -> StoreResult<Vec<HostInstallationRecord>> {
    let (sql, params): (&str, Vec<&str>) = if let Some(integration_id) = integration_id {
        (
            "SELECT
                installation_id,
                integration_id,
                host_kind,
                host_scope,
                server_name,
                config_target,
                managed_fingerprint,
                last_verified_status,
                created_at,
                updated_at,
                metadata_json
             FROM host_installations
             WHERE integration_id = ?1
             ORDER BY host_kind, host_scope, config_target, server_name, installation_id",
            vec![integration_id],
        )
    } else {
        (
            "SELECT
                installation_id,
                integration_id,
                host_kind,
                host_scope,
                server_name,
                config_target,
                managed_fingerprint,
                last_verified_status,
                created_at,
                updated_at,
                metadata_json
             FROM host_installations
             ORDER BY host_kind, host_scope, config_target, server_name, installation_id",
            Vec::new(),
        )
    };

    let mut stmt = conn.prepare(sql)?;
    let mut rows = stmt.query(rusqlite::params_from_iter(params))?;
    let mut installations = Vec::new();
    while let Some(row) = rows.next()? {
        installations.push(host_installation_record_from_row(row)?);
    }
    Ok(installations)
}

fn host_installation_id_for_target(
    conn: &Connection,
    registration: &HostInstallationRegistration,
) -> StoreResult<Option<String>> {
    conn.query_row(
        "SELECT installation_id
           FROM host_installations
          WHERE host_kind = ?1
            AND host_scope = ?2
            AND config_target = ?3
            AND server_name = ?4",
        params![
            registration.host_kind,
            registration.host_scope,
            registration.config_target,
            registration.server_name
        ],
        |row| row.get(0),
    )
    .optional()
    .map_err(StoreError::from)
}

fn integration_registration_is_compatible(
    existing: &AgentIntegrationRecord,
    registration: &AgentIntegrationRegistration,
) -> bool {
    existing.interaction_role == registration.interaction_role
        && existing.surface_id == registration.surface_id
        && existing.surface_instance_id == registration.surface_instance_id
        && existing.metadata_json == registration.metadata_json
}

fn host_installation_registration_is_compatible(
    existing: &HostInstallationRecord,
    registration: &HostInstallationRegistration,
) -> bool {
    existing.integration_id == registration.integration_id
        && existing.host_kind == registration.host_kind
        && existing.host_scope == registration.host_scope
        && existing.server_name == registration.server_name
        && existing.config_target == registration.config_target
}

fn conflict(entity: &'static str, id: &str, detail: impl Into<String>) -> StoreError {
    StoreError::Conflict {
        entity,
        id: id.to_owned(),
        detail: detail.into(),
    }
}

#[cfg(test)]
mod tests {
    use std::{error::Error, path::Path};

    use rusqlite::{params, Connection, ErrorCode};
    use volicord_test_support::TempRuntimeHome;

    use super::*;
    use crate::{
        bootstrap::{
            initialize_runtime_home, register_project, ProjectRegistration, ACTIVE_PROJECT_STATUS,
        },
        sqlite::{open_registry_database, registry_db_path},
    };

    #[test]
    fn integration_registration_lookup_and_repeated_compatible_registration(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = IntegrationFixture::new("agent-registration")?;

        let first =
            register_agent_integration(fixture.runtime_home.path(), integration("agent_a"))?;
        let second =
            register_agent_integration(fixture.runtime_home.path(), integration("agent_a"))?;
        let read = agent_integration_record(fixture.runtime_home.path(), "agent_a")?
            .expect("integration should be readable");
        let listed = list_agent_integrations(fixture.runtime_home.path())?;

        assert_eq!(first, second);
        assert_eq!(read.integration_id, "agent_a");
        assert_eq!(read.interaction_role, AGENT_INTERACTION_ROLE);
        assert!(read.enabled);
        assert!(read.default_project_id.is_none());
        assert_eq!(listed, vec![read]);
        Ok(())
    }

    #[test]
    fn incompatible_integration_identity_conflicts() -> Result<(), Box<dyn Error>> {
        let fixture = IntegrationFixture::new("agent-conflict")?;
        register_agent_integration(fixture.runtime_home.path(), integration("agent_conflict"))?;

        let error = register_agent_integration(
            fixture.runtime_home.path(),
            AgentIntegrationRegistration {
                surface_instance_id: "other_instance".to_owned(),
                ..integration("agent_conflict")
            },
        )
        .expect_err("same integration_id with different surface binding should conflict");

        assert!(matches!(error, StoreError::Conflict { .. }));
        Ok(())
    }

    #[test]
    fn integration_registration_does_not_grant_all_projects() -> Result<(), Box<dyn Error>> {
        let fixture = IntegrationFixture::new("agent-no-auto-grant")?;
        register_agent_integration(fixture.runtime_home.path(), integration("agent_limited"))?;

        assert!(
            list_integration_projects(fixture.runtime_home.path(), "agent_limited")?.is_empty()
        );
        assert!(!is_agent_integration_project_allowed(
            fixture.runtime_home.path(),
            "agent_limited",
            "project_a"
        )?);
        Ok(())
    }

    #[test]
    fn explicit_project_membership_and_access_lookup() -> Result<(), Box<dyn Error>> {
        let fixture = IntegrationFixture::new("agent-membership")?;
        register_agent_integration(fixture.runtime_home.path(), integration("agent_member"))?;

        let member = add_integration_project(
            fixture.runtime_home.path(),
            IntegrationProjectRegistration {
                integration_id: "agent_member".to_owned(),
                project_id: "project_a".to_owned(),
            },
        )?;
        let repeated = add_integration_project(
            fixture.runtime_home.path(),
            IntegrationProjectRegistration {
                integration_id: "agent_member".to_owned(),
                project_id: "project_a".to_owned(),
            },
        )?;
        let listed = list_integration_projects(fixture.runtime_home.path(), "agent_member")?;
        let access = agent_integration_project_access(
            fixture.runtime_home.path(),
            "agent_member",
            "project_a",
        )?
        .expect("integration should exist");

        assert_eq!(member, repeated);
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].project.status, ACTIVE_PROJECT_STATUS);
        assert!(access.integration_enabled);
        assert!(access.project_allowed);
        assert!(access.project.is_some());
        assert!(is_agent_integration_project_allowed(
            fixture.runtime_home.path(),
            "agent_member",
            "project_a"
        )?);
        Ok(())
    }

    #[test]
    fn default_project_must_be_allowed_and_blocks_removal() -> Result<(), Box<dyn Error>> {
        let fixture = IntegrationFixture::new("agent-default")?;
        register_agent_integration(fixture.runtime_home.path(), integration("agent_default"))?;

        let error = set_agent_integration_default_project(
            fixture.runtime_home.path(),
            "agent_default",
            "project_a",
        )
        .expect_err("default project must be an allowed project first");
        assert!(matches!(error, StoreError::InvalidInput { .. }));

        add_integration_project(
            fixture.runtime_home.path(),
            IntegrationProjectRegistration {
                integration_id: "agent_default".to_owned(),
                project_id: "project_a".to_owned(),
            },
        )?;
        let defaulted = set_agent_integration_default_project(
            fixture.runtime_home.path(),
            "agent_default",
            "project_a",
        )?;
        assert_eq!(defaulted.default_project_id.as_deref(), Some("project_a"));

        let remove_error =
            remove_integration_project(fixture.runtime_home.path(), "agent_default", "project_a")
                .expect_err("current default project removal should fail");
        assert!(matches!(remove_error, StoreError::Conflict { .. }));

        let cleared =
            clear_agent_integration_default_project(fixture.runtime_home.path(), "agent_default")?;
        assert!(cleared.default_project_id.is_none());
        assert!(remove_integration_project(
            fixture.runtime_home.path(),
            "agent_default",
            "project_a",
        )?);
        Ok(())
    }

    #[test]
    fn enabling_and_disabling_affect_access_lookup() -> Result<(), Box<dyn Error>> {
        let fixture = IntegrationFixture::new("agent-enabled")?;
        register_agent_integration(fixture.runtime_home.path(), integration("agent_enabled"))?;
        add_integration_project(
            fixture.runtime_home.path(),
            IntegrationProjectRegistration {
                integration_id: "agent_enabled".to_owned(),
                project_id: "project_a".to_owned(),
            },
        )?;

        let disabled =
            set_agent_integration_enabled(fixture.runtime_home.path(), "agent_enabled", false)?;
        assert!(!disabled.enabled);
        let access = agent_integration_project_access(
            fixture.runtime_home.path(),
            "agent_enabled",
            "project_a",
        )?
        .expect("integration should exist");
        assert!(!access.integration_enabled);
        assert!(access.project_allowed);
        assert!(!is_agent_integration_project_allowed(
            fixture.runtime_home.path(),
            "agent_enabled",
            "project_a"
        )?);

        let enabled =
            set_agent_integration_enabled(fixture.runtime_home.path(), "agent_enabled", true)?;
        assert!(enabled.enabled);
        assert!(is_agent_integration_project_allowed(
            fixture.runtime_home.path(),
            "agent_enabled",
            "project_a"
        )?);
        Ok(())
    }

    #[test]
    fn inactive_project_status_remains_visible_in_access_queries() -> Result<(), Box<dyn Error>> {
        let fixture = IntegrationFixture::new("agent-inactive-visible")?;
        register_agent_integration(fixture.runtime_home.path(), integration("agent_stale"))?;
        add_integration_project(
            fixture.runtime_home.path(),
            IntegrationProjectRegistration {
                integration_id: "agent_stale".to_owned(),
                project_id: "project_a".to_owned(),
            },
        )?;
        set_project_status(fixture.runtime_home.path(), "project_a", "inactive")?;

        let listed = list_integration_projects(fixture.runtime_home.path(), "agent_stale")?;
        let access = agent_integration_project_access(
            fixture.runtime_home.path(),
            "agent_stale",
            "project_a",
        )?
        .expect("integration should exist");

        assert_eq!(listed[0].project.status, "inactive");
        assert_eq!(
            access.project.expect("project should be visible").status,
            "inactive"
        );
        assert!(access.project_allowed);
        Ok(())
    }

    #[test]
    fn integration_project_access_rejects_invalid_project_registration(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = IntegrationFixture::new("agent-invalid-project")?;
        register_agent_integration(fixture.runtime_home.path(), integration("agent_invalid"))?;
        add_integration_project(
            fixture.runtime_home.path(),
            IntegrationProjectRegistration {
                integration_id: "agent_invalid".to_owned(),
                project_id: "project_a".to_owned(),
            },
        )?;
        replace_project_repo_root(
            fixture.runtime_home.path(),
            "project_a",
            fixture.runtime_home.path(),
        )?;

        let list_error = list_integration_projects(fixture.runtime_home.path(), "agent_invalid")
            .expect_err("invalid joined project should reject integration listing");
        assert_invalid_project_registration(list_error, "same_path");
        let access_error = agent_integration_project_access(
            fixture.runtime_home.path(),
            "agent_invalid",
            "project_a",
        )
        .expect_err("invalid joined project should reject access lookup");
        assert_invalid_project_registration(access_error, "same_path");
        let allowed_error = is_agent_integration_project_allowed(
            fixture.runtime_home.path(),
            "agent_invalid",
            "project_a",
        )
        .expect_err("invalid joined project should reject allow check");
        assert_invalid_project_registration(allowed_error, "same_path");
        Ok(())
    }

    #[test]
    fn adding_invalid_integration_project_leaves_membership_unchanged() -> Result<(), Box<dyn Error>>
    {
        let fixture = IntegrationFixture::new("agent-invalid-add")?;
        register_agent_integration(fixture.runtime_home.path(), integration("agent_add"))?;
        let alternate_state_path = fixture
            .runtime_home
            .path()
            .join("alternate/project-b-state.sqlite");
        replace_project_state_db_path(
            fixture.runtime_home.path(),
            "project_b",
            &alternate_state_path,
        )?;

        let error = add_integration_project(
            fixture.runtime_home.path(),
            IntegrationProjectRegistration {
                integration_id: "agent_add".to_owned(),
                project_id: "project_b".to_owned(),
            },
        )
        .expect_err("invalid project registration should not be added to integration");

        assert_invalid_project_registration(error, "state_db_path_mismatch");
        assert!(!integration_project_exists_raw(
            fixture.runtime_home.path(),
            "agent_add",
            "project_b",
        )?);
        assert!(!alternate_state_path.exists());
        Ok(())
    }

    #[test]
    fn host_installation_registration_update_and_uniqueness() -> Result<(), Box<dyn Error>> {
        let fixture = IntegrationFixture::new("host-installation")?;
        register_agent_integration(fixture.runtime_home.path(), integration("agent_host"))?;

        let created = register_host_installation(
            fixture.runtime_home.path(),
            host_installation("install_a", "agent_host"),
        )?;
        let updated = register_host_installation(
            fixture.runtime_home.path(),
            HostInstallationRegistration {
                managed_fingerprint: "fingerprint_v2".to_owned(),
                last_verified_status: VERIFIED_STATUS_COMPLETE.to_owned(),
                metadata_json: "{\"updated\":true}".to_owned(),
                ..host_installation("install_a", "agent_host")
            },
        )?;
        let read = host_installation_record(fixture.runtime_home.path(), "install_a")?
            .expect("host installation should be readable");
        let listed =
            list_host_installations_for_integration(fixture.runtime_home.path(), "agent_host")?;

        assert_eq!(created.installation_id, "install_a");
        assert_eq!(updated.managed_fingerprint, "fingerprint_v2");
        assert_eq!(updated.last_verified_status, VERIFIED_STATUS_COMPLETE);
        assert_eq!(read, updated);
        assert_eq!(listed, vec![updated.clone()]);

        let conflict_error = register_host_installation(
            fixture.runtime_home.path(),
            HostInstallationRegistration {
                installation_id: "install_b".to_owned(),
                ..host_installation("install_a", "agent_host")
            },
        )
        .expect_err("same managed host target with different id should conflict");
        assert!(matches!(conflict_error, StoreError::Conflict { .. }));
        Ok(())
    }

    #[test]
    fn host_installation_verification_update_and_remove() -> Result<(), Box<dyn Error>> {
        let fixture = IntegrationFixture::new("host-verification")?;
        register_agent_integration(fixture.runtime_home.path(), integration("agent_verify"))?;
        register_host_installation(
            fixture.runtime_home.path(),
            host_installation("install_verify", "agent_verify"),
        )?;

        let updated = update_host_installation_verification(
            fixture.runtime_home.path(),
            "install_verify",
            VERIFIED_STATUS_ACTION_REQUIRED,
            "fingerprint_after_check",
        )?;
        assert_eq!(
            updated.last_verified_status,
            VERIFIED_STATUS_ACTION_REQUIRED
        );
        assert_eq!(updated.managed_fingerprint, "fingerprint_after_check");

        assert!(remove_host_installation(
            fixture.runtime_home.path(),
            "install_verify",
        )?);
        assert!(host_installation_record(fixture.runtime_home.path(), "install_verify")?.is_none());
        Ok(())
    }

    #[test]
    fn host_installation_rejects_invalid_scope_matrix() -> Result<(), Box<dyn Error>> {
        let fixture = IntegrationFixture::new("host-invalid-scope")?;
        register_agent_integration(
            fixture.runtime_home.path(),
            integration("agent_invalid_host"),
        )?;

        let error = register_host_installation(
            fixture.runtime_home.path(),
            HostInstallationRegistration {
                host_kind: HOST_KIND_CODEX.to_owned(),
                host_scope: HOST_SCOPE_LOCAL.to_owned(),
                ..host_installation("install_invalid", "agent_invalid_host")
            },
        )
        .expect_err("codex local scope is not in the baseline matrix");
        assert!(matches!(error, StoreError::InvalidInput { .. }));
        Ok(())
    }

    #[test]
    fn membership_and_host_rows_restrict_parent_deletion() -> Result<(), Box<dyn Error>> {
        let fixture = IntegrationFixture::new("agent-restrict-delete")?;
        register_agent_integration(fixture.runtime_home.path(), integration("agent_restrict"))?;
        add_integration_project(
            fixture.runtime_home.path(),
            IntegrationProjectRegistration {
                integration_id: "agent_restrict".to_owned(),
                project_id: "project_a".to_owned(),
            },
        )?;
        register_host_installation(
            fixture.runtime_home.path(),
            host_installation("install_restrict", "agent_restrict"),
        )?;

        let conn = open_registry_database(registry_db_path(fixture.runtime_home.path()))?;
        let project_error = conn
            .execute("DELETE FROM projects WHERE project_id = 'project_a'", [])
            .expect_err("membership should restrict project deletion");
        assert_constraint_error(project_error);
        let integration_error = conn
            .execute(
                "DELETE FROM agent_integrations WHERE integration_id = 'agent_restrict'",
                [],
            )
            .expect_err("membership or host inventory should restrict integration deletion");
        assert_constraint_error(integration_error);
        Ok(())
    }

    struct IntegrationFixture {
        runtime_home: TempRuntimeHome,
    }

    impl IntegrationFixture {
        fn new(prefix: &str) -> Result<Self, Box<dyn Error>> {
            let runtime_home = TempRuntimeHome::new(prefix)?;
            initialize_runtime_home(
                runtime_home.path(),
                &format!("runtime_home_{}", prefix.replace('-', "_")),
                "{}",
            )?;
            register_project(
                runtime_home.path(),
                ProjectRegistration {
                    project_id: "project_a".to_owned(),
                    repo_root: runtime_home.create_product_repo("repo-a")?,
                    project_home: None,
                    status: ACTIVE_PROJECT_STATUS.to_owned(),
                    metadata_json: "{}".to_owned(),
                },
            )?;
            register_project(
                runtime_home.path(),
                ProjectRegistration {
                    project_id: "project_b".to_owned(),
                    repo_root: runtime_home.create_product_repo("repo-b")?,
                    project_home: None,
                    status: ACTIVE_PROJECT_STATUS.to_owned(),
                    metadata_json: "{}".to_owned(),
                },
            )?;
            Ok(Self { runtime_home })
        }
    }

    fn integration(integration_id: &str) -> AgentIntegrationRegistration {
        AgentIntegrationRegistration {
            integration_id: integration_id.to_owned(),
            interaction_role: AGENT_INTERACTION_ROLE.to_owned(),
            surface_id: "surface_main".to_owned(),
            surface_instance_id: "surface_instance_main".to_owned(),
            metadata_json: "{}".to_owned(),
        }
    }

    fn host_installation(
        installation_id: &str,
        integration_id: &str,
    ) -> HostInstallationRegistration {
        HostInstallationRegistration {
            installation_id: installation_id.to_owned(),
            integration_id: integration_id.to_owned(),
            host_kind: HOST_KIND_CODEX.to_owned(),
            host_scope: HOST_SCOPE_USER.to_owned(),
            server_name: "harness".to_owned(),
            config_target: "/tmp/codex/config.toml".to_owned(),
            managed_fingerprint: "fingerprint_v1".to_owned(),
            last_verified_status: VERIFIED_STATUS_NOT_VERIFIED.to_owned(),
            metadata_json: "{}".to_owned(),
        }
    }

    fn set_project_status(
        runtime_home: &Path,
        project_id: &str,
        status: &str,
    ) -> Result<(), Box<dyn Error>> {
        let conn = Connection::open(registry_db_path(runtime_home))?;
        conn.pragma_update(None, "ignore_check_constraints", "ON")?;
        conn.execute(
            "UPDATE projects SET status = ?2 WHERE project_id = ?1",
            params![project_id, status],
        )?;
        Ok(())
    }

    fn replace_project_repo_root(
        runtime_home: &Path,
        project_id: &str,
        repo_root: &Path,
    ) -> Result<(), Box<dyn Error>> {
        let conn = Connection::open(registry_db_path(runtime_home))?;
        conn.execute(
            "UPDATE projects SET repo_root = ?2 WHERE project_id = ?1",
            params![project_id, repo_root.to_string_lossy().as_ref()],
        )?;
        Ok(())
    }

    fn replace_project_state_db_path(
        runtime_home: &Path,
        project_id: &str,
        state_db_path: &Path,
    ) -> Result<(), Box<dyn Error>> {
        let conn = Connection::open(registry_db_path(runtime_home))?;
        conn.execute(
            "UPDATE projects SET state_db_path = ?2 WHERE project_id = ?1",
            params![project_id, state_db_path.to_string_lossy().as_ref()],
        )?;
        Ok(())
    }

    fn integration_project_exists_raw(
        runtime_home: &Path,
        integration_id: &str,
        project_id: &str,
    ) -> Result<bool, Box<dyn Error>> {
        let conn = Connection::open(registry_db_path(runtime_home))?;
        Ok(conn.query_row(
            "SELECT COUNT(*)
               FROM integration_projects
              WHERE integration_id = ?1
                AND project_id = ?2",
            params![integration_id, project_id],
            |row| Ok(row.get::<_, i64>(0)? == 1),
        )?)
    }

    fn assert_invalid_project_registration(error: StoreError, relationship: &str) {
        match error {
            StoreError::InvalidProjectRegistration {
                relationship: actual,
                detail,
                ..
            } => {
                assert_eq!(actual, relationship);
                assert!(!detail.is_empty());
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    fn assert_constraint_error(error: rusqlite::Error) {
        match error {
            rusqlite::Error::SqliteFailure(sqlite_error, _) => {
                assert_eq!(sqlite_error.code, ErrorCode::ConstraintViolation);
            }
            other => panic!("expected SQLite constraint error, got {other:?}"),
        }
    }
}
