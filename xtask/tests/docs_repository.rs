use std::path::Path;

#[test]
fn current_repository_documentation_passes_owner_derived_identifier_rules() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask has a workspace parent");
    let report = xtask::run_docs_check(root).expect("repository docs-check runs");

    assert!(report.is_ok(), "{:#?}", report.issues());
}
