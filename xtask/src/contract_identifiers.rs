use crate::diagnostics::ValidationIssue;
use crate::doc_index::{ContractSource, DocIndex, PairedDocument};
use crate::markdown;
use schemars::schema_for;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use volicord_types::methods::{public_request_schema, public_response_schema};
use volicord_types::values::{MethodName, OperationCategory};

const VALUE_SET_DOC_ID: &str = "reference.api.schema-value-sets";
const OPERATION_CATEGORY_ANCHOR: &str = "operation-category-values";
const OPERATION_CATEGORY_OWNER_PATH: &str = "crates/volicord-types/src/values.rs";

#[derive(Debug, Clone, Default)]
struct ContractCatalog {
    sources: BTreeMap<String, OwnerCatalog>,
}

#[derive(Debug, Clone)]
struct OwnerCatalog {
    owner: String,
    identifiers: BTreeSet<String>,
}

pub(crate) fn validate_contract_identifiers(
    root: &Path,
    index: &DocIndex,
    issues: &mut Vec<ValidationIssue>,
) {
    let catalog = load_contract_catalog(root, index, issues);
    for paired in index
        .paired_documents
        .values()
        .filter(|paired| !paired.contract_sources.is_empty())
    {
        validate_pair(root, paired, &catalog, issues);
    }
}

fn load_contract_catalog(
    root: &Path,
    index: &DocIndex,
    issues: &mut Vec<ValidationIssue>,
) -> ContractCatalog {
    let mut catalog = ContractCatalog::default();
    for source in index.contract_sources.values() {
        let identifiers = match source.kind.as_str() {
            "public_json_schemas" => public_api_identifiers(issues),
            "command_model" => volicord_command_model::public_contract_identifiers(),
            "diagnostic_registry" => diagnostic_registry_identifiers(root, source, issues),
            "protocol_registry" => volicord_mcp_protocol::public_protocol_identifiers(),
            kind => {
                issues.push(ValidationIssue::new(
                    "docs/doc-index.yaml",
                    "contract_identifier.source",
                    format!("contract source {} has unsupported kind {kind}", source.id),
                ));
                BTreeSet::new()
            }
        };
        if identifiers.is_empty() {
            issues.push(ValidationIssue::new(
                "docs/doc-index.yaml",
                "contract_identifier.source",
                format!(
                    "contract source {} ({}) resolved an empty current identifier catalog",
                    source.id, source.owner
                ),
            ));
        }
        catalog.sources.insert(
            source.id.clone(),
            OwnerCatalog {
                owner: source.owner.clone(),
                identifiers,
            },
        );
    }
    catalog
}

fn public_api_identifiers(issues: &mut Vec<ValidationIssue>) -> BTreeSet<String> {
    let method_schema =
        serde_json::to_value(schema_for!(MethodName)).expect("MethodName schema serializes");
    let method_names = schema_string_values(&method_schema, "enum");
    let mut identifiers = method_names.clone();
    collect_json_schema_identifiers(&method_schema, &mut identifiers);

    for method_name in method_names {
        for (shape, schema) in [
            ("request", public_request_schema(&method_name)),
            ("response", public_response_schema(&method_name)),
        ] {
            match schema {
                Some(schema) => collect_json_schema_identifiers(&schema, &mut identifiers),
                None => issues.push(ValidationIssue::new(
                    "crates/volicord-types/src/methods.rs",
                    "contract_identifier.public_schema",
                    format!(
                        "current MethodName {method_name} has no generated public {shape} schema"
                    ),
                )),
            }
        }
    }
    identifiers
}

fn collect_json_schema_identifiers(value: &Value, identifiers: &mut BTreeSet<String>) {
    match value {
        Value::Object(object) => {
            for (key, value) in object {
                match key.as_str() {
                    "properties" => {
                        if let Some(entries) = value.as_object() {
                            identifiers.extend(entries.keys().cloned());
                        }
                    }
                    "enum" => {
                        identifiers.extend(
                            value
                                .as_array()
                                .into_iter()
                                .flatten()
                                .filter_map(Value::as_str)
                                .map(str::to_owned),
                        );
                    }
                    "const" => {
                        if let Some(identifier) =
                            value.as_str().filter(|value| looks_like_identifier(value))
                        {
                            identifiers.insert(identifier.to_owned());
                        }
                    }
                    _ => {}
                }
                collect_json_schema_identifiers(value, identifiers);
            }
        }
        Value::Array(values) => {
            for value in values {
                collect_json_schema_identifiers(value, identifiers);
            }
        }
        _ => {}
    }
}

fn schema_string_values(schema: &Value, key: &str) -> BTreeSet<String> {
    schema
        .pointer(&format!("/schema/{key}"))
        .or_else(|| schema.get(key))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}

fn looks_like_identifier(value: &str) -> bool {
    !value.is_empty()
        && !value.chars().any(char::is_whitespace)
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.')
        })
}

fn diagnostic_registry_identifiers(
    root: &Path,
    source: &ContractSource,
    issues: &mut Vec<ValidationIssue>,
) -> BTreeSet<String> {
    let path = root.join(&source.owner);
    let contents = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(error) => {
            issues.push(ValidationIssue::new(
                &source.owner,
                "contract_identifier.diagnostic_registry",
                format!("failed to read current generated diagnostic registry: {error}"),
            ));
            return BTreeSet::new();
        }
    };
    let value: Value = match serde_json::from_str(&contents) {
        Ok(value) => value,
        Err(error) => {
            issues.push(ValidationIssue::new(
                &source.owner,
                "contract_identifier.diagnostic_registry",
                format!("failed to parse current generated diagnostic registry: {error}"),
            ));
            return BTreeSet::new();
        }
    };
    let Some(codes) = value.get("codes").and_then(Value::as_array) else {
        issues.push(ValidationIssue::new(
            &source.owner,
            "contract_identifier.diagnostic_registry",
            "current generated diagnostic registry must contain a codes array",
        ));
        return BTreeSet::new();
    };
    let mut identifiers = BTreeSet::new();
    for code in codes {
        let Some(code) = code.as_str().filter(|code| looks_like_identifier(code)) else {
            issues.push(ValidationIssue::new(
                &source.owner,
                "contract_identifier.diagnostic_registry",
                "current generated diagnostic registry codes must be non-empty identifier strings",
            ));
            continue;
        };
        if !identifiers.insert(code.to_owned()) {
            issues.push(ValidationIssue::new(
                &source.owner,
                "contract_identifier.diagnostic_registry",
                format!("current generated diagnostic registry repeats code {code}"),
            ));
        }
    }
    identifiers
}

fn validate_pair(
    root: &Path,
    paired: &PairedDocument,
    catalog: &ContractCatalog,
    issues: &mut Vec<ValidationIssue>,
) {
    let applicable_identifiers = paired
        .contract_sources
        .iter()
        .filter_map(|source| catalog.sources.get(source))
        .flat_map(|source| source.identifiers.iter().cloned())
        .collect::<BTreeSet<_>>();
    let en = match read_structure(root, &paired.path_en, &applicable_identifiers) {
        Ok(structure) => structure,
        Err(error) => {
            issues.push(ValidationIssue::new(
                &paired.path_en,
                "contract_identifier.read",
                error,
            ));
            return;
        }
    };
    let ko = match read_structure(root, &paired.path_ko, &applicable_identifiers) {
        Ok(structure) => structure,
        Err(error) => {
            issues.push(ValidationIssue::new(
                &paired.path_ko,
                "contract_identifier.read",
                error,
            ));
            return;
        }
    };

    validate_invalid_identifiers(
        paired,
        &en,
        &applicable_identifiers,
        catalog,
        &paired.path_en,
        issues,
    );
    validate_invalid_identifiers(
        paired,
        &ko,
        &applicable_identifiers,
        catalog,
        &paired.path_ko,
        issues,
    );

    for (position, (en_section, ko_section)) in en.sections.iter().zip(&ko.sections).enumerate() {
        let en_identifiers = section_identifiers(en_section);
        let ko_identifiers = section_identifiers(ko_section);
        report_missing_identifiers(
            paired,
            position,
            en_section,
            ko_section,
            &en_identifiers
                .difference(&ko_identifiers)
                .cloned()
                .collect(),
            "Korean",
            &paired.path_ko,
            ko_section.line,
            catalog,
            issues,
        );
        report_missing_identifiers(
            paired,
            position,
            en_section,
            ko_section,
            &ko_identifiers
                .difference(&en_identifiers)
                .cloned()
                .collect(),
            "English",
            &paired.path_en,
            en_section.line,
            catalog,
            issues,
        );
    }
}

fn read_structure(
    root: &Path,
    relative_path: &str,
    applicable_identifiers: &BTreeSet<String>,
) -> Result<markdown::MarkdownStructure, String> {
    let contents = fs::read_to_string(root.join(relative_path))
        .map_err(|error| format!("failed to read paired Markdown: {error}"))?;
    Ok(markdown::identifier_structure(
        &contents,
        applicable_identifiers,
    ))
}

fn section_identifiers(section: &markdown::MarkdownSection) -> BTreeSet<String> {
    section
        .units
        .iter()
        .flat_map(|unit| unit.identifiers.iter().cloned())
        .chain(section.heading_identifiers.iter().cloned())
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn report_missing_identifiers(
    paired: &PairedDocument,
    position: usize,
    en_section: &markdown::MarkdownSection,
    ko_section: &markdown::MarkdownSection,
    missing: &BTreeSet<String>,
    language: &str,
    path: &str,
    line: usize,
    catalog: &ContractCatalog,
    issues: &mut Vec<ValidationIssue>,
) {
    if missing.is_empty() {
        return;
    }
    issues.push(ValidationIssue::at_line(
        path,
        "contract_identifier.missing",
        Some(line),
        format!(
            "document pair {} ({} <-> {}), section {position} `{}` (English line {}, Korean line {}), contract owner(s) {}: {language} meaning unit is missing {}",
            paired.doc_id,
            paired.path_en,
            paired.path_ko,
            display_heading(en_section),
            en_section.line,
            ko_section.line,
            owners_for_identifiers(missing, &paired.contract_sources, catalog),
            format_identifiers(missing),
        ),
    ));
}

fn validate_invalid_identifiers(
    paired: &PairedDocument,
    structure: &markdown::MarkdownStructure,
    applicable_identifiers: &BTreeSet<String>,
    catalog: &ContractCatalog,
    path: &str,
    issues: &mut Vec<ValidationIssue>,
) {
    for (position, section) in structure.sections.iter().enumerate() {
        let exact_candidates = section
            .literals
            .iter()
            .flat_map(|literal| {
                literal_identifier_candidates(literal)
                    .into_iter()
                    .filter(|candidate| supported_candidate(literal, candidate))
            })
            .collect::<BTreeSet<_>>();
        for candidate in exact_candidates {
            if applicable_identifiers.contains(&candidate) {
                continue;
            }
            let suggestions = nearest_identifiers(&candidate, applicable_identifiers);
            if suggestions.is_empty() {
                continue;
            }
            let line = section
                .literals
                .iter()
                .find(|literal| literal_identifier_candidates(literal).contains(&candidate))
                .map_or(section.line, |literal| literal.line);
            issues.push(ValidationIssue::at_line(
                path,
                "contract_identifier.invalid",
                Some(line),
                format!(
                    "document pair {} ({} <-> {}), section {position} `{}`, contract owner(s) {}: identifier `{candidate}` does not exist in the referenced current contract; nearest current identifier(s): {}",
                    paired.doc_id,
                    paired.path_en,
                    paired.path_ko,
                    display_heading(section),
                    selected_owners(&paired.contract_sources, catalog),
                    format_identifiers(&suggestions),
                ),
            ));
        }
    }
}

fn display_heading(section: &markdown::MarkdownSection) -> &str {
    if section.heading.trim().is_empty() {
        "document preamble"
    } else {
        section.heading.trim()
    }
}

fn owners_for_identifiers(
    identifiers: &BTreeSet<String>,
    selected_sources: &BTreeSet<String>,
    catalog: &ContractCatalog,
) -> String {
    let sources = selected_sources
        .iter()
        .filter(|source_id| {
            catalog.sources.get(*source_id).is_some_and(|source| {
                identifiers
                    .iter()
                    .any(|identifier| source.identifiers.contains(identifier))
            })
        })
        .cloned()
        .collect::<BTreeSet<_>>();
    selected_owners(&sources, catalog)
}

fn selected_owners(selected_sources: &BTreeSet<String>, catalog: &ContractCatalog) -> String {
    selected_sources
        .iter()
        .filter_map(|source_id| {
            catalog
                .sources
                .get(source_id)
                .map(|source| format!("{source_id} ({})", source.owner))
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn identifier_candidates(literal: &str) -> BTreeSet<String> {
    literal
        .split(|character: char| {
            !(character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.'))
        })
        .map(|candidate| candidate.trim_matches('.'))
        .filter(|candidate| candidate.len() >= 4)
        .map(str::to_owned)
        .collect()
}

fn supported_candidate(literal: &markdown::MarkdownLiteral, candidate: &str) -> bool {
    literal.unit_kind == Some(markdown::MarkdownUnitKind::TableRow)
        || markdown::is_explicit_contract_identifier(candidate)
}

fn literal_identifier_candidates(literal: &markdown::MarkdownLiteral) -> BTreeSet<String> {
    if literal.kind == markdown::MarkdownLiteralKind::Fenced {
        return match literal.language.as_deref() {
            Some("json" | "yaml" | "yml") => {
                serde_yaml::from_str::<serde_yaml::Value>(&literal.text)
                    .map(|value| {
                        let mut candidates = BTreeSet::new();
                        collect_structured_candidates(&value, &mut candidates);
                        candidates
                    })
                    .unwrap_or_default()
            }
            Some("bash" | "console" | "sh" | "shell" | "zsh") => {
                identifier_candidates(&literal.text)
            }
            _ => BTreeSet::new(),
        };
    }
    identifier_candidates(&literal.text)
}

fn collect_structured_candidates(value: &serde_yaml::Value, candidates: &mut BTreeSet<String>) {
    match value {
        serde_yaml::Value::Mapping(mapping) => {
            for (key, value) in mapping {
                if let Some(key) = key.as_str() {
                    candidates.insert(key.strip_suffix('?').unwrap_or(key).to_owned());
                }
                collect_structured_candidates(value, candidates);
            }
        }
        serde_yaml::Value::Sequence(sequence) => {
            for value in sequence {
                collect_structured_candidates(value, candidates);
            }
        }
        serde_yaml::Value::String(value) if looks_like_identifier(value) => {
            candidates.insert(value.to_owned());
        }
        _ => {}
    }
}

fn nearest_identifiers(candidate: &str, identifiers: &BTreeSet<String>) -> BTreeSet<String> {
    let mut matches = identifiers
        .iter()
        .filter(|identifier| comparable_identifier_shape(candidate, identifier))
        .filter_map(|identifier| {
            if candidate.len() != identifier.len()
                || shared_prefix_length(candidate, identifier) * 2 < candidate.len()
            {
                return None;
            }
            let distance = edit_distance(candidate, identifier);
            (distance <= 2).then_some((distance, identifier.clone()))
        })
        .collect::<Vec<_>>();
    matches.sort();
    let Some(best) = matches.first().map(|(distance, _)| *distance) else {
        return BTreeSet::new();
    };
    matches
        .into_iter()
        .take_while(|(distance, _)| *distance == best)
        .map(|(_, identifier)| identifier)
        .collect()
}

fn shared_prefix_length(left: &str, right: &str) -> usize {
    left.chars()
        .zip(right.chars())
        .take_while(|(left, right)| left == right)
        .count()
}

fn comparable_identifier_shape(left: &str, right: &str) -> bool {
    let dotted = left.contains('.') && right.contains('.');
    let snake = left.contains('_') && right.contains('_');
    let option = left.starts_with('-') && right.starts_with('-');
    let upper = left
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_uppercase())
        && right
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_uppercase());
    let lowercase_value = left.len() >= 6
        && right.len() >= 6
        && left.chars().all(|character| character.is_ascii_lowercase())
        && right
            .chars()
            .all(|character| character.is_ascii_lowercase());
    dotted || snake || option || upper || lowercase_value
}

fn edit_distance(left: &str, right: &str) -> usize {
    let right = right.chars().collect::<Vec<_>>();
    let mut previous = (0..=right.len()).collect::<Vec<_>>();
    for (left_index, left_character) in left.chars().enumerate() {
        let mut current = vec![left_index + 1];
        for (right_index, right_character) in right.iter().enumerate() {
            current.push(
                (previous[right_index + 1] + 1)
                    .min(current[right_index] + 1)
                    .min(previous[right_index] + usize::from(left_character != *right_character)),
            );
        }
        previous = current;
    }
    previous[right.len()]
}

pub(crate) fn validate_operation_category_values(
    root: &Path,
    index: &DocIndex,
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn owner_catalog(owner: &str, identifiers: BTreeSet<String>) -> OwnerCatalog {
        OwnerCatalog {
            owner: owner.to_owned(),
            identifiers,
        }
    }

    fn synthetic_pair(sources: &[&str], english: &str, korean: &str) -> (TempDir, PairedDocument) {
        let root = tempfile::tempdir().expect("synthetic Markdown fixture root");
        fs::write(root.path().join("en.md"), english).expect("write English fixture");
        fs::write(root.path().join("ko.md"), korean).expect("write Korean fixture");
        (
            root,
            PairedDocument {
                doc_id: "synthetic.pair".to_owned(),
                path_en: "en.md".to_owned(),
                path_ko: "ko.md".to_owned(),
                contract_sources: sources.iter().map(|source| (*source).to_owned()).collect(),
            },
        )
    }

    fn validate_synthetic_pair(
        catalog: &ContractCatalog,
        sources: &[&str],
        english: &str,
        korean: &str,
    ) -> Vec<ValidationIssue> {
        let (root, pair) = synthetic_pair(sources, english, korean);
        let mut issues = Vec::new();
        validate_pair(root.path(), &pair, catalog, &mut issues);
        issues.sort();
        issues
    }

    #[test]
    fn extracts_public_fields_enum_values_and_schema_names() {
        let schema = serde_json::json!({
            "title": "SyntheticRequest",
            "properties": {
                "state_version": {"type": "integer"},
                "status": {"enum": ["queued", "complete"]}
            },
            "definitions": {
                "NestedState": {
                    "properties": {"accepted_for_close": {"type": "boolean"}}
                }
            }
        });
        let mut identifiers = BTreeSet::new();
        collect_json_schema_identifiers(&schema, &mut identifiers);

        assert_eq!(
            identifiers,
            [
                "accepted_for_close",
                "complete",
                "queued",
                "state_version",
                "status",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect()
        );
    }

    #[test]
    fn current_public_schema_catalog_contains_accepted_for_close() {
        let identifiers = public_api_identifiers(&mut Vec::new());
        assert!(identifiers.contains("accepted_for_close"));
    }

    #[test]
    fn unrelated_code_spans_have_no_contract_suggestion() {
        let identifiers = ["state_version", "accepted_for_close"]
            .into_iter()
            .map(str::to_owned)
            .collect();
        assert!(nearest_identifiers("local_note", &identifiers).is_empty());
    }

    #[test]
    fn misspelled_identifier_has_a_deterministic_suggestion() {
        let identifiers = ["accepted_for_close", "state_version"]
            .into_iter()
            .map(str::to_owned)
            .collect();
        assert_eq!(
            nearest_identifiers("accepted_for_clsoe", &identifiers),
            ["accepted_for_close".to_owned()].into_iter().collect()
        );
    }

    #[test]
    fn paired_schema_fixture_checks_public_fields_enum_and_status_values() {
        let schema = serde_json::json!({
            "properties": {
                "state_version": {"type": "integer"},
                "status": {"enum": ["queued", "complete"]}
            }
        });
        let mut identifiers = BTreeSet::new();
        collect_json_schema_identifiers(&schema, &mut identifiers);
        let catalog = ContractCatalog {
            sources: BTreeMap::from([(
                "public_api".to_owned(),
                owner_catalog("synthetic-public-schema.json", identifiers),
            )]),
        };
        let issues = validate_synthetic_pair(
            &catalog,
            &["public_api"],
            "# Result\n\n```yaml\nstate_version: 4\nstatus: queued\n```\n",
            "# 결과\n\n```yaml\nstatus: complete\n```\n",
        );

        assert_eq!(issues.len(), 2, "{issues:#?}");
        let messages = issues
            .iter()
            .map(ValidationIssue::message)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(messages.contains("`state_version`"));
        assert!(messages.contains("`queued`"));
        assert!(messages.contains("`complete`"));
        assert!(messages.contains("synthetic-public-schema.json"));
    }

    #[test]
    fn paired_shell_fixture_checks_cli_identifiers() {
        let catalog = ContractCatalog {
            sources: BTreeMap::from([(
                "administrative_cli".to_owned(),
                owner_catalog(
                    "synthetic-command-model",
                    ["volicord status", "--repo", "workflow"]
                        .into_iter()
                        .map(str::to_owned)
                        .collect(),
                ),
            )]),
        };
        let issues = validate_synthetic_pair(
            &catalog,
            &["administrative_cli"],
            "# Command\n\n```sh\nvolicord status --repo /tmp/product\n```\n",
            "# 명령\n\n```sh\nvolicord status\n```\n",
        );

        assert_eq!(issues.len(), 1, "{issues:#?}");
        assert!(issues[0].message().contains("`--repo`"));
    }

    #[test]
    fn paired_fixture_checks_typed_diagnostic_codes() {
        let catalog = ContractCatalog {
            sources: BTreeMap::from([(
                "diagnostics".to_owned(),
                owner_catalog(
                    "synthetic-diagnostic-registry.json",
                    ["mcp.protocol.unsupported_version".to_owned()]
                        .into_iter()
                        .collect(),
                ),
            )]),
        };
        let issues = validate_synthetic_pair(
            &catalog,
            &["diagnostics"],
            "# Failure\n\n`mcp.protocol.unsupported_version`\n",
            "# 실패\n\n진단 코드가 누락되었습니다.\n",
        );

        assert_eq!(issues.len(), 1, "{issues:#?}");
        assert!(issues[0]
            .message()
            .contains("mcp.protocol.unsupported_version"));
    }

    #[test]
    fn paired_fixture_checks_protocol_capability_fields() {
        let catalog = ContractCatalog {
            sources: BTreeMap::from([(
                "mcp_protocol".to_owned(),
                owner_catalog(
                    "synthetic-protocol-registry",
                    ["inputSchema", "isError"]
                        .into_iter()
                        .map(str::to_owned)
                        .collect(),
                ),
            )]),
        };
        let issues = validate_synthetic_pair(
            &catalog,
            &["mcp_protocol"],
            "# Protocol\n\n`inputSchema` and `isError` are wire fields.\n",
            "# 프로토콜\n\n`inputSchema`는 wire 필드입니다.\n",
        );

        assert_eq!(issues.len(), 1, "{issues:#?}");
        assert!(issues[0].message().contains("`isError`"));
    }

    #[test]
    fn misspelled_contract_identifiers_are_rejected_but_unrelated_spans_are_ignored() {
        let catalog = ContractCatalog {
            sources: BTreeMap::from([(
                "public_api".to_owned(),
                owner_catalog(
                    "synthetic-public-schema.json",
                    ["accepted_for_close".to_owned()].into_iter().collect(),
                ),
            )]),
        };
        let invalid = validate_synthetic_pair(
            &catalog,
            &["public_api"],
            "# Result\n\n`accepted_for_clsoe`\n",
            "# 결과\n\n`accepted_for_clsoe`\n",
        );
        let unrelated = validate_synthetic_pair(
            &catalog,
            &["public_api"],
            "# Result\n\n`local_note`\n",
            "# 결과\n\n`local_value`\n",
        );

        assert_eq!(invalid.len(), 2, "{invalid:#?}");
        assert!(invalid
            .iter()
            .all(|issue| issue.category() == "contract_identifier.invalid"));
        assert!(unrelated.is_empty(), "{unrelated:#?}");
    }

    #[test]
    fn multiple_contract_sources_produce_deterministic_focused_diagnostics() {
        let catalog = ContractCatalog {
            sources: BTreeMap::from([
                (
                    "diagnostics".to_owned(),
                    owner_catalog(
                        "synthetic-diagnostic-registry.json",
                        ["store.sqlite.busy".to_owned()].into_iter().collect(),
                    ),
                ),
                (
                    "public_api".to_owned(),
                    owner_catalog(
                        "synthetic-public-schema.json",
                        ["state_version".to_owned()].into_iter().collect(),
                    ),
                ),
            ]),
        };
        let english = "# Result\n\n`state_version` and `store.sqlite.busy` are reported.\n";
        let korean = "# 결과\n\n현재 결과입니다.\n";
        let first =
            validate_synthetic_pair(&catalog, &["diagnostics", "public_api"], english, korean);
        let second =
            validate_synthetic_pair(&catalog, &["diagnostics", "public_api"], english, korean);

        assert_eq!(first, second);
        assert_eq!(first.len(), 1, "{first:#?}");
        let message = first[0].message();
        assert!(message.contains("synthetic.pair"));
        assert!(message.contains("section 1"));
        assert!(message.contains("diagnostics"));
        assert!(message.contains("public_api"));
        assert!(message.contains("`state_version`"));
        assert!(message.contains("`store.sqlite.busy`"));
    }
}
