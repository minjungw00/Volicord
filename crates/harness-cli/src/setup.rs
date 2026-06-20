use std::{
    error::Error,
    fmt, fs,
    path::{Path, PathBuf},
};

use harness_store::{
    bootstrap::{
        initialize_runtime_home, project_record, register_project, register_surface,
        validate_project_id, validate_project_record_for_execution, ProjectRecord,
        ProjectRegistration, SurfaceRecord, SurfaceRegistration, ACTIVE_PROJECT_STATUS,
    },
    inspection::{
        inspect_project_state_database, inspect_runtime_home, DatabaseInspection,
        ProjectInspectionRecord, ProjectStateInspectionSnapshot, RegistryInspectionSnapshot,
        SurfaceInspectionRecord,
    },
    runtime_home::validate_runtime_home_product_repository,
    sqlite::{open_project_state_database, open_registry_database, registry_db_path},
    StoreError,
};
use harness_types::{AccessClass, SurfaceInteractionRole};

use crate::registration::{
    access_classes_match, baseline_workflow_access_classes, capability_profile_json,
    local_access_json, normalized_access_classes_from_local_access, parse_json_object,
    user_interaction_access_classes, validate_role_access_classes, RegistrationMetadataError,
};

pub const SETUP_METADATA_JSON: &str =
    r#"{"created_by":"harness_cli_setup","setup_profile":"local_mcp_v1"}"#;
pub const SETUP_RUNTIME_HOME_ID: &str = "runtime_home_local_mcp";

pub const AGENT_SURFACE_ID: &str = "agent_mcp";
pub const AGENT_SURFACE_INSTANCE_ID: &str = "agent_mcp_local";
pub const USER_INTERACTION_SURFACE_ID: &str = "user_ui";
pub const USER_INTERACTION_SURFACE_INSTANCE_ID: &str = "user_ui_local";
pub const LOCAL_MCP_SURFACE_KIND: &str = "mcp";

/// Already-resolved inputs for local MCP setup planning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalMcpSetupOptions {
    pub runtime_home: PathBuf,
    pub repo_root: PathBuf,
    pub project_id: Option<String>,
    pub include_user_interaction: bool,
    pub replace_conflicting_surfaces: bool,
    pub authorized_surface_replacements: Vec<SetupActionTarget>,
}

impl LocalMcpSetupOptions {
    pub fn new(runtime_home: impl Into<PathBuf>, repo_root: impl Into<PathBuf>) -> Self {
        Self {
            runtime_home: runtime_home.into(),
            repo_root: repo_root.into(),
            project_id: None,
            include_user_interaction: false,
            replace_conflicting_surfaces: false,
            authorized_surface_replacements: Vec::new(),
        }
    }
}

/// Read-only local MCP setup plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalMcpSetupPlan {
    pub runtime_home: PathBuf,
    pub selected_project_id: Option<String>,
    pub repo_root: PathBuf,
    pub runtime_home_action: SetupAction,
    pub project_action: SetupAction,
    pub surface_actions: Vec<SetupAction>,
    pub conflicts: Vec<SetupConflict>,
    pub include_user_interaction: bool,
    pub replace_conflicting_surfaces: bool,
}

impl LocalMcpSetupPlan {
    pub fn ordered_actions(&self) -> Vec<SetupAction> {
        let mut actions = Vec::with_capacity(2 + self.surface_actions.len());
        actions.push(self.runtime_home_action.clone());
        actions.push(self.project_action.clone());
        actions.extend(self.surface_actions.iter().cloned());
        actions
    }

    pub fn has_conflicts(&self) -> bool {
        !self.conflicts.is_empty()
    }
}

/// Result of applying an approved local MCP setup plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalMcpSetupResult {
    pub runtime_home: PathBuf,
    pub project_id: String,
    pub repo_root: PathBuf,
    pub completed_actions: Vec<SetupAction>,
    pub include_user_interaction: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupActionKind {
    Create,
    Reuse,
    Update,
    Conflict,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupActionTarget {
    RuntimeHome,
    Project,
    AgentSurface,
    UserInteractionSurface,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupSurfaceBinding {
    Agent,
    UserInteraction,
}

impl SetupSurfaceBinding {
    pub const fn target(self) -> SetupActionTarget {
        match self {
            Self::Agent => SetupActionTarget::AgentSurface,
            Self::UserInteraction => SetupActionTarget::UserInteractionSurface,
        }
    }

    pub const fn surface_id(self) -> &'static str {
        match self {
            Self::Agent => AGENT_SURFACE_ID,
            Self::UserInteraction => USER_INTERACTION_SURFACE_ID,
        }
    }

    pub const fn surface_instance_id(self) -> &'static str {
        match self {
            Self::Agent => AGENT_SURFACE_INSTANCE_ID,
            Self::UserInteraction => USER_INTERACTION_SURFACE_INSTANCE_ID,
        }
    }

    pub const fn interaction_role(self) -> SurfaceInteractionRole {
        match self {
            Self::Agent => SurfaceInteractionRole::Agent,
            Self::UserInteraction => SurfaceInteractionRole::UserInteraction,
        }
    }

    pub fn expected_access_classes(self) -> Vec<AccessClass> {
        match self {
            Self::Agent => baseline_workflow_access_classes(),
            Self::UserInteraction => user_interaction_access_classes(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetupResource {
    RuntimeHome {
        runtime_home: PathBuf,
    },
    Project {
        project_id: Option<String>,
        repo_root: PathBuf,
    },
    Surface {
        binding: SetupSurfaceBinding,
        project_id: String,
        surface_id: String,
        surface_instance_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupAction {
    pub kind: SetupActionKind,
    pub target: SetupActionTarget,
    pub resource: SetupResource,
}

impl SetupAction {
    fn runtime_home(kind: SetupActionKind, runtime_home: PathBuf) -> Self {
        Self {
            kind,
            target: SetupActionTarget::RuntimeHome,
            resource: SetupResource::RuntimeHome { runtime_home },
        }
    }

    fn project(kind: SetupActionKind, project_id: Option<String>, repo_root: PathBuf) -> Self {
        Self {
            kind,
            target: SetupActionTarget::Project,
            resource: SetupResource::Project {
                project_id,
                repo_root,
            },
        }
    }

    fn surface(kind: SetupActionKind, binding: SetupSurfaceBinding, project_id: String) -> Self {
        Self {
            kind,
            target: binding.target(),
            resource: SetupResource::Surface {
                binding,
                project_id,
                surface_id: binding.surface_id().to_owned(),
                surface_instance_id: binding.surface_instance_id().to_owned(),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetupConflictKind {
    ProjectIdBoundToDifferentRepository,
    ProjectInactive,
    AmbiguousRepositoryProjects,
    ExplicitProjectIdRequired,
    DerivedProjectIdCollision,
    ProjectPathBoundaryInvalid,
    ProjectSelectionChanged,
    SurfaceKindMismatch,
    SurfaceRoleMalformed,
    SurfaceRoleMismatch,
    SurfaceAccessMalformed,
    SurfaceAccessMismatch,
    SurfaceCapabilityMalformed,
    SurfaceMetadataMalformed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupConflict {
    pub target: SetupActionTarget,
    pub kind: SetupConflictKind,
    pub message: String,
    pub project_id: Option<String>,
    pub surface_id: Option<String>,
    pub surface_instance_id: Option<String>,
    pub surface_details: Option<SetupSurfaceConflictDetails>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupSurfaceConflictDetails {
    pub current_kind: String,
    pub desired_kind: String,
    pub current_role: String,
    pub desired_role: String,
    pub current_access_classes: Option<Vec<String>>,
    pub desired_access_classes: Vec<String>,
}

impl SetupConflict {
    fn project(
        kind: SetupConflictKind,
        project_id: Option<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            target: SetupActionTarget::Project,
            kind,
            message: message.into(),
            project_id,
            surface_id: None,
            surface_instance_id: None,
            surface_details: None,
        }
    }

    fn surface(
        kind: SetupConflictKind,
        binding: SetupSurfaceBinding,
        project_id: &str,
        message: impl Into<String>,
    ) -> Self {
        Self {
            target: binding.target(),
            kind,
            message: message.into(),
            project_id: Some(project_id.to_owned()),
            surface_id: Some(binding.surface_id().to_owned()),
            surface_instance_id: Some(binding.surface_instance_id().to_owned()),
            surface_details: None,
        }
    }

    fn with_surface_details(mut self, details: SetupSurfaceConflictDetails) -> Self {
        self.surface_details = Some(details);
        self
    }
}

#[derive(Debug)]
pub enum SetupPlanError {
    InvalidOptions { detail: String },
    RepositoryUnavailable { repo_root: PathBuf, detail: String },
    StorageInspection { detail: String },
    Store(StoreError),
}

impl fmt::Display for SetupPlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidOptions { detail } => formatter.write_str(detail),
            Self::RepositoryUnavailable { repo_root, detail } => {
                write!(
                    formatter,
                    "repo_root {} is unavailable: {detail}",
                    repo_root.display()
                )
            }
            Self::StorageInspection { detail } => formatter.write_str(detail),
            Self::Store(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for SetupPlanError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Store(error) => Some(error),
            Self::InvalidOptions { .. }
            | Self::RepositoryUnavailable { .. }
            | Self::StorageInspection { .. } => None,
        }
    }
}

impl From<StoreError> for SetupPlanError {
    fn from(error: StoreError) -> Self {
        Self::Store(error)
    }
}

#[derive(Debug)]
pub enum SetupApplyError {
    UnresolvedConflicts {
        conflicts: Vec<SetupConflict>,
    },
    RevalidationFailed {
        conflicts: Vec<SetupConflict>,
        completed_actions: Vec<SetupAction>,
    },
    InvalidPlan {
        message: String,
        completed_actions: Vec<SetupAction>,
    },
    OperationFailed {
        source: Box<dyn Error>,
        completed_actions: Vec<SetupAction>,
    },
}

impl SetupApplyError {
    pub fn completed_actions(&self) -> &[SetupAction] {
        match self {
            Self::UnresolvedConflicts { .. } => &[],
            Self::RevalidationFailed {
                completed_actions, ..
            }
            | Self::InvalidPlan {
                completed_actions, ..
            }
            | Self::OperationFailed {
                completed_actions, ..
            } => completed_actions,
        }
    }
}

impl fmt::Display for SetupApplyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnresolvedConflicts { conflicts } => {
                write!(
                    formatter,
                    "setup plan has unresolved conflict(s): {}",
                    conflicts.len()
                )
            }
            Self::RevalidationFailed { conflicts, .. } => {
                write!(
                    formatter,
                    "setup revalidation found conflict(s) after storage preparation: {}",
                    conflicts.len()
                )
            }
            Self::InvalidPlan { message, .. } => formatter.write_str(message),
            Self::OperationFailed { source, .. } => write!(formatter, "{source}"),
        }
    }
}

impl Error for SetupApplyError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::OperationFailed { source, .. } => Some(source.as_ref()),
            Self::UnresolvedConflicts { .. }
            | Self::RevalidationFailed { .. }
            | Self::InvalidPlan { .. } => None,
        }
    }
}

pub fn plan_local_mcp_setup(
    options: LocalMcpSetupOptions,
) -> Result<LocalMcpSetupPlan, SetupPlanError> {
    if !options.runtime_home.is_absolute() {
        return Err(SetupPlanError::InvalidOptions {
            detail: "runtime_home must be absolute".to_owned(),
        });
    }

    let path_validation =
        validate_runtime_home_product_repository(&options.runtime_home, &options.repo_root)
            .map_err(setup_path_boundary_error)?;
    let runtime_home = path_validation.runtime_home;
    let repo_root = path_validation.repo_root;

    if let Some(project_id) = options.project_id.as_deref() {
        validate_project_id(project_id).map_err(project_id_options_error)?;
    }

    let inspection = inspect_runtime_home(&runtime_home);
    let registry_snapshot = match &inspection.registry {
        DatabaseInspection::Missing { .. } => None,
        DatabaseInspection::Present(snapshot) => Some(snapshot),
        other => {
            return Err(SetupPlanError::StorageInspection {
                detail: database_inspection_failure("registry", other),
            });
        }
    };
    let runtime_home_exists = registry_snapshot.is_some();
    let runtime_home_action = SetupAction::runtime_home(
        if runtime_home_exists {
            SetupActionKind::Reuse
        } else {
            SetupActionKind::Create
        },
        runtime_home.clone(),
    );

    let projects = registry_snapshot
        .map(project_records_from_registry_inspection)
        .unwrap_or_default();
    let project_plan = plan_project(
        &runtime_home,
        &projects,
        &repo_root,
        options.project_id.as_deref(),
    );
    let mut conflicts = project_plan.conflicts;
    let project_action = project_plan.action;

    let mut surface_actions = Vec::new();
    if project_action.kind != SetupActionKind::Conflict {
        if let Some(project_id) = project_plan.selected_project_id.as_deref() {
            let existing_surfaces = if project_action.kind == SetupActionKind::Reuse {
                let project = selected_project_inspection(registry_snapshot, project_id)?;
                let project_state = present_project_state_inspection(project)?;
                surface_records_from_project_state_inspection(project_state)
            } else {
                Vec::new()
            };
            plan_surface(
                &mut surface_actions,
                &mut conflicts,
                &existing_surfaces,
                project_id,
                SetupSurfaceBinding::Agent,
                options.replace_conflicting_surfaces,
                &options.authorized_surface_replacements,
            );
            if options.include_user_interaction {
                plan_surface(
                    &mut surface_actions,
                    &mut conflicts,
                    &existing_surfaces,
                    project_id,
                    SetupSurfaceBinding::UserInteraction,
                    options.replace_conflicting_surfaces,
                    &options.authorized_surface_replacements,
                );
            }
        }
    }

    Ok(LocalMcpSetupPlan {
        runtime_home,
        selected_project_id: project_plan.selected_project_id,
        repo_root,
        runtime_home_action,
        project_action,
        surface_actions,
        conflicts,
        include_user_interaction: options.include_user_interaction,
        replace_conflicting_surfaces: options.replace_conflicting_surfaces,
    })
}

fn project_records_from_registry_inspection(
    snapshot: &RegistryInspectionSnapshot,
) -> Vec<ProjectRecord> {
    snapshot
        .projects
        .iter()
        .map(|project| ProjectRecord {
            project_id: project.project_id.clone(),
            runtime_home_id: project.runtime_home_id.clone(),
            repo_root: project.repo_root.clone(),
            project_home: project.project_home.clone(),
            state_db_path: project.state_db_path.clone(),
            status: project.status.clone(),
            metadata_json: project.metadata_json.clone(),
        })
        .collect()
}

fn selected_project_inspection<'a>(
    registry: Option<&'a RegistryInspectionSnapshot>,
    project_id: &str,
) -> Result<&'a ProjectInspectionRecord, SetupPlanError> {
    registry
        .and_then(|snapshot| {
            snapshot
                .projects
                .iter()
                .find(|project| project.project_id == project_id)
        })
        .ok_or_else(|| SetupPlanError::StorageInspection {
            detail: format!("selected project {project_id} was not found during setup inspection"),
        })
}

fn present_project_state_inspection(
    project: &ProjectInspectionRecord,
) -> Result<&ProjectStateInspectionSnapshot, SetupPlanError> {
    match &project.project_state {
        DatabaseInspection::Present(snapshot) => Ok(snapshot),
        other => Err(SetupPlanError::StorageInspection {
            detail: database_inspection_failure("project_state", other),
        }),
    }
}

fn surface_records_from_project_state_inspection(
    snapshot: &ProjectStateInspectionSnapshot,
) -> Vec<SurfaceRecord> {
    snapshot
        .surfaces
        .iter()
        .map(surface_record_from_inspection)
        .collect()
}

fn surface_record_from_inspection(surface: &SurfaceInspectionRecord) -> SurfaceRecord {
    SurfaceRecord {
        project_id: surface.project_id.clone(),
        surface_id: surface.surface_id.clone(),
        surface_instance_id: surface.surface_instance_id.clone(),
        surface_kind: surface.surface_kind.clone(),
        interaction_role: surface.interaction_role.clone(),
        display_name: surface.display_name.clone(),
        capability_profile_json: surface.capability_profile_json.clone(),
        local_access_json: surface.local_access_json.clone(),
        metadata_json: surface.metadata_json.clone(),
    }
}

fn database_inspection_failure<T>(label: &str, inspection: &DatabaseInspection<T>) -> String {
    match inspection {
        DatabaseInspection::Missing { path } => {
            format!("{label} database is missing: {}", path.display())
        }
        DatabaseInspection::Present(_) => {
            format!("{label} database inspection unexpectedly succeeded")
        }
        DatabaseInspection::Unsupported {
            path,
            detected_version,
            latest_supported_version,
            detail,
        } => format!(
            "{label} database {} has unsupported schema version {detected_version}; latest supported is {latest_supported_version}: {detail}",
            path.display()
        ),
        DatabaseInspection::Malformed { path, detail } => {
            format!("{label} database {} is incomplete or malformed: {detail}", path.display())
        }
        DatabaseInspection::Unreadable { path, detail } => {
            format!("{label} database {} is unreadable: {detail}", path.display())
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupPreparationResult {
    pub completed_actions: Vec<SetupAction>,
}

pub fn prepare_local_mcp_setup_storage(
    plan: &LocalMcpSetupPlan,
) -> Result<SetupPreparationResult, SetupApplyError> {
    let mut completed_actions = Vec::new();
    prepare_runtime_home(plan, &mut completed_actions)?;
    prepare_selected_existing_project_state(plan, &mut completed_actions)?;
    Ok(SetupPreparationResult { completed_actions })
}

fn prepare_runtime_home(
    plan: &LocalMcpSetupPlan,
    completed_actions: &mut Vec<SetupAction>,
) -> Result<(), SetupApplyError> {
    match plan.runtime_home_action.kind {
        SetupActionKind::Create => {
            initialize_runtime_home(
                &plan.runtime_home,
                SETUP_RUNTIME_HOME_ID,
                SETUP_METADATA_JSON,
            )
            .map_err(|source| operation_failed(Box::new(source), completed_actions.as_slice()))?;
            completed_actions.push(plan.runtime_home_action.clone());
            Ok(())
        }
        SetupActionKind::Reuse => {
            let registry_path = registry_db_path(&plan.runtime_home);
            if !registry_path.exists() {
                return Err(operation_failed(
                    Box::new(StoreError::NotFound {
                        entity: "registry",
                        id: registry_path.display().to_string(),
                    }),
                    completed_actions.as_slice(),
                ));
            }
            let conn = open_registry_database(&registry_path).map_err(|source| {
                operation_failed(Box::new(source), completed_actions.as_slice())
            })?;
            drop(conn);
            completed_actions.push(plan.runtime_home_action.clone());
            Ok(())
        }
        SetupActionKind::Update | SetupActionKind::Conflict => Err(SetupApplyError::InvalidPlan {
            message: "runtime_home preparation requires a create or reuse action".to_owned(),
            completed_actions: completed_actions.clone(),
        }),
    }
}

fn prepare_selected_existing_project_state(
    plan: &LocalMcpSetupPlan,
    completed_actions: &mut Vec<SetupAction>,
) -> Result<(), SetupApplyError> {
    if plan.project_action.kind != SetupActionKind::Reuse {
        return Ok(());
    }
    let Some(project_id) = plan.selected_project_id.as_deref() else {
        return Err(SetupApplyError::InvalidPlan {
            message: "setup plan has no selected project_id".to_owned(),
            completed_actions: completed_actions.clone(),
        });
    };

    let project = project_record(&plan.runtime_home, project_id)
        .map_err(|source| operation_failed(Box::new(source), completed_actions.as_slice()))?
        .ok_or_else(|| {
            operation_failed(
                Box::new(StoreError::NotFound {
                    entity: "project",
                    id: project_id.to_owned(),
                }),
                completed_actions.as_slice(),
            )
        })?;
    validate_project_record_for_execution(&plan.runtime_home, &project)
        .map_err(|source| operation_failed(Box::new(source), completed_actions.as_slice()))?;
    if !project.state_db_path.exists() {
        return Err(operation_failed(
            Box::new(StoreError::NotFound {
                entity: "project_state_database",
                id: project.state_db_path.display().to_string(),
            }),
            completed_actions.as_slice(),
        ));
    }

    let inspection = inspect_project_state_database(&project.state_db_path, project_id);
    if let Err(detail) = ensure_database_present_for_preparation("project_state", &inspection) {
        return Err(operation_failed(
            Box::new(SetupStoragePreparationError(detail)),
            completed_actions.as_slice(),
        ));
    }

    let conn = open_project_state_database(&project.state_db_path)
        .map_err(|source| operation_failed(Box::new(source), completed_actions.as_slice()))?;
    drop(conn);
    completed_actions.push(plan.project_action.clone());
    Ok(())
}

fn ensure_database_present_for_preparation<T>(
    label: &str,
    inspection: &DatabaseInspection<T>,
) -> Result<(), String> {
    match inspection {
        DatabaseInspection::Present(_) => Ok(()),
        other => Err(database_inspection_failure(label, other)),
    }
}

#[derive(Debug)]
struct SetupStoragePreparationError(String);

impl fmt::Display for SetupStoragePreparationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for SetupStoragePreparationError {}

pub fn apply_local_mcp_setup_plan(
    plan: &LocalMcpSetupPlan,
) -> Result<LocalMcpSetupResult, SetupApplyError> {
    let mut store = BootstrapSetupStore;
    apply_local_mcp_setup_plan_with_store(plan, &mut store)
}

trait SetupStore {
    fn initialize_runtime_home(&mut self, runtime_home: &Path) -> Result<(), Box<dyn Error>>;

    fn register_project(
        &mut self,
        runtime_home: &Path,
        registration: ProjectRegistration,
    ) -> Result<(), Box<dyn Error>>;

    fn register_surface(
        &mut self,
        runtime_home: &Path,
        registration: SurfaceRegistration,
    ) -> Result<(), Box<dyn Error>>;
}

struct BootstrapSetupStore;

impl SetupStore for BootstrapSetupStore {
    fn initialize_runtime_home(&mut self, runtime_home: &Path) -> Result<(), Box<dyn Error>> {
        initialize_runtime_home(runtime_home, SETUP_RUNTIME_HOME_ID, SETUP_METADATA_JSON)
            .map(|_| ())
            .map_err(|error| Box::new(error) as Box<dyn Error>)
    }

    fn register_project(
        &mut self,
        runtime_home: &Path,
        registration: ProjectRegistration,
    ) -> Result<(), Box<dyn Error>> {
        register_project(runtime_home, registration)
            .map(|_| ())
            .map_err(|error| Box::new(error) as Box<dyn Error>)
    }

    fn register_surface(
        &mut self,
        runtime_home: &Path,
        registration: SurfaceRegistration,
    ) -> Result<(), Box<dyn Error>> {
        register_surface(runtime_home, registration)
            .map(|_| ())
            .map_err(|error| Box::new(error) as Box<dyn Error>)
    }
}

fn apply_local_mcp_setup_plan_with_store(
    plan: &LocalMcpSetupPlan,
    store: &mut impl SetupStore,
) -> Result<LocalMcpSetupResult, SetupApplyError> {
    if !plan.conflicts.is_empty() {
        return Err(SetupApplyError::UnresolvedConflicts {
            conflicts: plan.conflicts.clone(),
        });
    }

    let Some(project_id) = plan.selected_project_id.clone() else {
        return Err(SetupApplyError::InvalidPlan {
            message: "setup plan has no selected project_id".to_owned(),
            completed_actions: Vec::new(),
        });
    };
    if let Err(error) = validate_project_id(&project_id) {
        return Err(SetupApplyError::InvalidPlan {
            message: error.to_string(),
            completed_actions: Vec::new(),
        });
    }

    let mut completed_actions = Vec::new();
    for action in plan.ordered_actions() {
        match action.kind {
            SetupActionKind::Reuse => completed_actions.push(action),
            SetupActionKind::Create => {
                apply_create_action(plan, store, &action, &mut completed_actions)?;
            }
            SetupActionKind::Update => {
                apply_update_action(plan, store, &action, &mut completed_actions)?;
            }
            SetupActionKind::Conflict => {
                return Err(SetupApplyError::InvalidPlan {
                    message: "setup plan contains an unresolved conflict action".to_owned(),
                    completed_actions,
                });
            }
        }
    }

    Ok(LocalMcpSetupResult {
        runtime_home: plan.runtime_home.clone(),
        project_id,
        repo_root: plan.repo_root.clone(),
        completed_actions,
        include_user_interaction: plan.include_user_interaction,
    })
}

fn apply_create_action(
    plan: &LocalMcpSetupPlan,
    store: &mut impl SetupStore,
    action: &SetupAction,
    completed_actions: &mut Vec<SetupAction>,
) -> Result<(), SetupApplyError> {
    match &action.resource {
        SetupResource::RuntimeHome { runtime_home } => store
            .initialize_runtime_home(runtime_home)
            .map_err(|source| operation_failed(source, completed_actions))?,
        SetupResource::Project {
            project_id,
            repo_root,
        } => {
            let Some(project_id) = project_id.clone() else {
                return Err(SetupApplyError::InvalidPlan {
                    message: "project create action has no project_id".to_owned(),
                    completed_actions: completed_actions.clone(),
                });
            };
            store
                .register_project(
                    &plan.runtime_home,
                    ProjectRegistration {
                        project_id,
                        repo_root: repo_root.clone(),
                        project_home: None,
                        status: ACTIVE_PROJECT_STATUS.to_owned(),
                        metadata_json: SETUP_METADATA_JSON.to_owned(),
                    },
                )
                .map_err(|source| operation_failed(source, completed_actions))?;
        }
        SetupResource::Surface {
            binding,
            project_id,
            ..
        } => {
            let registration = fixed_surface_registration(project_id, *binding)
                .map_err(|source| operation_failed(Box::new(source), completed_actions))?;
            store
                .register_surface(&plan.runtime_home, registration)
                .map_err(|source| operation_failed(source, completed_actions))?;
        }
    }
    completed_actions.push(action.clone());
    Ok(())
}

fn apply_update_action(
    plan: &LocalMcpSetupPlan,
    store: &mut impl SetupStore,
    action: &SetupAction,
    completed_actions: &mut Vec<SetupAction>,
) -> Result<(), SetupApplyError> {
    let SetupResource::Surface {
        binding,
        project_id,
        ..
    } = &action.resource
    else {
        return Err(SetupApplyError::InvalidPlan {
            message: "only surface actions may be updated by local MCP setup".to_owned(),
            completed_actions: completed_actions.clone(),
        });
    };
    let registration = fixed_surface_registration(project_id, *binding)
        .map_err(|source| operation_failed(Box::new(source), completed_actions))?;
    store
        .register_surface(&plan.runtime_home, registration)
        .map_err(|source| operation_failed(source, completed_actions))?;
    completed_actions.push(action.clone());
    Ok(())
}

fn operation_failed(source: Box<dyn Error>, completed_actions: &[SetupAction]) -> SetupApplyError {
    SetupApplyError::OperationFailed {
        source,
        completed_actions: completed_actions.to_vec(),
    }
}

struct ProjectPlan {
    selected_project_id: Option<String>,
    action: SetupAction,
    conflicts: Vec<SetupConflict>,
}

fn plan_project(
    runtime_home: &Path,
    projects: &[ProjectRecord],
    repo_root: &Path,
    explicit_project_id: Option<&str>,
) -> ProjectPlan {
    if let Some(project_id) = explicit_project_id {
        return plan_explicit_project(runtime_home, projects, repo_root, project_id);
    }

    let matches = projects
        .iter()
        .filter(|project| project_repo_matches(project, repo_root))
        .collect::<Vec<_>>();

    match matches.as_slice() {
        [project] if project.status == ACTIVE_PROJECT_STATUS => {
            if let Some(conflict) = project_reuse_path_conflict(runtime_home, project) {
                return ProjectPlan {
                    selected_project_id: Some(project.project_id.clone()),
                    action: SetupAction::project(
                        SetupActionKind::Conflict,
                        Some(project.project_id.clone()),
                        repo_root.to_path_buf(),
                    ),
                    conflicts: vec![conflict],
                };
            }
            let project_id = project.project_id.clone();
            ProjectPlan {
                selected_project_id: Some(project_id.clone()),
                action: SetupAction::project(
                    SetupActionKind::Reuse,
                    Some(project_id),
                    repo_root.to_path_buf(),
                ),
                conflicts: Vec::new(),
            }
        }
        [project] => {
            let conflict = SetupConflict::project(
                SetupConflictKind::ProjectInactive,
                Some(project.project_id.clone()),
                format!(
                    "project {} is {} and cannot be reused by setup",
                    project.project_id, project.status
                ),
            );
            ProjectPlan {
                selected_project_id: Some(project.project_id.clone()),
                action: SetupAction::project(
                    SetupActionKind::Conflict,
                    Some(project.project_id.clone()),
                    repo_root.to_path_buf(),
                ),
                conflicts: vec![conflict],
            }
        }
        [] => plan_derived_project(projects, repo_root),
        _ => {
            let ids = matches
                .iter()
                .map(|project| project.project_id.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            let conflict = SetupConflict::project(
                SetupConflictKind::AmbiguousRepositoryProjects,
                None,
                format!("multiple projects are registered for repo_root: {ids}"),
            );
            ProjectPlan {
                selected_project_id: None,
                action: SetupAction::project(
                    SetupActionKind::Conflict,
                    None,
                    repo_root.to_path_buf(),
                ),
                conflicts: vec![conflict],
            }
        }
    }
}

fn plan_explicit_project(
    runtime_home: &Path,
    projects: &[ProjectRecord],
    repo_root: &Path,
    project_id: &str,
) -> ProjectPlan {
    let existing = projects
        .iter()
        .find(|project| project.project_id == project_id);
    let Some(project) = existing else {
        return ProjectPlan {
            selected_project_id: Some(project_id.to_owned()),
            action: SetupAction::project(
                SetupActionKind::Create,
                Some(project_id.to_owned()),
                repo_root.to_path_buf(),
            ),
            conflicts: Vec::new(),
        };
    };

    if !project_repo_matches(project, repo_root) {
        let conflict = SetupConflict::project(
            SetupConflictKind::ProjectIdBoundToDifferentRepository,
            Some(project_id.to_owned()),
            format!("project {project_id} is registered to another repo_root"),
        );
        return ProjectPlan {
            selected_project_id: Some(project_id.to_owned()),
            action: SetupAction::project(
                SetupActionKind::Conflict,
                Some(project_id.to_owned()),
                repo_root.to_path_buf(),
            ),
            conflicts: vec![conflict],
        };
    }

    if project.status != ACTIVE_PROJECT_STATUS {
        let conflict = SetupConflict::project(
            SetupConflictKind::ProjectInactive,
            Some(project_id.to_owned()),
            format!(
                "project {project_id} is {} and cannot be reused by setup",
                project.status
            ),
        );
        return ProjectPlan {
            selected_project_id: Some(project_id.to_owned()),
            action: SetupAction::project(
                SetupActionKind::Conflict,
                Some(project_id.to_owned()),
                repo_root.to_path_buf(),
            ),
            conflicts: vec![conflict],
        };
    }

    if let Some(conflict) = project_reuse_path_conflict(runtime_home, project) {
        return ProjectPlan {
            selected_project_id: Some(project_id.to_owned()),
            action: SetupAction::project(
                SetupActionKind::Conflict,
                Some(project_id.to_owned()),
                repo_root.to_path_buf(),
            ),
            conflicts: vec![conflict],
        };
    }

    ProjectPlan {
        selected_project_id: Some(project_id.to_owned()),
        action: SetupAction::project(
            SetupActionKind::Reuse,
            Some(project_id.to_owned()),
            repo_root.to_path_buf(),
        ),
        conflicts: Vec::new(),
    }
}

fn project_reuse_path_conflict(
    runtime_home: &Path,
    project: &ProjectRecord,
) -> Option<SetupConflict> {
    validate_project_record_for_execution(runtime_home, project)
        .err()
        .map(|error| {
            SetupConflict::project(
                SetupConflictKind::ProjectPathBoundaryInvalid,
                Some(project.project_id.clone()),
                error.to_string(),
            )
        })
}

fn plan_derived_project(projects: &[ProjectRecord], repo_root: &Path) -> ProjectPlan {
    let Some(project_id) = repo_root
        .file_name()
        .and_then(|component| component.to_str())
        .filter(|component| !component.is_empty())
        .map(str::to_owned)
    else {
        let conflict = SetupConflict::project(
            SetupConflictKind::ExplicitProjectIdRequired,
            None,
            "repo_root final component cannot be used as a project_id",
        );
        return ProjectPlan {
            selected_project_id: None,
            action: SetupAction::project(SetupActionKind::Conflict, None, repo_root.to_path_buf()),
            conflicts: vec![conflict],
        };
    };

    if validate_project_id(&project_id).is_err() {
        let conflict = SetupConflict::project(
            SetupConflictKind::ExplicitProjectIdRequired,
            None,
            format!(
                "repository directory name {project_id:?} cannot be used as a project_id; pass --project-id with a valid project id"
            ),
        );
        return ProjectPlan {
            selected_project_id: None,
            action: SetupAction::project(SetupActionKind::Conflict, None, repo_root.to_path_buf()),
            conflicts: vec![conflict],
        };
    }

    if let Some(existing) = projects
        .iter()
        .find(|project| project.project_id == project_id)
    {
        if existing.status != ACTIVE_PROJECT_STATUS {
            let conflict = SetupConflict::project(
                SetupConflictKind::ProjectInactive,
                Some(project_id.clone()),
                format!(
                    "derived project_id {project_id} is registered as {}",
                    existing.status
                ),
            );
            return ProjectPlan {
                selected_project_id: Some(project_id.clone()),
                action: SetupAction::project(
                    SetupActionKind::Conflict,
                    Some(project_id),
                    repo_root.to_path_buf(),
                ),
                conflicts: vec![conflict],
            };
        }

        let conflict = SetupConflict::project(
            SetupConflictKind::DerivedProjectIdCollision,
            Some(project_id.clone()),
            format!("derived project_id {project_id} is registered to another repo_root"),
        );
        return ProjectPlan {
            selected_project_id: Some(project_id.clone()),
            action: SetupAction::project(
                SetupActionKind::Conflict,
                Some(project_id),
                repo_root.to_path_buf(),
            ),
            conflicts: vec![conflict],
        };
    }

    ProjectPlan {
        selected_project_id: Some(project_id.clone()),
        action: SetupAction::project(
            SetupActionKind::Create,
            Some(project_id),
            repo_root.to_path_buf(),
        ),
        conflicts: Vec::new(),
    }
}

fn project_id_options_error(error: StoreError) -> SetupPlanError {
    match error {
        StoreError::InvalidInput { detail } => SetupPlanError::InvalidOptions { detail },
        other => SetupPlanError::Store(other),
    }
}

fn setup_path_boundary_error(
    error: harness_store::runtime_home::RuntimePathBoundaryError,
) -> SetupPlanError {
    match error {
        harness_store::runtime_home::RuntimePathBoundaryError::InvalidPath {
            role: "repo_root",
            path,
            detail,
        } => SetupPlanError::RepositoryUnavailable {
            repo_root: path,
            detail,
        },
        other => SetupPlanError::InvalidOptions {
            detail: other.to_string(),
        },
    }
}

fn project_repo_matches(project: &ProjectRecord, selected_repo_root: &Path) -> bool {
    if project.repo_root == selected_repo_root {
        return true;
    }
    fs::canonicalize(&project.repo_root)
        .map(|repo_root| repo_root == selected_repo_root)
        .unwrap_or(false)
}

fn plan_surface(
    actions: &mut Vec<SetupAction>,
    conflicts: &mut Vec<SetupConflict>,
    existing_surfaces: &[SurfaceRecord],
    project_id: &str,
    binding: SetupSurfaceBinding,
    replace_conflicting_surfaces: bool,
    authorized_surface_replacements: &[SetupActionTarget],
) {
    let existing = existing_surfaces.iter().find(|surface| {
        surface.project_id == project_id
            && surface.surface_id == binding.surface_id()
            && surface.surface_instance_id == binding.surface_instance_id()
    });

    let Some(surface) = existing else {
        actions.push(SetupAction::surface(
            SetupActionKind::Create,
            binding,
            project_id.to_owned(),
        ));
        return;
    };

    if let Some(conflict) = surface_conflict(surface, binding) {
        if surface_replacement_authorized(
            binding,
            replace_conflicting_surfaces,
            authorized_surface_replacements,
        ) {
            actions.push(SetupAction::surface(
                SetupActionKind::Update,
                binding,
                project_id.to_owned(),
            ));
        } else {
            actions.push(SetupAction::surface(
                SetupActionKind::Conflict,
                binding,
                project_id.to_owned(),
            ));
            conflicts.push(conflict);
        }
        return;
    }

    actions.push(SetupAction::surface(
        SetupActionKind::Reuse,
        binding,
        project_id.to_owned(),
    ));
}

fn surface_replacement_authorized(
    binding: SetupSurfaceBinding,
    replace_conflicting_surfaces: bool,
    authorized_surface_replacements: &[SetupActionTarget],
) -> bool {
    replace_conflicting_surfaces || authorized_surface_replacements.contains(&binding.target())
}

fn surface_conflict(
    surface: &SurfaceRecord,
    binding: SetupSurfaceBinding,
) -> Option<SetupConflict> {
    let details = surface_conflict_details(surface, binding);
    if surface.surface_kind != LOCAL_MCP_SURFACE_KIND {
        return Some(
            SetupConflict::surface(
                SetupConflictKind::SurfaceKindMismatch,
                binding,
                &surface.project_id,
                format!(
                    "surface kind is {}, expected {}",
                    surface.surface_kind, LOCAL_MCP_SURFACE_KIND
                ),
            )
            .with_surface_details(details),
        );
    }

    let role = parse_surface_role(&surface.interaction_role).map_err(|message| {
        SetupConflict::surface(
            SetupConflictKind::SurfaceRoleMalformed,
            binding,
            &surface.project_id,
            message,
        )
        .with_surface_details(details.clone())
    });
    let role = match role {
        Ok(role) => role,
        Err(conflict) => return Some(conflict),
    };
    if role != binding.interaction_role() {
        return Some(
            SetupConflict::surface(
                SetupConflictKind::SurfaceRoleMismatch,
                binding,
                &surface.project_id,
                format!(
                    "surface interaction_role is {}, expected {}",
                    surface.interaction_role,
                    binding.interaction_role().as_str()
                ),
            )
            .with_surface_details(details),
        );
    }

    if let Err(error) = parse_json_object(
        "surfaces.capability_profile_json",
        &surface.capability_profile_json,
    ) {
        return Some(
            SetupConflict::surface(
                SetupConflictKind::SurfaceCapabilityMalformed,
                binding,
                &surface.project_id,
                error.to_string(),
            )
            .with_surface_details(details),
        );
    }

    let actual_access =
        match normalized_access_classes_from_local_access(&surface.local_access_json) {
            Ok(access_classes) => access_classes,
            Err(error) => {
                return Some(
                    SetupConflict::surface(
                        SetupConflictKind::SurfaceAccessMalformed,
                        binding,
                        &surface.project_id,
                        error.to_string(),
                    )
                    .with_surface_details(details),
                );
            }
        };
    let expected_access = binding.expected_access_classes();
    if !access_classes_match(&actual_access, &expected_access) {
        return Some(
            SetupConflict::surface(
                SetupConflictKind::SurfaceAccessMismatch,
                binding,
                &surface.project_id,
                format!(
                    "surface access classes are {}, expected {}",
                    format_access_classes(&actual_access),
                    format_access_classes(&expected_access)
                ),
            )
            .with_surface_details(details),
        );
    }

    if let Err(error) = parse_json_object("surfaces.metadata_json", &surface.metadata_json) {
        return Some(
            SetupConflict::surface(
                SetupConflictKind::SurfaceMetadataMalformed,
                binding,
                &surface.project_id,
                error.to_string(),
            )
            .with_surface_details(details),
        );
    }

    None
}

fn surface_conflict_details(
    surface: &SurfaceRecord,
    binding: SetupSurfaceBinding,
) -> SetupSurfaceConflictDetails {
    SetupSurfaceConflictDetails {
        current_kind: surface.surface_kind.clone(),
        desired_kind: LOCAL_MCP_SURFACE_KIND.to_owned(),
        current_role: surface.interaction_role.clone(),
        desired_role: binding.interaction_role().as_str().to_owned(),
        current_access_classes: normalized_access_classes_from_local_access(
            &surface.local_access_json,
        )
        .ok()
        .map(|access_classes| access_class_names(&access_classes)),
        desired_access_classes: access_class_names(&binding.expected_access_classes()),
    }
}

fn access_class_names(access_classes: &[AccessClass]) -> Vec<String> {
    access_classes
        .iter()
        .map(|access_class| access_class.as_str().to_owned())
        .collect()
}

fn parse_surface_role(value: &str) -> Result<SurfaceInteractionRole, String> {
    match value {
        "agent" => Ok(SurfaceInteractionRole::Agent),
        "user_interaction" => Ok(SurfaceInteractionRole::UserInteraction),
        other => Err(format!("unsupported surface interaction_role: {other}")),
    }
}

fn fixed_surface_registration(
    project_id: &str,
    binding: SetupSurfaceBinding,
) -> Result<SurfaceRegistration, RegistrationMetadataError> {
    let access_classes = binding.expected_access_classes();
    validate_role_access_classes(binding.interaction_role(), &access_classes)?;
    Ok(SurfaceRegistration {
        project_id: project_id.to_owned(),
        surface_id: binding.surface_id().to_owned(),
        surface_instance_id: binding.surface_instance_id().to_owned(),
        surface_kind: LOCAL_MCP_SURFACE_KIND.to_owned(),
        interaction_role: binding.interaction_role(),
        display_name: None,
        capability_profile_json: capability_profile_json(&access_classes, None)?,
        local_access_json: local_access_json(&access_classes)?,
        metadata_json: SETUP_METADATA_JSON.to_owned(),
    })
}

fn format_access_classes(access_classes: &[AccessClass]) -> String {
    access_classes
        .iter()
        .map(|access_class| access_class.as_str())
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(test)]
mod tests {
    use std::{
        error::Error,
        ffi::OsString,
        fmt, fs,
        path::{Path, PathBuf},
    };

    use harness_store::{
        bootstrap::{initialize_runtime_home, list_surfaces, register_project, register_surface},
        migrations::{
            test_support::create_project_state_fixture_version, PROJECT_STATE_DATABASE_KIND,
            PROJECT_STATE_SCHEMA_VERSION, REGISTRY_DATABASE_KIND,
        },
        sqlite::{open_read_only_database, project_state_db_path, registry_db_path},
    };
    use harness_test_support::TempRuntimeHome;
    use rusqlite::{params, Connection};

    use super::*;
    use crate::registration::{capability_profile_json, local_access_json};

    #[test]
    fn absent_runtime_home_plans_initialization_and_default_project() -> Result<(), Box<dyn Error>>
    {
        let runtime_home = TempRuntimeHome::new("setup-absent-runtime")?;
        let repo_root = repo_dir(runtime_home.path(), "product-alpha")?;

        let plan =
            plan_local_mcp_setup(LocalMcpSetupOptions::new(runtime_home.path(), &repo_root))?;

        assert_eq!(plan.runtime_home_action.kind, SetupActionKind::Create);
        assert_eq!(plan.project_action.kind, SetupActionKind::Create);
        assert_eq!(plan.selected_project_id.as_deref(), Some("product-alpha"));
        assert_eq!(plan.surface_actions.len(), 1);
        assert_eq!(plan.surface_actions[0].kind, SetupActionKind::Create);
        assert_eq!(
            plan.surface_actions[0].target,
            SetupActionTarget::AgentSurface
        );
        assert!(plan.conflicts.is_empty());
        assert!(!registry_db_path(runtime_home.path()).exists());
        Ok(())
    }

    #[test]
    fn setup_planning_rejects_same_runtime_home_and_repository() -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("setup-boundary-same")?;

        let error = plan_local_mcp_setup(LocalMcpSetupOptions::new(
            runtime_home.path(),
            runtime_home.path(),
        ))
        .expect_err("same Runtime Home and Product Repository should fail planning");

        assert!(error.to_string().contains("same path"));
        assert!(!registry_db_path(runtime_home.path()).exists());
        Ok(())
    }

    #[test]
    fn setup_planning_rejects_repository_inside_runtime_home() -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("setup-boundary-repo-inside")?;
        let repo_root = runtime_home.path().join("repo");
        fs::create_dir_all(&repo_root)?;

        let error =
            plan_local_mcp_setup(LocalMcpSetupOptions::new(runtime_home.path(), &repo_root))
                .expect_err("Product Repository under Runtime Home should fail planning");

        assert!(error
            .to_string()
            .contains("Product Repository must not be inside Harness Runtime Home"));
        assert!(!registry_db_path(runtime_home.path()).exists());
        Ok(())
    }

    #[test]
    fn setup_planning_rejects_runtime_home_inside_repository_without_creation(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = TempRuntimeHome::new("setup-boundary-runtime-inside")?;
        let repo_root = fixture.create_product_repo("repo")?;
        let runtime_home = repo_root.join(".harness");

        let error = plan_local_mcp_setup(LocalMcpSetupOptions::new(&runtime_home, &repo_root))
            .expect_err("Runtime Home under Product Repository should fail planning");

        assert!(error
            .to_string()
            .contains("Harness Runtime Home must not be inside Product Repository"));
        assert!(!runtime_home.exists());
        assert!(!registry_db_path(&runtime_home).exists());
        Ok(())
    }

    #[test]
    fn setup_planning_accepts_component_distinct_text_prefix_paths() -> Result<(), Box<dyn Error>> {
        let fixture = TempRuntimeHome::new("setup-boundary-text-prefix")?;
        let parent = fixture.path().parent().expect("runtime home has parent");
        let runtime_home = parent.join("repo");
        let repo_root = parent.join("repository");
        fs::create_dir_all(&repo_root)?;

        let plan = plan_local_mcp_setup(LocalMcpSetupOptions::new(&runtime_home, &repo_root))?;

        assert_eq!(plan.runtime_home_action.kind, SetupActionKind::Create);
        assert_eq!(plan.project_action.kind, SetupActionKind::Create);
        assert_eq!(plan.selected_project_id.as_deref(), Some("repository"));
        assert!(!registry_db_path(&runtime_home).exists());
        Ok(())
    }

    #[test]
    fn existing_runtime_home_is_reused() -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("setup-existing-runtime")?;
        initialize(runtime_home.path())?;
        let repo_root = repo_dir(runtime_home.path(), "product-runtime-reuse")?;

        let plan =
            plan_local_mcp_setup(LocalMcpSetupOptions::new(runtime_home.path(), &repo_root))?;

        assert_eq!(plan.runtime_home_action.kind, SetupActionKind::Reuse);
        assert_eq!(plan.project_action.kind, SetupActionKind::Create);
        Ok(())
    }

    #[test]
    fn explicit_project_id_plans_creation_when_absent() -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("setup-explicit-project")?;
        initialize(runtime_home.path())?;
        let repo_root = repo_dir(runtime_home.path(), "product-explicit")?;
        let mut options = LocalMcpSetupOptions::new(runtime_home.path(), &repo_root);
        options.project_id = Some("project_explicit".to_owned());

        let plan = plan_local_mcp_setup(options)?;

        assert_eq!(
            plan.selected_project_id.as_deref(),
            Some("project_explicit")
        );
        assert_eq!(plan.project_action.kind, SetupActionKind::Create);
        Ok(())
    }

    #[test]
    fn explicit_invalid_project_id_fails_setup_planning() -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("setup-explicit-invalid-project")?;
        initialize(runtime_home.path())?;
        let repo_root = repo_dir(runtime_home.path(), "product-explicit-invalid")?;
        let mut options = LocalMcpSetupOptions::new(runtime_home.path(), &repo_root);
        options.project_id = Some("a/b".to_owned());

        let error = plan_local_mcp_setup(options)
            .expect_err("invalid explicit project_id should fail planning");

        assert!(matches!(error, SetupPlanError::InvalidOptions { .. }));
        assert!(error.to_string().contains("project_id"));
        Ok(())
    }

    #[test]
    fn invalid_repository_directory_name_requires_explicit_project_id() -> Result<(), Box<dyn Error>>
    {
        let runtime_home = TempRuntimeHome::new("setup-invalid-derived-project")?;
        let repo_root = repo_dir(runtime_home.path(), "   ")?;

        let plan =
            plan_local_mcp_setup(LocalMcpSetupOptions::new(runtime_home.path(), &repo_root))?;

        assert_eq!(
            conflict_kinds(&plan),
            vec![SetupConflictKind::ExplicitProjectIdRequired]
        );
        assert!(plan.conflicts[0]
            .message
            .contains("repository directory name"));
        assert!(!registry_db_path(runtime_home.path()).exists());
        Ok(())
    }

    #[test]
    fn one_matching_repository_registration_is_reused() -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("setup-reuse-matching-project")?;
        initialize(runtime_home.path())?;
        let repo_root = repo_dir(runtime_home.path(), "product-match")?;
        register_test_project(runtime_home.path(), "project_existing", &repo_root)?;

        let plan =
            plan_local_mcp_setup(LocalMcpSetupOptions::new(runtime_home.path(), &repo_root))?;

        assert_eq!(
            plan.selected_project_id.as_deref(),
            Some("project_existing")
        );
        assert_eq!(plan.project_action.kind, SetupActionKind::Reuse);
        assert_eq!(plan.surface_actions[0].kind, SetupActionKind::Create);
        Ok(())
    }

    #[test]
    fn invalid_existing_project_home_registration_is_not_reused() -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("setup-reuse-invalid-project")?;
        initialize(runtime_home.path())?;
        let repo_root = repo_dir(runtime_home.path(), "product-invalid-reuse")?;
        register_test_project(runtime_home.path(), "project_invalid", &repo_root)?;
        replace_project_home(
            runtime_home.path(),
            "project_invalid",
            &repo_root.join(".harness-project"),
        )?;

        let plan =
            plan_local_mcp_setup(LocalMcpSetupOptions::new(runtime_home.path(), &repo_root))?;

        assert_eq!(plan.project_action.kind, SetupActionKind::Conflict);
        assert_eq!(
            conflict_kinds(&plan),
            vec![SetupConflictKind::ProjectPathBoundaryInvalid]
        );
        assert!(plan.conflicts[0]
            .message
            .contains("registered project paths conflict"));
        assert!(plan.conflicts[0]
            .message
            .contains("project_home_overlaps_product_repository"));
        assert!(plan.surface_actions.is_empty());
        Ok(())
    }

    #[test]
    fn setup_preparation_rechecks_reused_project_registration_before_mutation(
    ) -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("setup-prepare-invalid-project")?;
        initialize(runtime_home.path())?;
        let repo_root = repo_dir(runtime_home.path(), "product-invalid-prepare")?;
        register_test_project(runtime_home.path(), "project_prepare", &repo_root)?;
        let plan =
            plan_local_mcp_setup(LocalMcpSetupOptions::new(runtime_home.path(), &repo_root))?;
        assert_eq!(plan.project_action.kind, SetupActionKind::Reuse);

        replace_project_repo_root(runtime_home.path(), "project_prepare", runtime_home.path())?;
        let error = prepare_local_mcp_setup_storage(&plan)
            .expect_err("preparation should reject changed invalid registration");

        assert!(error
            .to_string()
            .contains("registered Product Repository conflicts with Runtime Home"));
        assert!(error.to_string().contains("same_path"));
        assert_eq!(
            error.completed_actions(),
            std::slice::from_ref(&plan.runtime_home_action)
        );
        assert_eq!(
            project_metadata(runtime_home.path(), "project_prepare")?,
            "{}"
        );
        Ok(())
    }

    #[test]
    fn multiple_matching_repository_registrations_are_ambiguous() -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("setup-ambiguous-project")?;
        initialize(runtime_home.path())?;
        let repo_root = repo_dir(runtime_home.path(), "product-ambiguous")?;
        register_test_project(runtime_home.path(), "project_a", &repo_root)?;
        register_test_project(runtime_home.path(), "project_b", &repo_root)?;

        let plan =
            plan_local_mcp_setup(LocalMcpSetupOptions::new(runtime_home.path(), &repo_root))?;

        assert_eq!(plan.project_action.kind, SetupActionKind::Conflict);
        assert_eq!(
            conflict_kinds(&plan),
            vec![SetupConflictKind::AmbiguousRepositoryProjects]
        );
        assert!(plan.surface_actions.is_empty());
        Ok(())
    }

    #[test]
    fn derived_project_id_collision_is_reported() -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("setup-derived-collision")?;
        initialize(runtime_home.path())?;
        let repo_root = repo_dir(runtime_home.path(), "product-collision")?;
        let other_repo = repo_dir(runtime_home.path(), "other-product")?;
        register_test_project(runtime_home.path(), "product-collision", &other_repo)?;

        let plan =
            plan_local_mcp_setup(LocalMcpSetupOptions::new(runtime_home.path(), &repo_root))?;

        assert_eq!(
            plan.selected_project_id.as_deref(),
            Some("product-collision")
        );
        assert_eq!(
            conflict_kinds(&plan),
            vec![SetupConflictKind::DerivedProjectIdCollision]
        );
        Ok(())
    }

    #[test]
    fn explicit_project_id_bound_to_another_repository_is_reported() -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("setup-explicit-collision")?;
        initialize(runtime_home.path())?;
        let repo_root = repo_dir(runtime_home.path(), "product-explicit-collision")?;
        let other_repo = repo_dir(runtime_home.path(), "other-explicit-product")?;
        register_test_project(runtime_home.path(), "project_explicit", &other_repo)?;
        let mut options = LocalMcpSetupOptions::new(runtime_home.path(), &repo_root);
        options.project_id = Some("project_explicit".to_owned());

        let plan = plan_local_mcp_setup(options)?;

        assert_eq!(
            conflict_kinds(&plan),
            vec![SetupConflictKind::ProjectIdBoundToDifferentRepository]
        );
        Ok(())
    }

    #[test]
    fn inactive_project_is_reported_without_reactivation() -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("setup-inactive-project")?;
        initialize(runtime_home.path())?;
        let repo_root = repo_dir(runtime_home.path(), "product-inactive")?;
        register_test_project(runtime_home.path(), "project_inactive", &repo_root)?;
        set_project_status(runtime_home.path(), "project_inactive", "inactive")?;

        let plan =
            plan_local_mcp_setup(LocalMcpSetupOptions::new(runtime_home.path(), &repo_root))?;

        assert_eq!(
            plan.selected_project_id.as_deref(),
            Some("project_inactive")
        );
        assert_eq!(
            conflict_kinds(&plan),
            vec![SetupConflictKind::ProjectInactive]
        );
        assert_eq!(
            project_status(runtime_home.path(), "project_inactive")?,
            "inactive"
        );
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_repository_final_component_requires_explicit_project_id(
    ) -> Result<(), Box<dyn Error>> {
        use std::os::unix::ffi::OsStringExt;

        let runtime_home = TempRuntimeHome::new("setup-non-utf8-project")?;
        let repo_root = runtime_home
            .path()
            .parent()
            .expect("runtime home has parent")
            .join("product-repositories")
            .join(PathBuf::from(OsString::from_vec(vec![
                b'r', b'e', b'p', b'o', 0xFF,
            ])));
        fs::create_dir_all(&repo_root)?;

        let plan =
            plan_local_mcp_setup(LocalMcpSetupOptions::new(runtime_home.path(), &repo_root))?;

        assert_eq!(
            conflict_kinds(&plan),
            vec![SetupConflictKind::ExplicitProjectIdRequired]
        );
        Ok(())
    }

    #[test]
    fn agent_only_and_user_interaction_plans_use_fixed_order() -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("setup-surface-order")?;
        let repo_root = repo_dir(runtime_home.path(), "product-order")?;

        let agent_only =
            plan_local_mcp_setup(LocalMcpSetupOptions::new(runtime_home.path(), &repo_root))?;
        assert_eq!(
            surface_targets(&agent_only),
            vec![SetupActionTarget::AgentSurface]
        );

        let mut options = LocalMcpSetupOptions::new(runtime_home.path(), &repo_root);
        options.include_user_interaction = true;
        let with_user = plan_local_mcp_setup(options)?;

        assert_eq!(
            with_user
                .ordered_actions()
                .iter()
                .map(|action| action.target)
                .collect::<Vec<_>>(),
            vec![
                SetupActionTarget::RuntimeHome,
                SetupActionTarget::Project,
                SetupActionTarget::AgentSurface,
                SetupActionTarget::UserInteractionSurface
            ]
        );
        assert_eq!(
            surface_targets(&with_user),
            vec![
                SetupActionTarget::AgentSurface,
                SetupActionTarget::UserInteractionSurface
            ]
        );
        Ok(())
    }

    #[test]
    fn compatible_target_surface_is_reused() -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("setup-compatible-surface")?;
        initialize(runtime_home.path())?;
        let repo_root = repo_dir(runtime_home.path(), "product-compatible-surface")?;
        register_test_project(runtime_home.path(), "project_surface", &repo_root)?;
        register_surface(
            runtime_home.path(),
            fixed_surface_registration("project_surface", SetupSurfaceBinding::Agent)?,
        )?;

        let plan =
            plan_local_mcp_setup(LocalMcpSetupOptions::new(runtime_home.path(), &repo_root))?;

        assert_eq!(plan.surface_actions[0].kind, SetupActionKind::Reuse);
        assert!(plan.conflicts.is_empty());
        Ok(())
    }

    #[test]
    fn read_only_agent_surface_is_a_non_replaceable_conflict() -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("setup-read-only-conflict")?;
        let repo_root = registered_project(runtime_home.path(), "project_read_only")?;
        register_custom_surface(
            runtime_home.path(),
            "project_read_only",
            SetupSurfaceBinding::Agent,
            SurfaceInteractionRole::Agent,
            &[AccessClass::ReadStatus],
            LOCAL_MCP_SURFACE_KIND,
            SETUP_METADATA_JSON,
        )?;

        let plan =
            plan_local_mcp_setup(LocalMcpSetupOptions::new(runtime_home.path(), &repo_root))?;

        assert_eq!(plan.surface_actions[0].kind, SetupActionKind::Conflict);
        assert_eq!(
            conflict_kinds(&plan),
            vec![SetupConflictKind::SurfaceAccessMismatch]
        );
        Ok(())
    }

    #[test]
    fn role_mismatch_is_a_surface_conflict() -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("setup-role-conflict")?;
        let repo_root = registered_project(runtime_home.path(), "project_role")?;
        register_custom_surface(
            runtime_home.path(),
            "project_role",
            SetupSurfaceBinding::Agent,
            SurfaceInteractionRole::UserInteraction,
            &baseline_workflow_access_classes(),
            LOCAL_MCP_SURFACE_KIND,
            SETUP_METADATA_JSON,
        )?;

        let plan =
            plan_local_mcp_setup(LocalMcpSetupOptions::new(runtime_home.path(), &repo_root))?;

        assert_eq!(
            conflict_kinds(&plan),
            vec![SetupConflictKind::SurfaceRoleMismatch]
        );
        Ok(())
    }

    #[test]
    fn malformed_access_json_is_a_surface_conflict() -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("setup-access-conflict")?;
        let repo_root = registered_project(runtime_home.path(), "project_access")?;
        register_surface(
            runtime_home.path(),
            fixed_surface_registration("project_access", SetupSurfaceBinding::Agent)?,
        )?;
        update_surface_column(
            runtime_home.path(),
            "project_access",
            SetupSurfaceBinding::Agent,
            "local_access_json",
            "{not-json",
        )?;

        let plan =
            plan_local_mcp_setup(LocalMcpSetupOptions::new(runtime_home.path(), &repo_root))?;

        assert_eq!(
            conflict_kinds(&plan),
            vec![SetupConflictKind::SurfaceAccessMalformed]
        );
        Ok(())
    }

    #[test]
    fn malformed_capability_and_metadata_objects_are_surface_conflicts(
    ) -> Result<(), Box<dyn Error>> {
        let capability_runtime = TempRuntimeHome::new("setup-capability-conflict")?;
        let capability_repo = registered_project(capability_runtime.path(), "project_capability")?;
        register_surface(
            capability_runtime.path(),
            fixed_surface_registration("project_capability", SetupSurfaceBinding::Agent)?,
        )?;
        update_surface_column(
            capability_runtime.path(),
            "project_capability",
            SetupSurfaceBinding::Agent,
            "capability_profile_json",
            "[]",
        )?;
        let capability_plan = plan_local_mcp_setup(LocalMcpSetupOptions::new(
            capability_runtime.path(),
            &capability_repo,
        ))?;
        assert_eq!(
            conflict_kinds(&capability_plan),
            vec![SetupConflictKind::SurfaceCapabilityMalformed]
        );

        let metadata_runtime = TempRuntimeHome::new("setup-metadata-conflict")?;
        let metadata_repo = registered_project(metadata_runtime.path(), "project_metadata")?;
        register_surface(
            metadata_runtime.path(),
            fixed_surface_registration("project_metadata", SetupSurfaceBinding::Agent)?,
        )?;
        update_surface_column(
            metadata_runtime.path(),
            "project_metadata",
            SetupSurfaceBinding::Agent,
            "metadata_json",
            "[]",
        )?;
        let metadata_plan = plan_local_mcp_setup(LocalMcpSetupOptions::new(
            metadata_runtime.path(),
            &metadata_repo,
        ))?;
        assert_eq!(
            conflict_kinds(&metadata_plan),
            vec![SetupConflictKind::SurfaceMetadataMalformed]
        );
        Ok(())
    }

    #[test]
    fn explicit_replacement_updates_only_the_conflicting_target_surface(
    ) -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("setup-replace-conflict")?;
        let repo_root = registered_project(runtime_home.path(), "project_replace")?;
        register_custom_surface(
            runtime_home.path(),
            "project_replace",
            SetupSurfaceBinding::Agent,
            SurfaceInteractionRole::Agent,
            &[AccessClass::ReadStatus],
            LOCAL_MCP_SURFACE_KIND,
            r#"{"preexisting":true}"#,
        )?;
        register_unrelated_surface(runtime_home.path(), "project_replace")?;
        let unrelated_before = surface_metadata(
            runtime_home.path(),
            "project_replace",
            "agent_mcp",
            "other_instance",
        )?;
        let mut options = LocalMcpSetupOptions::new(runtime_home.path(), &repo_root);
        options.replace_conflicting_surfaces = true;

        let plan = plan_local_mcp_setup(options)?;
        assert!(plan.conflicts.is_empty());
        assert_eq!(plan.surface_actions[0].kind, SetupActionKind::Update);

        apply_local_mcp_setup_plan(&plan)?;

        let surfaces = list_surfaces(runtime_home.path(), "project_replace")?;
        assert_eq!(surfaces.len(), 2);
        let target = surfaces
            .iter()
            .find(|surface| surface.surface_instance_id == AGENT_SURFACE_INSTANCE_ID)
            .expect("target surface should exist");
        assert_eq!(
            normalized_access_classes_from_local_access(&target.local_access_json)?,
            baseline_workflow_access_classes()
        );
        assert_eq!(target.metadata_json, SETUP_METADATA_JSON);
        assert_eq!(
            surface_metadata(
                runtime_home.path(),
                "project_replace",
                "agent_mcp",
                "other_instance"
            )?,
            unrelated_before
        );
        Ok(())
    }

    #[test]
    fn absent_target_creation_does_not_replace_unrelated_surface_instances(
    ) -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("setup-unrelated-instance")?;
        let repo_root = registered_project(runtime_home.path(), "project_unrelated")?;
        register_unrelated_surface(runtime_home.path(), "project_unrelated")?;
        let unrelated_before = surface_metadata(
            runtime_home.path(),
            "project_unrelated",
            "agent_mcp",
            "other_instance",
        )?;

        let plan =
            plan_local_mcp_setup(LocalMcpSetupOptions::new(runtime_home.path(), &repo_root))?;
        assert_eq!(plan.surface_actions[0].kind, SetupActionKind::Create);

        apply_local_mcp_setup_plan(&plan)?;

        assert_eq!(
            list_surfaces(runtime_home.path(), "project_unrelated")?.len(),
            2
        );
        assert_eq!(
            surface_metadata(
                runtime_home.path(),
                "project_unrelated",
                "agent_mcp",
                "other_instance"
            )?,
            unrelated_before
        );
        Ok(())
    }

    #[test]
    fn apply_refuses_unresolved_conflicts() -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("setup-apply-conflict")?;
        let repo_root = registered_project(runtime_home.path(), "project_apply_conflict")?;
        register_custom_surface(
            runtime_home.path(),
            "project_apply_conflict",
            SetupSurfaceBinding::Agent,
            SurfaceInteractionRole::Agent,
            &[AccessClass::ReadStatus],
            LOCAL_MCP_SURFACE_KIND,
            SETUP_METADATA_JSON,
        )?;
        let plan =
            plan_local_mcp_setup(LocalMcpSetupOptions::new(runtime_home.path(), &repo_root))?;

        let error = apply_local_mcp_setup_plan(&plan)
            .expect_err("apply should reject unresolved conflicts");

        match error {
            SetupApplyError::UnresolvedConflicts { conflicts } => {
                assert_eq!(conflicts.len(), 1);
                assert_eq!(conflicts[0].kind, SetupConflictKind::SurfaceAccessMismatch);
            }
            other => panic!("unexpected error: {other:?}"),
        }
        Ok(())
    }

    #[test]
    fn partial_action_error_reports_completed_actions() -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("setup-partial-error")?;
        let repo_root = repo_dir(runtime_home.path(), "product-partial")?;
        let mut options = LocalMcpSetupOptions::new(runtime_home.path(), &repo_root);
        options.include_user_interaction = true;
        let plan = plan_local_mcp_setup(options)?;
        let mut store = FailingUserSurfaceStore;

        let error = apply_local_mcp_setup_plan_with_store(&plan, &mut store)
            .expect_err("user surface registration should fail");

        match error {
            SetupApplyError::OperationFailed {
                source,
                completed_actions,
            } => {
                assert_eq!(source.to_string(), "planned user surface failure");
                assert_eq!(
                    completed_actions
                        .iter()
                        .map(|action| action.target)
                        .collect::<Vec<_>>(),
                    vec![
                        SetupActionTarget::RuntimeHome,
                        SetupActionTarget::Project,
                        SetupActionTarget::AgentSurface
                    ]
                );
            }
            other => panic!("unexpected error: {other:?}"),
        }
        Ok(())
    }

    #[test]
    fn exact_repeated_application_is_idempotent() -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("setup-idempotent")?;
        let repo_root = repo_dir(runtime_home.path(), "product-idempotent")?;
        let mut options = LocalMcpSetupOptions::new(runtime_home.path(), &repo_root);
        options.include_user_interaction = true;

        let first_plan = plan_local_mcp_setup(options.clone())?;
        let first_result = apply_local_mcp_setup_plan(&first_plan)?;
        assert_eq!(
            first_result
                .completed_actions
                .iter()
                .map(|action| action.kind)
                .collect::<Vec<_>>(),
            vec![
                SetupActionKind::Create,
                SetupActionKind::Create,
                SetupActionKind::Create,
                SetupActionKind::Create
            ]
        );

        let project_id = "product-idempotent";
        let before = Snapshot::read(runtime_home.path(), project_id)?;
        let second_plan = plan_local_mcp_setup(options)?;
        let second_result = apply_local_mcp_setup_plan(&second_plan)?;
        let after = Snapshot::read(runtime_home.path(), project_id)?;

        assert_eq!(
            second_result
                .completed_actions
                .iter()
                .map(|action| action.kind)
                .collect::<Vec<_>>(),
            vec![
                SetupActionKind::Reuse,
                SetupActionKind::Reuse,
                SetupActionKind::Reuse,
                SetupActionKind::Reuse
            ]
        );
        assert_eq!(after, before);
        assert_eq!(before.project_count, 1);
        assert_eq!(before.surface_count, 2);
        assert_eq!(before.task_count, 0);
        assert_eq!(before.tool_invocation_count, 0);
        assert_eq!(before.state_version, 0);
        Ok(())
    }

    #[test]
    fn planning_existing_setup_does_not_rewrite_sqlite_records() -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("setup-plan-read-only")?;
        let repo_root = repo_dir(runtime_home.path(), "product-plan-read-only")?;
        let mut options = LocalMcpSetupOptions::new(runtime_home.path(), &repo_root);
        options.include_user_interaction = true;
        let first_plan = plan_local_mcp_setup(options.clone())?;
        apply_local_mcp_setup_plan(&first_plan)?;
        let before = Snapshot::read(runtime_home.path(), "product-plan-read-only")?;

        let second_plan = plan_local_mcp_setup(options)?;
        let after = Snapshot::read(runtime_home.path(), "product-plan-read-only")?;

        assert_eq!(
            second_plan
                .ordered_actions()
                .iter()
                .map(|action| action.kind)
                .collect::<Vec<_>>(),
            vec![
                SetupActionKind::Reuse,
                SetupActionKind::Reuse,
                SetupActionKind::Reuse,
                SetupActionKind::Reuse
            ]
        );
        assert_eq!(after, before);
        Ok(())
    }

    #[test]
    fn planning_supported_historical_project_state_does_not_migrate() -> Result<(), Box<dyn Error>>
    {
        let runtime_home = TempRuntimeHome::new("setup-plan-historical-state")?;
        let repo_root = registered_historical_project_state(
            runtime_home.path(),
            "project_historical",
            PROJECT_STATE_SCHEMA_VERSION - 1,
            &baseline_workflow_access_classes(),
        )?;
        let before = migration_count(&project_state_db_path(
            runtime_home.path(),
            "project_historical",
        ))?;

        let plan =
            plan_local_mcp_setup(LocalMcpSetupOptions::new(runtime_home.path(), &repo_root))?;

        assert_eq!(plan.project_action.kind, SetupActionKind::Reuse);
        assert_eq!(plan.surface_actions[0].kind, SetupActionKind::Reuse);
        assert_eq!(
            migration_count(&project_state_db_path(
                runtime_home.path(),
                "project_historical"
            ))?,
            before
        );
        Ok(())
    }

    #[test]
    fn planning_unsupported_project_state_schema_fails_without_migration(
    ) -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("setup-plan-unsupported-state")?;
        let repo_root = registered_project(runtime_home.path(), "project_unsupported")?;
        let state_path = project_state_db_path(runtime_home.path(), "project_unsupported");
        let before = migration_count(&state_path)?;
        Connection::open(&state_path)?.execute(
            "INSERT INTO schema_migrations (
                database_kind,
                version,
                name,
                storage_profile,
                applied_at
            )
            VALUES (?1, 999, 'project_state_future_v999', 'baseline_sqlite', 't_future')",
            params![PROJECT_STATE_DATABASE_KIND],
        )?;
        let after_fixture_change = migration_count(&state_path)?;

        let error =
            plan_local_mcp_setup(LocalMcpSetupOptions::new(runtime_home.path(), &repo_root))
                .expect_err("unsupported schema should fail planning");

        assert!(error.to_string().contains("unsupported schema version"));
        assert_eq!(migration_count(&state_path)?, after_fixture_change);
        assert_eq!(after_fixture_change, before + 1);
        Ok(())
    }

    #[test]
    fn planning_unsupported_registry_schema_fails_without_migration() -> Result<(), Box<dyn Error>>
    {
        let runtime_home = TempRuntimeHome::new("setup-plan-unsupported-registry")?;
        initialize(runtime_home.path())?;
        let repo_root = repo_dir(runtime_home.path(), "product-unsupported-registry")?;
        let registry_path = registry_db_path(runtime_home.path());
        let before = registry_migration_count(&registry_path)?;
        Connection::open(&registry_path)?.execute(
            "INSERT INTO schema_migrations (
                database_kind,
                version,
                name,
                storage_profile,
                applied_at
            )
            VALUES (?1, 999, 'registry_future_v999', 'baseline_sqlite', 't_future')",
            params![REGISTRY_DATABASE_KIND],
        )?;
        let after_fixture_change = registry_migration_count(&registry_path)?;

        let error =
            plan_local_mcp_setup(LocalMcpSetupOptions::new(runtime_home.path(), &repo_root))
                .expect_err("unsupported registry schema should fail planning");

        assert!(error.to_string().contains("unsupported schema version"));
        assert_eq!(
            registry_migration_count(&registry_path)?,
            after_fixture_change
        );
        assert_eq!(after_fixture_change, before + 1);
        Ok(())
    }

    #[test]
    fn planning_incomplete_project_state_schema_fails_without_migration(
    ) -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("setup-plan-incomplete-state")?;
        let repo_root = registered_project(runtime_home.path(), "project_incomplete")?;
        let state_path = project_state_db_path(runtime_home.path(), "project_incomplete");
        let before = migration_count(&state_path)?;
        Connection::open(&state_path)?.execute("DROP TABLE surfaces", [])?;

        let error =
            plan_local_mcp_setup(LocalMcpSetupOptions::new(runtime_home.path(), &repo_root))
                .expect_err("incomplete schema should fail planning");

        assert!(error.to_string().contains("incomplete or malformed"));
        assert_eq!(migration_count(&state_path)?, before);
        Ok(())
    }

    #[test]
    fn planning_corrupt_project_state_database_fails_without_migration(
    ) -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("setup-plan-corrupt-state")?;
        let repo_root = registered_project(runtime_home.path(), "project_corrupt")?;
        let state_path = project_state_db_path(runtime_home.path(), "project_corrupt");
        fs::write(&state_path, b"this is not sqlite")?;

        let error =
            plan_local_mcp_setup(LocalMcpSetupOptions::new(runtime_home.path(), &repo_root))
                .expect_err("corrupt database should fail planning");

        assert!(error.to_string().contains("unreadable"));
        assert_eq!(fs::read(&state_path)?, b"this is not sqlite");
        Ok(())
    }

    #[test]
    fn compatible_project_and_surface_metadata_are_preserved() -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("setup-preserve-metadata")?;
        initialize(runtime_home.path())?;
        let repo_root = repo_dir(runtime_home.path(), "product-preserve")?;
        register_project(
            runtime_home.path(),
            ProjectRegistration {
                project_id: "project_preserve".to_owned(),
                repo_root: repo_root.clone(),
                project_home: None,
                status: ACTIVE_PROJECT_STATUS.to_owned(),
                metadata_json: r#"{"project":"keep"}"#.to_owned(),
            },
        )?;
        let mut registration =
            fixed_surface_registration("project_preserve", SetupSurfaceBinding::Agent)?;
        registration.metadata_json = r#"{"surface":"keep"}"#.to_owned();
        register_surface(runtime_home.path(), registration)?;

        let plan =
            plan_local_mcp_setup(LocalMcpSetupOptions::new(runtime_home.path(), &repo_root))?;
        assert_eq!(plan.project_action.kind, SetupActionKind::Reuse);
        assert_eq!(plan.surface_actions[0].kind, SetupActionKind::Reuse);
        apply_local_mcp_setup_plan(&plan)?;

        assert_eq!(
            project_metadata(runtime_home.path(), "project_preserve")?,
            r#"{"project":"keep"}"#
        );
        assert_eq!(
            surface_metadata(
                runtime_home.path(),
                "project_preserve",
                AGENT_SURFACE_ID,
                AGENT_SURFACE_INSTANCE_ID
            )?,
            r#"{"surface":"keep"}"#
        );
        Ok(())
    }

    fn initialize(runtime_home: &Path) -> Result<(), Box<dyn Error>> {
        initialize_runtime_home(runtime_home, "runtime_home_test", "{}")?;
        Ok(())
    }

    fn repo_dir(runtime_home: &Path, name: &str) -> Result<PathBuf, Box<dyn Error>> {
        let repo_root = runtime_home
            .parent()
            .expect("runtime home has parent")
            .join("product-repositories")
            .join(name);
        fs::create_dir_all(&repo_root)?;
        Ok(fs::canonicalize(repo_root)?)
    }

    fn registered_project(
        runtime_home: &Path,
        project_id: &str,
    ) -> Result<PathBuf, Box<dyn Error>> {
        initialize(runtime_home)?;
        let repo_root = repo_dir(runtime_home, &format!("repo-{project_id}"))?;
        register_test_project(runtime_home, project_id, &repo_root)?;
        Ok(repo_root)
    }

    fn registered_historical_project_state(
        runtime_home: &Path,
        project_id: &str,
        version: i64,
        access_classes: &[AccessClass],
    ) -> Result<PathBuf, Box<dyn Error>> {
        let repo_root = registered_project(runtime_home, project_id)?;
        let state_path = project_state_db_path(runtime_home, project_id);
        fs::remove_file(&state_path)?;
        let mut conn = Connection::open(&state_path)?;
        create_project_state_fixture_version(&mut conn, project_id, version)?;
        insert_historical_agent_surface(&conn, project_id, access_classes)?;
        drop(conn);
        Ok(repo_root)
    }

    fn insert_historical_agent_surface(
        conn: &Connection,
        project_id: &str,
        access_classes: &[AccessClass],
    ) -> Result<(), Box<dyn Error>> {
        conn.execute(
            "INSERT INTO surfaces (
                project_id,
                surface_id,
                surface_instance_id,
                surface_kind,
                display_name,
                capability_profile_json,
                local_access_json,
                registered_at,
                metadata_json
            )
            VALUES (?1, ?2, ?3, ?4, 'Agent MCP', ?5, ?6, 't0', '{}')",
            params![
                project_id,
                AGENT_SURFACE_ID,
                AGENT_SURFACE_INSTANCE_ID,
                LOCAL_MCP_SURFACE_KIND,
                capability_profile_json(access_classes, None)?,
                local_access_json(access_classes)?
            ],
        )?;
        Ok(())
    }

    fn register_test_project(
        runtime_home: &Path,
        project_id: &str,
        repo_root: &Path,
    ) -> Result<(), Box<dyn Error>> {
        register_project(
            runtime_home,
            ProjectRegistration {
                project_id: project_id.to_owned(),
                repo_root: repo_root.to_path_buf(),
                project_home: None,
                status: ACTIVE_PROJECT_STATUS.to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;
        Ok(())
    }

    fn register_custom_surface(
        runtime_home: &Path,
        project_id: &str,
        binding: SetupSurfaceBinding,
        role: SurfaceInteractionRole,
        access_classes: &[AccessClass],
        surface_kind: &str,
        metadata_json: &str,
    ) -> Result<(), Box<dyn Error>> {
        register_surface(
            runtime_home,
            SurfaceRegistration {
                project_id: project_id.to_owned(),
                surface_id: binding.surface_id().to_owned(),
                surface_instance_id: binding.surface_instance_id().to_owned(),
                surface_kind: surface_kind.to_owned(),
                interaction_role: role,
                display_name: Some("test surface".to_owned()),
                capability_profile_json: capability_profile_json(access_classes, None)?,
                local_access_json: local_access_json(access_classes)?,
                metadata_json: metadata_json.to_owned(),
            },
        )?;
        Ok(())
    }

    fn register_unrelated_surface(
        runtime_home: &Path,
        project_id: &str,
    ) -> Result<(), Box<dyn Error>> {
        register_surface(
            runtime_home,
            SurfaceRegistration {
                project_id: project_id.to_owned(),
                surface_id: AGENT_SURFACE_ID.to_owned(),
                surface_instance_id: "other_instance".to_owned(),
                surface_kind: LOCAL_MCP_SURFACE_KIND.to_owned(),
                interaction_role: SurfaceInteractionRole::Agent,
                display_name: Some("Other Agent".to_owned()),
                capability_profile_json: capability_profile_json(&[AccessClass::ReadStatus], None)?,
                local_access_json: local_access_json(&[AccessClass::ReadStatus])?,
                metadata_json: r#"{"unrelated":true}"#.to_owned(),
            },
        )?;
        Ok(())
    }

    fn update_surface_column(
        runtime_home: &Path,
        project_id: &str,
        binding: SetupSurfaceBinding,
        column: &str,
        value: &str,
    ) -> Result<(), Box<dyn Error>> {
        let conn = Connection::open(project_state_db_path(runtime_home, project_id))?;
        let sql = match column {
            "local_access_json" => {
                "UPDATE surfaces SET local_access_json = ?4
                  WHERE project_id = ?1 AND surface_id = ?2 AND surface_instance_id = ?3"
            }
            "capability_profile_json" => {
                "UPDATE surfaces SET capability_profile_json = ?4
                  WHERE project_id = ?1 AND surface_id = ?2 AND surface_instance_id = ?3"
            }
            "metadata_json" => {
                "UPDATE surfaces SET metadata_json = ?4
                  WHERE project_id = ?1 AND surface_id = ?2 AND surface_instance_id = ?3"
            }
            other => panic!("unsupported surface column: {other}"),
        };
        conn.execute(
            sql,
            params![
                project_id,
                binding.surface_id(),
                binding.surface_instance_id(),
                value
            ],
        )?;
        Ok(())
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

    fn replace_project_home(
        runtime_home: &Path,
        project_id: &str,
        project_home: &Path,
    ) -> Result<(), Box<dyn Error>> {
        let state_db_path = project_home.join("state.sqlite");
        let conn = Connection::open(registry_db_path(runtime_home))?;
        conn.execute(
            "UPDATE projects
                SET project_home = ?2,
                    state_db_path = ?3
              WHERE project_id = ?1",
            params![
                project_id,
                project_home.to_string_lossy().as_ref(),
                state_db_path.to_string_lossy().as_ref()
            ],
        )?;
        Ok(())
    }

    fn project_status(runtime_home: &Path, project_id: &str) -> Result<String, Box<dyn Error>> {
        Ok(Connection::open(registry_db_path(runtime_home))?.query_row(
            "SELECT status FROM projects WHERE project_id = ?1",
            params![project_id],
            |row| row.get(0),
        )?)
    }

    fn project_metadata(runtime_home: &Path, project_id: &str) -> Result<String, Box<dyn Error>> {
        Ok(Connection::open(registry_db_path(runtime_home))?.query_row(
            "SELECT metadata_json FROM projects WHERE project_id = ?1",
            params![project_id],
            |row| row.get(0),
        )?)
    }

    fn surface_metadata(
        runtime_home: &Path,
        project_id: &str,
        surface_id: &str,
        surface_instance_id: &str,
    ) -> Result<String, Box<dyn Error>> {
        Ok(
            Connection::open(project_state_db_path(runtime_home, project_id))?.query_row(
                "SELECT metadata_json
               FROM surfaces
              WHERE project_id = ?1
                AND surface_id = ?2
                AND surface_instance_id = ?3",
                params![project_id, surface_id, surface_instance_id],
                |row| row.get(0),
            )?,
        )
    }

    fn conflict_kinds(plan: &LocalMcpSetupPlan) -> Vec<SetupConflictKind> {
        plan.conflicts
            .iter()
            .map(|conflict| conflict.kind.clone())
            .collect()
    }

    fn surface_targets(plan: &LocalMcpSetupPlan) -> Vec<SetupActionTarget> {
        plan.surface_actions
            .iter()
            .map(|action| action.target)
            .collect()
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct Snapshot {
        project_count: i64,
        surface_count: i64,
        task_count: i64,
        tool_invocation_count: i64,
        state_version: i64,
        project_metadata: String,
        agent_metadata: String,
        user_metadata: String,
    }

    impl Snapshot {
        fn read(runtime_home: &Path, project_id: &str) -> Result<Self, Box<dyn Error>> {
            let registry = Connection::open(registry_db_path(runtime_home))?;
            let state = Connection::open(project_state_db_path(runtime_home, project_id))?;
            Ok(Self {
                project_count: count_registry_projects(&registry)?,
                surface_count: count_project_table(&state, "surfaces")?,
                task_count: count_project_table(&state, "tasks")?,
                tool_invocation_count: count_project_table(&state, "tool_invocations")?,
                state_version: state.query_row(
                    "SELECT state_version FROM project_state WHERE project_id = ?1",
                    params![project_id],
                    |row| row.get(0),
                )?,
                project_metadata: registry.query_row(
                    "SELECT metadata_json FROM projects WHERE project_id = ?1",
                    params![project_id],
                    |row| row.get(0),
                )?,
                agent_metadata: state.query_row(
                    "SELECT metadata_json FROM surfaces
                      WHERE project_id = ?1 AND surface_id = ?2 AND surface_instance_id = ?3",
                    params![project_id, AGENT_SURFACE_ID, AGENT_SURFACE_INSTANCE_ID],
                    |row| row.get(0),
                )?,
                user_metadata: state.query_row(
                    "SELECT metadata_json FROM surfaces
                      WHERE project_id = ?1 AND surface_id = ?2 AND surface_instance_id = ?3",
                    params![
                        project_id,
                        USER_INTERACTION_SURFACE_ID,
                        USER_INTERACTION_SURFACE_INSTANCE_ID
                    ],
                    |row| row.get(0),
                )?,
            })
        }
    }

    fn count_registry_projects(conn: &Connection) -> Result<i64, Box<dyn Error>> {
        Ok(conn.query_row("SELECT COUNT(*) FROM projects", [], |row| row.get(0))?)
    }

    fn count_project_table(conn: &Connection, table: &str) -> Result<i64, Box<dyn Error>> {
        let sql = match table {
            "surfaces" => "SELECT COUNT(*) FROM surfaces",
            "tasks" => "SELECT COUNT(*) FROM tasks",
            "tool_invocations" => "SELECT COUNT(*) FROM tool_invocations",
            other => panic!("unsupported count table: {other}"),
        };
        Ok(conn.query_row(sql, [], |row| row.get(0))?)
    }

    fn migration_count(path: &Path) -> Result<i64, Box<dyn Error>> {
        let conn = open_read_only_database(path)?;
        Ok(conn.query_row(
            "SELECT COUNT(*)
               FROM schema_migrations
              WHERE database_kind = ?1",
            params![PROJECT_STATE_DATABASE_KIND],
            |row| row.get(0),
        )?)
    }

    fn registry_migration_count(path: &Path) -> Result<i64, Box<dyn Error>> {
        let conn = open_read_only_database(path)?;
        Ok(conn.query_row(
            "SELECT COUNT(*)
               FROM schema_migrations
              WHERE database_kind = ?1",
            params![REGISTRY_DATABASE_KIND],
            |row| row.get(0),
        )?)
    }

    #[derive(Debug)]
    struct FakeStoreError;

    impl fmt::Display for FakeStoreError {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("planned user surface failure")
        }
    }

    impl Error for FakeStoreError {}

    struct FailingUserSurfaceStore;

    impl SetupStore for FailingUserSurfaceStore {
        fn initialize_runtime_home(&mut self, _runtime_home: &Path) -> Result<(), Box<dyn Error>> {
            Ok(())
        }

        fn register_project(
            &mut self,
            _runtime_home: &Path,
            _registration: ProjectRegistration,
        ) -> Result<(), Box<dyn Error>> {
            Ok(())
        }

        fn register_surface(
            &mut self,
            _runtime_home: &Path,
            registration: SurfaceRegistration,
        ) -> Result<(), Box<dyn Error>> {
            if registration.surface_id == USER_INTERACTION_SURFACE_ID {
                Err(Box::new(FakeStoreError))
            } else {
                Ok(())
            }
        }
    }
}
