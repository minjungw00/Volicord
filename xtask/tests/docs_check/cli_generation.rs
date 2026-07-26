use super::*;

#[test]
fn repository_documentation_and_cli_examples_match_their_sources() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask manifest has a repository parent");

    let report = report(root);

    assert!(report.is_ok(), "{:#?}", report.issues());
}

#[test]
fn cli_synopsis_generator_is_idempotent() {
    let fixture = valid_fixture();
    install_admin_cli_fixture(fixture.path(), false);

    let first = xtask::run_docs_sync(fixture.path()).expect("first docs sync");
    let first_en =
        fs::read_to_string(fixture.path().join("docs/en/reference/admin-cli.md")).expect("English");
    let first_ko =
        fs::read_to_string(fixture.path().join("docs/ko/reference/admin-cli.md")).expect("Korean");
    let second = xtask::run_docs_sync(fixture.path()).expect("second docs sync");

    assert_eq!(
        first.updated_paths(),
        [
            "docs/en/reference/admin-cli.md",
            "docs/ko/reference/admin-cli.md"
        ]
    );
    assert!(second.updated_paths().is_empty());
    assert_eq!(
        fs::read_to_string(fixture.path().join("docs/en/reference/admin-cli.md"))
            .expect("English after second sync"),
        first_en
    );
    assert_eq!(
        fs::read_to_string(fixture.path().join("docs/ko/reference/admin-cli.md"))
            .expect("Korean after second sync"),
        first_ko
    );
}

#[test]
fn docs_check_reports_generated_cli_region_drift() {
    let fixture = valid_fixture();
    install_admin_cli_fixture(fixture.path(), true);
    xtask::run_docs_sync(fixture.path()).expect("docs sync");

    let path = "docs/en/reference/admin-cli.md";
    let drifted = fs::read_to_string(fixture.path().join(path))
        .expect("generated owner")
        .replacen("do not edit this region", "drifted region", 1);
    write(fixture.path(), path, &drifted);

    let report = report(fixture.path());
    let errors = category_errors(&report, "generated_cli.drift");

    assert_eq!(errors.len(), 1, "{:#?}", report.issues());
    assert_eq!(errors[0].path(), path);
}

#[test]
fn generated_cli_regions_exclude_every_hidden_command_path() {
    let fixture = valid_fixture();
    install_admin_cli_fixture(fixture.path(), false);
    xtask::run_docs_sync(fixture.path()).expect("docs sync");

    let generated =
        fs::read_to_string(fixture.path().join("docs/en/reference/admin-cli.md")).expect("owner");
    let hidden = volicord_command_model::command_paths()
        .into_iter()
        .filter(|path| path.visibility() == volicord_command_model::CommandVisibility::Hidden)
        .collect::<Vec<_>>();

    assert!(!hidden.is_empty());
    for path in hidden {
        let command = format!("volicord {}", path.components().join(" "));
        assert!(
            !generated.contains(&command),
            "hidden command was generated: {command}"
        );
    }
}
