use super::*;

#[test]
fn reports_terminology_map_path_failure() {
    let fixture = valid_fixture();
    let terminology = valid_terminology_map()
        .replace("docs/en/example.md#overview", "docs/en/missing.md#overview");
    write(fixture.path(), "docs/terminology-map.yaml", &terminology);

    let report = report(fixture.path());

    assert!(has_category(&report, "terminology.missing_target"));
}

#[test]
fn reports_required_terminology_role_failure() {
    let fixture = valid_fixture();
    let terminology = valid_terminology_map().replace(
        r#"  project_selector:
    category: identifier
    roles:
      - mcp_public_selector
"#,
        r#"  project_selector:
    category: identifier
"#,
    );
    write(fixture.path(), "docs/terminology-map.yaml", &terminology);

    let report = report(fixture.path());
    let errors = category_errors(&report, "terminology.missing_role");

    assert_eq!(errors.len(), 1, "{:#?}", report.issues());
    assert!(
        errors[0].message().contains("project_selector"),
        "{:#?}",
        report.issues()
    );
}

#[test]
fn reports_invalid_terminology_role_value() {
    let fixture = valid_fixture();
    let terminology =
        valid_terminology_map().replace("      - mcp_public_selector", "      - public_id");
    write(fixture.path(), "docs/terminology-map.yaml", &terminology);

    let report = report(fixture.path());
    let errors = category_errors(&report, "terminology.invalid_role");

    assert_eq!(errors.len(), 1, "{:#?}", report.issues());
    assert!(
        errors[0].message().contains("public_id"),
        "{:#?}",
        report.issues()
    );
}

#[test]
fn accepts_sensitive_identifiers_in_document_prose_when_map_roles_are_valid() {
    let fixture = valid_fixture();
    write(
        fixture.path(),
        "docs/en/example.md",
        "# Overview\n\nA diagnostic can mention `connection_id` and `project_id`, while a public MCP call can use `project_selector`.\n",
    );
    write(
        fixture.path(),
        "docs/ko/example.md",
        "<a id=\"overview\"></a>\n# 개요\n\n진단에는 `connection_id`와 `project_id`가 나올 수 있고, 공개 MCP 호출은 `project_selector`를 사용할 수 있습니다.\n",
    );

    let report = report(fixture.path());

    assert!(report.is_ok(), "{:#?}", report.issues());
}
