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
