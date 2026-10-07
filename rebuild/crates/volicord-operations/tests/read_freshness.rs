use std::fs;
use volicord_operations::{LocalOperations, RuntimeLayout};
use volicord_repository_intelligence::FreshnessState;

#[test]
fn public_reads_compare_source_changes_without_persisting_new_observations() {
    for (file, before_text, after_text) in [
        (
            "main.py",
            "def original():\n    return 1\n",
            "def changed():\n    return 2\n",
        ),
        ("lib.rs", "pub fn original() {}", "pub fn changed() {}"),
        (
            "main.ts",
            "export function original() {}",
            "export function changed() {}",
        ),
    ] {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("repository");
        fs::create_dir(&root).unwrap();
        fs::write(root.join(file), before_text).unwrap();
        fs::write(root.join("excluded.txt"), "not analyzed").unwrap();
        let operations =
            LocalOperations::new(RuntimeLayout::new(temporary.path().join("runtime")).unwrap());
        let project = operations
            .initialize_project("Freshness fixture", Some(&root))
            .unwrap()
            .project
            .id;
        let analysis = operations
            .analyze(project, vec!["excluded.txt".into()])
            .unwrap()
            .value
            .unwrap();
        let canonical = operations.canonical_basis(project).unwrap();
        let cached = fs::read(&analysis.stored_at).unwrap();
        assert_eq!(
            operations.recall(project).unwrap().snapshots[0]
                .freshness
                .state,
            FreshnessState::Current
        );
        // Excluded content is not read; keep observed size metadata unchanged.
        fs::write(root.join("excluded.txt"), "also ignored").unwrap();
        assert_eq!(
            operations.recall(project).unwrap().snapshots[0]
                .freshness
                .state,
            FreshnessState::Current
        );
        fs::write(root.join(file), after_text).unwrap();
        let brief = operations.recall(project).unwrap();
        assert_eq!(brief.snapshots[0].freshness.state, FreshnessState::Stale);
        assert_eq!(
            brief.snapshots[0].analysis_snapshot,
            analysis.analysis.identity
        );
        assert_ne!(
            brief.snapshots[0].freshness.compared_repository_snapshot,
            Some(analysis.repository.identity)
        );
        let projection = operations.project_projection(project).unwrap();
        assert!(!projection.repository_map.entities.is_empty());
        assert!(projection
            .repository_map
            .entities
            .iter()
            .all(|entity| entity.freshness.state == FreshnessState::Stale));
        assert!(projection
            .repository_map
            .relations
            .iter()
            .all(|relation| relation.freshness.state == FreshnessState::Stale));
        assert!(projection
            .issues
            .iter()
            .any(|issue| issue.reason.contains("repository has changed")));
        fs::remove_dir_all(&root).unwrap();
        let unavailable = operations.project_projection(project).unwrap();
        assert!(unavailable
            .repository_map
            .entities
            .iter()
            .all(|entity| entity.freshness.state == FreshnessState::Unknown));
        assert_eq!(canonical, operations.canonical_basis(project).unwrap());
        assert_eq!(cached, fs::read(&analysis.stored_at).unwrap());
        assert_eq!(
            fs::read_dir(operations.layout().analysis_project_dir(project))
                .unwrap()
                .filter_map(Result::ok)
                .filter(|entry| {
                    entry.path().extension().and_then(|value| value.to_str()) == Some("json")
                })
                .count(),
            1
        );
    }
}

#[test]
fn analysis_status_is_explicit_read_only_and_retains_failed_attempt_basis(
) -> Result<(), Box<dyn std::error::Error>> {
    use volicord_projections::{
        ProjectionDetail, ProjectionReadRequirements, RepositoryAnalysisState, WorkSelector,
    };
    let temporary = tempfile::tempdir()?;
    let root = temporary.path().join("repository");
    fs::create_dir(&root)?;
    fs::write(root.join("main.py"), "def answer():\n    return 42\n")?;
    let operations = LocalOperations::new(RuntimeLayout::new(temporary.path().join("runtime"))?);
    let project = operations
        .initialize_project("Status without Work", Some(&root))?
        .project
        .id;
    let read = || {
        operations
            .project_projection_read_profiled(
                project,
                WorkSelector::Repository,
                ProjectionDetail::default(),
                ProjectionReadRequirements {
                    code: false,
                    inspection: false,
                },
            )
            .map(|(p, cost)| (p.repository_analysis, cost))
    };
    let before = operations.canonical_basis(project)?;
    assert_eq!(read()?.0.state, RepositoryAnalysisState::Absent);
    assert_eq!(before, operations.canonical_basis(project)?);
    let saved = operations
        .analyze(project, Vec::new())?
        .value
        .ok_or("analysis missing")?;
    let canonical = operations.canonical_basis(project)?;
    assert!(canonical.context_items.is_empty());
    assert!(canonical.active_decisions.is_empty());
    assert!(canonical.checkpoint_history.is_empty());
    let (status, cost) = read()?;
    assert_eq!(status.state, RepositoryAnalysisState::Partial);
    assert_eq!(
        status.freshness.as_ref().ok_or("freshness")?.state,
        FreshnessState::Current
    );
    assert_eq!(status.analysis_snapshot, Some(saved.analysis.identity));
    assert_eq!(status.refresh_command, "volicord analyze");
    assert_eq!(cost.analysis_snapshot_decodes, 0);
    assert_eq!(cost.analysis_metadata_decodes, 1);
    let receipt_dir = operations
        .layout()
        .artifacts_dir()
        .join("analysis-attempts")
        .join(project.to_string());
    let receipt_bytes = fs::read_dir(&receipt_dir)?
        .map(|e| fs::read(e?.path()))
        .collect::<Result<Vec<_>, std::io::Error>>()?;
    let _again = read()?;
    assert_eq!(canonical, operations.canonical_basis(project)?);
    assert_eq!(
        receipt_bytes,
        fs::read_dir(&receipt_dir)?
            .map(|e| fs::read(e?.path()))
            .collect::<Result<Vec<_>, std::io::Error>>()?
    );
    fs::write(root.join("main.py"), "def changed():\n    return 43\n")?;
    assert_eq!(read()?.0.state, RepositoryAnalysisState::Stale);
    fs::rename(&root, temporary.path().join("offline"))?;
    assert_eq!(read()?.0.state, RepositoryAnalysisState::FreshnessUnknown);
    assert!(operations.analyze(project, Vec::new()).is_err());
    let failed = read()?.0;
    assert_eq!(failed.state, RepositoryAnalysisState::Failed);
    assert!(failed.retained_prior_result);
    assert_eq!(failed.analysis_snapshot, Some(saved.analysis.identity));
    assert_eq!(
        failed.freshness.as_ref().ok_or("failed freshness")?.state,
        FreshnessState::Unknown
    );
    assert!(failed.latest_attempt.as_ref().is_some_and(|a| a.failed));
    assert_eq!(canonical, operations.canonical_basis(project)?);
    fs::rename(temporary.path().join("offline"), &root)?;
    operations.analyze(project, Vec::new())?;
    let recovered = read()?.0;
    assert_eq!(recovered.state, RepositoryAnalysisState::Partial);
    assert!(!recovered.retained_prior_result);
    assert!(recovered.latest_attempt.as_ref().is_some_and(|a| !a.failed));
    let canonical = operations.canonical_basis(project)?;
    let receipt = fs::read_dir(&receipt_dir)?.next().ok_or("receipt")??.path();
    fs::write(
        receipt,
        b"{\"format_kind\":\"volicord_analysis_attempt\",\"format_version\":99}",
    )?;
    assert!(read()?
        .0
        .latest_attempt_error
        .as_deref()
        .is_some_and(|e| e.contains("unsupported")));
    assert_eq!(canonical, operations.canonical_basis(project)?);
    Ok(())
}

#[test]
fn exact_evidence_reads_reject_changed_foreign_and_forgotten_bases(
) -> Result<(), Box<dyn std::error::Error>> {
    use volicord_context::{
        CanonicalRecordId, ContextItemCorrectionDraft, ContextItemRole, CorrectionKind, SourceId,
    };
    use volicord_operations::EvidenceDetailRequest;
    let root = tempfile::tempdir()?;
    let operations = LocalOperations::new(RuntimeLayout::new(root.path().join("runtime"))?);
    let project = operations
        .initialize_project("Exact read", None)?
        .project
        .id;
    let original = "Original retained goal ".repeat(600);
    let context = operations.record_current_host_user_context(
        project,
        "test".into(),
        "session".into(),
        original.clone(),
        ContextItemRole::Goal,
        original.clone(),
    )?;
    let mut request = EvidenceDetailRequest {
        record_kind: "context_item".into(),
        record_id: context.context_item_id.to_string(),
        revision: context.context_item_revision,
        field: "statement".into(),
        offset: 0,
        expected_fingerprint: None,
        work_item_id: Some(context.context_item_id),
    };
    let first = operations.canonical_evidence_detail(project, &request)?;
    request.offset = first["next_offset"].as_u64().ok_or("next offset")? as usize;
    assert!(operations
        .canonical_evidence_detail(project, &request)
        .is_err());
    request.expected_fingerprint = Some("0".repeat(64));
    assert!(operations
        .canonical_evidence_detail(project, &request)
        .is_err());
    request.expected_fingerprint = first["fingerprint"].as_str().map(str::to_owned);
    operations.correct_context_item(
        project,
        context.context_item_id,
        ContextItemCorrectionDraft {
            expected_revision: context.context_item_revision,
            corrected_statement: format!("{original}."),
            kind: CorrectionKind::Expression,
            user_authorization_source_id: context.source_id,
        },
    )?;
    let stale = operations.canonical_evidence_detail(project, &request)?;
    assert_eq!(stale["reason"], "revision_unavailable");
    assert!(stale.get("chunk").is_none());
    request.revision += 1;
    request.offset = 0;
    request.expected_fingerprint = None;
    let current = operations.canonical_evidence_detail(project, &request)?;
    assert_eq!(current["state"], "partial");
    assert_ne!(current["fingerprint"], first["fingerprint"]);
    let manifest = operations.canonical_inspection_page(project, 0, None)?;
    let foreign = operations.initialize_project("Foreign", None)?.project.id;
    assert_eq!(
        operations.canonical_evidence_detail(foreign, &request)?["state"],
        "unavailable"
    );
    assert!(operations
        .canonical_inspection_page(foreign, 1, manifest["fingerprint"].as_str())
        .is_err());
    operations.forget_record(
        project,
        CanonicalRecordId::ContextItem(context.context_item_id),
        context.source_id,
    )?;
    assert_eq!(
        operations.canonical_evidence_detail(project, &request)?["state"],
        "unavailable"
    );
    assert!(operations
        .canonical_inspection_page(project, 1, manifest["fingerprint"].as_str())
        .is_err());
    request.record_kind = "source".into();
    request.record_id = context.source_id.to_string();
    request.revision = 1;
    request.field = "body".into();
    request.work_item_id = None;
    assert_eq!(
        operations.canonical_evidence_detail(project, &request)?["reason"],
        "source_body_policy_withheld"
    );
    request.record_id = SourceId::from_bytes([255; 16]).to_string();
    assert_eq!(
        operations.canonical_evidence_detail(project, &request)?["reason"],
        "record_unavailable"
    );
    request.field = "/tmp/source".into();
    assert!(operations
        .canonical_evidence_detail(project, &request)
        .is_err());
    Ok(())
}
