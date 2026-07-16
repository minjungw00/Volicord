use serde::Deserialize;
use serde_json::Value;
use volicord_store::{
    core_pipeline::{CoreProjectStore, TaskRecord},
    workflow_records::task_policy_control_reevaluation,
    StoreError,
};
use volicord_types::{
    AcceptancePolicy, ProjectWorkflowPolicySummary, RequestedControlLevel, TaskControlLevel,
    TaskMode,
};

use crate::policy::path::path_is_within;

const WORKFLOW_POLICY_SCHEMA: &str = "volicord-policy-v2";

/// Core-owned view of the project workflow policy.
///
/// Absence is intentionally represented by the conservative built-in defaults. It does not
/// synthesize an authoritative policy summary.
#[derive(Debug, Clone)]
pub(crate) struct ProjectWorkflowPolicy {
    pub(crate) summary: Option<ProjectWorkflowPolicySummary>,
    pub(crate) default_direct_control: TaskControlLevel,
    pub(crate) default_work_control: TaskControlLevel,
    pub(crate) light: LightWorkflowPolicy,
    pub(crate) write_ticket_idle_timeout_minutes: Option<u64>,
}

#[derive(Debug, Clone)]
pub(crate) struct LightWorkflowPolicy {
    pub(crate) enabled: bool,
    pub(crate) max_intended_paths: usize,
    pub(crate) allowed_path_patterns: Vec<String>,
    pub(crate) denied_path_patterns: Vec<String>,
    pub(crate) final_acceptance: AcceptancePolicy,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredWorkflowPolicy {
    default_direct_control: TaskControlLevel,
    default_work_control: TaskControlLevel,
    light: StoredLightWorkflowPolicy,
    write_ticket: StoredWriteTicketWorkflowPolicy,
    detective: StoredDetectiveWorkflowPolicy,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredLightWorkflowPolicy {
    enabled: bool,
    max_intended_paths: usize,
    allowed_path_patterns: Vec<String>,
    denied_path_patterns: Vec<String>,
    final_acceptance: AcceptancePolicy,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredWriteTicketWorkflowPolicy {
    idle_timeout_minutes: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredDetectiveWorkflowPolicy {
    unknown_effect_behavior: String,
    stop_behavior: String,
}

impl ProjectWorkflowPolicy {
    fn conservative_default() -> Self {
        Self {
            summary: None,
            default_direct_control: TaskControlLevel::Tracked,
            default_work_control: TaskControlLevel::Tracked,
            light: LightWorkflowPolicy {
                enabled: false,
                max_intended_paths: 3,
                allowed_path_patterns: Vec::new(),
                denied_path_patterns: Vec::new(),
                final_acceptance: AcceptancePolicy::PolicyDependent,
            },
            write_ticket_idle_timeout_minutes: None,
        }
    }

    pub(crate) fn light_paths_are_allowed(&self, intended_paths: &[String]) -> bool {
        self.light.enabled
            && intended_paths.len() <= self.light.max_intended_paths
            && intended_paths.iter().all(|path| {
                self.light
                    .allowed_path_patterns
                    .iter()
                    .any(|allowed| path_is_within(path, allowed))
                    && !self
                        .light
                        .denied_path_patterns
                        .iter()
                        .any(|denied| path_is_within(path, denied))
            })
    }

    pub(crate) fn has_denied_path(&self, intended_paths: &[String]) -> bool {
        intended_paths.iter().any(|path| {
            self.light
                .denied_path_patterns
                .iter()
                .any(|denied| path_is_within(path, denied))
        })
    }
}

pub(crate) fn project_workflow_policy(
    store: &CoreProjectStore,
) -> Result<ProjectWorkflowPolicy, StoreError> {
    let Some(record) = store.project_workflow_policy()? else {
        return Ok(ProjectWorkflowPolicy::conservative_default());
    };
    let corrupt = || {
        StoreError::corrupt_owner_state_json(
            "project_workflow_policies",
            record.project_id.clone(),
            "policy_json",
        )
    };
    if record.policy_schema != WORKFLOW_POLICY_SCHEMA {
        return Err(corrupt());
    }
    let value: Value = serde_json::from_str(&record.policy_json).map_err(|_| corrupt())?;
    if value.get("schema").and_then(Value::as_str) != Some(WORKFLOW_POLICY_SCHEMA) {
        return Err(corrupt());
    }
    let stored: StoredWorkflowPolicy =
        serde_json::from_value(value.get("workflow").cloned().ok_or_else(corrupt)?)
            .map_err(|_| corrupt())?;
    if stored.light.max_intended_paths == 0
        || stored
            .write_ticket
            .idle_timeout_minutes
            .is_some_and(|minutes| minutes == 0)
        || stored.detective.unknown_effect_behavior != "warn"
        || stored.detective.stop_behavior != "allow_with_disclosure"
    {
        return Err(corrupt());
    }
    Ok(ProjectWorkflowPolicy {
        summary: Some(ProjectWorkflowPolicySummary {
            policy_schema: record.policy_schema,
            policy_version: record.policy_version,
            policy_fingerprint: record.policy_fingerprint,
            source: record.source,
        }),
        default_direct_control: stored.default_direct_control,
        default_work_control: stored.default_work_control,
        light: LightWorkflowPolicy {
            enabled: stored.light.enabled,
            max_intended_paths: stored.light.max_intended_paths,
            allowed_path_patterns: stored.light.allowed_path_patterns,
            denied_path_patterns: stored.light.denied_path_patterns,
            final_acceptance: stored.light.final_acceptance,
        },
        write_ticket_idle_timeout_minutes: stored.write_ticket.idle_timeout_minutes,
    })
}

#[derive(Debug, Clone)]
pub(crate) struct ResolvedTaskControlAuthority {
    pub(crate) effective_control_level: TaskControlLevel,
    pub(crate) acceptance_policy: AcceptancePolicy,
    pub(crate) control_level_reason: String,
    pub(crate) acceptance_policy_reason: String,
    pub(crate) control_raised: bool,
    pub(crate) acceptance_raised: bool,
    pub(crate) pending_policy_reevaluation: bool,
}

/// Resolves the strongest durable Task, current project-policy, and pending
/// policy-reevaluation authority without permitting an active Task downgrade.
pub(crate) fn resolve_task_control_authority(
    task: &TaskRecord,
    policy: &ProjectWorkflowPolicy,
) -> Result<ResolvedTaskControlAuthority, StoreError> {
    let requested = parse_requested_control_level(&task.requested_control_level)?;
    let current_control = parse_task_control_level(&task.effective_control_level)?;
    let mode = parse_task_mode(&task.mode)?;
    let current_acceptance = parse_acceptance_policy(&task.acceptance_policy)?;
    let (policy_control, policy_reason) = effective_control_level(mode, requested, policy);
    let mark = task_policy_control_reevaluation(task)?;
    let marked_control = mark
        .as_ref()
        .map(|mark| parse_task_control_level(&mark.required_effective_control_level))
        .transpose()?;
    let marked_acceptance = mark
        .as_ref()
        .and_then(|mark| mark.required_acceptance_policy.as_deref())
        .map(parse_acceptance_policy)
        .transpose()?;

    let policy_selected_control = std::cmp::max(current_control, policy_control);
    let effective_control_level = marked_control
        .map(|level| std::cmp::max(policy_selected_control, level))
        .unwrap_or(policy_selected_control);
    let control_acceptance = acceptance_policy_for_control(effective_control_level, policy);
    let acceptance_policy = max_acceptance_policy(
        max_acceptance_policy(current_acceptance, control_acceptance),
        marked_acceptance.unwrap_or(current_acceptance),
    );
    let control_raised = effective_control_level > current_control;
    let acceptance_raised =
        acceptance_policy_rank(acceptance_policy) > acceptance_policy_rank(current_acceptance);
    let pending_policy_reevaluation = marked_control.is_some_and(|level| level > current_control)
        || marked_acceptance.is_some_and(|required| {
            acceptance_policy_rank(required) > acceptance_policy_rank(current_acceptance)
        });
    let mark_determined_control_raise =
        marked_control.is_some_and(|level| level > std::cmp::max(current_control, policy_control));
    let mark_determined_acceptance_raise = marked_acceptance.is_some_and(|required| {
        acceptance_policy_rank(required)
            > acceptance_policy_rank(max_acceptance_policy(
                current_acceptance,
                control_acceptance,
            ))
    });
    let control_level_reason = if control_raised {
        if mark_determined_control_raise {
            format!(
                "Core raised control to `{}` for a pending project-policy reevaluation.",
                effective_control_level.as_str()
            )
        } else {
            policy_reason
        }
    } else {
        task.control_level_reason.clone()
    };
    let acceptance_policy_reason = if acceptance_raised {
        if mark_determined_acceptance_raise {
            "A pending project-policy reevaluation requires final acceptance for the current close basis."
                .to_owned()
        } else {
            format!(
                "Effective control `{}` requires final acceptance for the current close basis.",
                effective_control_level.as_str()
            )
        }
    } else {
        task.acceptance_policy_reason.clone()
    };

    Ok(ResolvedTaskControlAuthority {
        effective_control_level,
        acceptance_policy,
        control_level_reason,
        acceptance_policy_reason,
        control_raised,
        acceptance_raised,
        pending_policy_reevaluation,
    })
}

pub(crate) fn effective_control_level(
    mode: TaskMode,
    requested: RequestedControlLevel,
    policy: &ProjectWorkflowPolicy,
) -> (TaskControlLevel, String) {
    if mode == TaskMode::Advisor {
        return (
            TaskControlLevel::Observe,
            "Advisor mode is observe-only and cannot authorize product writes.".to_owned(),
        );
    }
    let requested_level = match requested {
        RequestedControlLevel::Auto => match mode {
            TaskMode::Advisor => TaskControlLevel::Observe,
            TaskMode::Direct => policy.default_direct_control,
            TaskMode::Work => policy.default_work_control,
        },
        RequestedControlLevel::Observe => TaskControlLevel::Observe,
        RequestedControlLevel::Light => TaskControlLevel::Light,
        RequestedControlLevel::Tracked => TaskControlLevel::Tracked,
        RequestedControlLevel::Sensitive => TaskControlLevel::Sensitive,
    };
    let project_minimum = match mode {
        TaskMode::Advisor => TaskControlLevel::Observe,
        TaskMode::Direct => policy.default_direct_control,
        TaskMode::Work => std::cmp::max(policy.default_work_control, TaskControlLevel::Tracked),
    };
    let mut effective = std::cmp::max(requested_level, project_minimum);
    if effective == TaskControlLevel::Light && !policy.light.enabled {
        effective = TaskControlLevel::Tracked;
    }
    let reason = if effective > requested_level {
        format!(
            "Core raised requested control `{}` to `{}` for the selected mode and project workflow policy.",
            requested.as_str(),
            effective.as_str()
        )
    } else {
        format!(
            "Core selected effective control `{}` from the caller request and project workflow policy.",
            effective.as_str()
        )
    };
    (effective, reason)
}

pub(crate) fn acceptance_policy_for_control(
    control: TaskControlLevel,
    policy: &ProjectWorkflowPolicy,
) -> AcceptancePolicy {
    match control {
        TaskControlLevel::Observe => AcceptancePolicy::NotRequired,
        TaskControlLevel::Light => policy.light.final_acceptance,
        TaskControlLevel::Tracked | TaskControlLevel::Sensitive => AcceptancePolicy::Required,
    }
}

pub(crate) fn parse_requested_control_level(
    value: &str,
) -> Result<RequestedControlLevel, StoreError> {
    serde_json::from_value(Value::String(value.to_owned())).map_err(|_| {
        StoreError::corrupt_owner_state_value("tasks", "current", "requested_control_level")
    })
}

pub(crate) fn parse_task_control_level(value: &str) -> Result<TaskControlLevel, StoreError> {
    serde_json::from_value(Value::String(value.to_owned())).map_err(|_| {
        StoreError::corrupt_owner_state_value("tasks", "current", "effective_control_level")
    })
}

fn parse_task_mode(value: &str) -> Result<TaskMode, StoreError> {
    serde_json::from_value(Value::String(value.to_owned()))
        .map_err(|_| StoreError::corrupt_owner_state_value("tasks", "current", "mode"))
}

fn parse_acceptance_policy(value: &str) -> Result<AcceptancePolicy, StoreError> {
    serde_json::from_value(Value::String(value.to_owned()))
        .map_err(|_| StoreError::corrupt_owner_state_value("tasks", "current", "acceptance_policy"))
}

fn acceptance_policy_rank(policy: AcceptancePolicy) -> u8 {
    match policy {
        AcceptancePolicy::NotRequired => 0,
        AcceptancePolicy::PolicyDependent => 1,
        AcceptancePolicy::Required => 2,
    }
}

fn max_acceptance_policy(left: AcceptancePolicy, right: AcceptancePolicy) -> AcceptancePolicy {
    if acceptance_policy_rank(left) >= acceptance_policy_rank(right) {
        left
    } else {
        right
    }
}
