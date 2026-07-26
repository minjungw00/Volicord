use std::path::Path;

#[test]
fn current_workspace_dependency_graph_matches_architecture_owner() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask should be a workspace-root child");

    let report = xtask::run_architecture_check(root).expect("architecture check should run");

    assert!(
        report.is_ok(),
        "current workspace architecture issues: {:#?}",
        report.issues()
    );
}
