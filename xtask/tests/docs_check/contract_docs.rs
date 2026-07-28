use super::*;

const REQUEST_REGION_ID: &str = "contract-structures api.method.intake.request[params]";
const RESPONSE_REGION_ID: &str = "contract-structures api.method.intake.response[response_variants] api.method.intake.response[result_body] api.method.intake.response[rejection] api.method.intake.response[dry_run]";
const COMMON_REGION_ID: &str = "contract-structures api.schema.core[schema_object.ToolResultBase] api.schema.core[schema_object.ToolRejectedBase] api.schema.core[schema_object.ToolDryRunBase] api.schema.core[schema_object.ToolRejectedResponse] api.schema.core[schema_object.ToolDryRunResponse]";

fn marker(kind: &str, id: &str) -> String {
    format!("<!-- {kind} GENERATED: {id} -->")
}

fn install_contract_fixture(root: &Path) {
    install_admin_cli_fixture(root);
    let method = format!(
        "# Intake\n\n## Request\n\n{}\n{}\n\n## Response\n\n{}\n{}\n",
        marker("BEGIN", REQUEST_REGION_ID),
        marker("END", REQUEST_REGION_ID),
        marker("BEGIN", RESPONSE_REGION_ID),
        marker("END", RESPONSE_REGION_ID),
    );
    let common = format!(
        "# API Schema Core\n\n<a id=\"common-response\"></a>\n## Common response\n\n{}\n{}\n",
        marker("BEGIN", COMMON_REGION_ID),
        marker("END", COMMON_REGION_ID),
    );
    write(root, "docs/en/reference/api/method-intake.md", &method);
    write(root, "docs/ko/reference/api/method-intake.md", &method);
    write(root, "docs/en/reference/api/schema-core.md", &common);
    write(root, "docs/ko/reference/api/schema-core.md", &common);

    let mut index =
        fs::read_to_string(root.join("docs/doc-index.yaml")).expect("documentation index");
    index.push_str(
        r#"- doc_id: reference.api.method-intake
  path_en: docs/en/reference/api/method-intake.md
  path_ko: docs/ko/reference/api/method-intake.md
  kind: reference
  summary: Intake method.
  normative_level: contract
  translation_policy: semantic_parity
  owner_area: developer_documentation
  created_on: '2026-06-20'
  last_updated_on: '2026-06-20'
  last_verified_on: '2026-06-23'
- doc_id: reference.api.schema-core
  path_en: docs/en/reference/api/schema-core.md
  path_ko: docs/ko/reference/api/schema-core.md
  kind: reference
  summary: Shared API schemas.
  normative_level: contract
  translation_policy: semantic_parity
  contracts:
  - api.schema.core
  owner_area: developer_documentation
  created_on: '2026-06-20'
  last_updated_on: '2026-06-20'
  last_verified_on: '2026-06-23'
"#,
    );
    write(root, "docs/doc-index.yaml", &index);
}

#[test]
fn production_contract_sync_is_bilingual_and_idempotent() {
    let fixture = valid_fixture();
    install_contract_fixture(fixture.path());

    let first = xtask::run_docs_sync(fixture.path()).expect("first docs sync");
    let english = fs::read_to_string(
        fixture
            .path()
            .join("docs/en/reference/api/method-intake.md"),
    )
    .expect("English method");
    let korean = fs::read_to_string(
        fixture
            .path()
            .join("docs/ko/reference/api/method-intake.md"),
    )
    .expect("Korean method");
    let common = fs::read_to_string(fixture.path().join("docs/en/reference/api/schema-core.md"))
        .expect("common schema");
    let second = xtask::run_docs_sync(fixture.path()).expect("second docs sync");

    assert_eq!(first.updated_paths().len(), 6);
    assert!(second.updated_paths().is_empty());
    assert!(english.contains("### `IntakeRequest` fields"));
    assert!(english.contains("### `IntakeResult` success fields"));
    assert!(korean.contains("### `IntakeRequest` 필드"));
    assert!(korean.contains("### `IntakeResult` 성공 필드"));
    assert!(common.contains("### `ToolRejectedBase` rejection fields"));
    assert!(common.contains("### `ToolDryRunBase` preview fields"));
    assert!(common.contains("### `ToolRejectedResponse` rejection fields"));
    assert!(category_errors(&report(fixture.path()), "generated_contract.drift").is_empty());
}

#[test]
fn docs_check_detects_added_and_removed_response_rows() {
    let fixture = valid_fixture();
    install_contract_fixture(fixture.path());
    xtask::run_docs_sync(fixture.path()).expect("docs sync");
    let path = "docs/en/reference/api/method-intake.md";
    let generated = fs::read_to_string(fixture.path().join(path)).expect("generated method");

    let missing = generated.replacen("| `task_ref` | yes | no | `StateRecordRef` |\n", "", 1);
    write(fixture.path(), path, &missing);
    let missing_report = report(fixture.path());
    assert_eq!(
        category_errors(&missing_report, "generated_contract.drift").len(),
        1,
        "{:#?}",
        missing_report.issues()
    );

    let extra = generated.replacen(
        "| `task_ref` | yes | no | `StateRecordRef` |\n",
        "| `retired_value` | yes | no | `string` |\n| `task_ref` | yes | no | `StateRecordRef` |\n",
        1,
    );
    write(fixture.path(), path, &extra);
    let extra_report = report(fixture.path());
    assert_eq!(
        category_errors(&extra_report, "generated_contract.drift").len(),
        1,
        "{:#?}",
        extra_report.issues()
    );
}

#[test]
fn docs_check_rejects_missing_or_wrong_response_bindings() {
    let fixture = valid_fixture();
    install_contract_fixture(fixture.path());
    xtask::run_docs_sync(fixture.path()).expect("docs sync");
    let path = "docs/en/reference/api/method-intake.md";
    let generated = fs::read_to_string(fixture.path().join(path)).expect("generated method");

    let missing = generated.replacen(&marker("BEGIN", RESPONSE_REGION_ID), "", 1);
    write(fixture.path(), path, &missing);
    let missing_report = report(fixture.path());
    assert_eq!(
        category_errors(&missing_report, "generated_contract.region").len(),
        1,
        "{:#?}",
        missing_report.issues()
    );

    let wrong = generated.replace(
        RESPONSE_REGION_ID,
        "contract-structures api.method.status.response[response_variants] api.method.status.response[result_body] api.method.status.response[rejection] api.method.status.response[dry_run]",
    );
    write(fixture.path(), path, &wrong);
    let wrong_report = report(fixture.path());
    assert_eq!(
        category_errors(&wrong_report, "generated_contract.region").len(),
        1,
        "{:#?}",
        wrong_report.issues()
    );
}
