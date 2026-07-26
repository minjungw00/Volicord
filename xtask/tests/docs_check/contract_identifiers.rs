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

fn operation_category_identifiers() -> Vec<String> {
    let mut identifiers = vec!["operation_category".to_owned()];
    identifiers.extend(operation_categories());
    identifiers
}

#[test]
fn operation_category_documents_match_the_runtime_owner() {
    let fixture = valid_fixture();
    let categories = operation_categories();
    let identifiers = operation_category_identifiers();
    install_operation_category_fixture(fixture.path(), &categories, &categories, &identifiers);

    let report = report(fixture.path());

    assert!(report.is_ok(), "{:#?}", report.issues());
}

#[test]
fn reports_operation_category_document_drift_from_the_runtime_owner() {
    let fixture = valid_fixture();
    let categories = operation_categories();
    let identifiers = operation_category_identifiers();
    let missing_category = categories
        .last()
        .expect("runtime schema should expose an operation category")
        .clone();
    let incomplete = categories
        .iter()
        .filter(|category| category.as_str() != missing_category.as_str())
        .cloned()
        .collect::<Vec<_>>();
    install_operation_category_fixture(fixture.path(), &categories, &incomplete, &identifiers);

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

#[test]
fn reports_runtime_owned_identifiers_missing_from_terminology() {
    let runtime_identifier = operation_categories()
        .into_iter()
        .next()
        .expect("runtime schema should expose an operation category");
    for missing_identifier in ["operation_category".to_owned(), runtime_identifier] {
        let fixture = valid_fixture();
        let categories = operation_categories();
        let preserved = operation_category_identifiers()
            .iter()
            .filter(|identifier| identifier.as_str() != missing_identifier.as_str())
            .cloned()
            .collect::<Vec<_>>();
        install_operation_category_fixture(fixture.path(), &categories, &categories, &preserved);

        let report = report(fixture.path());
        let errors = category_errors(
            &report,
            "contract_identifiers.operation_category_terminology",
        );

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
