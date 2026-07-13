use std::path::Path;

use serde_json::Value;
use volicord_store::{
    bootstrap::ProjectRecord,
    core_pipeline::CoreProjectStore,
    guards::list_unresolved_unrecorded_changes,
    session_watch::{
        latest_watch_baseline_for_session, watch_scan_summary_from_entries_json, WatchScanSummary,
        WatchSnapshot, DEFAULT_MAX_FILE_HASH_BYTES, DEFAULT_MAX_SCAN_FILE_COUNT,
    },
};
use volicord_types::{
    ProjectId, PromptCaptureStatus, SessionWatchScanSummary, TaskId, UtcTimestamp,
    WriteTicketAttemptScope,
};

use super::{
    args::GuardInput,
    core_current_timestamp,
    envelope::GuardEnvelope,
    json_error,
    prompt_capture::{
        pending_chat_user_action_summaries, prompt_capture_availability_for_event,
        GuardPendingUserActionSummary,
    },
    GuardCommandError,
};

#[derive(Debug, Clone, PartialEq)]
pub(super) struct GuardStateSummary {
    pub(super) project_id: String,
    pub(super) project_name: String,
    pub(super) repo_root: String,
    pub(super) state_version: u64,
    pub(super) active_task_id: Option<String>,
    pub(super) active_change_unit_id: Option<String>,
    pub(super) prompt_capture_status: PromptCaptureStatus,
    pub(super) prompt_capture_enabled: bool,
    pub(super) current_write_ticket_ids: Vec<String>,
    pub(super) stale_write_ticket_ids: Vec<String>,
    pub(super) active_write_tickets: Vec<ActiveWriteTicketSummary>,
    pub(super) pending_user_action_count: usize,
    pub(super) pending_user_actions: Vec<GuardPendingUserActionSummary>,
    pub(super) active_blocker_count: usize,
    pub(super) unresolved_unrecorded_change_count: usize,
    pub(super) session_watch_scan_summary: Option<SessionWatchScanSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ActiveWriteTicketSummary {
    pub(super) write_ticket_id: String,
    pub(super) change_unit_id: Option<String>,
    pub(super) intended_paths: Vec<String>,
    pub(super) expires_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct GuardReason {
    pub(super) code: &'static str,
    pub(super) message: String,
    pub(super) severity: &'static str,
}

pub(super) fn guard_state_summary(
    runtime_home: &Path,
    project: &ProjectRecord,
    envelope: &GuardEnvelope,
    input: &GuardInput,
) -> Result<GuardStateSummary, GuardCommandError> {
    let store = CoreProjectStore::open(runtime_home, &ProjectId::new(&project.project_id))?;
    let project_state = store.project_state()?;
    let now_timestamp = core_current_timestamp(&store)?;
    let mut current_write_ticket_ids = Vec::new();
    let mut stale_write_ticket_ids = Vec::new();
    let mut active_write_tickets = Vec::new();
    let mut active_change_unit_id = None;
    let mut pending_user_action_count = 0;
    let mut pending_user_actions = Vec::new();
    let mut active_blocker_count = 0;
    let prompt_capture_availability =
        prompt_capture_availability_for_event(runtime_home, project, envelope)?;
    let prompt_capture_status = prompt_capture_availability.status;
    let prompt_capture_enabled = prompt_capture_availability.can_use_chat_commands();
    if let Some(active_task_id) = project_state.active_task_id.as_deref() {
        let task_id = TaskId::new(active_task_id);
        active_change_unit_id = store
            .task_record(&task_id)?
            .and_then(|task| task.current_change_unit_id);
        for record in store.active_write_tickets(&task_id)? {
            let current_basis = record.basis_state_version == project_state.state_version;
            let not_expired = UtcTimestamp::parse(&record.expires_at)
                .map(|expires_at| now_timestamp < expires_at)
                .unwrap_or(false);
            if current_basis && not_expired {
                let write_ticket_id = record.write_ticket_id.clone();
                current_write_ticket_ids.push(write_ticket_id.clone());
                let attempt_scope: WriteTicketAttemptScope =
                    serde_json::from_str(&record.attempt_scope_json).map_err(json_error)?;
                if attempt_scope.product_file_write_intended {
                    active_write_tickets.push(ActiveWriteTicketSummary {
                        write_ticket_id,
                        change_unit_id: record.change_unit_id.clone(),
                        intended_paths: attempt_scope.intended_paths,
                        expires_at: record.expires_at,
                    });
                }
            } else {
                stale_write_ticket_ids.push(record.write_ticket_id);
            }
        }
        pending_user_action_count = store
            .pending_user_action_records(&task_id, &now_timestamp)?
            .len();
        if prompt_capture_enabled {
            pending_user_actions = pending_chat_user_action_summaries(
                runtime_home,
                &store,
                &task_id,
                envelope,
                &now_timestamp,
            )?;
        }
        active_blocker_count = store
            .active_blocker_refs(&task_id, project_state.state_version)?
            .len();
    }
    let unresolved_unrecorded_change_count = list_unresolved_unrecorded_changes(
        runtime_home,
        &project.project_id,
        Some(&envelope.connection_id),
    )?
    .len();
    let session_watch_scan_summary =
        guard_session_watch_scan_summary(runtime_home, project, envelope)?;
    let _ = input.raw_text.len();
    Ok(GuardStateSummary {
        project_id: project.project_id.clone(),
        project_name: project.project_name.clone(),
        repo_root: project.repo_root.display().to_string(),
        state_version: project_state.state_version,
        active_task_id: project_state.active_task_id,
        active_change_unit_id,
        prompt_capture_status,
        prompt_capture_enabled,
        current_write_ticket_ids,
        stale_write_ticket_ids,
        active_write_tickets,
        pending_user_action_count,
        pending_user_actions,
        active_blocker_count,
        unresolved_unrecorded_change_count,
        session_watch_scan_summary,
    })
}

pub(super) fn session_watch_scan_summary_from_snapshot(
    snapshot: &WatchSnapshot,
) -> SessionWatchScanSummary {
    session_watch_scan_summary_from_store(&snapshot.scan_summary)
}

fn guard_session_watch_scan_summary(
    runtime_home: &Path,
    project: &ProjectRecord,
    envelope: &GuardEnvelope,
) -> Result<Option<SessionWatchScanSummary>, GuardCommandError> {
    let Some(session_id) = envelope.session_id.as_deref() else {
        return Ok(None);
    };
    let Some(baseline) =
        latest_watch_baseline_for_session(runtime_home, &project.project_id, session_id)?
    else {
        return Ok(None);
    };
    if let Ok(metadata) = serde_json::from_str::<Value>(&baseline.metadata_json) {
        if let Some(raw_summary) = metadata.get("scan_summary") {
            if let Ok(summary) =
                serde_json::from_value::<SessionWatchScanSummary>(raw_summary.clone())
            {
                return Ok(Some(summary));
            }
        }
    }
    let summary = watch_scan_summary_from_entries_json(&baseline.snapshot_entries_json)?;
    Ok(Some(session_watch_scan_summary_from_store(&summary)))
}

fn session_watch_scan_summary_from_store(summary: &WatchScanSummary) -> SessionWatchScanSummary {
    SessionWatchScanSummary {
        files_scanned: summary.files_scanned,
        files_skipped: summary.files_skipped,
        unreadable_paths_count: summary.unreadable_paths_count,
        degraded_reasons: summary.degraded_reasons.clone(),
        degraded_reason_counts: summary.degraded_reason_counts.clone(),
        skipped_paths_sample: summary.skipped_paths_sample.clone(),
        skipped_paths_truncated: summary.skipped_paths_truncated,
        default_excluded_paths: volicord_store::session_watch::default_watch_excluded_paths(),
        max_file_size_bytes: DEFAULT_MAX_FILE_HASH_BYTES,
        max_file_count: DEFAULT_MAX_SCAN_FILE_COUNT,
        follows_symlinks: false,
        not_full_filesystem_monitoring: true,
    }
}
