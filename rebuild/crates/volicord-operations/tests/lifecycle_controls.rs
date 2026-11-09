use serde_json::{json, Value};
use std::process::Command;
use volicord_context::{Principal, PrincipalKind, ProjectId, TimestampMicros};
use volicord_inquiry::*;
use volicord_operations::{LocalOperations, RuntimeLayout};

fn observation(project_id: ProjectId) -> CandidateDraft {
    CandidateDraft {
        project_id,
        kind: CandidateKind::Observation,
        collection_mode: CandidateCollectionMode::Automatic,
        origin: CandidateOrigin {
            actor: Principal {
                kind: PrincipalKind::Agent,
                identity: "fixture".into(),
            },
            subsystem: "inventory".into(),
            session: Some("session-a".into()),
            provenance_summary: "bounded fixture observation".into(),
        },
        collection_scope: CandidateCollectionScope {
            project_id,
            session: Some("session-a".into()),
            source_operation: Some("inventory".into()),
            candidate_kind: CandidateKind::Observation,
        },
        observation_basis: CandidateObservationBasis::default(),
        observed_at: TimestampMicros::from_unix_micros(1),
        retention: CandidateRetention {
            retained_until: None,
            basis: "explicit test retention".into(),
        },
        content: CandidateContent {
            bounded_summary: "retained observation".into(),
            question: None,
            engineering_choice_discovery: None,
            materiality_review: None,
            learning_deliberation: None,
        },
    }
}
fn cli(
    ops: &LocalOperations,
    project: ProjectId,
    args: &[&str],
) -> Result<Value, Box<dyn std::error::Error>> {
    let output = Command::new(env!("CARGO_BIN_EXE_volicord"))
        .args([
            "--runtime",
            ops.layout().root().to_str().ok_or("path")?,
            "--project",
            &project.to_string(),
            "--json",
        ])
        .args(args)
        .output()?;
    if !output.status.success() {
        return Err(format!("{}", String::from_utf8_lossy(&output.stderr)).into());
    }
    Ok(serde_json::from_slice(&output.stdout)?)
}
#[test]
fn cli_collection_scopes_survive_restart_and_preserve_history(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let ops = LocalOperations::new(RuntimeLayout::new(root.path().join("runtime"))?);
    let project = ops.initialize_project("Collection", None)?.project.id;
    let other = ops.initialize_project("Other", None)?.project.id;
    let draft = observation(project);
    let before = match ops.submit_candidate(draft.clone())? {
        SubmissionOutcome::Stored(r) => r,
        _ => return Err("disabled".into()),
    };
    for filters in [
        vec![],
        vec!["--session", "session-a"],
        vec!["--source-operation", "inventory"],
        vec!["--kind", "observation"],
        vec![
            "--session",
            "session-a",
            "--source-operation",
            "inventory",
            "--kind",
            "observation",
        ],
    ] {
        let mut args = vec![
            "privacy",
            "collection",
            "disable",
            "--basis",
            "current user collection intent",
        ];
        args.extend(&filters);
        let changed = cli(&ops, project, &args)?;
        assert_eq!(changed["policies"][0]["opted_out"], true);
        assert!(matches!(
            ops.submit_candidate(draft.clone())?,
            SubmissionOutcome::CollectionDisabled { .. }
        ));
        assert!(matches!(
            ops.submit_candidate(observation(other))?,
            SubmissionOutcome::Stored(_)
        ));
        let mut explicit = draft.clone();
        explicit.collection_mode = CandidateCollectionMode::ExplicitUserDirected;
        let explicit = match ops.submit_candidate(explicit)? {
            SubmissionOutcome::Stored(r) => r,
            _ => return Err("explicit disabled".into()),
        };
        assert!(explicit
            .opt_out_state_at_collection
            .iter()
            .any(|p| p.opted_out));
        let mut args = vec![
            "privacy",
            "collection",
            "enable",
            "--basis",
            "current user resumed collection",
        ];
        args.extend(&filters);
        cli(&ops, project, &args)?;
        assert!(matches!(
            ops.submit_candidate(draft.clone())?,
            SubmissionOutcome::Stored(_)
        ));
    }
    cli(
        &ops,
        project,
        &[
            "privacy",
            "collection",
            "disable",
            "--session",
            "another-session",
            "--basis",
            "other session only",
        ],
    )?;
    assert!(matches!(
        ops.submit_candidate(draft.clone())?,
        SubmissionOutcome::Stored(_)
    ));
    let policy = cli(&ops, project, &["privacy", "collection", "inspect"])?;
    assert_eq!(policy["policies"].as_array().ok_or("policies")?.len(), 6);
    let reopened = LocalOperations::new(ops.layout().clone());
    assert_eq!(reopened.collection_policies(project)?.len(), 6);
    let original = reopened
        .candidate_basis(project)?
        .candidates
        .into_iter()
        .find(|r| r.id == before.id)
        .ok_or("original")?;
    assert_eq!(original, *before);
    let before_policies = reopened.collection_policies(project)?;
    let bad = CollectionOptOutScope {
        project_id: project,
        session: Some("".into()),
        source_operation: None,
        candidate_kind: None,
    };
    assert!(reopened
        .set_collection_opt_out(bad, true, "invalid scope")
        .is_err());
    assert!(reopened
        .set_collection_opt_out(
            CollectionOptOutScope {
                project_id: ProjectId::from_bytes([255; 16]),
                session: None,
                source_operation: None,
                candidate_kind: None
            },
            true,
            "missing Project"
        )
        .is_err());
    assert_eq!(before_policies, reopened.collection_policies(project)?);
    assert_eq!(
        cli(&ops, other, &["privacy", "collection", "inspect"])?["policies"],
        json!([])
    );
    Ok(())
}

fn managed(
    project_id: ProjectId,
    expiry: Option<i64>,
    content: &str,
) -> volicord_privacy::ManagedDerivedDraft {
    volicord_privacy::ManagedDerivedDraft {
        project_id,
        kind: volicord_privacy::ManagedDerivedKind::CachedSummary,
        provider: None,
        model: None,
        purpose: "local cleanup fixture".into(),
        analysis_snapshot: None,
        included_sources: vec![],
        canonical_links: vec![],
        content: content.into(),
        uncertainty: None,
        retained_until: expiry.map(TimestampMicros::from_unix_micros),
        retention_basis: "test retention".into(),
    }
}
#[test]
fn cleanup_cli_batches_failures_interruptions_and_restart_are_observable(
) -> Result<(), Box<dyn std::error::Error>> {
    use rusqlite::Connection;
    use std::process::Stdio;
    use volicord_privacy::{ManagedDerivedState, PrivacyStore};
    let root = tempfile::tempdir()?;
    let ops = LocalOperations::new(RuntimeLayout::new(root.path().join("runtime"))?);
    let project = ops.initialize_project("Expiry", None)?.project.id;
    let other = ops.initialize_project("Other", None)?.project.id;
    let mut ids = vec![];
    for _ in 0..3 {
        let mut d = observation(project);
        d.retention.retained_until = Some(TimestampMicros::from_unix_micros(2));
        match ops.submit_candidate(d)? {
            SubmissionOutcome::Stored(r) => ids.push(r.id),
            _ => return Err("disabled".into()),
        }
    }
    let live = match ops.submit_candidate(observation(project))? {
        SubmissionOutcome::Stored(r) => r,
        _ => return Err("disabled".into()),
    };
    let mut d = observation(other);
    d.retention.retained_until = Some(TimestampMicros::from_unix_micros(2));
    ops.submit_candidate(d)?;
    let mut privacy = PrivacyStore::open(ops.layout().privacy_store())?;
    let expired =
        privacy.record_managed_derived(managed(project, Some(2), "expired managed fixture"))?;
    let retained =
        privacy.record_managed_derived(managed(project, None, "retained independent article"))?;
    let foreign = privacy.record_managed_derived(managed(other, Some(2), "foreign"))?;
    drop(privacy);
    // Reads withhold before cleanup and do not mutate the persisted records.
    let inspected = cli(&ops, project, &["advanced", "candidates"])?;
    assert!(!inspected.to_string().contains("expired managed fixture"));
    assert_eq!(
        ops.candidate_basis(project)?
            .candidates
            .iter()
            .filter(|r| r.content.is_some())
            .count(),
        4
    );
    assert!(ops.cleanup_expired(project, 0).is_err());
    assert!(ops.cleanup_expired(project, 257).is_err());
    ids.sort();
    let conn = Connection::open(ops.layout().candidate_store())?;
    conn.execute_batch(&format!("CREATE TRIGGER fail_expiry BEFORE UPDATE ON candidates WHEN lower(hex(OLD.id))='{}' BEGIN SELECT RAISE(FAIL,'test injected storage failure'); END;",ids[1]))?;
    let result = cli(&ops, project, &["privacy", "cleanup", "--limit", "3"])?;
    assert_eq!(result["result"]["candidates"]["state"], "incomplete");
    assert!(result["result"]["candidates"]["removed_ids"].is_null());
    assert_eq!(result["result"]["candidates"]["remaining_eligible"], 2);
    assert_eq!(result["result"]["managed"]["state"], "completed");
    assert_eq!(
        result["result"]["local_guarantee"],
        "logical_content_removal"
    );
    conn.execute_batch("DROP TRIGGER fail_expiry; BEGIN IMMEDIATE;")?;
    let mut child = Command::new(env!("CARGO_BIN_EXE_volicord"))
        .args([
            "--runtime",
            ops.layout().root().to_str().ok_or("path")?,
            "--project",
            &project.to_string(),
            "--json",
            "privacy",
            "cleanup",
            "--limit",
            "1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    std::thread::sleep(std::time::Duration::from_millis(150));
    assert!(
        child.try_wait()?.is_none(),
        "cleanup must wait on the held write lock"
    );
    child.kill()?;
    let killed = child.wait_with_output()?;
    assert!(!killed.status.success());
    assert!(killed.stdout.is_empty());
    conn.execute_batch("ROLLBACK;")?;
    drop(conn);
    let batch = cli(&ops, project, &["privacy", "cleanup", "--limit", "1"])?;
    assert_eq!(
        batch["result"]["candidates"]["removed_ids"]
            .as_array()
            .ok_or("removed")?
            .len(),
        1
    );
    assert_eq!(batch["result"]["candidates"]["remaining_eligible"], 1);
    cli(&ops, project, &["privacy", "cleanup", "--limit", "1"])?;
    let repeated = cli(&ops, project, &["privacy", "cleanup", "--limit", "1"])?;
    assert_eq!(repeated["result"]["candidates"]["removed_ids"], json!([]));
    assert_eq!(repeated["result"]["managed"]["removed_ids"], json!([]));
    assert_eq!(repeated["result"]["candidates"]["remaining_eligible"], 0);
    let restarted = LocalOperations::new(ops.layout().clone());
    assert_eq!(
        restarted
            .candidate_basis(project)?
            .candidates
            .into_iter()
            .find(|c| c.id == live.id)
            .ok_or("live")?,
        *live
    );
    assert!(restarted.candidate_basis(other)?.candidates[0]
        .content
        .is_some());
    let privacy = PrivacyStore::open(ops.layout().privacy_store())?;
    assert_eq!(
        privacy.get_derived(project, expired.id)?.state,
        ManagedDerivedState::Deleted
    );
    assert_eq!(privacy.get_derived(project, retained.id)?, retained);
    assert_eq!(privacy.get_derived(other, foreign.id)?, foreign);
    let status = cli(&ops, project, &["privacy", "status"])?;
    assert!(status["managed_records"]
        .as_array()
        .ok_or("metadata")?
        .iter()
        .any(|r| r["id"] == expired.id.to_string() && r["expired_by_cleanup"] == true));
    assert!(!status.to_string().contains("retained independent article"));
    Ok(())
}
