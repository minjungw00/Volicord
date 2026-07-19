use std::{
    collections::BTreeMap,
    fmt,
    fs::{self, File},
    io::{self, Read},
    path::Path,
};

use serde::{
    de::{self, MapAccess, SeqAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};
use serde_json::{Number, Value};
use volicord_store::{
    agent_connections::{
        agent_connection_project_access_read_only, agent_connection_record_read_only,
    },
    core_pipeline::CoreProjectStore,
    guards::guard_installation,
    runtime_home::{resolve_runtime_home, RuntimeHomeResolutionError},
    workflow_records::{
        task_policy_control_reevaluation, ProjectWorkflowPolicyAuthorityApply,
        ProjectWorkflowPolicyRecord,
    },
    StoreError,
};
use volicord_types::{
    canonical_json_sha256, canonical_json_string, guard_manifest_from_json, AcceptancePolicy,
    GuardManagedArtifact, ProjectId, RequestedControlLevel, TaskControlLevel, TaskMode,
};

use crate::{
    cli::{PolicyApplyArgs, PolicyArgs, PolicyCommand, PolicyShowArgs, PolicyValidateArgs},
    guard_integration::{
        files::{
            plan_policy_file, write_managed_file_if_fresh, FilePlanStatus, VOLICORD_POLICY_SCHEMA,
        },
        policy::{validate_workflow_policy, PolicyValidationIssue},
    },
    project_context::{registered_project_for_repo, resolve_repository_root, ProjectCommandError},
};

pub const MAX_POLICY_FILE_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyCommandError {
    Usage(String),
    Validation {
        code: String,
        field_path: String,
        message: String,
    },
    FailureOutput(String),
    Runtime(String),
}

impl fmt::Display for PolicyCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(message) | Self::FailureOutput(message) | Self::Runtime(message) => {
                formatter.write_str(message)
            }
            Self::Validation {
                code,
                field_path,
                message,
            } => write!(formatter, "{code} at {field_path}: {message}"),
        }
    }
}

impl std::error::Error for PolicyCommandError {}

impl From<StoreError> for PolicyCommandError {
    fn from(error: StoreError) -> Self {
        Self::Runtime(error.to_string())
    }
}

impl From<RuntimeHomeResolutionError> for PolicyCommandError {
    fn from(error: RuntimeHomeResolutionError) -> Self {
        Self::Runtime(error.to_string())
    }
}

impl From<ProjectCommandError> for PolicyCommandError {
    fn from(error: ProjectCommandError) -> Self {
        match error {
            ProjectCommandError::Usage(message) => Self::Usage(message),
            ProjectCommandError::Runtime(message) => Self::Runtime(message),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ValidatedPolicyFile {
    pub(crate) value: Value,
    pub(crate) canonical_json: String,
    pub(crate) fingerprint: String,
}

pub fn run_policy_command<F>(
    args: PolicyArgs,
    env_var: F,
    current_dir: &Path,
) -> Result<String, PolicyCommandError>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    match args.command {
        PolicyCommand::Validate(options) => validate_command(options, current_dir),
        PolicyCommand::Show(options) => show_command(options, &env_var, current_dir),
        PolicyCommand::Apply(options) => apply_command(options, &env_var, current_dir),
    }
}

#[derive(Debug, Clone, Serialize)]
struct PolicyAction {
    action: &'static str,
    command: &'static str,
    arguments: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct PolicyShowReport {
    schema: String,
    policy_version: u64,
    policy_fingerprint: String,
    source: String,
    managed_file_schema: Option<String>,
    managed_file_fingerprint: Option<String>,
    file_matches_authority: bool,
    file_status: String,
    active_task_requires_escalation: bool,
    actions: Vec<PolicyAction>,
}

#[derive(Debug, Clone, Serialize)]
struct PolicyApplyReport {
    status: &'static str,
    changed: bool,
    database_changed: bool,
    file_changed: bool,
    restart_required: bool,
    schema: String,
    prior_policy_version: Option<u64>,
    resulting_policy_version: u64,
    prior_policy_fingerprint: Option<String>,
    resulting_policy_fingerprint: String,
    file_matches_authority: bool,
    file_status: String,
    active_task_requires_escalation: bool,
    active_task_requires_policy_reevaluation: bool,
    write_authority_changed: bool,
    prior_write_authority_fingerprint: String,
    resulting_write_authority_fingerprint: String,
    affected_task_ids: Vec<String>,
    invalidated_write_ticket_ids: Vec<String>,
    actions: Vec<PolicyAction>,
}

#[derive(Debug, Clone)]
struct ManagedPolicyFileState {
    schema: Option<String>,
    fingerprint: Option<String>,
    matches_authority: bool,
    status: String,
}

fn show_command<F>(
    options: PolicyShowArgs,
    env_var: &F,
    current_dir: &Path,
) -> Result<String, PolicyCommandError>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    let runtime_home = resolve_runtime_home(env_var, current_dir)?;
    let repo_root = resolve_repository_root(current_dir, Some(&options.repo))?;
    let project = registered_project_for_repo(&runtime_home, &repo_root)?;
    let store = CoreProjectStore::open_read_only(
        &runtime_home,
        &ProjectId::new(project.project_id.clone()),
    )?;
    let authority = store.project_workflow_policy()?.ok_or_else(|| {
        PolicyCommandError::Runtime(
            "PROJECT_POLICY_MISSING: the registered project has no authoritative workflow policy; rerun `volicord init` for this Product Repository"
                .to_owned(),
        )
    })?;
    let authority_value = authority_policy_value(&authority)?;
    let managed_path = GuardManagedArtifact::VolicordPolicy
        .expected_path(&repo_root, None)
        .expect("the Guard policy has a repository-owned path");
    let file_state = managed_policy_file_state(&managed_path, &authority_value, &authority)?;
    let active_task_requires_escalation =
        active_task_requires_escalation(&store, &authority_value)?;
    let actions = if file_state.matches_authority {
        Vec::new()
    } else {
        vec![policy_repair_action(&repo_root, None)]
    };
    render_json(&PolicyShowReport {
        schema: authority.policy_schema,
        policy_version: authority.policy_version,
        policy_fingerprint: authority.policy_fingerprint,
        source: "project_database".to_owned(),
        managed_file_schema: file_state.schema,
        managed_file_fingerprint: file_state.fingerprint,
        file_matches_authority: file_state.matches_authority,
        file_status: file_state.status,
        active_task_requires_escalation,
        actions,
    })
}

fn apply_command<F>(
    options: PolicyApplyArgs,
    env_var: &F,
    current_dir: &Path,
) -> Result<String, PolicyCommandError>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    let input_path = if options.file.is_absolute() {
        options.file.clone()
    } else {
        current_dir.join(&options.file)
    };
    let candidate = read_validated_policy_file(&input_path)?;
    let runtime_home = resolve_runtime_home(env_var, current_dir)?;
    let repo_root = resolve_repository_root(current_dir, Some(&options.repo))?;
    let project = registered_project_for_repo(&runtime_home, &repo_root)?;
    validate_policy_bindings(
        &candidate.value,
        &runtime_home,
        &repo_root,
        &project.project_id,
    )?;

    let project_id = ProjectId::new(project.project_id.clone());
    let mut store = CoreProjectStore::open(&runtime_home, &project_id)?;
    let prior = store.project_workflow_policy()?;
    if let Some(prior) = &prior {
        let prior_value = authority_policy_value(prior)?;
        if !policy_bindings_match(&candidate.value, &prior_value) {
            return Err(validation_error(
                "POLICY_BINDING_MISMATCH",
                "$",
                "policy apply may change workflow policy but must retain the authoritative repository, connection, MCP, and host-hook bindings",
            ));
        }
    }
    let database_changed = prior
        .as_ref()
        .is_none_or(|record| record.policy_fingerprint != candidate.fingerprint);
    let requested_version = if database_changed {
        prior.as_ref().map_or(Ok(1), |record| {
            record.policy_version.checked_add(1).ok_or_else(|| {
                PolicyCommandError::Runtime(
                    "project workflow policy version is exhausted".to_owned(),
                )
            })
        })?
    } else {
        prior
            .as_ref()
            .expect("matching fingerprint requires prior authority")
            .policy_version
    };
    let authority_apply =
        store.apply_project_workflow_policy_authority(ProjectWorkflowPolicyAuthorityApply {
            policy_version: requested_version,
            policy_json: candidate.canonical_json.clone(),
            policy_fingerprint: candidate.fingerprint.clone(),
            source: "project_database".to_owned(),
            expected_prior_fingerprint: prior
                .as_ref()
                .map(|record| record.policy_fingerprint.clone()),
        })?;
    let database_changed = authority_apply.database_changed;
    let resulting_version = authority_apply.policy.policy_version;
    let active_task_requires_escalation = authority_apply.active_task_requires_escalation;
    let managed_path = GuardManagedArtifact::VolicordPolicy
        .expected_path(&repo_root, None)
        .expect("the Guard policy has a repository-owned path");
    let file_apply =
        plan_policy_file(&repo_root, &managed_path, &candidate.value).and_then(|plan| {
            let changed = plan.status != FilePlanStatus::Unchanged;
            write_managed_file_if_fresh(&plan, &plan.content, false)?;
            Ok(changed)
        });
    let (status, file_changed, actions) = match file_apply {
        Ok(file_changed) => ("complete", file_changed, Vec::new()),
        Err(_error) => (
            "failed",
            false,
            vec![policy_repair_action(&repo_root, Some(&input_path))],
        ),
    };
    let file_state =
        managed_policy_file_state(&managed_path, &candidate.value, &authority_apply.policy)?;
    let report = PolicyApplyReport {
        status,
        changed: database_changed || file_changed,
        database_changed,
        file_changed,
        restart_required: database_changed,
        schema: VOLICORD_POLICY_SCHEMA.to_owned(),
        prior_policy_version: prior.as_ref().map(|record| record.policy_version),
        resulting_policy_version: resulting_version,
        prior_policy_fingerprint: prior
            .as_ref()
            .map(|record| record.policy_fingerprint.clone()),
        resulting_policy_fingerprint: candidate.fingerprint,
        file_matches_authority: file_state.matches_authority,
        file_status: file_state.status,
        active_task_requires_escalation,
        active_task_requires_policy_reevaluation: authority_apply
            .active_task_requires_policy_reevaluation,
        write_authority_changed: authority_apply.write_authority_changed,
        prior_write_authority_fingerprint: authority_apply.prior_write_authority_fingerprint,
        resulting_write_authority_fingerprint: authority_apply
            .resulting_write_authority_fingerprint,
        affected_task_ids: authority_apply.affected_task_ids,
        invalidated_write_ticket_ids: authority_apply.invalidated_write_ticket_ids,
        actions,
    };
    let output = render_json(&report)?;
    if status == "failed" {
        Err(PolicyCommandError::FailureOutput(output))
    } else {
        Ok(output)
    }
}

fn authority_policy_value(
    authority: &ProjectWorkflowPolicyRecord,
) -> Result<Value, PolicyCommandError> {
    if authority.policy_schema != VOLICORD_POLICY_SCHEMA {
        return Err(corrupt_policy_authority());
    }
    let value = serde_json::from_str::<Value>(&authority.policy_json)
        .map_err(|_| corrupt_policy_authority())?;
    validate_workflow_policy(&value, None).map_err(|_| corrupt_policy_authority())?;
    let canonical = canonical_json_string(&value).map_err(|_| corrupt_policy_authority())?;
    let fingerprint = canonical_json_sha256(&value)
        .map_err(|_| corrupt_policy_authority())?
        .into_inner();
    if canonical != authority.policy_json || fingerprint != authority.policy_fingerprint {
        return Err(corrupt_policy_authority());
    }
    Ok(value)
}

fn corrupt_policy_authority() -> PolicyCommandError {
    PolicyCommandError::Runtime(
        "PROJECT_POLICY_CORRUPT: authoritative project workflow policy is malformed or has an inconsistent fingerprint"
            .to_owned(),
    )
}

fn validate_policy_bindings(
    policy: &Value,
    runtime_home: &Path,
    repo_root: &Path,
    project_id: &str,
) -> Result<(), PolicyCommandError> {
    let policy_repo = policy["repo_root"]
        .as_str()
        .expect("strict policy validation requires repo_root");
    let canonical_policy_repo = fs::canonicalize(policy_repo).map_err(|_| {
        validation_error(
            "POLICY_BINDING_MISMATCH",
            "$.repo_root",
            "policy repo_root does not identify the selected Product Repository",
        )
    })?;
    if canonical_policy_repo != repo_root {
        return Err(validation_error(
            "POLICY_BINDING_MISMATCH",
            "$.repo_root",
            "policy repo_root does not identify the selected Product Repository",
        ));
    }

    let connection_id = policy["connection_id"]
        .as_str()
        .expect("strict policy validation requires connection_id");
    let connection = agent_connection_record_read_only(runtime_home, connection_id)?
        .ok_or_else(|| binding_mismatch("$.connection_id", "policy connection is not recorded"))?;
    let access =
        agent_connection_project_access_read_only(runtime_home, connection_id, project_id)?
            .ok_or_else(|| {
                binding_mismatch("$.connection_id", "policy connection is not recorded")
            })?;
    if !access.connection_enabled || !access.project_allowed {
        return Err(binding_mismatch(
            "$.connection_id",
            "policy connection is not enabled and allowlisted for the selected project",
        ));
    }
    if public_store_host(&connection.host_kind) != policy["host"].as_str().expect("validated host")
    {
        return Err(binding_mismatch(
            "$.host",
            "policy host does not match the recorded Agent Connection",
        ));
    }
    if connection.intent
        != policy["connection_intent"]
            .as_str()
            .expect("validated connection intent")
    {
        return Err(binding_mismatch(
            "$.connection_intent",
            "policy connection intent does not match the recorded Agent Connection",
        ));
    }

    let guard_installation_id = policy["guard_installation_id"]
        .as_str()
        .expect("strict policy validation requires guard_installation_id");
    let guard = guard_installation(runtime_home, guard_installation_id)?.ok_or_else(|| {
        binding_mismatch(
            "$.guard_installation_id",
            "policy guard installation is not recorded",
        )
    })?;
    let manifest = guard_manifest_from_json(&guard.manifest_json).map_err(|_| {
        binding_mismatch(
            "$.guard_installation_id",
            "recorded Guard manifest is malformed",
        )
    })?;
    if guard.connection_internal_id != connection_id || guard.project_id != project_id {
        return Err(binding_mismatch(
            "$.guard_installation_id",
            "policy guard installation is not bound to the selected connection and project",
        ));
    }
    if manifest.host_kind.as_str() != policy["host"].as_str().expect("validated host") {
        return Err(binding_mismatch(
            "$.host",
            "policy host does not match the recorded guard installation",
        ));
    }
    if manifest.integration_profile.as_str()
        != policy["selected_profile"]
            .as_str()
            .expect("validated selected profile")
    {
        return Err(binding_mismatch(
            "$.selected_profile",
            "policy profile does not match the recorded guard installation",
        ));
    }
    Ok(())
}

fn public_store_host(value: &str) -> &str {
    value
}

fn binding_mismatch(field_path: &'static str, message: &'static str) -> PolicyCommandError {
    validation_error("POLICY_BINDING_MISMATCH", field_path, message)
}

fn policy_bindings_match(candidate: &Value, authority: &Value) -> bool {
    let mut candidate = candidate.clone();
    let mut authority = authority.clone();
    candidate
        .as_object_mut()
        .expect("validated policy is an object")
        .remove("workflow");
    authority
        .as_object_mut()
        .expect("validated policy is an object")
        .remove("workflow");
    candidate == authority
}

fn managed_policy_file_state(
    path: &Path,
    authority_value: &Value,
    authority: &ProjectWorkflowPolicyRecord,
) -> Result<ManagedPolicyFileState, PolicyCommandError> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(ManagedPolicyFileState {
                schema: None,
                fingerprint: None,
                matches_authority: false,
                status: "missing".to_owned(),
            });
        }
        Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {
            return Ok(ManagedPolicyFileState {
                schema: None,
                fingerprint: None,
                matches_authority: false,
                status: "permission_failure".to_owned(),
            });
        }
        Err(error) => return Err(policy_file_io(path, error)),
        Ok(_) => {}
    }
    let file = match read_validated_policy_file(path) {
        Ok(file) => file,
        Err(PolicyCommandError::Validation { .. }) => {
            return Ok(ManagedPolicyFileState {
                schema: None,
                fingerprint: None,
                matches_authority: false,
                status: "malformed".to_owned(),
            });
        }
        Err(PolicyCommandError::Runtime(message))
            if message.starts_with("POLICY_FILE_ACCESS_FAILED:") =>
        {
            return Ok(ManagedPolicyFileState {
                schema: None,
                fingerprint: None,
                matches_authority: false,
                status: "permission_failure".to_owned(),
            });
        }
        Err(error) => return Err(error),
    };
    let schema = file.value["schema"].as_str().map(str::to_owned);
    if policy_permissions_are_too_open(path)? {
        return Ok(ManagedPolicyFileState {
            schema,
            fingerprint: Some(file.fingerprint),
            matches_authority: false,
            status: "permission_failure".to_owned(),
        });
    }
    if !policy_bindings_match(&file.value, authority_value) {
        return Ok(ManagedPolicyFileState {
            schema,
            fingerprint: Some(file.fingerprint),
            matches_authority: false,
            status: "binding_mismatch".to_owned(),
        });
    }
    let matches_authority = file.fingerprint == authority.policy_fingerprint;
    Ok(ManagedPolicyFileState {
        schema,
        fingerprint: Some(file.fingerprint),
        matches_authority,
        status: if matches_authority {
            "matches".to_owned()
        } else {
            "fingerprint_mismatch".to_owned()
        },
    })
}

#[cfg(unix)]
fn policy_permissions_are_too_open(path: &Path) -> Result<bool, PolicyCommandError> {
    use std::os::unix::fs::PermissionsExt;

    fs::metadata(path)
        .map(|metadata| metadata.permissions().mode() & 0o077 != 0)
        .map_err(|error| policy_file_io(path, error))
}

#[cfg(not(unix))]
fn policy_permissions_are_too_open(_path: &Path) -> Result<bool, PolicyCommandError> {
    Ok(false)
}

fn active_task_requires_escalation(
    store: &CoreProjectStore,
    policy: &Value,
) -> Result<bool, PolicyCommandError> {
    let Some(task) = store.active_task_record()? else {
        return Ok(false);
    };
    let mode = parse_mode(&task.mode)?;
    let requested = parse_requested_control(&task.requested_control_level)?;
    let current = parse_control(&task.effective_control_level)?;
    let current_acceptance = parse_acceptance(&task.acceptance_policy)?;
    if let Some(mark) = task_policy_control_reevaluation(&task)? {
        if parse_control(&mark.required_effective_control_level)? > current {
            return Ok(true);
        }
        if mark
            .required_acceptance_policy
            .as_deref()
            .map(parse_acceptance)
            .transpose()?
            .is_some_and(|required| acceptance_rank(required) > acceptance_rank(current_acceptance))
        {
            return Ok(true);
        }
    }
    let workflow = &policy["workflow"];
    let direct_default = parse_control(
        workflow["default_direct_control"]
            .as_str()
            .expect("validated direct default"),
    )?;
    let work_default = parse_control(
        workflow["default_work_control"]
            .as_str()
            .expect("validated work default"),
    )?;
    let requested_level = match requested {
        RequestedControlLevel::Auto => match mode {
            TaskMode::Advisor => TaskControlLevel::Observe,
            TaskMode::Direct => direct_default,
            TaskMode::Work => work_default,
        },
        RequestedControlLevel::Observe => TaskControlLevel::Observe,
        RequestedControlLevel::Light => TaskControlLevel::Light,
        RequestedControlLevel::Tracked => TaskControlLevel::Tracked,
        RequestedControlLevel::Sensitive => TaskControlLevel::Sensitive,
    };
    let project_minimum = match mode {
        TaskMode::Advisor => TaskControlLevel::Observe,
        TaskMode::Direct => direct_default,
        TaskMode::Work => std::cmp::max(work_default, TaskControlLevel::Tracked),
    };
    let mut required = std::cmp::max(requested_level, project_minimum);
    if required == TaskControlLevel::Light
        && !workflow["light"]["enabled"]
            .as_bool()
            .expect("validated Light enabled")
    {
        required = TaskControlLevel::Tracked;
    }
    let required_acceptance = match required {
        TaskControlLevel::Observe => AcceptancePolicy::NotRequired,
        TaskControlLevel::Light => parse_acceptance(
            workflow["light"]["final_acceptance"]
                .as_str()
                .expect("validated Light final acceptance"),
        )?,
        TaskControlLevel::Tracked | TaskControlLevel::Sensitive => AcceptancePolicy::Required,
    };
    Ok(required > current
        || acceptance_rank(required_acceptance) > acceptance_rank(current_acceptance))
}

fn parse_mode(value: &str) -> Result<TaskMode, PolicyCommandError> {
    serde_json::from_value(Value::String(value.to_owned())).map_err(|_| corrupt_active_task("mode"))
}

fn parse_requested_control(value: &str) -> Result<RequestedControlLevel, PolicyCommandError> {
    serde_json::from_value(Value::String(value.to_owned()))
        .map_err(|_| corrupt_active_task("requested_control_level"))
}

fn parse_control(value: &str) -> Result<TaskControlLevel, PolicyCommandError> {
    serde_json::from_value(Value::String(value.to_owned()))
        .map_err(|_| corrupt_active_task("effective_control_level"))
}

fn parse_acceptance(value: &str) -> Result<AcceptancePolicy, PolicyCommandError> {
    serde_json::from_value(Value::String(value.to_owned()))
        .map_err(|_| corrupt_active_task("acceptance_policy"))
}

fn acceptance_rank(policy: AcceptancePolicy) -> u8 {
    match policy {
        AcceptancePolicy::NotRequired => 0,
        AcceptancePolicy::PolicyDependent => 1,
        AcceptancePolicy::Required => 2,
    }
}

fn corrupt_active_task(field: &str) -> PolicyCommandError {
    PolicyCommandError::Runtime(format!(
        "PROJECT_TASK_CORRUPT: active Task has an invalid {field}"
    ))
}

fn policy_repair_action(repo_root: &Path, input: Option<&Path>) -> PolicyAction {
    PolicyAction {
        action: "repair_managed_policy",
        command: "volicord policy apply",
        arguments: vec![
            "--repo".to_owned(),
            repo_root.display().to_string(),
            "--file".to_owned(),
            input
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "<validated-policy-file>".to_owned()),
            "--json".to_owned(),
        ],
    }
}

fn render_json<T: Serialize>(value: &T) -> Result<String, PolicyCommandError> {
    serde_json::to_string_pretty(value)
        .map(|output| format!("{output}\n"))
        .map_err(|error| PolicyCommandError::Runtime(error.to_string()))
}

fn validate_command(
    options: PolicyValidateArgs,
    current_dir: &Path,
) -> Result<String, PolicyCommandError> {
    let file = options.file;
    let file = if file.is_absolute() {
        file
    } else {
        current_dir.join(file)
    };
    let policy = read_validated_policy_file(&file)?;
    Ok(format!(
        "Policy schema: {VOLICORD_POLICY_SCHEMA}\nPolicy fingerprint: {}\n",
        policy.fingerprint
    ))
}

pub(crate) fn read_validated_policy_file(
    path: &Path,
) -> Result<ValidatedPolicyFile, PolicyCommandError> {
    let path_metadata = fs::symlink_metadata(path).map_err(|error| policy_file_io(path, error))?;
    if !path_metadata.file_type().is_file() {
        return Err(validation_error(
            "POLICY_FILE_NOT_REGULAR",
            "$",
            "policy input must be one regular file",
        ));
    }
    if path_metadata.len() > MAX_POLICY_FILE_BYTES {
        return Err(validation_error(
            "POLICY_FILE_TOO_LARGE",
            "$",
            format!("policy input exceeds the {MAX_POLICY_FILE_BYTES}-byte limit"),
        ));
    }

    let file = File::open(path).map_err(|error| policy_file_io(path, error))?;
    let opened_metadata = file
        .metadata()
        .map_err(|error| policy_file_io(path, error))?;
    if !opened_metadata.is_file() {
        return Err(validation_error(
            "POLICY_FILE_NOT_REGULAR",
            "$",
            "policy input must be one regular file",
        ));
    }
    let mut bytes = Vec::with_capacity(
        usize::try_from(opened_metadata.len().min(MAX_POLICY_FILE_BYTES)).unwrap_or(0),
    );
    file.take(MAX_POLICY_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| policy_file_io(path, error))?;
    if bytes.len() as u64 > MAX_POLICY_FILE_BYTES {
        return Err(validation_error(
            "POLICY_FILE_TOO_LARGE",
            "$",
            format!("policy input exceeds the {MAX_POLICY_FILE_BYTES}-byte limit"),
        ));
    }

    let strict = serde_json::from_slice::<StrictJsonValue>(&bytes).map_err(|error| {
        validation_error(
            "POLICY_JSON_MALFORMED",
            "$",
            format!("policy input is not strict JSON: {error}"),
        )
    })?;
    let value = strict.0;
    validate_workflow_policy(&value, None).map_err(validation_issue_error)?;
    let canonical_json = canonical_json_string(&value).map_err(|error| {
        PolicyCommandError::Runtime(format!("policy canonicalization failed: {error}"))
    })?;
    let fingerprint = canonical_json_sha256(&value)
        .map_err(|error| {
            PolicyCommandError::Runtime(format!("policy fingerprinting failed: {error}"))
        })?
        .into_inner();
    Ok(ValidatedPolicyFile {
        value,
        canonical_json,
        fingerprint,
    })
}

fn validation_issue_error(issue: PolicyValidationIssue) -> PolicyCommandError {
    validation_error(issue.code, issue.field_path, issue.message)
}

fn validation_error(
    code: impl Into<String>,
    field_path: impl Into<String>,
    message: impl Into<String>,
) -> PolicyCommandError {
    PolicyCommandError::Validation {
        code: code.into(),
        field_path: field_path.into(),
        message: message.into(),
    }
}

fn policy_file_io(path: &Path, error: io::Error) -> PolicyCommandError {
    PolicyCommandError::Runtime(format!(
        "POLICY_FILE_ACCESS_FAILED: could not read policy file {}: {error}",
        path.display()
    ))
}

struct StrictJsonValue(Value);

impl<'de> Deserialize<'de> for StrictJsonValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(StrictJsonVisitor)
    }
}

struct StrictJsonVisitor;

impl<'de> Visitor<'de> for StrictJsonVisitor {
    type Value = StrictJsonValue;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON value without duplicate object fields")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(StrictJsonValue(Value::Bool(value)))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(StrictJsonValue(Value::Number(Number::from(value))))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(StrictJsonValue(Value::Number(Number::from(value))))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Number::from_f64(value)
            .map(Value::Number)
            .map(StrictJsonValue)
            .ok_or_else(|| E::custom("JSON number is not finite"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_string(value.to_owned())
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(StrictJsonValue(Value::String(value)))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(StrictJsonValue(Value::Null))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(StrictJsonValue(Value::Null))
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        while let Some(value) = sequence.next_element::<StrictJsonValue>()? {
            values.push(value.0);
        }
        Ok(StrictJsonValue(Value::Array(values)))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut values = BTreeMap::new();
        while let Some((key, value)) = map.next_entry::<String, StrictJsonValue>()? {
            if values.insert(key.clone(), value.0).is_some() {
                return Err(de::Error::custom(format!("duplicate object field {key}")));
            }
        }
        Ok(StrictJsonValue(Value::Object(values.into_iter().collect())))
    }
}
