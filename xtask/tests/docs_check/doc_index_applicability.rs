use super::*;

#[test]
fn accepts_valid_version_3_metadata() {
    let fixture = valid_fixture();

    let report = report(fixture.path());

    assert!(report.is_ok(), "{:#?}", report.issues());
}

#[test]
fn resolves_current_applicability_sources_without_copying_values_into_keys() {
    let fixture = valid_fixture();

    let report = report(fixture.path());

    assert!(
        category_errors(&report, "metadata.unresolved_applicability_source").is_empty(),
        "{:#?}",
        report.issues()
    );
}

#[test]
fn rejects_version_number_in_applicability_identifier() {
    let fixture = valid_fixture();
    let index = valid_doc_index().replace("sample_workspace:", "sample_workspace_7:");
    write(fixture.path(), "docs/doc-index.yaml", &index);

    let report = report(fixture.path());
    let errors = category_errors(&report, "metadata.invalid_applicability_identifier");

    assert_eq!(errors.len(), 1, "{:#?}", report.issues());
    assert!(errors[0].message().contains("sample_workspace_7"));
}

#[test]
fn reports_missing_current_applicability_source() {
    let fixture = valid_fixture();
    let index = valid_doc_index().replace("    version_source: workspace_package\n", "");
    write(fixture.path(), "docs/doc-index.yaml", &index);

    let report = report(fixture.path());
    let errors = category_errors(&report, "metadata.invalid_applicability_source");

    assert_eq!(errors.len(), 1, "{:#?}", report.issues());
    assert!(errors[0].message().contains("sample_workspace"));
}

#[test]
fn reports_malformed_workspace_package_version() {
    let fixture = valid_fixture();
    write(
        fixture.path(),
        "Cargo.toml",
        "[workspace.package]\nversion = [\"1.2.3\"]\n",
    );

    let report = report(fixture.path());
    let errors = category_errors(&report, "metadata.unresolved_applicability_source");

    assert_eq!(errors.len(), 1, "{:#?}", report.issues());
    assert_eq!(errors[0].path(), "docs/doc-index.yaml");
    assert!(errors[0].message().contains("workspace_package"));
}

#[test]
fn reports_unsupported_applicability_source() {
    let fixture = valid_fixture();
    let index = valid_doc_index().replace(
        "version_source: workspace_package",
        "version_source: archive",
    );
    write(fixture.path(), "docs/doc-index.yaml", &index);

    let report = report(fixture.path());
    let errors = category_errors(&report, "metadata.invalid_applicability_source");

    assert_eq!(errors.len(), 1, "{:#?}", report.issues());
    assert!(errors[0].message().contains("archive"));
}

#[test]
fn rejects_noncurrent_required_entry_fields() {
    let fixture = valid_fixture();
    let index = valid_doc_index().replace(
        "  - last_verified_on\n  paired_required:",
        "  - last_verified_on\n  - applies_to\n  paired_required:",
    );
    write(fixture.path(), "docs/doc-index.yaml", &index);

    let report = report(fixture.path());
    let errors = category_errors(&report, "metadata.entry_schema");

    assert_eq!(errors.len(), 1, "{:#?}", report.issues());
    assert!(errors[0].message().contains("shared_required"));
}
