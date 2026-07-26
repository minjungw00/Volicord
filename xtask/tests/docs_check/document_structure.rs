use super::*;

#[test]
fn accepts_current_architecture_design_section_schema_without_scanning_prose() {
    let fixture = valid_fixture();
    write(
        fixture.path(),
        "docs/en/architecture-guide/design/core-adapter-boundary.md",
        &architecture_design_document(
            "# Core and adapter dependency boundary",
            &[
                "Purpose",
                "Design",
                "Invariants",
                "Responsibility boundaries",
                "Execution flow",
                "Failure behavior",
                "Scope exclusions",
                "Implementation routes",
                "Reference owners",
            ],
            "\nRejected alternatives can be ordinary prose without becoming a section.\n",
        ),
    );
    write(
        fixture.path(),
        "docs/ko/architecture-guide/design/core-adapter-boundary.md",
        &architecture_design_document(
            "# Core와 어댑터 의존 경계",
            &[
                "목적",
                "설계",
                "불변 조건",
                "책임 경계",
                "실행 흐름",
                "실패 동작",
                "범위 제외",
                "구현 경로",
                "참조 담당 문서",
            ],
            "\n거부한 대안이라는 표현은 절 제목이 아닌 일반 본문에 쓸 수 있습니다.\n",
        ),
    );

    let report = report(fixture.path());

    assert!(
        category_errors(&report, "architecture_design.section_schema").is_empty(),
        "{:#?}",
        report.issues()
    );
    assert!(
        category_errors(&report, "architecture_design.prohibited_heading").is_empty(),
        "{:#?}",
        report.issues()
    );
}

#[test]
fn reports_invalid_current_architecture_design_section_sequence() {
    let fixture = valid_fixture();
    write(
        fixture.path(),
        "docs/en/architecture-guide/design/core-adapter-boundary.md",
        &architecture_design_document(
            "# Core and adapter dependency boundary",
            &[
                "Purpose",
                "Design",
                "Invariants",
                "Execution flow",
                "Responsibility boundaries",
                "Failure behavior",
                "Scope exclusions",
                "Implementation routes",
                "Reference owners",
            ],
            "",
        ),
    );

    let report = report(fixture.path());
    let errors = category_errors(&report, "architecture_design.section_schema");

    assert_eq!(errors.len(), 1, "{:#?}", report.issues());
    assert!(
        errors[0].message().contains("`Responsibility boundaries`"),
        "{:#?}",
        report.issues()
    );
}

#[test]
fn reports_transitional_architecture_design_heading_at_any_level() {
    let fixture = valid_fixture();
    write(
        fixture.path(),
        "docs/en/architecture-guide/design/core-adapter-boundary.md",
        &architecture_design_document(
            "# Core and adapter dependency boundary",
            &[
                "Purpose",
                "Design",
                "Invariants",
                "Responsibility boundaries",
                "Execution flow",
                "Failure behavior",
                "Scope exclusions",
                "Implementation routes",
                "Reference owners",
            ],
            "\n### Before-and-after\n\nTransitional comparison.\n",
        ),
    );

    let report = report(fixture.path());
    let errors = category_errors(&report, "architecture_design.prohibited_heading");

    assert_eq!(errors.len(), 1, "{:#?}", report.issues());
    assert_eq!(errors[0].line(), Some(39), "{:#?}", report.issues());
}

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

fn architecture_design_document(
    title: &str,
    h2_headings: &[&str],
    trailing_contents: &str,
) -> String {
    let sections = h2_headings
        .iter()
        .map(|heading| format!("## {heading}\n\nCurrent design.\n"))
        .collect::<Vec<_>>()
        .join("\n");
    format!("{title}\n\n{sections}{trailing_contents}")
}
