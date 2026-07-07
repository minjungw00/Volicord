use std::path::Path;

use serde_json::json;
use volicord_store::{
    bootstrap::ProjectRecord,
    session_watch::{
        create_watch_baseline, latest_watch_baseline_for_session, snapshot_product_repository,
        SessionWatchStatus, WatchBaselineCreate, WatchSnapshotOptions,
    },
};
use volicord_types::{GuardDecision, IntegrationProfile, SessionWatchCoverageBasis};

use super::GuardPhaseResult;
use crate::guard_command::{
    args::{GuardInput, GuardPhase},
    context::{guard_state_summary, session_watch_scan_summary_from_snapshot},
    envelope::{event_time_or_now, GuardEnvelope},
    format_timestamp,
    render::context_json,
    stable_id, GuardCommandError, SESSION_WATCH_METADATA_SOURCE,
};

pub(in crate::guard_command) fn handle_session_start(
    runtime_home: &Path,
    project: &ProjectRecord,
    envelope: &GuardEnvelope,
    input: &GuardInput,
) -> Result<GuardPhaseResult, GuardCommandError> {
    initialize_observe_session_watch(runtime_home, project, envelope, GuardPhase::SessionStart)?;
    let summary = guard_state_summary(runtime_home, project, envelope, input)?;
    Ok(GuardPhaseResult::new(
        GuardDecision::InjectContext,
        json!({
            "decision": GuardDecision::InjectContext.as_str(),
            "message": "Volicord context is available for this host session.",
            "context": context_json(&summary),
            "enforcement_level": "cooperative_detective"
        }),
    ))
}

fn initialize_observe_session_watch(
    runtime_home: &Path,
    project: &ProjectRecord,
    envelope: &GuardEnvelope,
    phase: GuardPhase,
) -> Result<(), GuardCommandError> {
    if phase != GuardPhase::SessionStart
        || envelope.guard_mode != IntegrationProfile::Detective.as_str()
    {
        return Ok(());
    }
    let Some(session_id) = envelope.session_id.as_deref() else {
        return Ok(());
    };
    if latest_watch_baseline_for_session(runtime_home, &project.project_id, session_id)?.is_some() {
        return Ok(());
    }
    let snapshot = snapshot_product_repository(
        runtime_home,
        &project.repo_root,
        WatchSnapshotOptions::default(),
    )
    .map_err(|error| {
        GuardCommandError::Runtime(format!(
            "failed to start detective session watcher for {}: {error}",
            project.repo_root.display()
        ))
    })?;
    let started_at = format_timestamp(event_time_or_now(&envelope.occurred_at));
    let watch_baseline_id = stable_id(
        "watch_base",
        &[
            &project.project_id,
            session_id,
            &envelope.connection_id,
            &snapshot.digest,
        ],
    );
    let scan_summary = session_watch_scan_summary_from_snapshot(&snapshot);
    create_watch_baseline(
        runtime_home,
        &project.project_id,
        WatchBaselineCreate {
            watch_baseline_id,
            session_id: session_id.to_owned(),
            connection_internal_id: envelope.connection_id.clone(),
            guard_installation_id: envelope.guard_installation_id.clone(),
            status: SessionWatchStatus::Active,
            snapshot,
            created_at: started_at.clone(),
            metadata_json: json!({
                "source": SESSION_WATCH_METADATA_SOURCE,
                "status_detail": "active",
                "detector_role": "detective",
                "does_not_prevent_writes": true,
                "does_not_identify_actor": true,
                "coverage_start_at": started_at,
                "coverage_basis": SessionWatchCoverageBasis::McpStart.as_str(),
                "coverage_started_by": "session_start_hook",
                "scan_summary": scan_summary
            })
            .to_string(),
        },
    )?;
    Ok(())
}
