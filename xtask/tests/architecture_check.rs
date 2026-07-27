use std::{fs, path::Path};

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

#[test]
fn user_action_service_group_excludes_core_and_adapters() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask should be a workspace-root child");
    let manifest = fs::read_to_string(root.join("Cargo.toml"))
        .expect("workspace manifest should be readable")
        .parse::<toml_edit::DocumentMut>()
        .expect("workspace manifest should be valid TOML");

    let service_package = &manifest["workspace"]["metadata"]["architecture"]["packages"]
        ["volicord-user-action-service"];
    assert_eq!(
        service_package["group"].as_str(),
        Some("user-action-service")
    );

    let service_group =
        &manifest["workspace"]["metadata"]["architecture"]["groups"]["user-action-service"];
    let dependency_groups = |kind: &str| {
        service_group[kind]
            .as_array()
            .expect("service dependency kind should be an array")
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .expect("service dependency group should be a string")
            })
            .collect::<Vec<_>>()
    };

    assert_eq!(dependency_groups("normal"), ["shared-types", "storage"]);
    assert_eq!(dependency_groups("development"), ["test-support"]);
    assert!(dependency_groups("build").is_empty());

    for forbidden in ["core", "cli-adapter", "mcp-adapter", "adapter-presentation"] {
        assert!(
            !["normal", "development", "build"]
                .into_iter()
                .flat_map(dependency_groups)
                .any(|group| group == forbidden),
            "UserAction service architecture group must exclude {forbidden}"
        );
    }
}
