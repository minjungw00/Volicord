use super::*;
use schemars::schema_for;
use volicord_types::values::OperationCategory;

fn operation_categories() -> Vec<String> {
    schema_for!(OperationCategory)
        .schema
        .enum_values
        .expect("OperationCategory should have a closed schema value set")
        .into_iter()
        .map(|value| {
            value
                .as_str()
                .expect("OperationCategory schema values should be strings")
                .to_owned()
        })
        .collect()
}

#[test]
fn operation_category_documents_match_the_runtime_owner() {
    let fixture = valid_fixture();
    let categories = operation_categories();
    install_operation_category_fixture(fixture.path(), &categories, &categories);

    let report = report(fixture.path());

    assert!(report.is_ok(), "{:#?}", report.issues());
}

#[test]
fn reports_operation_category_document_drift_from_the_runtime_owner() {
    let fixture = valid_fixture();
    let categories = operation_categories();
    let missing_category = categories
        .last()
        .expect("runtime schema should expose an operation category")
        .clone();
    let incomplete = categories
        .iter()
        .filter(|category| category.as_str() != missing_category.as_str())
        .cloned()
        .collect::<Vec<_>>();
    install_operation_category_fixture(fixture.path(), &categories, &incomplete);

    let report = report(fixture.path());
    let errors = category_errors(&report, "contract_identifiers.operation_category_drift");

    assert_eq!(errors.len(), 1, "{:#?}", report.issues());
    assert!(
        errors[0]
            .message()
            .contains(&format!("`{missing_category}`")),
        "{:#?}",
        report.issues()
    );
}
