use crate::cli_docs;
use crate::diagnostics::ValidationIssue;
use crate::doc_index::{ContractSource, DocIndex, PairedDocument};
use crate::markdown::{
    self, MarkdownLiteral, MarkdownLiteralKind, MarkdownOwnerRegion, MarkdownStructure,
    MarkdownUnit, MeaningUnitKey,
};
use schemars::schema_for;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use volicord_types::methods::{public_request_schema, public_response_schema};
use volicord_types::values::{MethodName, OperationCategory};

const VALUE_SET_DOC_ID: &str = "reference.api.schema-value-sets";
const ADMIN_CLI_DOC_ID: &str = "reference.admin-cli";
const ADMIN_CLI_SOURCE_ID: &str = "administrative_cli";
const OPERATION_CATEGORY_ANCHOR: &str = "operation-category-values";
const OPERATION_CATEGORY_OWNER_PATH: &str = "crates/volicord-types/src/values.rs";

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum ContractSourceKind {
    PublicJsonSchemas,
    CommandModel,
    DiagnosticRegistry,
    ProtocolRegistry,
}

#[derive(Debug, Clone, Default)]
struct ContractCatalog {
    sources: BTreeMap<String, OwnerCatalog>,
}

#[derive(Debug, Clone)]
struct OwnerCatalog {
    owner: String,
    kind: ContractSourceKind,
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
        let (kind, identifiers) = match source.kind.as_str() {
            "public_json_schemas" => (
                ContractSourceKind::PublicJsonSchemas,
                public_api_identifiers(issues),
            ),
            "command_model" => (
                ContractSourceKind::CommandModel,
                volicord_command_model::public_contract_identifiers(),
            ),
            "diagnostic_registry" => (
                ContractSourceKind::DiagnosticRegistry,
                diagnostic_registry_identifiers(root, source, issues),
            ),
            "protocol_registry" => (
                ContractSourceKind::ProtocolRegistry,
                volicord_mcp_protocol::public_protocol_identifiers(),
            ),
            unsupported => {
                issues.push(ValidationIssue::new(
                    "docs/doc-index.yaml",
                    "contract_identifier.source",
                    format!(
                        "contract source {} has unsupported kind {unsupported}",
                        source.id
                    ),
                ));
                continue;
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
                kind,
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
                    "properties" | "definitions" | "$defs" => {
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
                                .filter(|identifier| !identifier.is_empty())
                                .map(str::to_owned),
                        );
                    }
                    "const" | "title" => {
                        if let Some(identifier) =
                            value.as_str().filter(|identifier| !identifier.is_empty())
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
    let en = match read_structure(root, paired, &paired.path_en) {
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
    let ko = match read_structure(root, paired, &paired.path_ko) {
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

    validate_invalid_identifiers(paired, &en, catalog, &paired.path_en, issues);
    validate_invalid_identifiers(paired, &ko, catalog, &paired.path_ko, issues);

    let en_units = units_by_key(&en);
    let ko_units = units_by_key(&ko);
    let keys = en_units
        .keys()
        .chain(ko_units.keys())
        .cloned()
        .collect::<BTreeSet<_>>();

    for key in keys {
        let en_unit = en_units.get(&key).copied();
        let ko_unit = ko_units.get(&key).copied();
        for source_id in &paired.contract_sources {
            let Some(source) = catalog.sources.get(source_id) else {
                continue;
            };
            let en_identifiers = en_unit
                .map(|unit| unit_identifiers(unit, source_id, source))
                .unwrap_or_default();
            let ko_identifiers = ko_unit
                .map(|unit| unit_identifiers(unit, source_id, source))
                .unwrap_or_default();
            report_missing_identifiers(
                paired,
                &key,
                en_unit,
                ko_unit,
                &en_identifiers
                    .difference(&ko_identifiers)
                    .cloned()
                    .collect(),
                "Korean",
                &paired.path_ko,
                source_id,
                source,
                &en,
                &ko,
                issues,
            );
            report_missing_identifiers(
                paired,
                &key,
                en_unit,
                ko_unit,
                &ko_identifiers
                    .difference(&en_identifiers)
                    .cloned()
                    .collect(),
                "English",
                &paired.path_en,
                source_id,
                source,
                &en,
                &ko,
                issues,
            );
        }
    }
}

fn read_structure(
    root: &Path,
    paired: &PairedDocument,
    relative_path: &str,
) -> Result<MarkdownStructure, String> {
    let contents = fs::read_to_string(root.join(relative_path))
        .map_err(|error| format!("failed to read paired Markdown: {error}"))?;
    let owner_regions = if paired.doc_id == ADMIN_CLI_DOC_ID
        && paired.contract_sources.contains(ADMIN_CLI_SOURCE_ID)
    {
        cli_docs::generated_region_range(&contents)
            .ok()
            .map(|range| {
                vec![MarkdownOwnerRegion {
                    range,
                    source_id: ADMIN_CLI_SOURCE_ID.to_owned(),
                }]
            })
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    Ok(markdown::structure(&contents, &owner_regions))
}

fn units_by_key(structure: &MarkdownStructure) -> BTreeMap<MeaningUnitKey, &MarkdownUnit> {
    structure
        .units()
        .map(|unit| (unit.key.clone(), unit))
        .collect()
}

fn unit_identifiers(
    unit: &MarkdownUnit,
    source_id: &str,
    source: &OwnerCatalog,
) -> BTreeSet<String> {
    if unit
        .owner_source
        .as_deref()
        .is_some_and(|owner| owner != source_id)
    {
        return BTreeSet::new();
    }
    unit.literals
        .iter()
        .flat_map(|literal| literal_identifiers(literal, unit, source_id, source))
        .collect()
}

fn literal_identifiers(
    literal: &MarkdownLiteral,
    unit: &MarkdownUnit,
    source_id: &str,
    source: &OwnerCatalog,
) -> BTreeSet<String> {
    if declared_contract_source(literal).is_some_and(|declared| declared != source_id) {
        return BTreeSet::new();
    }
    match literal.kind {
        MarkdownLiteralKind::Inline => {
            exact_identifier_mentions(&literal.text, &source.identifiers)
        }
        MarkdownLiteralKind::Fenced => match literal.language.as_deref() {
            Some("json" | "yaml" | "yml") => {
                let Ok(value) = serde_yaml::from_str::<serde_yaml::Value>(&literal.text) else {
                    return BTreeSet::new();
                };
                let mut tokens = BTreeSet::new();
                collect_structured_tokens(&value, &mut tokens);
                tokens.intersection(&source.identifiers).cloned().collect()
            }
            Some("bash" | "console" | "sh" | "shell" | "zsh") => {
                exact_identifier_mentions(&literal.text, &source.identifiers)
            }
            Some("text") if unit.owner_source.is_some() => {
                exact_identifier_mentions(&literal.text, &source.identifiers)
            }
            _ => BTreeSet::new(),
        },
    }
}

fn exact_identifier_mentions(
    literal: &str,
    exact_identifiers: &BTreeSet<String>,
) -> BTreeSet<String> {
    exact_identifiers
        .iter()
        .filter(|identifier| markdown::contains_exact_identifier(literal, identifier))
        .cloned()
        .collect()
}

fn collect_structured_tokens(value: &serde_yaml::Value, tokens: &mut BTreeSet<String>) {
    match value {
        serde_yaml::Value::Mapping(mapping) => {
            for (key, value) in mapping {
                if let Some(key) = key.as_str() {
                    tokens.insert(normalize_structured_key(key).to_owned());
                }
                collect_structured_tokens(value, tokens);
            }
        }
        serde_yaml::Value::Sequence(sequence) => {
            for value in sequence {
                collect_structured_tokens(value, tokens);
            }
        }
        serde_yaml::Value::String(value) => {
            tokens.insert(value.to_owned());
        }
        _ => {}
    }
}

fn normalize_structured_key(key: &str) -> &str {
    key.strip_suffix('?').unwrap_or(key)
}

#[allow(clippy::too_many_arguments)]
fn report_missing_identifiers(
    paired: &PairedDocument,
    key: &MeaningUnitKey,
    en_unit: Option<&MarkdownUnit>,
    ko_unit: Option<&MarkdownUnit>,
    missing: &BTreeSet<String>,
    language: &str,
    path: &str,
    source_id: &str,
    source: &OwnerCatalog,
    en: &MarkdownStructure,
    ko: &MarkdownStructure,
    issues: &mut Vec<ValidationIssue>,
) {
    if missing.is_empty() {
        return;
    }
    let en_line = en_unit.map_or_else(
        || en.line_for_heading_path(&key.heading_path),
        |unit| unit.line,
    );
    let ko_line = ko_unit.map_or_else(
        || ko.line_for_heading_path(&key.heading_path),
        |unit| unit.line,
    );
    let issue_line = if language == "Korean" {
        ko_line
    } else {
        en_line
    };
    issues.push(ValidationIssue::at_line(
        path,
        "contract_identifier.missing",
        Some(issue_line),
        format!(
            "document pair {} ({} <-> {}), structural unit `{key}` (English line {en_line}, Korean line {ko_line}), contract source {source_id} ({}): {language} meaning unit is missing {}",
            paired.doc_id,
            paired.path_en,
            paired.path_ko,
            source.owner,
            format_identifiers(missing),
        ),
    ));
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum CandidateKind {
    OwnedInline,
    StructuredKey,
}

#[derive(Debug)]
struct ContractCandidate {
    value: String,
    line: usize,
    kind: CandidateKind,
    declared_source: Option<String>,
}

fn validate_invalid_identifiers(
    paired: &PairedDocument,
    structure: &MarkdownStructure,
    catalog: &ContractCatalog,
    path: &str,
    issues: &mut Vec<ValidationIssue>,
) {
    for unit in structure.units() {
        validate_declared_sources(paired, unit, catalog, path, issues);
        for candidate in unit_candidates(unit) {
            let source_ids = candidate_sources(paired, &candidate, catalog);
            for source_id in source_ids {
                let Some(source) = catalog.sources.get(&source_id) else {
                    continue;
                };
                if source.identifiers.contains(&candidate.value) {
                    continue;
                }
                let suggestions = nearest_identifiers(&candidate.value, &source.identifiers);
                let suggestion = if suggestions.is_empty() {
                    String::new()
                } else {
                    format!(
                        "; nearest current identifier(s): {}",
                        format_identifiers(&suggestions)
                    )
                };
                issues.push(ValidationIssue::at_line(
                    path,
                    "contract_identifier.invalid",
                    Some(candidate.line),
                    format!(
                        "document pair {} ({} <-> {}), structural unit `{}`, contract source {source_id} ({}): contract-bound identifier `{}` does not exist in the current owner catalog{suggestion}",
                        paired.doc_id,
                        paired.path_en,
                        paired.path_ko,
                        unit.key,
                        source.owner,
                        candidate.value,
                    ),
                ));
            }
        }
    }
}

fn validate_declared_sources(
    paired: &PairedDocument,
    unit: &MarkdownUnit,
    catalog: &ContractCatalog,
    path: &str,
    issues: &mut Vec<ValidationIssue>,
) {
    let source_ids = unit
        .owner_source
        .iter()
        .map(String::as_str)
        .chain(unit.literals.iter().filter_map(declared_contract_source))
        .collect::<BTreeSet<_>>();
    for source_id in source_ids {
        if paired.contract_sources.contains(source_id) && catalog.sources.contains_key(source_id) {
            continue;
        }
        issues.push(ValidationIssue::at_line(
            path,
            "contract_identifier.source",
            Some(unit.line),
            format!(
                "document pair {} ({} <-> {}), structural unit `{}`: declared contract source `{source_id}` is not an applicable current owner",
                paired.doc_id, paired.path_en, paired.path_ko, unit.key,
            ),
        ));
    }
}

fn unit_candidates(unit: &MarkdownUnit) -> Vec<ContractCandidate> {
    let mut candidates = Vec::new();
    for literal in &unit.literals {
        match (literal.kind, literal.language.as_deref()) {
            (MarkdownLiteralKind::Fenced, Some("json" | "yaml" | "yml")) => {
                let Some(source_id) = declared_contract_source(literal) else {
                    continue;
                };
                let Ok(value) = serde_yaml::from_str::<serde_yaml::Value>(&literal.text) else {
                    continue;
                };
                let mut keys = BTreeSet::new();
                collect_structured_keys(&value, &mut keys);
                candidates.extend(keys.into_iter().map(|value| ContractCandidate {
                    value,
                    line: literal.line,
                    kind: CandidateKind::StructuredKey,
                    declared_source: Some(source_id.to_owned()),
                }));
            }
            (MarkdownLiteralKind::Inline, None) => {
                let Some(source_id) = unit.owner_source.as_deref() else {
                    continue;
                };
                if looks_like_identifier(&literal.text) {
                    candidates.push(ContractCandidate {
                        value: literal.text.clone(),
                        line: literal.line,
                        kind: CandidateKind::OwnedInline,
                        declared_source: Some(source_id.to_owned()),
                    });
                }
            }
            _ => {}
        }
    }
    candidates
}

fn collect_structured_keys(value: &serde_yaml::Value, keys: &mut BTreeSet<String>) {
    match value {
        serde_yaml::Value::Mapping(mapping) => {
            for (key, value) in mapping {
                if let Some(key) = key.as_str() {
                    keys.insert(normalize_structured_key(key).to_owned());
                }
                collect_structured_keys(value, keys);
            }
        }
        serde_yaml::Value::Sequence(sequence) => {
            for value in sequence {
                collect_structured_keys(value, keys);
            }
        }
        _ => {}
    }
}

fn candidate_sources(
    paired: &PairedDocument,
    candidate: &ContractCandidate,
    catalog: &ContractCatalog,
) -> BTreeSet<String> {
    let sources = candidate
        .declared_source
        .as_ref()
        .filter(|source_id| paired.contract_sources.contains(*source_id))
        .cloned()
        .into_iter()
        .collect::<BTreeSet<_>>();
    match candidate.kind {
        CandidateKind::OwnedInline => sources,
        CandidateKind::StructuredKey => sources
            .into_iter()
            .filter(|source_id| {
                catalog.sources.get(source_id).is_some_and(|source| {
                    matches!(
                        source.kind,
                        ContractSourceKind::PublicJsonSchemas
                            | ContractSourceKind::ProtocolRegistry
                    )
                })
            })
            .collect(),
    }
}

fn declared_contract_source(literal: &MarkdownLiteral) -> Option<&str> {
    literal
        .attributes
        .iter()
        .find_map(|attribute| attribute.strip_prefix("contract="))
}

fn nearest_identifiers(candidate: &str, identifiers: &BTreeSet<String>) -> BTreeSet<String> {
    let mut matches = identifiers
        .iter()
        .filter(|identifier| candidate.len().abs_diff(identifier.len()) <= 2)
        .filter_map(|identifier| {
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

    fn owner_catalog(owner: &str, kind: ContractSourceKind, identifiers: &[&str]) -> OwnerCatalog {
        OwnerCatalog {
            owner: owner.to_owned(),
            kind,
            identifiers: identifiers
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
        }
    }

    fn catalog(entries: &[(&str, OwnerCatalog)]) -> ContractCatalog {
        ContractCatalog {
            sources: entries
                .iter()
                .map(|(id, source)| ((*id).to_owned(), source.clone()))
                .collect(),
        }
    }

    fn synthetic_pair(
        doc_id: &str,
        sources: &[&str],
        english: &str,
        korean: &str,
    ) -> (TempDir, PairedDocument) {
        let root = tempfile::tempdir().expect("synthetic Markdown fixture root");
        fs::write(root.path().join("en.md"), english).expect("write English fixture");
        fs::write(root.path().join("ko.md"), korean).expect("write Korean fixture");
        (
            root,
            PairedDocument {
                doc_id: doc_id.to_owned(),
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
        validate_synthetic_pair_with_id("synthetic.pair", catalog, sources, english, korean)
    }

    fn validate_synthetic_pair_with_id(
        doc_id: &str,
        catalog: &ContractCatalog,
        sources: &[&str],
        english: &str,
        korean: &str,
    ) -> Vec<ValidationIssue> {
        let (root, pair) = synthetic_pair(doc_id, sources, english, korean);
        let mut issues = Vec::new();
        validate_pair(root.path(), &pair, catalog, &mut issues);
        issues.sort();
        issues
    }

    #[test]
    fn public_schema_extractor_includes_nested_properties_and_all_literal_value_forms() {
        let schema = serde_json::json!({
            "title": "SyntheticEnvelope",
            "properties": {
                "display_mode": {"enum": ["ready", "blocked"]},
                "status": {"const": "record"}
            },
            "$defs": {
                "SyntheticState": {
                    "properties": {"state_version": {"type": "integer"}}
                }
            }
        });
        let mut identifiers = BTreeSet::new();
        collect_json_schema_identifiers(&schema, &mut identifiers);

        assert_eq!(
            identifiers,
            [
                "SyntheticEnvelope",
                "SyntheticState",
                "blocked",
                "display_mode",
                "ready",
                "record",
                "state_version",
                "status",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect()
        );
    }

    #[test]
    fn simple_lowercase_and_snake_case_schema_identifiers_match_in_one_paragraph() {
        let catalog = catalog(&[(
            "public_api",
            owner_catalog(
                "synthetic-schema.json",
                ContractSourceKind::PublicJsonSchemas,
                &["ready", "display_mode"],
            ),
        )]);
        let issues = validate_synthetic_pair(
            &catalog,
            &["public_api"],
            "# State\n\n`display_mode` is `ready`.\n",
            "# 상태\n\n`display_mode`는 `ready`입니다.\n",
        );

        assert!(issues.is_empty(), "{issues:#?}");
    }

    #[test]
    fn moving_an_identifier_to_another_paragraph_in_the_same_section_fails() {
        let catalog = catalog(&[(
            "public_api",
            owner_catalog(
                "synthetic-schema.json",
                ContractSourceKind::PublicJsonSchemas,
                &["ready"],
            ),
        )]);
        let issues = validate_synthetic_pair(
            &catalog,
            &["public_api"],
            "# State\n\nThe state is `ready`.\n\nMore detail.\n",
            "# 상태\n\n상세 설명입니다.\n\n상태는 `ready`입니다.\n",
        );

        assert_eq!(issues.len(), 2, "{issues:#?}");
        assert!(issues
            .iter()
            .all(|issue| issue.category() == "contract_identifier.missing"));
    }

    #[test]
    fn hyphenated_cli_values_are_compared_exactly() {
        let catalog = catalog(&[(
            "administrative_cli",
            owner_catalog(
                "synthetic-command-model",
                ContractSourceKind::CommandModel,
                &["--output-mode"],
            ),
        )]);
        let issues = validate_synthetic_pair(
            &catalog,
            &["administrative_cli"],
            "# Command\n\nUse `--output-mode`.\n",
            "# 명령\n\n`--output-mode`를 사용합니다.\n",
        );

        assert!(issues.is_empty(), "{issues:#?}");
    }

    #[test]
    fn dotted_diagnostic_codes_are_compared_exactly() {
        let catalog = catalog(&[(
            "diagnostics",
            owner_catalog(
                "synthetic-diagnostic-registry.json",
                ContractSourceKind::DiagnosticRegistry,
                &["store.record.unavailable"],
            ),
        )]);
        let issues = validate_synthetic_pair(
            &catalog,
            &["diagnostics"],
            "# Failure\n\n`store.record.unavailable` is reported.\n",
            "# 실패\n\n`store.record.unavailable`을 보고합니다.\n",
        );

        assert!(issues.is_empty(), "{issues:#?}");
    }

    #[test]
    fn protocol_identifiers_are_compared_exactly() {
        let catalog = catalog(&[(
            "mcp_protocol",
            owner_catalog(
                "synthetic-protocol-registry",
                ContractSourceKind::ProtocolRegistry,
                &["inputSchema"],
            ),
        )]);
        let issues = validate_synthetic_pair(
            &catalog,
            &["mcp_protocol"],
            "# Protocol\n\n`inputSchema` is a wire field.\n",
            "# 프로토콜\n\n`inputSchema`는 wire 필드입니다.\n",
        );

        assert!(issues.is_empty(), "{issues:#?}");
    }

    #[test]
    fn list_items_match_by_nested_item_coordinate() {
        let catalog = catalog(&[(
            "public_api",
            owner_catalog(
                "synthetic-schema.json",
                ContractSourceKind::PublicJsonSchemas,
                &["ready", "blocked"],
            ),
        )]);
        let matching = validate_synthetic_pair(
            &catalog,
            &["public_api"],
            "# States\n\n- `ready`\n  - `blocked`\n",
            "# 상태\n\n- `ready`\n  - `blocked`\n",
        );
        let mismatching = validate_synthetic_pair(
            &catalog,
            &["public_api"],
            "# States\n\n- `ready`\n- `blocked`\n",
            "# 상태\n\n- `blocked`\n- `ready`\n",
        );

        assert!(matching.is_empty(), "{matching:#?}");
        assert_eq!(mismatching.len(), 4, "{mismatching:#?}");
    }

    #[test]
    fn table_identifiers_match_by_row_and_cell_coordinate() {
        let catalog = catalog(&[(
            "public_api",
            owner_catalog(
                "synthetic-schema.json",
                ContractSourceKind::PublicJsonSchemas,
                &["ready", "blocked"],
            ),
        )]);
        let matching = validate_synthetic_pair(
            &catalog,
            &["public_api"],
            "# States\n\n| State | Result |\n|---|---|\n| `ready` | `blocked` |\n",
            "# 상태\n\n| 상태 | 결과 |\n|---|---|\n| `ready` | `blocked` |\n",
        );
        let mismatching = validate_synthetic_pair(
            &catalog,
            &["public_api"],
            "# States\n\n| State | Result |\n|---|---|\n| `ready` | `blocked` |\n",
            "# 상태\n\n| 상태 | 결과 |\n|---|---|\n| `blocked` | `ready` |\n",
        );

        assert!(matching.is_empty(), "{matching:#?}");
        assert_eq!(mismatching.len(), 4, "{mismatching:#?}");
    }

    #[test]
    fn unrelated_inline_code_is_not_an_accidental_contract() {
        let catalog = catalog(&[(
            "public_api",
            owner_catalog(
                "synthetic-schema.json",
                ContractSourceKind::PublicJsonSchemas,
                &["ready", "status"],
            ),
        )]);
        let issues = validate_synthetic_pair(
            &catalog,
            &["public_api"],
            "# State\n\n`local_note` accompanies `ready`.\n",
            "# 상태\n\n`ready`에는 `local_value`가 함께 표시됩니다.\n",
        );

        assert!(issues.is_empty(), "{issues:#?}");
    }

    #[test]
    fn contract_bound_structured_key_must_exist_in_its_owner_catalog() {
        let catalog = catalog(&[
            (
                "public_api",
                owner_catalog(
                    "synthetic-schema.json",
                    ContractSourceKind::PublicJsonSchemas,
                    &["status", "ready"],
                ),
            ),
            (
                "mcp_protocol",
                owner_catalog(
                    "synthetic-protocol-registry",
                    ContractSourceKind::ProtocolRegistry,
                    &["unknown_field"],
                ),
            ),
        ]);
        let issues = validate_synthetic_pair(
            &catalog,
            &["mcp_protocol", "public_api"],
            "# State\n\n```yaml contract=public_api\nstatus: ready\nunknown_field: value\n```\n",
            "# 상태\n\n```yaml contract=public_api\nstatus: ready\nunknown_field: value\n```\n",
        );

        assert_eq!(issues.len(), 2, "{issues:#?}");
        assert!(issues
            .iter()
            .all(|issue| issue.category() == "contract_identifier.invalid"));
        assert!(issues
            .iter()
            .all(|issue| issue.message().contains("unknown_field")));
    }

    #[test]
    fn contract_bound_table_token_must_exist_in_its_owner_catalog() {
        let catalog = catalog(&[(
            "public_api",
            owner_catalog(
                "synthetic-schema.json",
                ContractSourceKind::PublicJsonSchemas,
                &["status", "ready"],
            ),
        )]);
        let english = "# State\n\n<!-- contract-source: public_api -->\n| Field | Value |\n|---|---|\n| `status` | `unknown_state` |\n";
        let korean = "# 상태\n\n<!-- contract-source: public_api -->\n| 필드 | 값 |\n|---|---|\n| `status` | `unknown_state` |\n";
        let issues = validate_synthetic_pair(&catalog, &["public_api"], english, korean);

        assert_eq!(issues.len(), 2, "{issues:#?}");
        assert!(issues
            .iter()
            .all(|issue| issue.category() == "contract_identifier.invalid"));
        assert!(issues
            .iter()
            .all(|issue| issue.message().contains("unknown_state")));
    }

    #[test]
    fn contract_source_metadata_must_select_an_applicable_current_owner() {
        let catalog = catalog(&[(
            "public_api",
            owner_catalog(
                "synthetic-schema.json",
                ContractSourceKind::PublicJsonSchemas,
                &["status"],
            ),
        )]);
        let english =
            "# State\n\n<!-- contract-source: unavailable_owner -->\n`status` is shown.\n";
        let korean =
            "# 상태\n\n<!-- contract-source: unavailable_owner -->\n`status`를 표시합니다.\n";
        let issues = validate_synthetic_pair(&catalog, &["public_api"], english, korean);

        assert_eq!(issues.len(), 2, "{issues:#?}");
        assert!(issues
            .iter()
            .all(|issue| issue.category() == "contract_identifier.source"));
    }

    #[test]
    fn diagnostics_are_deterministic_and_name_the_structural_unit_and_source() {
        let catalog = catalog(&[
            (
                "diagnostics",
                owner_catalog(
                    "synthetic-diagnostic-registry.json",
                    ContractSourceKind::DiagnosticRegistry,
                    &["store.record.unavailable"],
                ),
            ),
            (
                "public_api",
                owner_catalog(
                    "synthetic-schema.json",
                    ContractSourceKind::PublicJsonSchemas,
                    &["state_version"],
                ),
            ),
        ]);
        let english = "# Result\n\n`state_version` and `store.record.unavailable` are reported.\n";
        let korean = "# 결과\n\n현재 결과입니다.\n";
        let first =
            validate_synthetic_pair(&catalog, &["diagnostics", "public_api"], english, korean);
        let second =
            validate_synthetic_pair(&catalog, &["diagnostics", "public_api"], english, korean);

        assert_eq!(first, second);
        assert_eq!(first.len(), 2, "{first:#?}");
        assert!(first
            .iter()
            .all(|issue| issue.message().contains("structural unit")));
        assert!(first
            .iter()
            .all(|issue| issue.message().contains("contract source")));
    }

    #[test]
    fn generated_cli_regions_use_the_command_model_owner() {
        const BEGIN: &str = "<!-- BEGIN GENERATED: volicord-cli-synopses -->";
        const END: &str = "<!-- END GENERATED: volicord-cli-synopses -->";
        let catalog = catalog(&[(
            "administrative_cli",
            owner_catalog(
                "synthetic-command-model",
                ContractSourceKind::CommandModel,
                &["volicord inspect", "--report"],
            ),
        )]);
        let english = format!(
            "# CLI\n\n{BEGIN}\n### `volicord inspect`\n\n```text\nUsage: volicord inspect --report\n```\n{END}\n"
        );
        let korean = format!(
            "# CLI\n\n{BEGIN}\n### `volicord inspect`\n\n```text\nUsage: volicord inspect\n```\n{END}\n"
        );
        let issues = validate_synthetic_pair_with_id(
            ADMIN_CLI_DOC_ID,
            &catalog,
            &["administrative_cli"],
            &english,
            &korean,
        );

        assert_eq!(issues.len(), 1, "{issues:#?}");
        assert!(issues[0].message().contains("administrative_cli"));
        assert!(issues[0].message().contains("`--report`"));
    }
}
