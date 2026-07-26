use crate::diagnostics::ValidationIssue;
use crate::doc_index::{DocIndex, TERMINOLOGY_MAP_PATH};
use crate::terminology::ExactIdentifierCatalog;
use schemars::schema_for;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use volicord_types::values::OperationCategory;

const VALUE_SET_DOC_ID: &str = "reference.api.schema-value-sets";
const OPERATION_CATEGORY_ANCHOR: &str = "operation-category-values";
const OPERATION_CATEGORY_IDENTIFIER: &str = "operation_category";
const OPERATION_CATEGORY_OWNER_PATH: &str = "crates/volicord-types/src/values.rs";

pub(crate) fn validate_operation_category_values(
    root: &Path,
    index: &DocIndex,
    exact_identifiers: &ExactIdentifierCatalog,
    issues: &mut Vec<ValidationIssue>,
) {
    let Some(owner) = index.paired_documents.get(VALUE_SET_DOC_ID) else {
        return;
    };
    let expected = match operation_category_schema_values() {
        Ok(expected) => expected,
        Err(message) => {
            issues.push(ValidationIssue::new(
                OPERATION_CATEGORY_OWNER_PATH,
                "contract_identifiers.operation_category_schema",
                message,
            ));
            return;
        }
    };

    for relative_path in [&owner.path_en, &owner.path_ko] {
        let contents = match fs::read_to_string(root.join(relative_path)) {
            Ok(contents) => contents,
            Err(error) => {
                issues.push(ValidationIssue::new(
                    relative_path,
                    "contract_identifiers.operation_category_read",
                    format!("failed to read operation-category owner: {error}"),
                ));
                continue;
            }
        };
        let Some(section) = extract_anchored_section(&contents, OPERATION_CATEGORY_ANCHOR) else {
            issues.push(ValidationIssue::new(
                relative_path,
                "contract_identifiers.operation_category_section",
                format!("missing anchored operation-category section #{OPERATION_CATEGORY_ANCHOR}"),
            ));
            continue;
        };
        let actual = first_column_identifiers(section);
        if actual != expected {
            let missing = expected.difference(&actual).cloned().collect();
            let unexpected = actual.difference(&expected).cloned().collect();
            issues.push(ValidationIssue::new(
                relative_path,
                "contract_identifiers.operation_category_drift",
                format!(
                    "documented operation categories differ from the JSON Schema for volicord_types::values::OperationCategory; missing: {}; unexpected: {}",
                    format_identifiers(&missing),
                    format_identifiers(&unexpected),
                ),
            ));
        }
    }

    let mut required = expected;
    required.insert(OPERATION_CATEGORY_IDENTIFIER.to_owned());
    let missing = required
        .difference(&exact_identifiers.identifiers)
        .cloned()
        .collect::<BTreeSet<_>>();
    if !missing.is_empty() {
        issues.push(ValidationIssue::new(
            TERMINOLOGY_MAP_PATH,
            "contract_identifiers.operation_category_terminology",
            format!(
                "exact-identifier catalog is missing runtime-owned operation-category identifiers: {}",
                format_identifiers(&missing)
            ),
        ));
    }
}

fn operation_category_schema_values() -> Result<BTreeSet<String>, String> {
    let schema = schema_for!(OperationCategory);
    let values = schema.schema.enum_values.ok_or_else(|| {
        "OperationCategory JSON Schema does not expose a closed enum value set".to_owned()
    })?;
    let mut identifiers = BTreeSet::new();
    for value in values {
        let Some(value) = value.as_str() else {
            return Err(
                "OperationCategory JSON Schema contains a non-string enum value".to_owned(),
            );
        };
        identifiers.insert(value.to_owned());
    }
    if identifiers.is_empty() {
        return Err("OperationCategory JSON Schema exposes an empty enum value set".to_owned());
    }
    Ok(identifiers)
}

fn extract_anchored_section<'a>(contents: &'a str, anchor: &str) -> Option<&'a str> {
    let marker = format!("<a id=\"{anchor}\"></a>");
    let section_start = contents.find(&marker)? + marker.len();
    let remaining = &contents[section_start..];
    let section_end = remaining.find("\n<a id=\"").unwrap_or(remaining.len());
    Some(&remaining[..section_end])
}

fn first_column_identifiers(section: &str) -> BTreeSet<String> {
    section
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if !line.starts_with('|') || !line.ends_with('|') {
                return None;
            }
            let first_cell = line.trim_matches('|').split('|').next()?.trim();
            let value = first_cell.strip_prefix('`')?.strip_suffix('`')?;
            (!value.is_empty()).then(|| value.to_owned())
        })
        .collect()
}

fn format_identifiers(values: &BTreeSet<String>) -> String {
    if values.is_empty() {
        return "none".to_owned();
    }
    values
        .iter()
        .map(|value| format!("`{value}`"))
        .collect::<Vec<_>>()
        .join(", ")
}
