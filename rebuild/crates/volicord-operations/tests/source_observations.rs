use serde_json::Value;
use std::{error::Error, fs, time::Duration};
use volicord_context::ContextItemRole;
use volicord_operations::{
    FileObservationRequest, LocalOperations, RuntimeLayout, WorkCommandRequest,
};

fn observe(
    ops: &LocalOperations,
    project: volicord_context::ProjectId,
    work: volicord_context::ContextItemId,
    analysis: volicord_repository_intelligence::AnalysisSnapshotId,
    before: Option<volicord_context::SourceId>,
) -> Result<volicord_operations::SourceObservationReceipt, volicord_operations::Error> {
    ops.observe_work_file(FileObservationRequest {
        byte_range: None,
        project_id: project,
        work_item_id: work,
        analysis_snapshot_id: analysis,
        path: "main.py".into(),
        before_source_id: before,
        retain_seconds: 3600,
    })
}
fn stored(
    ops: &LocalOperations,
    project: volicord_context::ProjectId,
    source: volicord_context::SourceId,
) -> Result<Value, Box<dyn Error>> {
    let record = volicord_privacy::PrivacyStore::open(ops.layout().privacy_store())?
        .inspect_project(project)?
        .managed_derived
        .into_iter()
        .find(|r| r.purpose.ends_with(&source.to_string()))
        .ok_or("missing owned body")?;
    Ok(serde_json::from_str(
        record.content.as_deref().ok_or("missing content")?,
    )?)
}

#[test]
fn actual_dirty_work_records_changed_and_no_change_sources_before_any_checkpoint(
) -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path().join("repo");
    fs::create_dir(&root)?;
    // Authored fixture expectations are independent of generated Product data.
    fs::write(root.join("main.py"), "def value():\n    return 1\n")?;
    for args in [
        vec!["init", "-q"],
        vec!["add", "main.py"],
        vec![
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "-qm",
            "baseline",
        ],
    ] {
        assert!(std::process::Command::new("git")
            .current_dir(&root)
            .args(args)
            .status()?
            .success());
    }
    // Dirty initial bytes differ from HEAD. They must be observed, never reconstructed from HEAD.
    fs::write(root.join("main.py"), "def value():\n    return 4\n")?;
    let ops = LocalOperations::new(RuntimeLayout::new(temp.path().join("runtime"))?);
    let project = ops
        .initialize_project("ordinary investigation", Some(&root))?
        .project
        .id;
    let turn="Investigate value(). If a comparison is needed, inspect the helper and test before changing code.";
    let work = ops
        .record_current_host_user_context(
            project,
            "codex".into(),
            "fixture".into(),
            turn.into(),
            ContextItemRole::Goal,
            turn.into(),
        )?
        .context_item_id;
    let before = ops
        .analyze(project, vec![])?
        .value
        .ok_or("missing analysis")?;
    let first = observe(&ops, project, work, before.analysis.identity, None)?;
    let body = stored(&ops, project, first.source_id)?;
    assert_eq!(body["detail"]["body"], "def value():\n    return 4\n");
    assert_eq!(body["detail"]["comparison"], "missing_before_state");
    assert_eq!(body["work_item_id"], serde_json::to_value(work)?);
    assert_eq!(
        body["repository_snapshot_id"],
        before.repository.identity.to_string()
    );
    assert_eq!(
        body["repository_source_id"],
        serde_json::to_value(before.analysis.repository_source.identity())?
    );
    assert_eq!(body["detail"]["repository_worktree"]["kind"], "git");
    let equal = observe(
        &ops,
        project,
        work,
        before.analysis.identity,
        Some(first.source_id),
    )?;
    assert_eq!(
        stored(&ops, project, equal.source_id)?["detail"]["comparison"],
        "no_change"
    );
    fs::write(root.join("main.py"), "def value():\n    return 5\n")?;
    assert!(observe(
        &ops,
        project,
        work,
        before.analysis.identity,
        Some(first.source_id)
    )
    .is_err());
    let after = ops
        .analyze(project, vec![])?
        .value
        .ok_or("missing analysis")?;
    let changed = observe(
        &ops,
        project,
        work,
        after.analysis.identity,
        Some(first.source_id),
    )?;
    let data = stored(&ops, project, changed.source_id)?;
    assert_eq!(data["detail"]["comparison"], "changed");
    assert_eq!(
        data["detail"]["before_source_id"],
        first.source_id.to_string()
    );
    assert_eq!(data["detail"]["body"], "def value():\n    return 5\n");
    assert!(first.observed_at_unix_micros < changed.observed_at_unix_micros);
    let result = ops.run_work_command(WorkCommandRequest {
        project_id: project,
        work_item_id: work,
        analysis_snapshot_id: after.analysis.identity,
        command_label: "verify value".into(),
        program: "python3".into(),
        arguments: vec![
            "-c".into(),
            "from main import value; assert value() == 5; print('verified 5')".into(),
        ],
        timeout: Duration::from_secs(5),
        retain_seconds: 3600,
    })?;
    let command = stored(&ops, project, result.source_id)?;
    assert_eq!(command["detail"]["exit_code"], 0);
    assert_eq!(command["detail"]["termination"], "exited");
    assert_eq!(command["detail"]["stdout"]["body"], "verified 5\n");
    assert_eq!(command["detail"]["stderr"]["body"], "");
    assert!(!command.to_string().contains("from main import"));
    let operation = command["detail"]["operation_id"]
        .as_str()
        .ok_or("missing operation")?;
    assert!(!ops.layout().artifacts_dir().join(operation).exists());
    println!("Product Work evidence: project={project} work={work} before_source={} equal_source={} after_source={} command_source={} before_repository={} after_repository={} command_exit=0", first.source_id, equal.source_id, changed.source_id, result.source_id, before.repository.identity, after.repository.identity);
    let canonical = ops.canonical_basis(project)?;
    assert!(canonical.latest_checkpoint.is_none());
    assert!(canonical.active_decisions.is_empty());
    assert_eq!(
        canonical
            .context_items
            .iter()
            .filter(|i| i.role == ContextItemRole::Learning)
            .count(),
        0
    );
    // Same text/path is a distinct Work; no implicit borrowing of first's body.
    let other = ops
        .record_current_host_user_context(
            project,
            "codex".into(),
            "fixture".into(),
            turn.into(),
            ContextItemRole::Goal,
            turn.into(),
        )?
        .context_item_id;
    assert_ne!(other, work);
    assert!(observe(
        &ops,
        project,
        other,
        after.analysis.identity,
        Some(first.source_id)
    )
    .is_err());
    let foreign = ops.initialize_project("foreign", None)?.project.id;
    assert!(observe(&ops, foreign, work, after.analysis.identity, None).is_err());
    Ok(())
}

#[test]
fn source_capture_refuses_exclusions_scope_escape_symlinks_missing_and_oversize(
) -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path().join("repo");
    fs::create_dir(&root)?;
    fs::write(root.join("main.py"), "print('ok')\n")?;
    fs::write(root.join("large.py"), "x".repeat(16385))?;
    fs::write(root.join("private.py"), "private")?;
    let ops = LocalOperations::new(RuntimeLayout::new(temp.path().join("runtime"))?);
    let p = ops.initialize_project("scope", Some(&root))?.project.id;
    let w = ops
        .record_current_host_user_context(
            p,
            "codex".into(),
            "fixture".into(),
            "Inspect main".into(),
            ContextItemRole::Goal,
            "Inspect main".into(),
        )?
        .context_item_id;
    let a = ops
        .analyze(p, vec!["private.py".into()])?
        .value
        .ok_or("missing analysis")?;
    for path in [
        "../private.py",
        "/etc/passwd",
        "private.py",
        "large.py",
        "absent.py",
    ] {
        assert!(
            ops.observe_work_file(FileObservationRequest {
                byte_range: None,
                project_id: p,
                work_item_id: w,
                analysis_snapshot_id: a.analysis.identity,
                path: path.into(),
                before_source_id: None,
                retain_seconds: 3600
            })
            .is_err(),
            "{path}"
        );
    }
    fs::remove_file(root.join("main.py"))?;
    #[cfg(unix)]
    {
        fs::write(temp.path().join("outside.py"), "print('ok')\n")?;
        std::os::unix::fs::symlink(temp.path().join("outside.py"), root.join("main.py"))?;
        assert!(observe(&ops, p, w, a.analysis.identity, None).is_err());
        fs::remove_file(root.join("main.py"))?;
    }
    assert!(observe(&ops, p, w, a.analysis.identity, None).is_err());
    assert!(ops.privacy_status(p)?.managed_derived.is_empty());
    Ok(())
}

#[test]
fn expired_or_deleted_before_state_cannot_be_reused() -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path().join("repo");
    fs::create_dir(&root)?;
    fs::write(root.join("main.py"), "value = 1\n")?;
    let ops = LocalOperations::new(RuntimeLayout::new(temp.path().join("runtime"))?);
    let p = ops.initialize_project("lifecycle", Some(&root))?.project.id;
    let w = ops
        .record_current_host_user_context(
            p,
            "codex".into(),
            "fixture".into(),
            "Inspect value".into(),
            ContextItemRole::Goal,
            "Inspect value".into(),
        )?
        .context_item_id;
    let a = ops.analyze(p, vec![])?.value.ok_or("missing analysis")?;
    let first = ops.observe_work_file(FileObservationRequest {
        byte_range: None,
        project_id: p,
        work_item_id: w,
        analysis_snapshot_id: a.analysis.identity,
        path: "main.py".into(),
        before_source_id: None,
        retain_seconds: 1,
    })?;
    std::thread::sleep(Duration::from_millis(1100));
    assert!(observe(&ops, p, w, a.analysis.identity, Some(first.source_id)).is_err());
    let retained = observe(&ops, p, w, a.analysis.identity, None)?;
    let record = ops
        .privacy_status(p)?
        .managed_derived
        .into_iter()
        .find(|r| r.purpose.ends_with(&retained.source_id.to_string()))
        .ok_or("missing body")?;
    let mut store = volicord_privacy::PrivacyStore::open(ops.layout().privacy_store())?;
    store.delete_managed_ids(p, &[record.id], "Explicit test body deletion")?;
    assert!(store.get_derived(p, record.id)?.content.is_none());
    assert!(observe(&ops, p, w, a.analysis.identity, Some(retained.source_id)).is_err());
    assert!(ops
        .canonical_basis(p)?
        .sources
        .iter()
        .any(|s| s.source.id == retained.source_id));
    // The format owner refuses non-current metadata without rewriting canonical state.
    let connection = rusqlite::Connection::open(ops.layout().privacy_store())?;
    connection.execute("UPDATE privacy_metadata SET schema_version=1", [])?;
    assert!(volicord_privacy::PrivacyStore::open(ops.layout().privacy_store()).is_err());
    Ok(())
}

#[test]
fn source_body_capability_is_language_independent_and_parent_links_are_refused(
) -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path().join("repo");
    fs::create_dir(&root)?;
    fs::create_dir(root.join("src"))?;
    let fixtures = [
        ("main.py", "def value():\n    return 2\n"),
        ("main.ts", "export function value() { return 2; }\n"),
        ("main.rs", "pub fn value() -> i32 { 2 }\n"),
        ("main.cob", "IDENTIFICATION DIVISION.\nPROGRAM-ID. HELLO.\n"),
    ];
    for (path, body) in fixtures {
        fs::write(root.join("src").join(path), body)?;
    }
    let ops = LocalOperations::new(RuntimeLayout::new(temp.path().join("runtime"))?);
    let p = ops.initialize_project("polyglot", Some(&root))?.project.id;
    let w = ops
        .record_current_host_user_context(
            p,
            "codex".into(),
            "fixture".into(),
            "Inspect source".into(),
            ContextItemRole::Goal,
            "Inspect source".into(),
        )?
        .context_item_id;
    let a = ops.analyze(p, vec![])?.value.ok_or("analysis")?;
    for (path, body) in fixtures {
        let source = ops.observe_work_file(FileObservationRequest {
            byte_range: None,
            project_id: p,
            work_item_id: w,
            analysis_snapshot_id: a.analysis.identity,
            path: format!("src/{path}"),
            before_source_id: None,
            retain_seconds: 3600,
        })?;
        assert_eq!(stored(&ops, p, source.source_id)?["detail"]["body"], body);
    }
    #[cfg(unix)]
    {
        fs::rename(root.join("src"), temp.path().join("outside"))?;
        std::os::unix::fs::symlink(temp.path().join("outside"), root.join("src"))?;
        assert!(ops
            .observe_work_file(FileObservationRequest {
                byte_range: None,
                project_id: p,
                work_item_id: w,
                analysis_snapshot_id: a.analysis.identity,
                path: "src/main.py".into(),
                before_source_id: None,
                retain_seconds: 3600
            })
            .is_err());
    }
    Ok(())
}

#[test]
fn excerpt_authority_rejects_invalid_scope_boundaries_bytes_and_pairs() -> Result<(), Box<dyn Error>>
{
    use volicord_operations::FileByteRange;
    let temp = tempfile::tempdir()?;
    let root = temp.path().join("repo");
    fs::create_dir_all(root.join("src"))?;
    let text = format!(
        "{}한\nvalue = 7\n{}",
        "# context\n".repeat(2000),
        "# tail\n".repeat(100)
    );
    let start = 20_000;
    let end = start + "한\nvalue = 7\n".len();
    fs::write(root.join("src/main.custom"), &text)?;
    fs::write(root.join("excluded.py"), &text)?;
    fs::write(root.join("invalid.py"), b"abc\xffdef")?;
    let ops = LocalOperations::new(RuntimeLayout::new(temp.path().join("runtime"))?);
    let p = ops
        .initialize_project("excerpt controls", Some(&root))?
        .project
        .id;
    let w = ops
        .record_current_host_user_context(
            p,
            "test".into(),
            "fixture".into(),
            "Inspect value".into(),
            ContextItemRole::Goal,
            "Inspect value".into(),
        )?
        .context_item_id;
    let a = ops
        .analyze(p, vec!["excluded.py".into()])?
        .value
        .ok_or("analysis")?;
    let mut request = FileObservationRequest {
        project_id: p,
        work_item_id: w,
        analysis_snapshot_id: a.analysis.identity,
        path: "src/main.custom".into(),
        byte_range: Some(FileByteRange {
            start_byte: start,
            end_byte: end,
        }),
        before_source_id: None,
        retain_seconds: 3600,
    };
    let baseline = ops.canonical_basis(p)?;
    for (from, to) in [
        (end, start),
        (start, start),
        (0, 16_385),
        (start + 1, end),
        (start, start + 2),
        (end, text.len() + 1),
        (0, usize::MAX),
    ] {
        let mut bad = request.clone();
        bad.byte_range = Some(FileByteRange {
            start_byte: from,
            end_byte: to,
        });
        assert!(ops.observe_work_file(bad).is_err(), "{from}..{to}");
    }
    for path in [
        "../excluded.py",
        "/etc/passwd",
        "excluded.py",
        "absent.py",
        "invalid.py",
    ] {
        let mut bad = request.clone();
        bad.path = path.into();
        bad.byte_range = Some(FileByteRange {
            start_byte: 0,
            end_byte: 3,
        });
        assert!(ops.observe_work_file(bad).is_err(), "{path}");
    }
    for retain in [0, 604801] {
        let mut bad = request.clone();
        bad.retain_seconds = retain;
        assert!(ops.observe_work_file(bad).is_err());
    }
    assert_eq!(baseline, ops.canonical_basis(p)?);
    assert!(ops.privacy_status(p)?.managed_derived.is_empty());
    let first = ops.observe_work_file(request.clone())?;
    let body = ops.read_explanation_source(p, w, first.source_id)?;
    assert_eq!(body["observation"]["detail"]["body"], "한\nvalue = 7\n");
    assert_eq!(body["observation"]["detail"]["region"]["start_line"], 2001);
    assert_eq!(body["observation"]["detail"]["region"]["end_line"], 2002);
    request.before_source_id = Some(first.source_id);
    let equal = ops.observe_work_file(request.clone())?;
    assert_eq!(
        ops.read_explanation_source(p, w, equal.source_id)?["observation"]["detail"]["comparison"],
        "no_change"
    );
    let mut different = request.clone();
    different.byte_range = Some(FileByteRange {
        start_byte: start + 4,
        end_byte: end,
    });
    assert!(ops.observe_work_file(different).is_err());
    let other = ops
        .record_current_host_user_context(
            p,
            "test".into(),
            "other".into(),
            "Other Work".into(),
            ContextItemRole::Goal,
            "Other Work".into(),
        )?
        .context_item_id;
    let mut foreign = request.clone();
    foreign.work_item_id = other;
    assert!(ops.observe_work_file(foreign).is_err());
    assert!(ops
        .read_explanation_source(p, other, first.source_id)
        .is_err());
    let foreign_project = ops.initialize_project("foreign", None)?.project.id;
    let mut foreign = request.clone();
    foreign.project_id = foreign_project;
    assert!(ops.observe_work_file(foreign).is_err());
    fs::write(
        root.join("src/main.custom"),
        text.replacen("value = 7", "value = 8", 1),
    )?;
    assert!(ops.observe_work_file(request.clone()).is_err());
    let next = ops
        .analyze(p, vec!["excluded.py".into()])?
        .value
        .ok_or("analysis")?;
    request.analysis_snapshot_id = next.analysis.identity;
    let changed = ops.observe_work_file(request.clone())?;
    assert_eq!(
        ops.read_explanation_source(p, w, changed.source_id)?["observation"]["detail"]
            ["comparison"],
        "changed"
    );
    #[cfg(unix)]
    {
        fs::rename(root.join("src"), temp.path().join("outside"))?;
        std::os::unix::fs::symlink(temp.path().join("outside"), root.join("src"))?;
        assert!(ops.observe_work_file(request.clone()).is_err());
        fs::remove_file(root.join("src"))?;
        fs::create_dir(root.join("src"))?;
        std::os::unix::fs::symlink(
            temp.path().join("outside/main.custom"),
            root.join("src/main.custom"),
        )?;
        assert!(ops.observe_work_file(request.clone()).is_err());
    }
    ops.delete_work_source_observation(p, w, first.source_id)?;
    assert!(ops.read_explanation_source(p, w, first.source_id).is_err());
    assert!(ops
        .read_explanation_source(p, w, changed.source_id)
        .is_err());
    assert!(ops
        .canonical_basis(p)?
        .context_items
        .iter()
        .any(|i| i.id == w));
    Ok(())
}

#[test]
fn earlier_body_expiry_preserves_permitted_after_bytes_but_not_new_pair_authority(
) -> Result<(), Box<dyn Error>> {
    use volicord_context::{FixedClock, SystemIdGenerator, TimestampMicros};
    use volicord_privacy::PrivacyStore;
    let temp = tempfile::tempdir()?;
    let root = temp.path().join("repository");
    fs::create_dir(&root)?;
    fs::write(root.join("main.py"), "value = 1\n")?;
    let ops = LocalOperations::new(RuntimeLayout::new(temp.path().join("runtime"))?);
    let p = ops
        .initialize_project("Pair lifetime", Some(&root))?
        .project
        .id;
    let work = ops
        .record_current_host_user_context(
            p,
            "control".into(),
            "pair".into(),
            "Inspect pair lifetime".into(),
            ContextItemRole::Goal,
            "Inspect pair lifetime".into(),
        )?
        .context_item_id;
    let a = ops
        .analyze(p, vec![])?
        .value
        .ok_or("analysis")?
        .analysis
        .identity;
    let first = ops.observe_work_file(FileObservationRequest {
        project_id: p,
        work_item_id: work,
        analysis_snapshot_id: a,
        path: "main.py".into(),
        byte_range: None,
        before_source_id: None,
        retain_seconds: 60,
    })?;
    fs::write(root.join("main.py"), "value = 2\n")?;
    let a = ops
        .analyze(p, vec![])?
        .value
        .ok_or("analysis")?
        .analysis
        .identity;
    let after = observe(&ops, p, work, a, Some(first.source_id))?;
    let frozen = ops.read_explanation_source(p, work, after.source_id)?;
    let deadline = ops
        .privacy_status(p)?
        .managed_derived
        .iter()
        .filter_map(|r| r.retained_until)
        .min()
        .ok_or("expiry")?;
    let mut privacy = PrivacyStore::open_with(
        ops.layout().privacy_store(),
        SystemIdGenerator,
        FixedClock::new(TimestampMicros::from_unix_micros(
            deadline.as_unix_micros() + 1,
        )),
    )?;
    assert_eq!(privacy.cleanup_expired(p)?.len(), 1);
    drop(privacy);
    let restarted = LocalOperations::new(ops.layout().clone());
    assert!(restarted
        .read_explanation_source(p, work, first.source_id)
        .is_err());
    assert_eq!(
        restarted.read_explanation_source(p, work, after.source_id)?,
        frozen
    );
    assert_eq!(frozen["observation"]["detail"]["body"], "value = 2\n");
    assert!(observe(&restarted, p, work, a, Some(first.source_id)).is_err());
    restarted.delete_work_source_observation(p, work, first.source_id)?;
    assert!(restarted
        .read_explanation_source(p, work, after.source_id)
        .is_err());
    Ok(())
}
