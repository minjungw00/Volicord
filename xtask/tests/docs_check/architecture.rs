use super::*;

#[test]
fn maintainability_report_is_informational_for_long_files() {
    let fixture = tempfile::tempdir().expect("tempdir");
    let root = fixture.path();
    write(root, "Cargo.toml", "[workspace]\n");
    write(root, "src/large.rs", &"fn example() {}\n".repeat(200));

    let report = xtask::run_maintainability_report(root).expect("maintainability report");
    let rendered = report.render();

    assert!(rendered.contains("Informational only"));
    assert!(rendered.contains("src/large.rs"));
    assert_eq!(report.largest_rust_files()[0].lines(), 200);
}
