use crate::pipeline::{CorePipelineError, VerifiedInvocationContext};
use crate::policy::workflow::{project_workflow_policy, resolve_task_control_authority};
use crate::product_path::{observe_product_paths, ProductPathValidationError};
use crate::record_refs::sorted_unique;
use crate::recording::{
    recording_store_error, recording_validation_error, RecordingError, RecordingRejection,
};
use crate::write_ticket::normalized_string_set;
use crate::write_ticket::{baseline_matches, workspace_context_matches};
use volicord_platform_fs::PlatformDiagnosticClass;
use volicord_store::core_pipeline::{ChangeUnitStatus, CoreProjectStore, ProjectStateHeader};
use volicord_types::schema::ObservedChanges;
use volicord_types::values::{RunKind, TaskMode, WorkPhase};

use super::{
    evidence::normalize_record_run_evidence_targets,
    model::{RecordRunFacts, RecordRunNormalizedRequest, RecordRunRawRequest},
};

pub(super) fn task_mode_allows_run_kind(
    task_mode: TaskMode,
    work_phase: WorkPhase,
    run_kind: RunKind,
) -> bool {
    match task_mode {
        TaskMode::Advisor => run_kind == RunKind::ShapingUpdate,
        TaskMode::Direct => run_kind == RunKind::Direct,
        TaskMode::Work => match work_phase {
            WorkPhase::Shaping => run_kind == RunKind::ShapingUpdate,
            WorkPhase::Implementation => run_kind == RunKind::Implementation,
        },
    }
}

pub(super) fn normalize_record_run_request(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    mut raw: RecordRunRawRequest,
) -> Result<RecordRunNormalizedRequest, RecordingError> {
    let request = &mut raw.request;
    let normalized_performed_operation = request
        .performed_operation
        .as_ref()
        .map(|operation| operation.trim().to_owned());
    if normalized_performed_operation
        .as_ref()
        .is_some_and(String::is_empty)
    {
        return recording_validation_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "performed_operation",
            "performed_operation must be null, omitted, or a non-empty operation",
        );
    }
    request.performed_operation = normalized_performed_operation.into();
    if request.summary.trim().is_empty() {
        return recording_validation_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "summary",
            "summary must not be empty",
        );
    }
    if request
        .run_id
        .as_ref()
        .is_some_and(|id| id.as_str().trim().is_empty())
    {
        return recording_validation_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "run_id",
            "run_id must be null or a non-empty identifier",
        );
    }

    let normalized_changed_paths = sorted_unique(
        match observe_product_paths(
            &store.project_record().repo_root,
            &request.observed_changes.changed_paths,
        ) {
            Ok(paths) => paths,
            Err(ProductPathValidationError::Lexical(_)) => {
                return recording_validation_error(
                    request.envelope.dry_run,
                    Some(project_state.state_version),
                    "observed_changes.changed_paths",
                    "changed_paths must be normalized relative Product Repository paths",
                )
            }
            Err(error @ ProductPathValidationError::Platform(_))
                if error.platform_class() == Some(PlatformDiagnosticClass::Rejected) =>
            {
                return Err(RecordingError::Rejected(
                    RecordingRejection::ProductPathContainment {
                        message: "changed_paths resolve outside the Product Repository",
                    },
                ))
            }
            Err(ProductPathValidationError::Platform(error)) => {
                return Err(RecordingError::Core(CorePipelineError::from(error)))
            }
        },
    );
    if request.observed_changes.product_file_write_observed && normalized_changed_paths.is_empty() {
        return recording_validation_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "observed_changes",
            "product_file_write_observed requires at least one changed_path",
        );
    }
    if !request.observed_changes.product_file_write_observed && !normalized_changed_paths.is_empty()
    {
        return recording_validation_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "observed_changes",
            "changed_paths require product_file_write_observed=true",
        );
    }
    if request
        .observed_changes
        .baseline_ref
        .as_ref()
        .is_some_and(|baseline_ref| baseline_ref != &request.baseline_ref)
    {
        return recording_validation_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "observed_changes.baseline_ref",
            "observed_changes.baseline_ref must match request baseline_ref when present",
        );
    }
    let normalized_sensitive_categories =
        normalized_string_set(&request.observed_changes.sensitive_categories);

    normalize_record_run_evidence_targets(request);
    let normalized_observed_changes = ObservedChanges {
        changed_paths: normalized_changed_paths.clone(),
        product_file_write_observed: request.observed_changes.product_file_write_observed,
        sensitive_categories: normalized_sensitive_categories,
        baseline_ref: Some(request.baseline_ref.clone()).into(),
    };
    Ok(RecordRunNormalizedRequest {
        raw,
        planned_state_version: project_state.state_version + 1,
        normalized_changed_paths,
        normalized_observed_changes,
    })
}

pub(super) fn acquire_record_run_facts(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    normalized: RecordRunNormalizedRequest,
    verified_invocation: &VerifiedInvocationContext,
) -> Result<RecordRunFacts, RecordingError> {
    let request = &normalized.raw.request;
    let normalized_changed_paths = &normalized.normalized_changed_paths;
    let normalized_sensitive_categories =
        &normalized.normalized_observed_changes.sensitive_categories;
    let task = store
        .task_record(&request.task_id)
        .map_err(|error| recording_store_error(&request.envelope, project_state, error))?
        .ok_or(RecordingError::Rejected(RecordingRejection::NoActiveTask))?;
    let task_mode = task.mode;
    let workflow_policy = project_workflow_policy(store).map_err(CorePipelineError::from)?;
    let resolved_control =
        resolve_task_control_authority(&task, &workflow_policy).map_err(CorePipelineError::from)?;
    let work_phase = task.work_phase;
    if !task_mode_allows_run_kind(task_mode, work_phase, request.kind) {
        return recording_validation_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "kind",
            "kind is not compatible with the current Task mode and work phase",
        );
    }
    if task_mode == TaskMode::Advisor
        && (request.observed_changes.product_file_write_observed
            || !normalized_changed_paths.is_empty()
            || !normalized_sensitive_categories.is_empty())
    {
        return recording_validation_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "observed_changes",
            "advisor Task runs cannot report Product Repository file changes or sensitive effects",
        );
    }
    if task_mode == TaskMode::Advisor && request.write_ticket_id.is_some() {
        return recording_validation_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "write_ticket_id",
            "advisor Task runs cannot consume a write ticket",
        );
    }
    let change_unit = store
        .change_unit_record(&request.task_id, request.change_unit_id.as_str())
        .map_err(|error| recording_store_error(&request.envelope, project_state, error))?
        .ok_or(RecordingError::Rejected(
            RecordingRejection::NoActiveChangeUnit {
                message: "change_unit_id does not identify a Change Unit for the Task",
            },
        ))?;
    if change_unit.status != ChangeUnitStatus::Active || !change_unit.is_current {
        return Err(RecordingError::Rejected(
            RecordingRejection::NoActiveChangeUnit {
                message: "record_run requires the current active Change Unit",
            },
        ));
    }
    if !baseline_matches(&change_unit, &task, &request.baseline_ref)? {
        return Err(RecordingError::Rejected(RecordingRejection::BaselineStale));
    }
    if !request.observed_changes.product_file_write_observed
        && !workspace_context_matches(&change_unit, verified_invocation)?
    {
        return Err(RecordingError::Rejected(RecordingRejection::WorkspaceStale));
    }

    Ok(RecordRunFacts {
        normalized,
        task,
        change_unit,
        workflow_policy,
        resolved_control,
    })
}
