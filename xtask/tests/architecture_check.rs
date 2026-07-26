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

#[test]
fn current_workspace_package_inputs_come_from_cargo_metadata() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask should be a workspace-root child");

    let packages =
        xtask::derive_workspace_package_inputs(root).expect("Cargo metadata should resolve");
    let xtask = packages
        .iter()
        .find(|package| package.name() == "xtask")
        .expect("xtask is a current workspace member");

    assert_eq!(xtask.manifest_path(), "xtask/Cargo.toml");
    assert!(xtask.source_roots().iter().any(|root| root == "xtask/src"));
    assert!(xtask
        .target_source_paths()
        .iter()
        .any(|path| path == "xtask/src/lib.rs"));
    assert!(packages
        .iter()
        .any(|package| package.name() == "volicord-command-model"));
}
