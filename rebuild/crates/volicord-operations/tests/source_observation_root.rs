//! A registered repository boundary cannot silently become another root.
use std::{error::Error, fs};
use volicord_context::ContextItemRole;
use volicord_local_platform::RepositoryRoot;
use volicord_operations::{FileObservationRequest, LocalOperations, RuntimeLayout};

#[test]
#[cfg(target_os = "linux")]
fn redirected_root_and_changed_ancestor_are_denied_even_with_identical_bytes(
) -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let parent = temp.path().join("parent");
    let root = parent.join("repo");
    fs::create_dir_all(&root)?;
    fs::write(root.join("main.py"), "value = 1\n")?;
    let ops = LocalOperations::new(RuntimeLayout::new(temp.path().join("runtime"))?);
    let p = ops.initialize_project("scope", Some(&root))?.project.id;
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
    let before = ops.canonical_basis(p)?;
    let selected = RepositoryRoot::open(&root)?;
    let moved = temp.path().join("moved");
    fs::rename(&root, &moved)?;
    std::os::unix::fs::symlink(&moved, &root)?;
    let denied = ops.observe_work_file(FileObservationRequest {
        project_id: p,
        work_item_id: w,
        analysis_snapshot_id: a.analysis.identity,
        path: "main.py".into(),
        before_source_id: None,
        retain_seconds: 3600,
    });
    assert!(denied.is_err());
    assert!(selected
        .read_bounded_regular_file(std::path::Path::new("main.py"), 16_384)
        .is_err());
    assert_eq!(before, ops.canonical_basis(p)?);
    assert!(ops.privacy_status(p)?.managed_derived.is_empty());
    fs::remove_file(&root)?;
    fs::rename(&moved, &root)?;
    // The root object was already opened before its parent was redirected.
    let changed_parent = temp.path().join("changed-parent");
    fs::rename(&parent, &changed_parent)?;
    std::os::unix::fs::symlink(&changed_parent, &parent)?;
    assert!(selected
        .read_bounded_regular_file(std::path::Path::new("main.py"), 16_384)
        .is_err());
    Ok(())
}
