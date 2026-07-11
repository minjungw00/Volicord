use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use volicord_store::{
    agent_connections::ConnectionProjectRecord,
    guards::GuardInstallationRecord,
    inspection::{GuardInstallationInspectionRecord, ProjectInspectionRecord},
};

use crate::host_integration::{
    contracts::{
        contract_for, hook_event_for_phase, validate_contract_config, HostContractConfigKind,
    },
    HostIntegrationFileKind, HostKind, HostLifecyclePhase, REQUIRED_GUARD_PHASES,
};

use super::policy::required_guard_phase_names;

pub(crate) const HOOK_WRAPPER_MARKER: &str = "VOLICORD_MANAGED_HOOK_WRAPPER";
pub(crate) const CODEX_DISPATCH_WRAPPER: &str = ".codex/hooks/volicord-dispatch.sh";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedJsonProjection {
    ClaudeCodeSettingsHooks,
    ClaudeCodeMcpEntry,
}

impl ManagedJsonProjection {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::ClaudeCodeSettingsHooks => "claude_code_settings_hooks",
            Self::ClaudeCodeMcpEntry => "claude_code_mcp_entry",
        }
    }

    fn from_str(value: &str) -> Option<Self> {
        match value {
            "claude_code_settings_hooks" => Some(Self::ClaudeCodeSettingsHooks),
            "claude_code_mcp_entry" => Some(Self::ClaudeCodeMcpEntry),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HookWrapperResolutionStatus {
    Ok,
    RelativePathUnsafe,
    WrapperMissing,
    WrapperNotExecutable,
    DispatchMissing,
    PlaceholderUnsupported,
    AbsolutePathStale,
    PolicyHashMismatch,
    HostOutputMismatch,
    AuthorityMismatch,
    MetadataMissing,
}

impl HookWrapperResolutionStatus {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::RelativePathUnsafe => "relative_path_unsafe",
            Self::WrapperMissing => "wrapper_missing",
            Self::WrapperNotExecutable => "wrapper_not_executable",
            Self::DispatchMissing => "dispatch_missing",
            Self::PlaceholderUnsupported => "placeholder_unsupported",
            Self::AbsolutePathStale => "absolute_path_stale",
            Self::PolicyHashMismatch => "policy_hash_mismatch",
            Self::HostOutputMismatch => "host_output_mismatch",
            Self::AuthorityMismatch => "authority_mismatch",
            Self::MetadataMissing => "metadata_missing",
        }
    }

    fn from_str(value: &str) -> Option<Self> {
        match value {
            "ok" => Some(Self::Ok),
            "relative_path_unsafe" => Some(Self::RelativePathUnsafe),
            "wrapper_missing" => Some(Self::WrapperMissing),
            "wrapper_not_executable" => Some(Self::WrapperNotExecutable),
            "dispatch_missing" => Some(Self::DispatchMissing),
            "placeholder_unsupported" => Some(Self::PlaceholderUnsupported),
            "absolute_path_stale" => Some(Self::AbsolutePathStale),
            "policy_hash_mismatch" => Some(Self::PolicyHashMismatch),
            "host_output_mismatch" => Some(Self::HostOutputMismatch),
            "authority_mismatch" => Some(Self::AuthorityMismatch),
            "metadata_missing" => Some(Self::MetadataMissing),
            _ => None,
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
    pub(crate) managed_sources: Vec<String>,
    pub(crate) managed_bundle_hashes: Vec<String>,
    pub(crate) managed_verification_statuses: Vec<String>,
    pub(crate) native_host_output_adapter_verified_values: Vec<bool>,
    pub(crate) bash_shell_mutation_coverage_values: Vec<bool>,
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
        self.managed_sources.extend(other.managed_sources);
        self.managed_bundle_hashes
            .extend(other.managed_bundle_hashes);
        self.managed_verification_statuses
            .extend(other.managed_verification_statuses);
        self.native_host_output_adapter_verified_values
            .extend(other.native_host_output_adapter_verified_values);
        self.bash_shell_mutation_coverage_values
            .extend(other.bash_shell_mutation_coverage_values);
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
        self.managed_sources.sort();
        self.managed_sources.dedup();
        self.managed_bundle_hashes.sort();
        self.managed_bundle_hashes.dedup();
        self.managed_verification_statuses.sort();
        self.managed_verification_statuses.dedup();
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
        if !matches!(
            status,
            HookWrapperResolutionStatus::Ok
                | HookWrapperResolutionStatus::WrapperMissing
                | HookWrapperResolutionStatus::DispatchMissing
        ) {
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
            && self.hook_path_safety_ok()
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

    fn hook_path_safety_ok(&self) -> bool {
        !self.hook_path_safety_statuses.is_empty()
            && self
                .hook_path_safety_statuses
                .iter()
                .all(|status| status == HookWrapperResolutionStatus::Ok.as_str())
            && all_recorded_values_true(&self.hook_cwd_independent_values)
            && all_recorded_values_true(&self.hook_subdirectory_safe_values)
    }

    pub(crate) fn native_host_output_adapter_verified(&self) -> bool {
        self.generated_config_verified()
            && all_recorded_values_true(&self.native_host_output_adapter_verified_values)
    }

    pub(crate) fn bash_shell_mutation_coverage(&self) -> bool {
        self.generated_config_verified()
            && all_recorded_values_true(&self.bash_shell_mutation_coverage_values)
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

fn more_severe_hook_wrapper_status(
    left: HookWrapperResolutionStatus,
    right: HookWrapperResolutionStatus,
) -> HookWrapperResolutionStatus {
    if hook_path_status_rank(left.as_str()) <= hook_path_status_rank(right.as_str()) {
        left
    } else {
        right
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
    project_repo_roots: &'a [PathBuf],
    strict_authority: bool,
}

#[cfg(test)]
pub(crate) fn guard_file_findings(capability_json: &str) -> GuardFileFindings {
    guard_file_findings_with_context(capability_json, None)
}

pub(crate) fn guard_file_findings_for_installation(
    installation: &GuardInstallationRecord,
    projects: &[ConnectionProjectRecord],
) -> GuardFileFindings {
    let project_repo_roots = projects
        .iter()
        .map(|project| project.project.repo_root.clone())
        .collect::<Vec<_>>();
    let context = GuardAuthorityContext {
        host_kind: &installation.host_kind,
        project_repo_roots: &project_repo_roots,
        strict_authority: false,
    };
    guard_file_findings_with_context(&installation.host_capability_json, Some(context))
}

pub(crate) fn guard_file_findings_for_inspection(
    installation: &GuardInstallationInspectionRecord,
    projects: &[ProjectInspectionRecord],
) -> GuardFileFindings {
    let project_repo_roots = installation
        .project_internal_id
        .as_deref()
        .map(|project_internal_id| {
            projects
                .iter()
                .filter(|project| project.project_internal_id == project_internal_id)
                .map(|project| project.repo_root.clone())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let context = GuardAuthorityContext {
        host_kind: &installation.host_kind,
        project_repo_roots: &project_repo_roots,
        strict_authority: true,
    };
    guard_file_findings_with_context(&installation.host_capability_json, Some(context))
}

pub(crate) fn missing_required_hooks_from_capability_json(capability_json: &str) -> Vec<String> {
    serde_json::from_str::<Value>(capability_json)
        .ok()
        .map(|value| missing_required_hooks_from_capability(&value))
        .unwrap_or_default()
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
    findings.prompt_capture_configured = value
        .get("prompt_capture")
        .and_then(Value::as_bool)
        .unwrap_or(false);
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
        .native_host_output_adapter_verified_values
        .push(bool_json_field(
            &value,
            "native_host_output_adapter_verified",
        ));
    findings
        .bash_shell_mutation_coverage_values
        .push(bool_json_field(&value, "bash_shell_mutation_coverage"));
    findings
        .direct_file_write_matcher_coverage_values
        .push(bool_json_field(
            &value,
            "direct_file_write_matcher_coverage",
        ));
    findings.missing_required_hooks = missing_required_hooks_from_capability(&value);

    verify_recorded_hook_path_safety(&value, context, &mut findings);

    let files = value
        .get("files")
        .and_then(Value::as_array)
        .into_iter()
        .flatten();
    for file in files {
        if record_profile_ignores_detective_file(&value, file) {
            continue;
        }
        verify_guard_file(file, &value, &mut findings);
    }
    findings
}

fn record_profile_ignores_detective_file(capability: &Value, file: &Value) -> bool {
    capability.get("selected_profile").and_then(Value::as_str) == Some("record")
        && file
            .get("kind")
            .and_then(Value::as_str)
            .and_then(host_integration_file_kind_from_str)
            .is_some_and(|kind| {
                matches!(
                    kind,
                    HostIntegrationFileKind::HostHookConfig
                        | HostIntegrationFileKind::HostHookDispatch
                        | HostIntegrationFileKind::HostHookWrapper
                        | HostIntegrationFileKind::HostRuleInstruction
                )
            })
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
    if capability.get("selected_profile").and_then(Value::as_str) == Some("record") {
        return Vec::new();
    }
    let configured_required_hooks = capability
        .get("required_hook_phases")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .collect::<Vec<_>>();
    let mut missing_required_hooks = capability
        .get("missing_required_hooks")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    for required_hook in required_guard_phase_names() {
        if !configured_required_hooks.contains(&required_hook) {
            missing_required_hooks.push(required_hook.to_owned());
        }
    }
    missing_required_hooks.sort();
    missing_required_hooks.dedup();
    missing_required_hooks
}

fn verify_recorded_hook_path_safety(
    capability: &Value,
    context: Option<GuardAuthorityContext<'_>>,
    findings: &mut GuardFileFindings,
) {
    let requires_path_safety = capability_requires_hook_path_safety(capability);
    let Some(commands) = capability
        .get("host_hook_commands")
        .and_then(Value::as_array)
    else {
        if requires_path_safety {
            findings.record_hook_path_status(
                HookWrapperResolutionStatus::MetadataMissing,
                json!({ "source": "host_hook_commands" }),
            );
        }
        return;
    };
    if commands.is_empty() {
        if requires_path_safety {
            findings.record_hook_path_status(
                HookWrapperResolutionStatus::MetadataMissing,
                json!({ "source": "host_hook_commands" }),
            );
        }
        return;
    }
    for command in commands {
        verify_recorded_hook_command_path_safety(command, context, findings);
    }
}

fn capability_requires_hook_path_safety(capability: &Value) -> bool {
    match capability.get("selected_profile").and_then(Value::as_str) {
        Some("record") => false,
        Some("detective" | "mixed") => true,
        _ => capability
            .get("required_hook_phases")
            .and_then(Value::as_array)
            .is_some_and(|phases| !phases.is_empty()),
    }
}

fn verify_recorded_hook_command_path_safety(
    command: &Value,
    context: Option<GuardAuthorityContext<'_>>,
    findings: &mut GuardFileFindings,
) {
    let host_kind = command
        .get("host_kind")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let phase = command
        .get("phase")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let command_text = command
        .get("command")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let args = command
        .get("args")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let expected_wrapper_path = command
        .get("expected_wrapper_path")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let expected_phase_wrapper_path = command
        .get("expected_phase_wrapper_path")
        .and_then(Value::as_str)
        .unwrap_or(expected_wrapper_path);
    let phase_command = phase_command_name_from_capability(phase).unwrap_or_default();
    let mut status = classify_hook_command_path(
        host_kind,
        phase_command,
        command_text,
        args,
        expected_wrapper_path,
        expected_phase_wrapper_path,
    );
    if command.get("cwd_independent").and_then(Value::as_bool) != Some(true)
        || command.get("subdirectory_safe").and_then(Value::as_bool) != Some(true)
    {
        status = HookWrapperResolutionStatus::RelativePathUnsafe;
    }
    if let Some(recorded_status) = command
        .get("wrapper_resolution_status")
        .and_then(Value::as_str)
        .filter(|value| *value != HookWrapperResolutionStatus::Ok.as_str())
    {
        let recorded_status = HookWrapperResolutionStatus::from_str(recorded_status)
            .unwrap_or(HookWrapperResolutionStatus::MetadataMissing);
        status = more_severe_hook_wrapper_status(status, recorded_status);
    }
    if let Some(context) = context {
        if (context.strict_authority || !host_kind.is_empty()) && host_kind != context.host_kind {
            status = HookWrapperResolutionStatus::AuthorityMismatch;
        }
        if !expected_phase_wrapper_path.is_empty()
            && (context.strict_authority || !context.project_repo_roots.is_empty())
            && !context.project_repo_roots.iter().any(|repo_root| {
                path_starts_with_text(expected_phase_wrapper_path, &path_text(repo_root))
            })
        {
            status = HookWrapperResolutionStatus::AuthorityMismatch;
        }
    }
    verify_recorded_hook_wrapper_path(
        expected_phase_wrapper_path,
        HookWrapperResolutionStatus::WrapperMissing,
        findings,
    );
    if host_kind == HostKind::Codex.as_str() {
        verify_recorded_hook_wrapper_path(
            expected_wrapper_path,
            HookWrapperResolutionStatus::DispatchMissing,
            findings,
        );
    }
    findings.record_hook_path_status(
        status,
        json!({
            "phase": phase,
            "host_kind": host_kind,
            "command": command_text,
            "hook_command_path_basis": command.get("hook_command_path_basis").and_then(Value::as_str).unwrap_or("unknown"),
            "cwd_independent": command.get("cwd_independent").and_then(Value::as_bool).unwrap_or(false),
            "subdirectory_safe": command.get("subdirectory_safe").and_then(Value::as_bool).unwrap_or(false),
            "wrapper_resolution_status": status.as_str(),
            "expected_wrapper_path": expected_wrapper_path,
            "expected_phase_wrapper_path": expected_phase_wrapper_path,
        }),
    );
}

fn verify_recorded_hook_wrapper_path(
    path_text_value: &str,
    missing_status: HookWrapperResolutionStatus,
    findings: &mut GuardFileFindings,
) {
    if path_text_value.trim().is_empty() {
        findings.record_hook_path_status(
            HookWrapperResolutionStatus::MetadataMissing,
            json!({ "source": "expected_wrapper_path" }),
        );
        return;
    }
    let path = Path::new(path_text_value);
    match fs::metadata(path) {
        Ok(metadata) if metadata.is_file() => {
            if !script_is_executable(path) {
                findings.stale_files.push(path_text_value.to_owned());
                findings.record_hook_path_status(
                    HookWrapperResolutionStatus::WrapperNotExecutable,
                    json!({ "path": path_text_value }),
                );
            }
        }
        Ok(_) => {
            findings.broken_files.push(path_text_value.to_owned());
            findings.record_hook_path_status(
                HookWrapperResolutionStatus::WrapperMissing,
                json!({ "path": path_text_value }),
            );
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            findings.missing_files.push(path_text_value.to_owned());
            findings.record_hook_path_status(missing_status, json!({ "path": path_text_value }));
        }
        Err(_) => {
            findings.broken_files.push(path_text_value.to_owned());
            findings.record_hook_path_status(
                HookWrapperResolutionStatus::WrapperMissing,
                json!({ "path": path_text_value }),
            );
        }
    }
}

fn verify_hook_config_commands_path_safety(
    host_kind: HostKind,
    config: &Value,
    capability: &Value,
    findings: &mut GuardFileFindings,
) -> bool {
    let Some(hooks) = config.get("hooks").and_then(Value::as_object) else {
        findings.record_hook_path_status(
            HookWrapperResolutionStatus::MetadataMissing,
            json!({ "source": "hooks" }),
        );
        return false;
    };
    let mut ok = true;
    for (event_name, groups) in hooks {
        let Some(phase_name) = phase_capability_name_from_event(event_name) else {
            continue;
        };
        let Some(phase_command) = phase_command_name_from_capability(phase_name) else {
            ok = false;
            continue;
        };
        let (expected_wrapper_path, expected_phase_wrapper_path) =
            expected_hook_paths_from_capability(capability, phase_name);
        let Some(groups) = groups.as_array() else {
            ok = false;
            findings.record_hook_path_status(
                HookWrapperResolutionStatus::MetadataMissing,
                json!({ "event": event_name }),
            );
            continue;
        };
        for group in groups {
            let Some(handlers) = group.get("hooks").and_then(Value::as_array) else {
                ok = false;
                findings.record_hook_path_status(
                    HookWrapperResolutionStatus::MetadataMissing,
                    json!({ "event": event_name, "phase": phase_name }),
                );
                continue;
            };
            for handler in handlers {
                let command = handler
                    .get("command")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let args = handler
                    .get("args")
                    .and_then(Value::as_array)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]);
                let status = classify_hook_command_path(
                    host_kind.as_str(),
                    phase_command,
                    command,
                    args,
                    expected_wrapper_path.as_deref().unwrap_or_default(),
                    expected_phase_wrapper_path.as_deref().unwrap_or_default(),
                );
                findings.record_hook_path_status(
                    status,
                    json!({
                        "source": "host_hook_config",
                        "event": event_name,
                        "phase": phase_name,
                        "command": command,
                        "wrapper_resolution_status": status.as_str(),
                    }),
                );
                if status != HookWrapperResolutionStatus::Ok {
                    ok = false;
                }
            }
        }
    }
    ok
}

fn expected_hook_paths_from_capability(
    capability: &Value,
    phase_name: &str,
) -> (Option<String>, Option<String>) {
    capability
        .get("host_hook_commands")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|command| command.get("phase").and_then(Value::as_str) == Some(phase_name))
        .map(|command| {
            (
                command
                    .get("expected_wrapper_path")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                command
                    .get("expected_phase_wrapper_path")
                    .and_then(Value::as_str)
                    .or_else(|| command.get("expected_wrapper_path").and_then(Value::as_str))
                    .map(str::to_owned),
            )
        })
        .unwrap_or((None, None))
}

fn phase_capability_name_from_event(event_name: &str) -> Option<&'static str> {
    match event_name {
        "SessionStart" => Some("session_start_hook"),
        "PreToolUse" => Some("pre_tool_hook"),
        "PostToolUse" => Some("post_tool_hook"),
        "UserPromptSubmit" => Some("user_prompt_submit_hook"),
        "Stop" => Some("stop_hook"),
        _ => None,
    }
}

pub(crate) fn classify_hook_command_path(
    host_kind: &str,
    phase_command: &str,
    command_text: &str,
    args: &[Value],
    expected_wrapper_path: &str,
    expected_phase_wrapper_path: &str,
) -> HookWrapperResolutionStatus {
    if phase_command.is_empty() || command_text.trim().is_empty() {
        return HookWrapperResolutionStatus::MetadataMissing;
    }
    match host_kind {
        "codex" => classify_codex_hook_command_path(
            phase_command,
            command_text,
            expected_wrapper_path,
            expected_phase_wrapper_path,
        ),
        "claude_code" => classify_claude_hook_command_path(
            phase_command,
            command_text,
            args,
            expected_phase_wrapper_path,
        ),
        _ => HookWrapperResolutionStatus::MetadataMissing,
    }
}

fn classify_codex_hook_command_path(
    phase_command: &str,
    command_text: &str,
    expected_dispatch_path: &str,
    expected_phase_wrapper_path: &str,
) -> HookWrapperResolutionStatus {
    let relative_wrapper = format!(".codex/hooks/volicord-{phase_command}.sh");
    if contains_bare_relative_hook_path(command_text, ".codex/hooks/") {
        return HookWrapperResolutionStatus::RelativePathUnsafe;
    }
    if command_text.contains(CODEX_DISPATCH_WRAPPER) || command_text.contains(&relative_wrapper) {
        if command_text.contains("git rev-parse --show-toplevel")
            && command_text.contains(CODEX_DISPATCH_WRAPPER)
            && command_text.contains(phase_command)
        {
            return HookWrapperResolutionStatus::Ok;
        }
        if let Some(path) = absolute_path_ending_with(command_text, CODEX_DISPATCH_WRAPPER) {
            return if paths_equivalent_text(&path, expected_dispatch_path) {
                HookWrapperResolutionStatus::Ok
            } else {
                HookWrapperResolutionStatus::AbsolutePathStale
            };
        }
        if let Some(path) = absolute_path_ending_with(command_text, &relative_wrapper) {
            return if paths_equivalent_text(&path, expected_phase_wrapper_path) {
                HookWrapperResolutionStatus::Ok
            } else {
                HookWrapperResolutionStatus::AbsolutePathStale
            };
        }
        return HookWrapperResolutionStatus::RelativePathUnsafe;
    }
    if command_text.contains(&format!("volicord _hook {phase_command}")) {
        return HookWrapperResolutionStatus::Ok;
    }
    HookWrapperResolutionStatus::MetadataMissing
}

fn classify_claude_hook_command_path(
    phase_command: &str,
    command_text: &str,
    args: &[Value],
    expected_phase_wrapper_path: &str,
) -> HookWrapperResolutionStatus {
    let relative_wrapper = format!(".claude/hooks/volicord-{phase_command}.sh");
    let placeholder_wrapper = format!("${{CLAUDE_PROJECT_DIR}}/{relative_wrapper}");
    if contains_bare_relative_hook_path(command_text, ".claude/hooks/") {
        return HookWrapperResolutionStatus::RelativePathUnsafe;
    }
    if command_text.contains("${CLAUDE_PROJECT_DIR}") {
        return if command_text == placeholder_wrapper && args.is_empty() {
            HookWrapperResolutionStatus::Ok
        } else {
            HookWrapperResolutionStatus::PlaceholderUnsupported
        };
    }
    if command_text.contains(&relative_wrapper) {
        if let Some(path) = absolute_path_ending_with(command_text, &relative_wrapper) {
            return if paths_equivalent_text(&path, expected_phase_wrapper_path) {
                HookWrapperResolutionStatus::Ok
            } else {
                HookWrapperResolutionStatus::AbsolutePathStale
            };
        }
        return HookWrapperResolutionStatus::RelativePathUnsafe;
    }
    if command_text.contains(&format!("volicord _hook {phase_command}")) {
        return HookWrapperResolutionStatus::Ok;
    }
    HookWrapperResolutionStatus::MetadataMissing
}

fn contains_bare_relative_hook_path(command_text: &str, prefix: &str) -> bool {
    let trimmed = command_text.trim_start_matches([' ', '\'', '"']);
    trimmed.starts_with(prefix)
        || trimmed.starts_with(&format!("./{prefix}"))
        || command_text.contains(&format!(" {prefix}"))
        || command_text.contains(&format!(" './{prefix}"))
        || command_text.contains(&format!(" \"./{prefix}"))
        || command_text.contains(&format!(" '{prefix}"))
        || command_text.contains(&format!(" \"{prefix}"))
}

fn absolute_path_ending_with(command_text: &str, suffix: &str) -> Option<String> {
    let index = command_text.find(suffix)?;
    let prefix = &command_text[..index];
    let start = prefix
        .rfind([' ', '\'', '"', '=', ';', '('])
        .map(|position| position + 1)
        .unwrap_or(0);
    let path_prefix = prefix.get(start..)?;
    if !path_prefix.starts_with('/') {
        return None;
    }
    Some(format!("{path_prefix}{suffix}"))
}

fn paths_equivalent_text(left: &str, right: &str) -> bool {
    lexical_absolute_path(left)
        .is_some_and(|left| lexical_absolute_path(right).is_some_and(|right| left == right))
}

fn path_starts_with_text(path: &str, prefix: &str) -> bool {
    let Some(path) = lexical_absolute_path(path) else {
        return false;
    };
    let Some(prefix) = lexical_absolute_path(prefix) else {
        return false;
    };
    path == prefix || path.starts_with(&format!("{prefix}/"))
}

fn lexical_absolute_path(path_text_value: &str) -> Option<String> {
    let path = Path::new(path_text_value);
    if !path.is_absolute() {
        return None;
    }
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::RootDir => {}
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                parts.pop();
            }
            std::path::Component::Normal(part) => parts.push(part.to_string_lossy().into_owned()),
            std::path::Component::Prefix(_) => return None,
        }
    }
    Some(format!("/{}", parts.join("/")))
}

fn phase_command_name_from_capability(phase: &str) -> Option<&'static str> {
    match phase {
        "session_start_hook" | "session_start" => Some("session-start"),
        "pre_tool_hook" | "pre_tool" => Some("pre-tool"),
        "post_tool_hook" | "post_tool" => Some("post-tool"),
        "user_prompt_submit_hook" | "prompt_capture" => Some("prompt-capture"),
        "stop_hook" | "stop" => Some("stop"),
        _ => None,
    }
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
        Some("managed_json_projection") => verify_managed_json_projection_file(
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
        let value = match serde_json::from_str::<Value>(text) {
            Ok(value) if is_volicord_codex_hook_config(&value) => value,
            Ok(_) | Err(_) => {
                findings.broken_files.push(path_text.to_owned());
                if let Some(kind) = kind {
                    findings.set_kind_state(kind, "broken");
                }
                return;
            }
        };
        if validate_contract_config(HostKind::Codex, HostContractConfigKind::HookConfig, text)
            .is_err()
        {
            findings.stale_files.push(path_text.to_owned());
            state = "stale";
        }
        if !verify_hook_config_commands_path_safety(HostKind::Codex, &value, capability, findings) {
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
        || !text.contains(HOOK_WRAPPER_MARKER)
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
    for required in [
        "volicord _hook ",
        "--repo ",
        "--connection ",
        "--guard-installation ",
        "--host ",
        "--integration-profile ",
        "--policy-hash ",
        "--host-output ",
    ] {
        if !expected_command.contains(required) {
            findings.broken_files.push(path_text.to_owned());
            if let Some(kind) = kind {
                findings.set_kind_state(kind, "broken");
            }
            return;
        }
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
        "session-start|pre-tool|post-tool|prompt-capture|stop",
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

fn verify_managed_json_projection_file(
    file: &Value,
    kind: Option<HostIntegrationFileKind>,
    capability: &Value,
    path_text: &str,
    text: &str,
    expected_hash: &str,
    findings: &mut GuardFileFindings,
) {
    let Some(projection) = file
        .get("managed_projection")
        .and_then(Value::as_str)
        .and_then(ManagedJsonProjection::from_str)
    else {
        findings.broken_files.push(path_text.to_owned());
        if let Some(kind) = kind {
            findings.set_kind_state(kind, "broken");
        }
        return;
    };
    let actual = match serde_json::from_str::<Value>(text) {
        Ok(actual) => actual,
        Err(_) => {
            findings.broken_files.push(path_text.to_owned());
            if let Some(kind) = kind {
                findings.set_kind_state(kind, "broken");
            }
            return;
        }
    };
    let expected_projection_json = file
        .get("managed_projection_json")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let desired = match serde_json::from_str::<Value>(expected_projection_json) {
        Ok(desired) => desired,
        Err(_) => {
            findings.broken_files.push(path_text.to_owned());
            if let Some(kind) = kind {
                findings.set_kind_state(kind, "broken");
            }
            return;
        }
    };
    let actual_projection = match managed_json_projection_from_actual(&actual, &desired, projection)
    {
        Ok(Some(actual_projection)) => actual_projection,
        Ok(None) => {
            findings.stale_files.push(path_text.to_owned());
            if let Some(kind) = kind {
                findings.set_kind_state(kind, "stale");
            }
            return;
        }
        Err(()) => {
            findings.broken_files.push(path_text.to_owned());
            if let Some(kind) = kind {
                findings.set_kind_state(kind, "broken");
            }
            return;
        }
    };
    if actual_projection == desired && sha256_text(expected_projection_json) == expected_hash {
        if projection == ManagedJsonProjection::ClaudeCodeSettingsHooks
            && serde_json::to_string(&actual_projection)
                .ok()
                .is_none_or(|text| {
                    validate_contract_config(
                        HostKind::ClaudeCode,
                        HostContractConfigKind::ProjectSettings,
                        &text,
                    )
                    .is_err()
                })
        {
            findings.stale_files.push(path_text.to_owned());
            if let Some(kind) = kind {
                findings.set_kind_state(kind, "stale");
            }
            return;
        }
        if projection == ManagedJsonProjection::ClaudeCodeSettingsHooks
            && !verify_hook_config_commands_path_safety(
                HostKind::ClaudeCode,
                &actual_projection,
                capability,
                findings,
            )
        {
            findings.stale_files.push(path_text.to_owned());
            if let Some(kind) = kind {
                findings.set_kind_state(kind, "stale");
            }
            return;
        }
        if let Some(kind) = kind {
            findings.set_kind_state(kind, "installed");
        }
    } else {
        findings.stale_files.push(path_text.to_owned());
        if let Some(kind) = kind {
            findings.set_kind_state(kind, "stale");
        }
    }
}

fn managed_json_projection_from_actual(
    actual: &Value,
    desired: &Value,
    projection: ManagedJsonProjection,
) -> Result<Option<Value>, ()> {
    match projection {
        ManagedJsonProjection::ClaudeCodeSettingsHooks => {
            claude_settings_hooks_projection_from_actual(actual, desired)
        }
        ManagedJsonProjection::ClaudeCodeMcpEntry => {
            claude_mcp_projection_from_actual(actual, desired)
        }
    }
}

fn claude_mcp_projection_from_actual(actual: &Value, desired: &Value) -> Result<Option<Value>, ()> {
    let actual_servers = actual
        .get("mcpServers")
        .and_then(Value::as_object)
        .ok_or(())?;
    let desired_servers = desired
        .get("mcpServers")
        .and_then(Value::as_object)
        .ok_or(())?;
    let mut projection_servers = serde_json::Map::new();
    for name in desired_servers.keys() {
        let Some(entry) = actual_servers.get(name) else {
            return Ok(None);
        };
        projection_servers.insert(name.clone(), entry.clone());
    }
    Ok(Some(json!({ "mcpServers": projection_servers })))
}

fn claude_settings_hooks_projection_from_actual(
    actual: &Value,
    desired: &Value,
) -> Result<Option<Value>, ()> {
    let actual_hooks = actual.get("hooks").and_then(Value::as_object).ok_or(())?;
    let desired_hooks = desired.get("hooks").and_then(Value::as_object).ok_or(())?;
    let mut projected_hooks = serde_json::Map::new();
    for phase in REQUIRED_GUARD_PHASES {
        let event_name = claude_event_name(phase)?;
        let desired_groups = desired_hooks
            .get(event_name)
            .and_then(Value::as_array)
            .ok_or(())?;
        let desired_group = desired_groups.first().ok_or(())?;
        let Some(actual_groups) = actual_hooks.get(event_name).and_then(Value::as_array) else {
            return Ok(None);
        };
        let matches = actual_groups
            .iter()
            .filter(|group| **group == *desired_group)
            .count();
        if matches != 1 {
            return Ok(None);
        }
        projected_hooks.insert(
            event_name.to_owned(),
            Value::Array(vec![desired_group.clone()]),
        );
    }
    Ok(Some(json!({ "hooks": projected_hooks })))
}

fn claude_event_name(phase: HostLifecyclePhase) -> Result<&'static str, ()> {
    let contract = contract_for(HostKind::ClaudeCode).ok_or(())?;
    hook_event_for_phase(contract, phase)
        .map(|event| event.event_name)
        .ok_or(())
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
    REQUIRED_GUARD_PHASES.iter().all(|phase| {
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

pub(crate) fn hook_wrapper_comment_value<'a>(content: &'a str, key: &str) -> Option<&'a str> {
    let prefix = format!("# {key}=");
    content
        .lines()
        .find_map(|line| line.strip_prefix(&prefix))
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

pub(crate) fn policy_hash(policy: &Value) -> Result<String, serde_json::Error> {
    serde_json::to_string(policy).map(|text| sha256_text(&text))
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

fn path_text(path: &Path) -> String {
    path.display().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    #[test]
    fn claude_settings_projection_allows_unmanaged_hook_groups_but_rejects_duplicate_managed() {
        let desired = desired_claude_hooks_projection();
        let desired_hooks = desired
            .get("hooks")
            .and_then(Value::as_object)
            .expect("desired hooks should be an object");

        let mut unmanaged_hooks = desired_hooks.clone();
        unmanaged_hooks
            .get_mut("PreToolUse")
            .and_then(Value::as_array_mut)
            .expect("PreToolUse hooks should be an array")
            .insert(
                0,
                json!({
                    "matcher": "Bash",
                    "hooks": [
                        {
                            "type": "command",
                            "command": "echo keep"
                        }
                    ]
                }),
            );
        let actual_with_unmanaged = json!({
            "theme": "dark",
            "hooks": unmanaged_hooks,
        });
        assert_eq!(
            claude_settings_hooks_projection_from_actual(&actual_with_unmanaged, &desired),
            Ok(Some(desired.clone()))
        );

        let mut duplicate_hooks = desired_hooks.clone();
        let pre_tool_hooks = duplicate_hooks
            .get_mut("PreToolUse")
            .and_then(Value::as_array_mut)
            .expect("PreToolUse hooks should be an array");
        let managed_group = pre_tool_hooks
            .first()
            .cloned()
            .expect("managed hook group should be present");
        pre_tool_hooks.push(managed_group);
        let actual_with_duplicate = json!({ "hooks": duplicate_hooks });
        assert_eq!(
            claude_settings_hooks_projection_from_actual(&actual_with_duplicate, &desired),
            Ok(None)
        );
    }

    fn desired_claude_hooks_projection() -> Value {
        let mut hooks = serde_json::Map::new();
        for phase in REQUIRED_GUARD_PHASES {
            let event_name = claude_event_name(phase).expect("phase should map to Claude event");
            hooks.insert(
                event_name.to_owned(),
                Value::Array(vec![json!({
                    "matcher": "Bash|Edit|Write|MultiEdit|mcp__.*__(write|edit|create|update|delete|remove|move|patch).*",
                    "hooks": [
                        {
                            "type": "command",
                            "command": format!(
                                "volicord _hook {} --host claude-code --host-output claude-code",
                                phase.command_name()
                            ),
                            "timeout": 30
                        }
                    ]
                })]),
            );
        }
        json!({ "hooks": hooks })
    }
}
