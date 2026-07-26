use super::*;

#[test]
fn accepts_synchronized_operation_category_value_sets() {
    let fixture = valid_fixture();
    let values = ["read", "agent_workflow", "user_only"];
    let preserved = ["operation_category", "read", "agent_workflow", "user_only"];
    install_operation_category_fixture(fixture.path(), &values, &values, &preserved);

    let report = report(fixture.path());

    assert!(report.is_ok(), "{:#?}", report.issues());
}

#[test]
fn reports_operation_category_language_value_set_drift() {
    let fixture = valid_fixture();
    let en_values = ["read", "agent_workflow", "user_only"];
    let ko_values = ["read", "agent_workflow"];
    let preserved = ["operation_category", "read", "agent_workflow", "user_only"];
    install_operation_category_fixture(fixture.path(), &en_values, &ko_values, &preserved);

    let report = report(fixture.path());
    let errors = category_errors(&report, "operation_category_values.language_drift");

    assert_eq!(errors.len(), 1, "{:#?}", report.issues());
    assert!(
        errors[0].message().contains("`user_only`"),
        "{:#?}",
        report.issues()
    );
}

#[test]
fn reports_missing_operation_category_terminology_identifiers() {
    let values = ["read", "agent_workflow", "user_only"];
    let all_identifiers = ["operation_category", "read", "agent_workflow", "user_only"];

    for missing_identifier in ["operation_category", "user_only"] {
        let fixture = valid_fixture();
        let preserved = all_identifiers
            .iter()
            .copied()
            .filter(|identifier| *identifier != missing_identifier)
            .collect::<Vec<_>>();
        install_operation_category_fixture(fixture.path(), &values, &values, &preserved);

        let report = report(fixture.path());
        let errors = category_errors(&report, "operation_category_values.terminology_missing");

        assert_eq!(errors.len(), 1, "{:#?}", report.issues());
        assert!(
            errors[0]
                .message()
                .contains(&format!("`{missing_identifier}`")),
            "{:#?}",
            report.issues()
        );
    }
}
#[test]
fn reports_missing_required_surface_stability_section() {
    let fixture = valid_fixture();
    index_admin_cli_surface_doc(fixture.path());
    write(
        fixture.path(),
        "docs/en/reference/admin-cli.md",
        "# Administrative CLI reference\n",
    );
    write(
        fixture.path(),
        "docs/ko/reference/admin-cli.md",
        "<a id=\"administrative-cli-reference\"></a>\n# 관리 CLI 참조\n",
    );

    let report = report(fixture.path());
    let errors = category_errors(&report, "surface_stability.missing_section");

    assert_eq!(errors.len(), 2, "{:#?}", report.issues());
}

#[test]
fn reports_missing_required_surface_stability_label() {
    let fixture = valid_fixture();
    index_admin_cli_surface_doc(fixture.path());
    let section = "# Administrative CLI reference\n\n<a id=\"surface-stability\"></a>\n## Surface Stability\n\nFor label meanings, see [Documentation Policy](../maintain/documentation-policy.md#surface-stability-labels).\n\n| Surface | Stability | Notes |\n|---|---|---|\n| Commands | `stable` | Local CLI command contract. |\n";
    write(fixture.path(), "docs/en/reference/admin-cli.md", section);
    write(
        fixture.path(),
        "docs/ko/reference/admin-cli.md",
        &section.replace("# Administrative CLI reference", "# 관리 CLI 참조"),
    );

    let report = report(fixture.path());
    let errors = category_errors(&report, "surface_stability.missing_label");

    assert!(
        errors
            .iter()
            .any(|error| error.path() == "docs/en/reference/admin-cli.md"
                && error.message().contains("`beta`")),
        "{:#?}",
        report.issues()
    );
    assert!(
        errors
            .iter()
            .any(|error| error.path() == "docs/ko/reference/admin-cli.md"
                && error.message().contains("`diagnostic`")),
        "{:#?}",
        report.issues()
    );
}
