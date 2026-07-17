use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use volicord_store::{
    agent_connections::{AgentConnectionRecord, ConnectionProjectRecord},
    core_pipeline::CoreProjectStore,
    guards::GuardInstallationRecord,
    inspection::{
        AgentConnectionInspectionRecord, GuardInstallationInspectionRecord, ProjectInspectionRecord,
    },
};
use volicord_types::{
    host_hook_capability_matches_owner_binding, HostHookCapabilityOwnerBinding, ProjectId,
};

use crate::host_integration::{
    contracts::{
        contract_for, hook_event_for_phase, validate_contract_config, HostContractConfigKind,
    },
    HostIntegrationFileKind, HostKind, HostLifecyclePhase, MANAGED_WRAPPER_ENV,
    MANAGED_WRAPPER_VALUE, REQUIRED_GUARD_PHASES,
};

use super::{
    git_exclude::git_exclude_path,
    host_hook_capability_has_exact_current_shape,
    policy::{required_guard_phase_names, validate_policy_schema},
    HOST_HOOK_CAPABILITY_SCHEMA,
};

pub(crate) const HOOK_WRAPPER_MARKER: &str = "VOLICORD_MANAGED_HOOK_WRAPPER";
pub(crate) const CODEX_DISPATCH_WRAPPER: &str = ".codex/hooks/volicord-dispatch.sh";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HookWrapperResolutionStatus {
    Ok,
    PolicyHashMismatch,
    HostOutputMismatch,
    AuthorityMismatch,
    MetadataMissing,
}

impl HookWrapperResolutionStatus {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::PolicyHashMismatch => "policy_hash_mismatch",
            Self::HostOutputMismatch => "host_output_mismatch",
            Self::AuthorityMismatch => "authority_mismatch",
            Self::MetadataMissing => "metadata_missing",
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct GuardFileFindings {
    pub(crate) missing_files: Vec<String>,
    pub(crate) stale_files: Vec<String>,
    pub(crate) broken_files: Vec<String>,
    pub(crate) file_kind_states: BTreeMap<String, String>,
    pub(crate) guard_profiles: Vec<String>,
    pub(crate) direct_file_write_matcher_coverage_values: Vec<bool>,
    pub(crate) missing_required_hooks: Vec<String>,
    pub(crate) hook_path_safety_statuses: Vec<String>,
    pub(crate) hook_path_safety_details: Vec<Value>,
    pub(crate) hook_cwd_independent_values: Vec<bool>,
    pub(crate) hook_subdirectory_safe_values: Vec<bool>,
    pub(crate) prompt_capture_configured: bool,
    pub(crate) prompt_capture_host_supported: bool,
    pub(crate) rule_file_supported: bool,
}

impl GuardFileFindings {
    pub(crate) fn merge(&mut self, other: GuardFileFindings) {
        self.missing_files.extend(other.missing_files);
        self.stale_files.extend(other.stale_files);
        self.broken_files.extend(other.broken_files);
        for (kind, state) in other.file_kind_states {
            self.set_kind_state_text(&kind, &state);
        }
        self.guard_profiles.extend(other.guard_profiles);
        self.direct_file_write_matcher_coverage_values
            .extend(other.direct_file_write_matcher_coverage_values);
        self.missing_required_hooks
            .extend(other.missing_required_hooks);
        self.hook_path_safety_statuses
            .extend(other.hook_path_safety_statuses);
        self.hook_path_safety_details
            .extend(other.hook_path_safety_details);
        self.hook_cwd_independent_values
            .extend(other.hook_cwd_independent_values);
        self.hook_subdirectory_safe_values
            .extend(other.hook_subdirectory_safe_values);
        self.prompt_capture_configured |= other.prompt_capture_configured;
        self.prompt_capture_host_supported |= other.prompt_capture_host_supported;
        self.rule_file_supported |= other.rule_file_supported;
    }

    pub(crate) fn sort_dedup(&mut self) {
        self.missing_files.sort();
        self.missing_files.dedup();
        self.stale_files.sort();
        self.stale_files.dedup();
        self.broken_files.sort();
        self.broken_files.dedup();
        self.guard_profiles.sort();
        self.guard_profiles.dedup();
        self.missing_required_hooks.sort();
        self.missing_required_hooks.dedup();
        self.hook_path_safety_statuses
            .sort_by_key(|status| hook_path_status_rank(status));
        self.hook_path_safety_statuses.dedup();
    }

    fn set_kind_state(&mut self, kind: HostIntegrationFileKind, state: &str) {
        self.set_kind_state_text(kind.as_str(), state);
    }

    fn set_kind_state_text(&mut self, kind: &str, state: &str) {
        let update = self
            .file_kind_states
            .get(kind)
            .is_none_or(|current| file_state_rank(state) > file_state_rank(current));
        if update {
            self.file_kind_states
                .insert(kind.to_owned(), state.to_owned());
        }
    }

    pub(crate) fn kind_state(&self, kind: HostIntegrationFileKind) -> &str {
        self.file_kind_states
            .get(kind.as_str())
            .map(String::as_str)
            .unwrap_or("not_configured")
    }

    fn record_hook_path_status(&mut self, status: HookWrapperResolutionStatus, detail: Value) {
        self.hook_path_safety_statuses
            .push(status.as_str().to_owned());
        self.hook_path_safety_details.push(detail);
        self.hook_cwd_independent_values
            .push(status == HookWrapperResolutionStatus::Ok);
        self.hook_subdirectory_safe_values
            .push(status == HookWrapperResolutionStatus::Ok);
        if status != HookWrapperResolutionStatus::Ok {
            self.stale_files
                .push("host_hook_capability_json:hook_path_safety".to_owned());
        }
    }

    pub(crate) fn rule_instruction_state(&self, guard_disabled: bool) -> String {
        if guard_disabled {
            return "not_applicable".to_owned();
        }
        let state = self.kind_state(HostIntegrationFileKind::HostRuleInstruction);
        if state != "not_configured" {
            state.to_owned()
        } else if self.rule_file_supported {
            "not_configured".to_owned()
        } else {
            "unsupported_by_host".to_owned()
        }
    }

    pub(crate) fn hook_config_state(&self, guard_disabled: bool) -> String {
        if guard_disabled {
            return "disabled".to_owned();
        }
        let state = combine_optional_file_states(
            &combine_optional_file_states(
                self.kind_state(HostIntegrationFileKind::HostHookConfig),
                self.kind_state(HostIntegrationFileKind::HostHookDispatch),
            ),
            self.kind_state(HostIntegrationFileKind::HostHookWrapper),
        );
        if state != "not_configured" {
            state
        } else if self.missing_required_hooks.is_empty() {
            "not_recorded".to_owned()
        } else {
            "missing_required_hooks".to_owned()
        }
    }

    pub(crate) fn generated_config_verified(&self) -> bool {
        self.missing_files.is_empty()
            && self.stale_files.is_empty()
            && self.broken_files.is_empty()
            && self.kind_state(HostIntegrationFileKind::VolicordPolicy) == "installed"
            && self.kind_state(HostIntegrationFileKind::HostHookConfig) == "installed"
            && matches!(
                self.kind_state(HostIntegrationFileKind::HostHookDispatch),
                "not_configured" | "installed"
            )
            && self.kind_state(HostIntegrationFileKind::HostHookWrapper) == "installed"
    }

    pub(crate) fn hook_path_safety_state(&self) -> String {
        self.hook_path_safety_statuses
            .iter()
            .filter(|status| status.as_str() != HookWrapperResolutionStatus::Ok.as_str())
            .min_by_key(|status| hook_path_status_rank(status))
            .cloned()
            .unwrap_or_else(|| {
                if self.hook_path_safety_statuses.is_empty() {
                    "not_recorded".to_owned()
                } else {
                    HookWrapperResolutionStatus::Ok.as_str().to_owned()
                }
            })
    }

    pub(crate) fn direct_file_write_matcher_coverage(&self) -> bool {
        self.generated_config_verified()
            && all_recorded_values_true(&self.direct_file_write_matcher_coverage_values)
    }
}

fn hook_path_status_rank(status: &str) -> u8 {
    match status {
        "ok" => 100,
        "metadata_missing" => 0,
        "authority_mismatch" => 1,
        "policy_hash_mismatch" => 2,
        "host_output_mismatch" => 3,
        "relative_path_unsafe" => 4,
        "absolute_path_stale" => 5,
        "placeholder_unsupported" => 6,
        "dispatch_missing" => 7,
        "wrapper_missing" => 8,
        "wrapper_not_executable" => 9,
        _ => 10,
    }
}

pub(crate) fn all_recorded_values_true(values: &[bool]) -> bool {
    !values.is_empty() && values.iter().all(|value| *value)
}

pub(crate) fn combine_optional_file_states(left: &str, right: &str) -> String {
    if file_state_rank(right) > file_state_rank(left) {
        right.to_owned()
    } else {
        left.to_owned()
    }
}

pub(crate) fn file_state_rank(value: &str) -> u8 {
    match value {
        "broken" => 8,
        "missing" => 7,
        "stale" => 6,
        "updated" | "created" => 5,
        "planned_update" | "planned_create" => 4,
        "unchanged" | "installed" => 3,
        "disabled" => 2,
        "unsupported_by_host" | "not_applicable" => 1,
        _ => 0,
    }
}

#[derive(Debug, Clone, Copy)]
struct GuardAuthorityContext<'a> {
    host_kind: &'a str,
    guard_mode: &'a str,
    guard_installation_id: &'a str,
    connection_internal_id: &'a str,
    connection_host_kind: &'a str,
    connection_intent: &'a str,
    project_repo_roots: &'a [PathBuf],
    projectless_owner: bool,
}

pub(crate) fn guard_file_findings_for_installation(
    runtime_home: &Path,
    installation: &GuardInstallationRecord,
    connection: &AgentConnectionRecord,
    projects: &[ConnectionProjectRecord],
) -> GuardFileFindings {
    let matched_projects = projects
        .iter()
        .filter(|project| {
            installation
                .project_internal_id
                .as_deref()
                .is_some_and(|id| id == project.project_internal_id)
                || (installation.project_internal_id.is_none()
                    && installation
                        .project_id
                        .as_deref()
                        .is_some_and(|id| id == project.project_id))
        })
        .collect::<Vec<_>>();
    let project_repo_roots = if matched_projects.len() == 1 {
        vec![matched_projects[0].project.repo_root.clone()]
    } else {
        Vec::new()
    };
    let context = GuardAuthorityContext {
        host_kind: &installation.host_kind,
        guard_mode: &installation.guard_mode,
        guard_installation_id: &installation.guard_installation_id,
        connection_internal_id: &connection.connection_internal_id,
        connection_host_kind: &connection.host_kind,
        connection_intent: &connection.intent,
        project_repo_roots: &project_repo_roots,
        projectless_owner: installation.project_internal_id.is_none()
            && installation.project_id.is_none(),
    };
    let mut findings =
        guard_file_findings_with_context(&installation.host_capability_json, Some(context));
    if let [project] = matched_projects.as_slice() {
        audit_authoritative_project_policy(
            runtime_home,
            &project.project.project_id,
            &project.project.repo_root,
            &connection.intent,
            &mut findings,
        );
    }
    findings
}

fn audit_authoritative_project_policy(
    runtime_home: &Path,
    project_id: &str,
    repo_root: &Path,
    connection_intent: &str,
    findings: &mut GuardFileFindings,
) {
    let path = repo_root.join(super::files::VOLICORD_POLICY_FILE);
    let path_text = path.display().to_string();
    let valid = (|| {
        let store =
            CoreProjectStore::open_read_only(runtime_home, &ProjectId::new(project_id)).ok()?;
        let authority = store.project_workflow_policy().ok()??;
        let text = super::files::read_managed_text(repo_root, &path).ok()??;
        let policy = serde_json::from_str::<Value>(&text).ok()?;
        validate_policy_schema(&policy, connection_intent).ok()?;
        let fingerprint = policy_hash(&policy).ok()?;
        (authority.policy_schema == super::files::VOLICORD_POLICY_SCHEMA
            && fingerprint == authority.policy_fingerprint)
            .then_some(())
    })()
    .is_some();
    if !valid {
        findings.broken_files.push(path_text);
        findings.set_kind_state(HostIntegrationFileKind::VolicordPolicy, "broken");
    }
}

pub(crate) fn host_hook_capability_binding_valid_for_installation(
    installation: &GuardInstallationRecord,
    connection: &AgentConnectionRecord,
    projects: &[ConnectionProjectRecord],
) -> bool {
    let matched_repo_roots = projects
        .iter()
        .filter(|project| {
            installation
                .project_internal_id
                .as_deref()
                .is_some_and(|id| id == project.project_internal_id)
                || (installation.project_internal_id.is_none()
                    && installation
                        .project_id
                        .as_deref()
                        .is_some_and(|id| id == project.project_id))
        })
        .map(|project| project.project.repo_root.clone())
        .collect::<Vec<_>>();
    let project_repo_roots = if matched_repo_roots.len() == 1 {
        matched_repo_roots
    } else {
        Vec::new()
    };
    let context = GuardAuthorityContext {
        host_kind: &installation.host_kind,
        guard_mode: &installation.guard_mode,
        guard_installation_id: &installation.guard_installation_id,
        connection_internal_id: &connection.connection_internal_id,
        connection_host_kind: &connection.host_kind,
        connection_intent: &connection.intent,
        project_repo_roots: &project_repo_roots,
        projectless_owner: installation.project_internal_id.is_none()
            && installation.project_id.is_none(),
    };
    serde_json::from_str::<Value>(&installation.host_capability_json)
        .ok()
        .is_some_and(|value| host_hook_capability_matches_authority_context(&value, context))
}

pub(crate) fn guard_file_findings_for_inspection(
    installation: &GuardInstallationInspectionRecord,
    connection: &AgentConnectionInspectionRecord,
    projects: &[ProjectInspectionRecord],
) -> GuardFileFindings {
    let matched_repo_roots = projects
        .iter()
        .filter(|project| {
            installation
                .project_internal_id
                .as_deref()
                .is_some_and(|id| id == project.project_internal_id)
                || (installation.project_internal_id.is_none()
                    && installation
                        .project_id
                        .as_deref()
                        .is_some_and(|id| id == project.project_id))
        })
        .map(|project| project.repo_root.clone())
        .collect::<Vec<_>>();
    let project_repo_roots = if matched_repo_roots.len() == 1 {
        matched_repo_roots
    } else {
        Vec::new()
    };
    let context = GuardAuthorityContext {
        host_kind: &installation.host_kind,
        guard_mode: &installation.guard_mode,
        guard_installation_id: &installation.guard_installation_id,
        connection_internal_id: &connection.connection_internal_id,
        connection_host_kind: &connection.host_kind,
        connection_intent: &connection.intent,
        project_repo_roots: &project_repo_roots,
        projectless_owner: installation.project_internal_id.is_none()
            && installation.project_id.is_none(),
    };
    guard_file_findings_with_context(&installation.host_capability_json, Some(context))
}

pub(crate) fn host_hook_capability_binding_valid_for_inspection(
    installation: &GuardInstallationInspectionRecord,
    connection: &AgentConnectionInspectionRecord,
    projects: &[ProjectInspectionRecord],
) -> bool {
    let matched_repo_roots = projects
        .iter()
        .filter(|project| {
            installation
                .project_internal_id
                .as_deref()
                .is_some_and(|id| id == project.project_internal_id)
                || (installation.project_internal_id.is_none()
                    && installation
                        .project_id
                        .as_deref()
                        .is_some_and(|id| id == project.project_id))
        })
        .map(|project| project.repo_root.clone())
        .collect::<Vec<_>>();
    let project_repo_roots = if matched_repo_roots.len() == 1 {
        matched_repo_roots
    } else {
        Vec::new()
    };
    let context = GuardAuthorityContext {
        host_kind: &installation.host_kind,
        guard_mode: &installation.guard_mode,
        guard_installation_id: &installation.guard_installation_id,
        connection_internal_id: &connection.connection_internal_id,
        connection_host_kind: &connection.host_kind,
        connection_intent: &connection.intent,
        project_repo_roots: &project_repo_roots,
        projectless_owner: installation.project_internal_id.is_none()
            && installation.project_id.is_none(),
    };
    serde_json::from_str::<Value>(&installation.host_capability_json)
        .ok()
        .is_some_and(|value| host_hook_capability_matches_authority_context(&value, context))
}

fn host_hook_capability_matches_authority_context(
    value: &Value,
    context: GuardAuthorityContext<'_>,
) -> bool {
    let (project_repo_root, project_git_info_exclude_path) = match context.project_repo_roots {
        [repo_root] => {
            let Ok(git_info_exclude_path) = git_exclude_path(repo_root) else {
                return false;
            };
            (Some(repo_root.as_path()), git_info_exclude_path)
        }
        [] if context.projectless_owner => (None, None),
        _ => return false,
    };
    host_hook_capability_matches_owner_binding(
        value,
        HostHookCapabilityOwnerBinding {
            row_host_kind: context.host_kind,
            row_guard_mode: context.guard_mode,
            row_guard_installation_id: context.guard_installation_id,
            connection_internal_id: context.connection_internal_id,
            connection_host_kind: context.connection_host_kind,
            connection_intent: context.connection_intent,
            project_repo_root,
            project_git_info_exclude_path: project_git_info_exclude_path.as_deref(),
        },
    )
}

pub(crate) fn missing_required_hooks_from_capability_json(capability_json: &str) -> Vec<String> {
    serde_json::from_str::<Value>(capability_json)
        .ok()
        .filter(host_hook_capability_has_exact_current_shape)
        .map(|value| missing_required_hooks_from_capability(&value))
        .unwrap_or_else(|| {
            required_guard_phase_names()
                .into_iter()
                .map(str::to_owned)
                .collect()
        })
}

fn guard_file_findings_with_context(
    capability_json: &str,
    context: Option<GuardAuthorityContext<'_>>,
) -> GuardFileFindings {
    let mut findings = GuardFileFindings::default();
    let Ok(value) = serde_json::from_str::<Value>(capability_json) else {
        findings
            .broken_files
            .push("host_hook_capability_json".to_owned());
        findings.record_hook_path_status(
            HookWrapperResolutionStatus::MetadataMissing,
            json!({ "source": "host_hook_capability_json" }),
        );
        return findings;
    };
    if !record_host_hook_capability_schema(&value, &mut findings) {
        return findings;
    }
    if !host_hook_capability_has_exact_current_shape(&value) {
        findings
            .broken_files
            .push("host_hook_capability_json:shape".to_owned());
        findings.hook_path_safety_statuses.push(
            HookWrapperResolutionStatus::MetadataMissing
                .as_str()
                .to_owned(),
        );
        findings.hook_path_safety_details.push(json!({
            "source": "host_hook_capability_json",
            "reason": "invalid_shape",
            "expected_schema": HOST_HOOK_CAPABILITY_SCHEMA,
        }));
        findings.hook_cwd_independent_values.push(false);
        findings.hook_subdirectory_safe_values.push(false);
        return findings;
    }
    if context
        .is_some_and(|context| !host_hook_capability_matches_authority_context(&value, context))
    {
        findings
            .broken_files
            .push("host_hook_capability_json:binding".to_owned());
        findings.hook_path_safety_statuses.push(
            HookWrapperResolutionStatus::AuthorityMismatch
                .as_str()
                .to_owned(),
        );
        findings.hook_path_safety_details.push(json!({
            "source": "host_hook_capability_json",
            "reason": "owner_binding_mismatch",
        }));
        findings.hook_cwd_independent_values.push(false);
        findings.hook_subdirectory_safe_values.push(false);
        return findings;
    }
    findings.prompt_capture_configured = value
        .get("commands")
        .and_then(|commands| commands.get("prompt_capture"))
        .is_some_and(Value::is_object);
    findings.prompt_capture_host_supported = value
        .get("host_capabilities")
        .and_then(|capabilities| capabilities.get("user_prompt_submit_hook"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    findings.rule_file_supported = value
        .get("host_capabilities")
        .and_then(|capabilities| capabilities.get("rule_file_support"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if let Some(value) = nonempty_json_string(&value, "selected_profile") {
        findings.guard_profiles.push(value);
    }
    findings
        .direct_file_write_matcher_coverage_values
        .push(bool_json_field(
            &value,
            "direct_file_write_matcher_coverage",
        ));
    findings.missing_required_hooks = missing_required_hooks_from_capability(&value);

    let files = value
        .get("files")
        .and_then(Value::as_array)
        .into_iter()
        .flatten();
    for file in files {
        verify_guard_file(file, &value, &mut findings);
    }
    findings
}

fn host_hook_capability_schema_is_current(value: &Value) -> bool {
    value.get("schema").and_then(Value::as_str) == Some(HOST_HOOK_CAPABILITY_SCHEMA)
}

fn record_host_hook_capability_schema(value: &Value, findings: &mut GuardFileFindings) -> bool {
    if host_hook_capability_schema_is_current(value) {
        return true;
    }

    findings
        .broken_files
        .push("host_hook_capability_json:schema".to_owned());
    findings.hook_path_safety_statuses.push(
        HookWrapperResolutionStatus::MetadataMissing
            .as_str()
            .to_owned(),
    );
    findings.hook_path_safety_details.push(json!({
        "source": "host_hook_capability_json",
        "reason": "invalid_schema",
        "expected_schema": HOST_HOOK_CAPABILITY_SCHEMA,
    }));
    findings.hook_cwd_independent_values.push(false);
    findings.hook_subdirectory_safe_values.push(false);
    false
}

fn nonempty_json_string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
}

fn bool_json_field(value: &Value, key: &str) -> bool {
    value.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn missing_required_hooks_from_capability(capability: &Value) -> Vec<String> {
    let capabilities = capability.get("host_capabilities");
    required_guard_phase_names()
        .into_iter()
        .filter(|required_hook| {
            capabilities
                .and_then(|capabilities| capabilities.get(*required_hook))
                .and_then(Value::as_bool)
                != Some(true)
        })
        .map(str::to_owned)
        .collect()
}

fn verify_guard_file(file: &Value, capability: &Value, findings: &mut GuardFileFindings) {
    let kind = file
        .get("kind")
        .and_then(Value::as_str)
        .and_then(host_integration_file_kind_from_str);
    let Some(path_text) = file.get("path").and_then(Value::as_str) else {
        findings
            .broken_files
            .push("host_hook_capability_json:files.path".to_owned());
        if let Some(kind) = kind {
            findings.set_kind_state(kind, "broken");
        }
        return;
    };
    let path = Path::new(path_text);
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            findings.missing_files.push(path_text.to_owned());
            if let Some(kind) = kind {
                findings.set_kind_state(kind, "missing");
            }
            return;
        }
        Err(_) => {
            findings.broken_files.push(path_text.to_owned());
            if let Some(kind) = kind {
                findings.set_kind_state(kind, "broken");
            }
            return;
        }
    };
    let expected_hash = file
        .get("content_hash")
        .and_then(Value::as_str)
        .unwrap_or_default();
    match file.get("ownership").and_then(Value::as_str) {
        Some("managed_block") => verify_managed_block_file(file, kind, path_text, &text, findings),
        Some("managed_json") => verify_managed_json_file(
            file,
            kind,
            capability,
            path_text,
            &text,
            expected_hash,
            findings,
        ),
        Some("managed_script") => verify_managed_script_file(
            file,
            kind,
            capability,
            ManagedFileRead {
                path,
                path_text,
                text: &text,
                expected_hash,
            },
            findings,
        ),
        _ => {
            findings.broken_files.push(path_text.to_owned());
            if let Some(kind) = kind {
                findings.set_kind_state(kind, "broken");
            }
        }
    }
}

fn verify_managed_block_file(
    file: &Value,
    kind: Option<HostIntegrationFileKind>,
    path_text: &str,
    text: &str,
    findings: &mut GuardFileFindings,
) {
    let Some(start_marker) = file.get("managed_marker_start").and_then(Value::as_str) else {
        findings.broken_files.push(path_text.to_owned());
        if let Some(kind) = kind {
            findings.set_kind_state(kind, "broken");
        }
        return;
    };
    let Some(end_marker) = file.get("managed_marker_end").and_then(Value::as_str) else {
        findings.broken_files.push(path_text.to_owned());
        if let Some(kind) = kind {
            findings.set_kind_state(kind, "broken");
        }
        return;
    };
    if marker_count(text, start_marker) != 1 || marker_count(text, end_marker) != 1 {
        findings.broken_files.push(path_text.to_owned());
        if let Some(kind) = kind {
            findings.set_kind_state(kind, "broken");
        }
        return;
    }
    let Some(block) = managed_block_slice(text, start_marker, end_marker) else {
        findings.broken_files.push(path_text.to_owned());
        if let Some(kind) = kind {
            findings.set_kind_state(kind, "broken");
        }
        return;
    };
    let expected_hash = file
        .get("content_hash")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if sha256_text(block) != expected_hash {
        findings.stale_files.push(path_text.to_owned());
        if let Some(kind) = kind {
            findings.set_kind_state(kind, "stale");
        }
    } else if let Some(kind) = kind {
        findings.set_kind_state(kind, "installed");
    }
}

fn verify_managed_json_file(
    file: &Value,
    kind: Option<HostIntegrationFileKind>,
    capability: &Value,
    path_text: &str,
    text: &str,
    expected_hash: &str,
    findings: &mut GuardFileFindings,
) {
    let mut state = "installed";
    if sha256_text(text) != expected_hash {
        findings.stale_files.push(path_text.to_owned());
        state = "stale";
    }
    if file.get("kind").and_then(Value::as_str) == Some("host_hook_config") {
        match serde_json::from_str::<Value>(text) {
            Ok(value) if is_volicord_codex_hook_config(&value) => {}
            Ok(_) | Err(_) => {
                findings.broken_files.push(path_text.to_owned());
                if let Some(kind) = kind {
                    findings.set_kind_state(kind, "broken");
                }
                return;
            }
        }
        let validation =
            validate_contract_config(HostKind::Codex, HostContractConfigKind::HookConfig, text);
        if validation.is_err() {
            findings.stale_files.push(path_text.to_owned());
            state = "stale";
        }
    }
    if file.get("kind").and_then(Value::as_str) != Some("volicord_policy") {
        if let Some(kind) = kind {
            findings.set_kind_state(kind, state);
        }
        return;
    }
    let policy = match serde_json::from_str::<Value>(text) {
        Ok(policy) => policy,
        Err(_) => {
            findings.broken_files.push(path_text.to_owned());
            if let Some(kind) = kind {
                findings.set_kind_state(kind, "broken");
            }
            return;
        }
    };
    let Some(connection_intent) = capability.get("connection_intent").and_then(Value::as_str)
    else {
        findings.broken_files.push(path_text.to_owned());
        if let Some(kind) = kind {
            findings.set_kind_state(kind, "broken");
        }
        return;
    };
    if validate_policy_schema(&policy, connection_intent).is_err() {
        findings.broken_files.push(path_text.to_owned());
        if let Some(kind) = kind {
            findings.set_kind_state(kind, "broken");
        }
        return;
    }
    let expected_policy_hash = capability
        .get("policy_hash")
        .and_then(Value::as_str)
        .unwrap_or_default();
    match policy_hash(&policy) {
        Ok(actual) if actual == expected_policy_hash => {}
        Ok(_) => {
            findings.stale_files.push(path_text.to_owned());
            state = "stale";
        }
        Err(_) => {
            findings.broken_files.push(path_text.to_owned());
            if let Some(kind) = kind {
                findings.set_kind_state(kind, "broken");
            }
            return;
        }
    }
    if policy
        .get("host_hook")
        .and_then(|guard| guard.get("commands"))
        != capability.get("commands")
    {
        findings.stale_files.push(path_text.to_owned());
        state = "stale";
    }
    if let Some(kind) = kind {
        findings.set_kind_state(kind, state);
    }
}

#[derive(Clone, Copy)]
struct ManagedFileRead<'a> {
    path: &'a Path,
    path_text: &'a str,
    text: &'a str,
    expected_hash: &'a str,
}

fn verify_managed_script_file(
    file: &Value,
    kind: Option<HostIntegrationFileKind>,
    capability: &Value,
    managed: ManagedFileRead<'_>,
    findings: &mut GuardFileFindings,
) {
    let ManagedFileRead {
        path,
        path_text,
        text,
        expected_hash,
    } = managed;
    let mut state = "installed";
    if file.get("managed_marker").and_then(Value::as_str) != Some(HOOK_WRAPPER_MARKER)
        || !text
            .lines()
            .any(|line| line == format!("# {HOOK_WRAPPER_MARKER}"))
    {
        findings.broken_files.push(path_text.to_owned());
        if let Some(kind) = kind {
            findings.set_kind_state(kind, "broken");
        }
        return;
    }
    if kind == Some(HostIntegrationFileKind::HostHookDispatch) {
        verify_managed_dispatch_script_file(file, kind, managed, findings);
        return;
    }
    if !has_current_managed_process_binding(text) {
        findings.broken_files.push(path_text.to_owned());
        if let Some(kind) = kind {
            findings.set_kind_state(kind, "broken");
        }
        return;
    }
    let Some(expected_command) = file
        .get("managed_script_command")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    else {
        findings.broken_files.push(path_text.to_owned());
        if let Some(kind) = kind {
            findings.set_kind_state(kind, "broken");
        }
        return;
    };
    if hook_wrapper_exec_command(text) != Some(expected_command) {
        findings.stale_files.push(path_text.to_owned());
        state = "stale";
    }
    let expected_policy_hash = capability
        .get("policy_hash")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let expected_host_output = file
        .get("host_output")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if !generated_managed_command_shape_verified(file, expected_command) {
        findings.broken_files.push(path_text.to_owned());
        if let Some(kind) = kind {
            findings.set_kind_state(kind, "broken");
        }
        return;
    }
    if !expected_policy_hash.is_empty()
        && hook_wrapper_comment_value(text, "policy_hash") != Some(expected_policy_hash)
    {
        findings.stale_files.push(path_text.to_owned());
        findings.record_hook_path_status(
            HookWrapperResolutionStatus::PolicyHashMismatch,
            json!({ "path": path_text, "expected_policy_hash": expected_policy_hash }),
        );
        state = "stale";
    }
    if !expected_host_output.is_empty()
        && hook_wrapper_comment_value(text, "host_output") != Some(expected_host_output)
    {
        findings.stale_files.push(path_text.to_owned());
        findings.record_hook_path_status(
            HookWrapperResolutionStatus::HostOutputMismatch,
            json!({ "path": path_text, "expected_host_output": expected_host_output }),
        );
        state = "stale";
    }
    for key in [
        "host_kind",
        "phase",
        "purpose",
        "connection_id",
        "guard_installation_id",
    ] {
        let Some(expected) = file.get(key).and_then(Value::as_str) else {
            findings.broken_files.push(path_text.to_owned());
            if let Some(kind) = kind {
                findings.set_kind_state(kind, "broken");
            }
            return;
        };
        if hook_wrapper_comment_value(text, key) != Some(expected) {
            findings.stale_files.push(path_text.to_owned());
            findings.record_hook_path_status(
                HookWrapperResolutionStatus::AuthorityMismatch,
                json!({ "path": path_text, "field": key, "expected": expected }),
            );
            state = "stale";
        }
    }
    if sha256_text(text) != expected_hash {
        findings.stale_files.push(path_text.to_owned());
        state = "stale";
    }
    if file
        .get("executable_required")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        && !script_is_executable(path)
    {
        findings.stale_files.push(path_text.to_owned());
        state = "stale";
    }
    if let Some(kind) = kind {
        findings.set_kind_state(kind, state);
    }
}

fn verify_managed_dispatch_script_file(
    file: &Value,
    kind: Option<HostIntegrationFileKind>,
    managed: ManagedFileRead<'_>,
    findings: &mut GuardFileFindings,
) {
    let ManagedFileRead {
        path,
        path_text,
        text,
        expected_hash,
    } = managed;
    let mut state = "installed";
    if file.get("managed_script_role").and_then(Value::as_str) != Some("codex_dispatch")
        || hook_wrapper_comment_value(text, "host_kind") != Some("codex")
        || hook_wrapper_comment_value(text, "phase") != Some("dispatch")
        || hook_wrapper_comment_value(text, "script_role") != Some("codex_dispatch")
    {
        findings.broken_files.push(path_text.to_owned());
        if let Some(kind) = kind {
            findings.set_kind_state(kind, "broken");
        }
        return;
    }
    for required in [
        "git rev-parse --show-toplevel",
        "pre-tool|post-tool|prompt-capture",
        ".codex/hooks/volicord-$phase.sh",
        "exec \"$wrapper\"",
    ] {
        if !text.contains(required) {
            findings.broken_files.push(path_text.to_owned());
            if let Some(kind) = kind {
                findings.set_kind_state(kind, "broken");
            }
            return;
        }
    }
    if sha256_text(text) != expected_hash {
        findings.stale_files.push(path_text.to_owned());
        state = "stale";
    }
    if file
        .get("executable_required")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        && !script_is_executable(path)
    {
        findings.stale_files.push(path_text.to_owned());
        state = "stale";
    }
    if let Some(kind) = kind {
        findings.set_kind_state(kind, state);
    }
}

fn host_integration_file_kind_from_str(value: &str) -> Option<HostIntegrationFileKind> {
    match value {
        "volicord_policy" => Some(HostIntegrationFileKind::VolicordPolicy),
        "git_info_exclude" => Some(HostIntegrationFileKind::GitInfoExclude),
        "host_mcp_config" => Some(HostIntegrationFileKind::HostMcpConfig),
        "host_hook_config" => Some(HostIntegrationFileKind::HostHookConfig),
        "host_hook_dispatch" => Some(HostIntegrationFileKind::HostHookDispatch),
        "host_hook_wrapper" => Some(HostIntegrationFileKind::HostHookWrapper),
        "host_rule_instruction" => Some(HostIntegrationFileKind::HostRuleInstruction),
        "agents_managed_block" => Some(HostIntegrationFileKind::AgentsManagedBlock),
        _ => None,
    }
}

fn marker_count(text: &str, marker: &str) -> usize {
    text.match_indices(marker).count()
}

fn managed_block_slice<'a>(text: &'a str, start_marker: &str, end_marker: &str) -> Option<&'a str> {
    let start = text.find(start_marker)?;
    let end = start + text[start..].find(end_marker)? + end_marker.len();
    let end = if text[end..].starts_with('\n') {
        end + 1
    } else {
        end
    };
    text.get(start..end)
}

pub(crate) fn is_volicord_codex_hook_config(value: &Value) -> bool {
    let Some(root) = value.as_object() else {
        return false;
    };
    if root.keys().any(|key| key != "hooks") {
        return false;
    }
    let Some(hooks) = root.get("hooks").and_then(Value::as_object) else {
        return false;
    };
    let Some(contract) = contract_for(HostKind::Codex) else {
        return false;
    };
    if hooks.len() != REQUIRED_GUARD_PHASES.len() {
        return false;
    }
    let phases: &[HostLifecyclePhase] = &REQUIRED_GUARD_PHASES;
    phases.iter().all(|phase| {
        let Some(event) = hook_event_for_phase(contract, *phase) else {
            return false;
        };
        let Some(groups) = hooks.get(event.event_name).and_then(Value::as_array) else {
            return false;
        };
        groups.len() == 1
            && groups
                .first()
                .is_some_and(|group| is_volicord_codex_hook_group(*phase, group))
    })
}

fn is_volicord_codex_hook_group(phase: HostLifecyclePhase, group: &Value) -> bool {
    let Some(group) = group.as_object() else {
        return false;
    };
    let Some(handlers) = group.get("hooks").and_then(Value::as_array) else {
        return false;
    };
    handlers.len() == 1
        && handlers
            .first()
            .is_some_and(|handler| is_volicord_codex_hook_handler(phase, handler))
}

fn is_volicord_codex_hook_handler(phase: HostLifecyclePhase, handler: &Value) -> bool {
    let Some(object) = handler.as_object() else {
        return false;
    };
    object.get("type").and_then(Value::as_str) == Some("command")
        && object
            .get("command")
            .and_then(Value::as_str)
            .is_some_and(|command| {
                let direct_guard = command
                    .contains(&format!("volicord _hook {}", phase.command_name()))
                    && command.contains("--connection")
                    && command.contains("--guard-installation")
                    && command.contains("--host codex")
                    && command.contains("--host-output codex");
                let wrapper = command.contains(&format!(
                    ".codex/hooks/volicord-{}.sh",
                    phase.command_name()
                )) || (command.contains(CODEX_DISPATCH_WRAPPER)
                    && command.contains(phase.command_name()));
                direct_guard || wrapper
            })
}

pub(crate) fn hook_wrapper_exec_command(content: &str) -> Option<&str> {
    content
        .lines()
        .find_map(|line| line.strip_prefix("exec "))
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn generated_shell_words(command: &str) -> Option<Vec<String>> {
    let mut chars = command.chars().peekable();
    let mut words = Vec::new();
    while chars.peek().is_some() {
        while chars
            .peek()
            .is_some_and(|character| character.is_whitespace())
        {
            chars.next();
        }
        if chars.peek().is_none() {
            break;
        }
        let mut word = String::new();
        let mut consumed = false;
        while chars
            .peek()
            .is_some_and(|character| !character.is_whitespace())
        {
            match chars.next()? {
                '\'' => {
                    consumed = true;
                    loop {
                        match chars.next() {
                            Some('\'') => break,
                            Some(character) => word.push(character),
                            None => return None,
                        }
                    }
                }
                '\\' => {
                    consumed = true;
                    word.push(chars.next()?);
                }
                character
                    if character.is_ascii_alphanumeric()
                        || matches!(character, '_' | '-' | '.' | '/' | ':' | '=') =>
                {
                    consumed = true;
                    word.push(character);
                }
                _ => return None,
            }
        }
        if !consumed {
            return None;
        }
        words.push(word);
    }
    Some(words)
}

fn generated_managed_command_shape_verified(file: &Value, command: &str) -> bool {
    let Some(purpose) = file.get("purpose").and_then(Value::as_str) else {
        return false;
    };
    let Some(words) = generated_shell_words(command) else {
        return false;
    };
    if !words
        .first()
        .is_some_and(|word| !word.is_empty() && Path::new(word).is_absolute())
    {
        return false;
    }
    let required_options = [
        "--repo",
        "--connection",
        "--guard-installation",
        "--host",
        "--integration-profile",
        "--policy-hash",
        "--host-output",
    ];
    let argument_start = match purpose {
        "guard" => {
            let Some(phase_key) = file.get("phase").and_then(Value::as_str) else {
                return false;
            };
            let Some(phase) = REQUIRED_GUARD_PHASES
                .into_iter()
                .find(|phase| phase.policy_key() == phase_key)
            else {
                return false;
            };
            if words.get(1).map(String::as_str) != Some("_hook")
                || words.get(2).map(String::as_str) != Some(phase.command_name())
            {
                return false;
            }
            3
        }
        _ => return false,
    };
    if words.len() != argument_start + required_options.len() * 2 {
        return false;
    }
    let arguments = &words[argument_start..];
    required_options.into_iter().all(|option| {
        arguments
            .chunks_exact(2)
            .filter(|pair| pair[0] == option)
            .count()
            == 1
            && arguments
                .chunks_exact(2)
                .any(|pair| pair[0] == option && !pair[1].is_empty() && !pair[1].starts_with("--"))
    })
}

fn has_current_managed_process_binding(content: &str) -> bool {
    let binding_export = format!("export {MANAGED_WRAPPER_ENV}");
    let binding_assignment = format!("{MANAGED_WRAPPER_ENV}={MANAGED_WRAPPER_VALUE}");
    if hook_wrapper_comment_value(content, "runtime_home_binding")
        != Some("selected_init_runtime_home")
        || content
            .lines()
            .filter(|line| *line == "export VOLICORD_HOME")
            .count()
            != 1
        || content
            .lines()
            .filter(|line| *line == binding_export)
            .count()
            != 1
        || content
            .lines()
            .filter(|line| *line == binding_assignment)
            .count()
            != 1
    {
        return false;
    }
    let mut assignments = content
        .lines()
        .filter(|line| line.starts_with("VOLICORD_HOME="));
    let Some(assignment) = assignments.next() else {
        return false;
    };
    if assignments.next().is_some() {
        return false;
    }
    generated_shell_words(assignment)
        .filter(|words| words.len() == 1)
        .and_then(|words| words.into_iter().next())
        .and_then(|word| word.strip_prefix("VOLICORD_HOME=").map(str::to_owned))
        .is_some_and(|runtime_home| {
            !runtime_home.is_empty() && Path::new(&runtime_home).is_absolute()
        })
}

pub(crate) fn hook_wrapper_comment_value<'a>(content: &'a str, key: &str) -> Option<&'a str> {
    let prefix = format!("# {key}=");
    content
        .lines()
        .find_map(|line| line.strip_prefix(&prefix))
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

pub(crate) fn policy_hash(policy: &Value) -> Result<String, serde_json::Error> {
    volicord_types::canonical_json_sha256(policy).map(|hash| hash.into_inner())
}

pub(crate) fn sha256_text(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    format!("sha256:{}", hex_bytes(&hasher.finalize()))
}

fn hex_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

#[cfg(unix)]
pub(crate) fn script_is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    fs::metadata(path)
        .map(|metadata| metadata.permissions().mode() & 0o100 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
pub(crate) fn script_is_executable(_path: &Path) -> bool {
    true
}
