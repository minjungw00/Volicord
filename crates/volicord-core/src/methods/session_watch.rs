use std::{collections::BTreeSet, path::PathBuf};

use chrono::{DateTime, Utc};
use volicord_store::{
    guards::{
        list_expected_writes_for_connection, list_unresolved_unrecorded_changes,
        AgentSessionInsert, ExpectedWriteRecord, UnrecordedChangeInsert, UnrecordedChangeRecord,
    },
    session_watch::{
        compare_watch_snapshots, create_watch_baseline, latest_watch_baseline_for_connection,
        latest_watch_baseline_for_session, link_watch_observation_to_unrecorded_change,
        record_watch_observation, snapshot_product_repository, update_watch_status, watch_baseline,
        watch_observation_for_baseline_digest, watch_observations_for_unrecorded_change,
        WatchBaselineCreate, WatchBaselineRecord, WatchObservationInsert, WatchObservationRecord,
        WatchScopeKind, WatchSnapshot, WatchSnapshotEntry, WatchSnapshotOptions, WatchStatusUpdate,
    },
};

use super::reconcile_changes::{observed_paths, system_resolution, ResolutionCandidate};
use super::*;

const WATCH_METADATA_SOURCE: &str = "volicord_session_watch";
const METHOD_BOUNDARY_PARTIAL_COVERAGE_WARNING: &str =
    "Session-watch coverage starts at a method boundary; Product Repository changes before that boundary are outside watcher coverage.";
const UNKNOWN_BASIS_PARTIAL_COVERAGE_WARNING: &str =
    "Session-watch coverage basis was not recorded; treat coverage as starting at a method boundary.";

pub(super) fn initialize_session_watch_baseline(
    store: &CoreProjectStore,
    verified_invocation: &VerifiedInvocationContext,
    now: &UtcTimestamp,
) -> CoreResult<()> {
    let Some(connection_id) = verified_invocation.actor_source.agent_connection_id() else {
        return Ok(());
    };
    let Some(session_id) = verified_invocation.session_id.as_deref() else {
        return Ok(());
    };
    ensure_agent_session(
        store,
        verified_invocation,
        session_id,
        connection_id.as_str(),
        now,
    )?;
    if latest_watch_baseline_for_session(
        store.runtime_home(),
        verified_invocation.project_id.as_str(),
        session_id,
    )
    .map_err(CorePipelineError::from)?
    .is_some()
    {
        return Ok(());
    }
    let snapshot = match snapshot_product_repository(
        store.runtime_home(),
        &store.project_record().repo_root,
        WatchSnapshotOptions::default(),
    ) {
        Ok(snapshot) => snapshot,
        Err(_) => return Ok(()),
    };
    let baseline_id = stable_watch_id(
        "watch_base",
        &[
            verified_invocation.project_id.as_str(),
            session_id,
            connection_id.as_str(),
            &snapshot.digest,
        ],
    );
    let scan_summary = session_watch_scan_summary_from_snapshot(&snapshot);
    create_watch_baseline(
        store.runtime_home(),
        verified_invocation.project_id.as_str(),
        WatchBaselineCreate {
            watch_baseline_id: baseline_id,
            session_id: session_id.to_owned(),
            connection_internal_id: connection_id.as_str().to_owned(),
            guard_installation_id: selected_guard_installation_id(store, verified_invocation)?,
            status: volicord_store::session_watch::SessionWatchStatus::Active,
            snapshot,
            created_at: now.to_string(),
            metadata_json: serde_json::to_string(&json!({
                "source": WATCH_METADATA_SOURCE,
                "status_detail": "active",
                "detector_role": "detective",
                "does_not_prevent_writes": true,
                "does_not_identify_actor": true,
                "coverage_start_at": now.to_string(),
                "coverage_basis": SessionWatchCoverageBasis::MethodBoundary.as_str(),
                "partial_coverage_warning": METHOD_BOUNDARY_PARTIAL_COVERAGE_WARNING,
                "scan_summary": scan_summary
            }))?,
        },
    )
    .map_err(CorePipelineError::from)?;
    Ok(())
}

pub(super) fn run_session_watch_check(
    store: &CoreProjectStore,
    verified_invocation: &VerifiedInvocationContext,
    task_id: Option<&TaskId>,
    now: &UtcTimestamp,
) -> CoreResult<()> {
    initialize_session_watch_baseline(store, verified_invocation, now)?;
    let Some(connection_id) = verified_invocation.actor_source.agent_connection_id() else {
        return Ok(());
    };
    let Some(session_id) = verified_invocation.session_id.as_deref() else {
        return Ok(());
    };
    let Some(baseline) = latest_watch_baseline_for_session(
        store.runtime_home(),
        verified_invocation.project_id.as_str(),
        session_id,
    )
    .map_err(CorePipelineError::from)?
    else {
        return Ok(());
    };
    if baseline.status == volicord_store::session_watch::SessionWatchStatus::Disabled.as_str() {
        return Ok(());
    }
    let options = snapshot_options_from_baseline(&baseline)?;
    let current = match snapshot_product_repository(
        store.runtime_home(),
        &store.project_record().repo_root,
        options,
    ) {
        Ok(snapshot) => snapshot,
        Err(error) => {
            update_watch_unavailable(store, &baseline, now, &error.to_string())?;
            return Ok(());
        }
    };
    if current.digest == baseline.snapshot_digest {
        update_watch_active(store, &baseline, now, "active")?;
        return Ok(());
    }
    update_watch_active(store, &baseline, now, "change_detected")?;
    if watch_observation_for_baseline_digest(
        store.runtime_home(),
        verified_invocation.project_id.as_str(),
        &baseline.watch_baseline_id,
        &current.digest,
    )
    .map_err(CorePipelineError::from)?
    .is_some()
    {
        return Ok(());
    }

    let baseline_snapshot = snapshot_from_baseline(&baseline)?;
    let diff = compare_watch_snapshots(&baseline_snapshot, &current);
    if diff.changes.is_empty() {
        return Ok(());
    }
    let observed_paths = diff
        .changes
        .iter()
        .map(|change| change.path.clone())
        .collect::<Vec<_>>();
    let correlation = watch_write_correlation(
        store,
        verified_invocation.project_id.as_str(),
        connection_id.as_str(),
        Some(session_id),
        task_id,
        &observed_paths,
        *now.as_datetime(),
    )?;
    let observation_id = stable_watch_id(
        "watch_obs",
        &[
            verified_invocation.project_id.as_str(),
            &baseline.watch_baseline_id,
            &current.digest,
        ],
    );
    let observation = record_watch_observation(
        store.runtime_home(),
        verified_invocation.project_id.as_str(),
        WatchObservationInsert {
            watch_observation_id: observation_id,
            watch_baseline_id: baseline.watch_baseline_id.clone(),
            expected_write_id: correlation
                .expected_write
                .as_ref()
                .map(|write| write.expected_write_id.clone()),
            snapshot: current.clone(),
            diff,
            observed_at: now.to_string(),
            metadata_json: serde_json::to_string(&json!({
                "source": WATCH_METADATA_SOURCE,
                "correlation_status": correlation.status.clone(),
                "write_ticket_ids": correlation.write_ticket_ids.clone(),
                "detector_role": "detective",
                "does_not_prevent_writes": true,
                "does_not_identify_actor": true
            }))?,
        },
    )
    .map_err(CorePipelineError::from)?;
    if !correlation.deterministically_linked {
        if let Some(existing_change) = existing_unrecorded_change_covering_paths(
            store,
            verified_invocation.project_id.as_str(),
            connection_id.as_str(),
            session_id,
            task_id,
            &observed_paths,
        )? {
            link_watch_observation_to_unrecorded_change(
                store.runtime_home(),
                verified_invocation.project_id.as_str(),
                &observation.watch_observation_id,
                &existing_change.unrecorded_change_id,
                &now.to_string(),
            )
            .map_err(CorePipelineError::from)?;
        } else {
            insert_unrecorded_change_for_observation(
                store,
                verified_invocation,
                task_id,
                &baseline,
                &observation,
                &current,
                &observed_paths,
                &correlation.status,
                &correlation.write_ticket_ids,
                now,
            )?;
        }
    }
    Ok(())
}

pub(super) fn apply_session_watch_status(
    store: &CoreProjectStore,
    verified_invocation: &VerifiedInvocationContext,
    summary: &mut GuardHealthSummary,
) -> Result<(), PlanError> {
    let Some(connection_id) = verified_invocation.actor_source.agent_connection_id() else {
        return Ok(());
    };
    let baseline = if let Some(session_id) = verified_invocation.session_id.as_deref() {
        latest_watch_baseline_for_session(
            store.runtime_home(),
            verified_invocation.project_id.as_str(),
            session_id,
        )
    } else {
        latest_watch_baseline_for_connection(
            store.runtime_home(),
            verified_invocation.project_id.as_str(),
            connection_id.as_str(),
        )
    }
    .map_err(CorePipelineError::from)
    .map_err(PlanError::Core)?;
    let Some(baseline) = baseline else {
        if let Some(session_id) = verified_invocation.session_id.as_deref() {
            if volicord_store::guards::agent_session(
                store.runtime_home(),
                verified_invocation.project_id.as_str(),
                session_id,
            )
            .map_err(CorePipelineError::from)
            .map_err(PlanError::Core)?
            .is_some()
            {
                summary.session_watch_status = SessionWatchStatus::Unavailable;
                summary.last_session_watch_checked_at = RequiredNullable::null();
                summary.session_watch_baseline_created_at = RequiredNullable::null();
                summary.session_watch_coverage_start_at = RequiredNullable::null();
                summary.session_watch_coverage_basis = RequiredNullable::null();
                summary.session_watch_partial_coverage_warning = Some(
                    "Session-watch baseline is unavailable; Product Repository changes are outside watcher coverage until a baseline exists."
                        .to_owned(),
                )
                .into();
                summary.session_watch_detail =
                    Some("session_watch_baseline_unavailable".to_owned()).into();
                summary.session_watch_scan_summary = RequiredNullable::null();
            }
        }
        return Ok(());
    };
    summary.session_watch_status = parse_session_watch_status(
        "session_watch_baselines",
        &baseline.watch_baseline_id,
        &baseline.status,
    )?;
    summary.last_session_watch_checked_at = Some(parse_owner_storage_value(
        "session_watch_baselines",
        baseline.watch_baseline_id.clone(),
        "updated_at",
        &baseline.updated_at,
    )?)
    .into();
    summary.session_watch_baseline_created_at = Some(parse_owner_storage_value(
        "session_watch_baselines",
        baseline.watch_baseline_id.clone(),
        "created_at",
        &baseline.created_at,
    )?)
    .into();
    apply_session_watch_coverage_metadata(summary, &baseline)?;
    summary.session_watch_detail = watch_status_detail(&baseline.metadata_json).into();
    summary.session_watch_scan_summary =
        Some(session_watch_scan_summary_from_baseline(&baseline)?).into();
    Ok(())
}

pub(super) fn watcher_reverted_resolution(
    store: &CoreProjectStore,
    record: &UnrecordedChangeRecord,
) -> CoreResult<Option<ResolutionCandidate>> {
    if !is_session_watch_change(record) {
        return Ok(None);
    }
    let observations = watch_observations_for_unrecorded_change(
        store.runtime_home(),
        &record.project_id,
        &record.unrecorded_change_id,
    )
    .map_err(CorePipelineError::from)?;
    let Some(observation) = observations.first() else {
        return Ok(None);
    };
    let Some(baseline) = watch_baseline(
        store.runtime_home(),
        &record.project_id,
        &observation.watch_baseline_id,
    )
    .map_err(CorePipelineError::from)?
    else {
        return Ok(None);
    };
    let options = snapshot_options_from_baseline(&baseline)?;
    let current = snapshot_product_repository(
        store.runtime_home(),
        &store.project_record().repo_root,
        options,
    )
    .map_err(CorePipelineError::from)?;
    if current.digest == baseline.snapshot_digest {
        return Ok(Some(system_resolution(
            UnrecordedChangeResolutionBasis::Reverted,
            "core_deterministic_session_watch_reverted",
        )));
    }
    Ok(None)
}

pub(super) fn watcher_expected_write_resolution(
    store: &CoreProjectStore,
    record: &UnrecordedChangeRecord,
    task_id: &TaskId,
) -> CoreResult<Option<ResolutionCandidate>> {
    if !is_session_watch_change(record) {
        return Ok(None);
    }
    let observed_paths = match observed_paths(record) {
        Ok(paths) => paths,
        Err(()) => return Ok(None),
    };
    let expected_write = expected_write_covering_paths(
        store,
        &record.project_id,
        &record.connection_internal_id,
        record.session_id.as_deref(),
        Some(task_id),
        &observed_paths,
    )?;
    Ok(expected_write.map(|_| {
        system_resolution(
            UnrecordedChangeResolutionBasis::RecordedAsExpectedWrite,
            "core_deterministic_session_watch_expected_write",
        )
    }))
}

fn ensure_agent_session(
    store: &CoreProjectStore,
    verified_invocation: &VerifiedInvocationContext,
    session_id: &str,
    connection_internal_id: &str,
    now: &UtcTimestamp,
) -> CoreResult<()> {
    if volicord_store::guards::agent_session(
        store.runtime_home(),
        verified_invocation.project_id.as_str(),
        session_id,
    )
    .map_err(CorePipelineError::from)?
    .is_some()
    {
        return Ok(());
    }
    let record = volicord_store::guards::guard_health_record(
        store.runtime_home(),
        verified_invocation.project_id.as_str(),
        connection_internal_id,
    )
    .map_err(CorePipelineError::from)?;
    let guard_installation_id = record
        .guard_installation
        .as_ref()
        .map(|installation| installation.guard_installation_id.clone());
    let guard_mode = record
        .guard_installation
        .as_ref()
        .map(|installation| installation.guard_mode.clone())
        .or_else(|| {
            record
                .latest_session
                .as_ref()
                .map(|session| session.guard_mode.clone())
        })
        .unwrap_or_else(|| IntegrationProfile::Record.as_str().to_owned());
    let host_kind = record
        .guard_installation
        .as_ref()
        .map(|installation| installation.host_kind.clone())
        .or_else(|| {
            record
                .connection
                .as_ref()
                .map(|connection| connection.host_kind.clone())
        })
        .unwrap_or_else(|| "unknown".to_owned());
    volicord_store::guards::insert_agent_session(
        store.runtime_home(),
        verified_invocation.project_id.as_str(),
        AgentSessionInsert {
            session_id: session_id.to_owned(),
            connection_internal_id: connection_internal_id.to_owned(),
            guard_installation_id,
            host_kind,
            guard_mode,
            started_at: now.to_string(),
            metadata_json: serde_json::to_string(&json!({
                "source": WATCH_METADATA_SOURCE,
                "session_watch_initialized": true
            }))?,
        },
    )
    .map_err(CorePipelineError::from)?;
    Ok(())
}

fn selected_guard_installation_id(
    store: &CoreProjectStore,
    verified_invocation: &VerifiedInvocationContext,
) -> CoreResult<Option<String>> {
    let Some(connection_id) = verified_invocation.actor_source.agent_connection_id() else {
        return Ok(None);
    };
    let record = volicord_store::guards::guard_health_record(
        store.runtime_home(),
        verified_invocation.project_id.as_str(),
        connection_id.as_str(),
    )
    .map_err(CorePipelineError::from)?;
    Ok(record
        .guard_installation
        .map(|installation| installation.guard_installation_id))
}

fn existing_unrecorded_change_covering_paths(
    store: &CoreProjectStore,
    project_id: &str,
    connection_internal_id: &str,
    session_id: &str,
    task_id: Option<&TaskId>,
    observed_paths: &[String],
) -> CoreResult<Option<UnrecordedChangeRecord>> {
    let records = list_unresolved_unrecorded_changes(
        store.runtime_home(),
        project_id,
        Some(connection_internal_id),
    )
    .map_err(CorePipelineError::from)?;
    for record in records {
        if record.session_id.as_deref() != Some(session_id) {
            continue;
        }
        if task_id.is_some_and(|task_id| record.task_id.as_deref() != Some(task_id.as_str())) {
            continue;
        }
        let recorded_paths = decode_json_string_array(
            "unrecorded_changes",
            &record.unrecorded_change_id,
            "observed_paths_json",
            &record.observed_paths_json,
        )?;
        if paths_are_authorized(observed_paths, &recorded_paths) {
            return Ok(Some(record));
        }
    }
    Ok(None)
}

#[allow(clippy::too_many_arguments)]
fn insert_unrecorded_change_for_observation(
    store: &CoreProjectStore,
    verified_invocation: &VerifiedInvocationContext,
    task_id: Option<&TaskId>,
    baseline: &WatchBaselineRecord,
    observation: &WatchObservationRecord,
    snapshot: &WatchSnapshot,
    observed_paths: &[String],
    correlation_status: &str,
    write_ticket_ids: &[String],
    now: &UtcTimestamp,
) -> CoreResult<()> {
    let change_id = stable_watch_id(
        "unrec_watch",
        &[
            verified_invocation.project_id.as_str(),
            &observation.watch_observation_id,
        ],
    );
    let change = volicord_store::guards::insert_unrecorded_change(
        store.runtime_home(),
        verified_invocation.project_id.as_str(),
        UnrecordedChangeInsert {
            unrecorded_change_id: change_id,
            session_id: Some(observation.session_id.clone()),
            connection_internal_id: observation.connection_internal_id.clone(),
            task_id: task_id.map(|id| id.as_str().to_owned()),
            summary:
                "Session watch detected Product Repository changes not covered by expected writes."
                    .to_owned(),
            observed_paths_json: serde_json::to_string(observed_paths)?,
            detection_json: serde_json::to_string(&json!({
                "source": WATCH_METADATA_SOURCE,
                "watch_observation_id": observation.watch_observation_id,
                "watch_baseline_id": baseline.watch_baseline_id,
                "baseline_snapshot_digest": baseline.snapshot_digest,
                "snapshot_algorithm": snapshot.algorithm,
                "snapshot_digest": snapshot.digest,
                "correlation_status": correlation_status,
                "write_ticket_ids": write_ticket_ids,
                "detector_role": "detective",
                "does_not_prevent_writes": true,
                "does_not_identify_actor": true
            }))?,
            detected_at: now.to_string(),
            metadata_json: serde_json::to_string(&json!({
                "source": WATCH_METADATA_SOURCE,
                "resolution_basis_owner": "volicord.reconcile_changes",
                "stored_payload": "path_hash_size_metadata_only"
            }))?,
        },
    )
    .map_err(CorePipelineError::from)?;
    link_watch_observation_to_unrecorded_change(
        store.runtime_home(),
        verified_invocation.project_id.as_str(),
        &observation.watch_observation_id,
        &change.unrecorded_change_id,
        &now.to_string(),
    )
    .map_err(CorePipelineError::from)?;
    Ok(())
}

struct WatchWriteCorrelation {
    status: String,
    expected_write: Option<ExpectedWriteRecord>,
    write_ticket_ids: Vec<String>,
    deterministically_linked: bool,
}

fn watch_write_correlation(
    store: &CoreProjectStore,
    project_id: &str,
    connection_internal_id: &str,
    session_id: Option<&str>,
    task_id: Option<&TaskId>,
    observed_paths: &[String],
    now: DateTime<Utc>,
) -> CoreResult<WatchWriteCorrelation> {
    let expected_matches = expected_writes_covering_paths(
        store,
        project_id,
        connection_internal_id,
        session_id,
        task_id,
        observed_paths,
    )?;
    if expected_matches.len() == 1 {
        let expected_write = expected_matches.into_iter().next().expect("length checked");
        return Ok(WatchWriteCorrelation {
            status: "expected_write".to_owned(),
            write_ticket_ids: decode_json_string_array(
                "expected_writes",
                &expected_write.expected_write_id,
                "write_ticket_ids_json",
                &expected_write.write_ticket_ids_json,
            )?,
            expected_write: Some(expected_write),
            deterministically_linked: true,
        });
    }
    if expected_matches.len() > 1 {
        let mut write_ticket_ids = BTreeSet::new();
        for write in &expected_matches {
            write_ticket_ids.extend(decode_json_string_array(
                "expected_writes",
                &write.expected_write_id,
                "write_ticket_ids_json",
                &write.write_ticket_ids_json,
            )?);
        }
        return Ok(WatchWriteCorrelation {
            status: "ambiguous_expected_write".to_owned(),
            expected_write: None,
            write_ticket_ids: write_ticket_ids.into_iter().collect(),
            deterministically_linked: false,
        });
    }

    let active_ticket_matches =
        active_write_tickets_covering_paths(store, task_id, observed_paths, now)?;
    if active_ticket_matches.len() == 1 {
        return Ok(WatchWriteCorrelation {
            status: "write_ticket".to_owned(),
            expected_write: None,
            write_ticket_ids: vec![active_ticket_matches[0].write_ticket_id.clone()],
            deterministically_linked: true,
        });
    }
    if active_ticket_matches.len() > 1 {
        return Ok(WatchWriteCorrelation {
            status: "ambiguous_write_ticket".to_owned(),
            expected_write: None,
            write_ticket_ids: active_ticket_matches
                .iter()
                .map(|record| record.write_ticket_id.clone())
                .collect(),
            deterministically_linked: false,
        });
    }

    Ok(WatchWriteCorrelation {
        status: "unexpected_product_file_change".to_owned(),
        expected_write: None,
        write_ticket_ids: Vec::new(),
        deterministically_linked: false,
    })
}

fn active_write_tickets_covering_paths(
    store: &CoreProjectStore,
    task_id: Option<&TaskId>,
    observed_paths: &[String],
    now: DateTime<Utc>,
) -> CoreResult<Vec<WriteTicketRecord>> {
    let Some(task_id) = task_id else {
        return Ok(Vec::new());
    };
    let project_state = store.project_state()?;
    let mut matches = Vec::new();
    for record in store.active_write_tickets(task_id)? {
        if record.basis_state_version != project_state.state_version
            || write_ticket_is_expired(&record, now)?
        {
            continue;
        }
        let attempt_scope: WriteTicketAttemptScope = decode_required_json(
            "write_tickets",
            record.write_ticket_id.clone(),
            "attempt_scope_json",
            Some(&record.attempt_scope_json),
        )?;
        if attempt_scope.product_file_write_intended
            && paths_are_authorized(observed_paths, &attempt_scope.intended_paths)
        {
            matches.push(record);
        }
    }
    Ok(matches)
}

fn expected_write_covering_paths(
    store: &CoreProjectStore,
    project_id: &str,
    connection_internal_id: &str,
    session_id: Option<&str>,
    task_id: Option<&TaskId>,
    observed_paths: &[String],
) -> CoreResult<Option<ExpectedWriteRecord>> {
    let matches = expected_writes_covering_paths(
        store,
        project_id,
        connection_internal_id,
        session_id,
        task_id,
        observed_paths,
    )?;
    Ok(match matches.as_slice() {
        [write] => Some(write.clone()),
        _ => None,
    })
}

fn expected_writes_covering_paths(
    store: &CoreProjectStore,
    project_id: &str,
    connection_internal_id: &str,
    session_id: Option<&str>,
    task_id: Option<&TaskId>,
    observed_paths: &[String],
) -> CoreResult<Vec<ExpectedWriteRecord>> {
    let writes = list_expected_writes_for_connection(
        store.runtime_home(),
        project_id,
        connection_internal_id,
    )
    .map_err(CorePipelineError::from)?;
    let mut matches = Vec::new();
    for write in writes {
        if let Some(write_session_id) = write.session_id.as_deref() {
            if Some(write_session_id) != session_id {
                continue;
            }
        }
        if task_id.is_some_and(|task_id| write.task_id != task_id.as_str()) {
            continue;
        }
        let authorized_paths = expected_write_paths(&write)?;
        if paths_are_authorized(observed_paths, &authorized_paths) {
            matches.push(write);
        }
    }
    Ok(matches)
}

fn expected_write_paths(write: &ExpectedWriteRecord) -> CoreResult<Vec<String>> {
    if let Some(matched_paths_json) = write.matched_paths_json.as_deref() {
        let matched = decode_json_string_array(
            "expected_writes",
            &write.expected_write_id,
            "matched_paths_json",
            matched_paths_json,
        )?;
        if !matched.is_empty() {
            return Ok(matched);
        }
    }
    decode_json_string_array(
        "expected_writes",
        &write.expected_write_id,
        "expected_paths_json",
        &write.expected_paths_json,
    )
}

fn snapshot_options_from_baseline(
    baseline: &WatchBaselineRecord,
) -> CoreResult<WatchSnapshotOptions> {
    Ok(WatchSnapshotOptions {
        watch_paths: decode_json_string_array(
            "session_watch_baselines",
            &baseline.watch_baseline_id,
            "watched_paths_json",
            &baseline.watched_paths_json,
        )?
        .into_iter()
        .map(PathBuf::from)
        .collect(),
        excluded_paths: decode_json_string_array(
            "session_watch_baselines",
            &baseline.watch_baseline_id,
            "exclusions_json",
            &baseline.exclusions_json,
        )?
        .into_iter()
        .map(PathBuf::from)
        .collect(),
        max_file_size_bytes: volicord_store::session_watch::DEFAULT_MAX_FILE_HASH_BYTES,
        max_file_count: volicord_store::session_watch::DEFAULT_MAX_SCAN_FILE_COUNT,
    })
}

fn snapshot_from_baseline(baseline: &WatchBaselineRecord) -> CoreResult<WatchSnapshot> {
    let scope_kind = match baseline.scope_kind.as_str() {
        "repository" => WatchScopeKind::Repository,
        "path_set" => WatchScopeKind::PathSet,
        _ => {
            return Err(CorePipelineError::Store(
                StoreError::corrupt_owner_state_value(
                    "session_watch_baselines",
                    baseline.watch_baseline_id.clone(),
                    "scope_kind",
                ),
            ))
        }
    };
    let entries = serde_json::from_str::<Vec<WatchSnapshotEntry>>(&baseline.snapshot_entries_json)
        .map_err(|_| {
            CorePipelineError::Store(StoreError::corrupt_owner_state_json(
                "session_watch_baselines",
                baseline.watch_baseline_id.clone(),
                "snapshot_entries_json",
            ))
        })?;
    let scan_summary = volicord_store::session_watch::watch_scan_summary_from_entries(&entries);
    Ok(WatchSnapshot {
        repo_root: PathBuf::from(&baseline.repo_root),
        scope_kind,
        watched_paths: decode_json_string_array(
            "session_watch_baselines",
            &baseline.watch_baseline_id,
            "watched_paths_json",
            &baseline.watched_paths_json,
        )?,
        excluded_paths: decode_json_string_array(
            "session_watch_baselines",
            &baseline.watch_baseline_id,
            "exclusions_json",
            &baseline.exclusions_json,
        )?,
        algorithm: baseline.snapshot_algorithm.clone(),
        digest: baseline.snapshot_digest.clone(),
        entries,
        scan_summary,
    })
}

fn decode_json_string_array(
    table: &'static str,
    record_ref: &str,
    logical_column: &'static str,
    raw: &str,
) -> CoreResult<Vec<String>> {
    serde_json::from_str::<Vec<String>>(raw).map_err(|_| {
        CorePipelineError::Store(StoreError::corrupt_owner_state_json(
            table,
            record_ref.to_owned(),
            logical_column,
        ))
    })
}

fn update_watch_active(
    store: &CoreProjectStore,
    baseline: &WatchBaselineRecord,
    now: &UtcTimestamp,
    detail: &str,
) -> CoreResult<()> {
    update_watch_status(
        store.runtime_home(),
        &baseline.project_id,
        &baseline.watch_baseline_id,
        WatchStatusUpdate {
            status: volicord_store::session_watch::SessionWatchStatus::Active,
            updated_at: now.to_string(),
            metadata_json: watch_status_metadata_json(baseline, detail, None)?,
        },
    )
    .map(|_| ())
    .map_err(CorePipelineError::from)
}

fn update_watch_unavailable(
    store: &CoreProjectStore,
    baseline: &WatchBaselineRecord,
    now: &UtcTimestamp,
    detail: &str,
) -> CoreResult<()> {
    update_watch_status(
        store.runtime_home(),
        &baseline.project_id,
        &baseline.watch_baseline_id,
        WatchStatusUpdate {
            status: volicord_store::session_watch::SessionWatchStatus::Unavailable,
            updated_at: now.to_string(),
            metadata_json: watch_status_metadata_json(
                baseline,
                "snapshot_unavailable",
                Some(detail),
            )?,
        },
    )
    .map(|_| ())
    .map_err(CorePipelineError::from)
}

fn parse_session_watch_status(
    table: &'static str,
    record_ref: &str,
    value: &str,
) -> Result<SessionWatchStatus, PlanError> {
    serde_json::from_value(Value::String(value.to_owned()))
        .map_err(|_| {
            CorePipelineError::Store(StoreError::corrupt_owner_state_value(
                table,
                record_ref.to_owned(),
                "status",
            ))
        })
        .map_err(PlanError::Core)
}

fn apply_session_watch_coverage_metadata(
    summary: &mut GuardHealthSummary,
    baseline: &WatchBaselineRecord,
) -> Result<(), PlanError> {
    let metadata = watch_metadata_object(baseline).map_err(PlanError::Core)?;
    let coverage_start_raw = metadata
        .get("coverage_start_at")
        .and_then(Value::as_str)
        .unwrap_or(&baseline.created_at);
    summary.session_watch_coverage_start_at = Some(parse_owner_storage_value(
        "session_watch_baselines",
        baseline.watch_baseline_id.clone(),
        "coverage_start_at",
        coverage_start_raw,
    )?)
    .into();

    let (coverage_basis, fallback_warning) = match metadata
        .get("coverage_basis")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    {
        Some(raw) => (
            parse_session_watch_coverage_basis(&baseline.watch_baseline_id, raw)?,
            None,
        ),
        None => (
            SessionWatchCoverageBasis::MethodBoundary,
            Some(UNKNOWN_BASIS_PARTIAL_COVERAGE_WARNING.to_owned()),
        ),
    };
    summary.session_watch_coverage_basis = Some(coverage_basis).into();

    let warning = metadata
        .get("partial_coverage_warning")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
        .or(fallback_warning)
        .or_else(|| {
            (coverage_basis != SessionWatchCoverageBasis::McpStart)
                .then(|| METHOD_BOUNDARY_PARTIAL_COVERAGE_WARNING.to_owned())
        });
    summary.session_watch_partial_coverage_warning = warning.into();
    Ok(())
}

fn parse_session_watch_coverage_basis(
    record_ref: &str,
    value: &str,
) -> Result<SessionWatchCoverageBasis, PlanError> {
    serde_json::from_value(Value::String(value.to_owned()))
        .map_err(|_| {
            CorePipelineError::Store(StoreError::corrupt_owner_state_value(
                "session_watch_baselines",
                record_ref.to_owned(),
                "coverage_basis",
            ))
        })
        .map_err(PlanError::Core)
}

fn watch_status_metadata_json(
    baseline: &WatchBaselineRecord,
    status_detail: &str,
    error: Option<&str>,
) -> CoreResult<String> {
    let mut object = watch_metadata_object(baseline)?;
    object.insert("source".to_owned(), json!(WATCH_METADATA_SOURCE));
    object.insert("status_detail".to_owned(), json!(status_detail));
    object.insert(
        "scan_summary".to_owned(),
        serde_json::to_value(session_watch_scan_summary_from_baseline(baseline)?)?,
    );
    if let Some(error) = error {
        object.insert("error".to_owned(), json!(error));
    } else {
        object.remove("error");
    }
    object.insert("detector_role".to_owned(), json!("detective"));
    object.insert("does_not_prevent_writes".to_owned(), json!(true));
    object.insert("does_not_identify_actor".to_owned(), json!(true));
    serde_json::to_string(&Value::Object(object)).map_err(CorePipelineError::from)
}

fn watch_metadata_object(
    baseline: &WatchBaselineRecord,
) -> CoreResult<serde_json::Map<String, Value>> {
    match serde_json::from_str::<Value>(&baseline.metadata_json) {
        Ok(Value::Object(object)) => Ok(object),
        Ok(_) | Err(_) => Err(CorePipelineError::Store(
            StoreError::corrupt_owner_state_json(
                "session_watch_baselines",
                baseline.watch_baseline_id.clone(),
                "metadata_json",
            ),
        )),
    }
}

fn watch_status_detail(metadata_json: &str) -> Option<String> {
    serde_json::from_str::<Value>(metadata_json)
        .ok()
        .and_then(|value| {
            value
                .get("status_detail")
                .or_else(|| value.get("error"))
                .and_then(Value::as_str)
                .filter(|detail| !detail.trim().is_empty())
                .map(str::to_owned)
        })
}

fn session_watch_scan_summary_from_snapshot(snapshot: &WatchSnapshot) -> SessionWatchScanSummary {
    session_watch_scan_summary_from_store(&snapshot.scan_summary)
}

fn session_watch_scan_summary_from_baseline(
    baseline: &WatchBaselineRecord,
) -> CoreResult<SessionWatchScanSummary> {
    let metadata = watch_metadata_object(baseline)?;
    if let Some(raw_summary) = metadata.get("scan_summary") {
        if let Ok(summary) = serde_json::from_value::<SessionWatchScanSummary>(raw_summary.clone())
        {
            return Ok(summary);
        }
    }
    let store_summary = volicord_store::session_watch::watch_scan_summary_from_entries_json(
        &baseline.snapshot_entries_json,
    )
    .map_err(CorePipelineError::from)?;
    Ok(session_watch_scan_summary_from_store(&store_summary))
}

fn session_watch_scan_summary_from_store(
    summary: &volicord_store::session_watch::WatchScanSummary,
) -> SessionWatchScanSummary {
    SessionWatchScanSummary {
        files_scanned: summary.files_scanned,
        files_skipped: summary.files_skipped,
        unreadable_paths_count: summary.unreadable_paths_count,
        degraded_reasons: summary.degraded_reasons.clone(),
        degraded_reason_counts: summary.degraded_reason_counts.clone(),
        skipped_paths_sample: summary.skipped_paths_sample.clone(),
        skipped_paths_truncated: summary.skipped_paths_truncated,
        default_excluded_paths: volicord_store::session_watch::default_watch_excluded_paths(),
        max_file_size_bytes: volicord_store::session_watch::DEFAULT_MAX_FILE_HASH_BYTES,
        max_file_count: volicord_store::session_watch::DEFAULT_MAX_SCAN_FILE_COUNT,
        follows_symlinks: false,
        not_full_filesystem_monitoring: true,
    }
}

fn is_session_watch_change(record: &UnrecordedChangeRecord) -> bool {
    serde_json::from_str::<Value>(&record.detection_json)
        .ok()
        .and_then(|value| {
            value
                .get("source")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .is_some_and(|source| source == WATCH_METADATA_SOURCE)
}

fn stable_watch_id(prefix: &str, parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update((part.len() as u64).to_be_bytes());
        hasher.update(part.as_bytes());
    }
    let digest = lowercase_hex(&hasher.finalize());
    format!("{prefix}_{}", &digest[..24])
}

fn lowercase_hex(bytes: &[u8]) -> String {
    let mut value = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        value.push_str(&format!("{byte:02x}"));
    }
    value
}
