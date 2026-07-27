use crate::diagnostics::ValidationIssue;
use crate::doc_index::{DocIndex, PairedDocument};
use crate::markdown::{
    self, MarkdownLiteral, MarkdownLiteralKind, MarkdownStructure, MarkdownUnit, MeaningUnitKey,
};
use schemars::schema_for;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use volicord_types::values::OperationCategory;

const VALUE_SET_DOC_ID: &str = "reference.api.schema-value-sets";
const OPERATION_CATEGORY_ANCHOR: &str = "operation-category-values";
const OPERATION_CATEGORY_OWNER_PATH: &str = "crates/volicord-types/src/values.rs";
const DIAGNOSTIC_DESCRIPTOR_PATH: &str =
    "crates/volicord-cli/tests/fixtures/diagnostic-registry.json";
const CLI_OUTPUT_DESCRIPTOR_PATH: &str =
    "crates/volicord-user-action-presentation/tests/fixtures/cli-output-contracts.json";

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum IdentifierCategory {
    ApiProperty,
    ApiValue,
    ApiSchema,
    CliSyntax,
    CliValue,
    CliOutputProperty,
    CliOutputValue,
    CliOutputSchema,
    DiagnosticCode,
    ProtocolIdentifier,
}

impl IdentifierCategory {
    const fn label(self) -> &'static str {
        match self {
            Self::ApiProperty => "API property",
            Self::ApiValue => "API value",
            Self::ApiSchema => "API schema name",
            Self::CliSyntax => "CLI syntax",
            Self::CliValue => "CLI value",
            Self::CliOutputProperty => "CLI output property",
            Self::CliOutputValue => "CLI output value",
            Self::CliOutputSchema => "CLI output schema name",
            Self::DiagnosticCode => "diagnostic code",
            Self::ProtocolIdentifier => "MCP protocol identifier",
        }
    }

    const fn is_structured_key(self) -> bool {
        matches!(
            self,
            Self::ApiProperty
                | Self::ApiSchema
                | Self::CliOutputProperty
                | Self::CliOutputSchema
                | Self::ProtocolIdentifier
        )
    }

    const fn is_structured_value(self) -> bool {
        matches!(
            self,
            Self::ApiValue | Self::CliOutputValue | Self::ProtocolIdentifier
        )
    }

    const fn domain(self) -> ContractDomain {
        match self {
            Self::ApiProperty | Self::ApiValue | Self::ApiSchema => ContractDomain::PublicApi,
            Self::CliSyntax | Self::CliValue => ContractDomain::CliCommand,
            Self::CliOutputProperty | Self::CliOutputValue | Self::CliOutputSchema => {
                ContractDomain::CliOutput
            }
            Self::DiagnosticCode => ContractDomain::Diagnostic,
            Self::ProtocolIdentifier => ContractDomain::Protocol,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum ContractDomain {
    PublicApi,
    CliCommand,
    CliOutput,
    Diagnostic,
    Protocol,
}

impl ContractDomain {
    const fn supports_automatic_structured_examples(self) -> bool {
        matches!(self, Self::PublicApi | Self::Protocol)
    }
}

#[derive(Debug, Clone)]
struct OwnerCatalog {
    owner: String,
    domain: ContractDomain,
    identifiers: BTreeMap<IdentifierCategory, BTreeSet<String>>,
    related_contracts: BTreeSet<String>,
    schema: Option<Value>,
}

impl OwnerCatalog {
    fn is_empty(&self) -> bool {
        self.identifiers.values().all(BTreeSet::is_empty)
    }
}

#[derive(Debug, Clone, Default)]
struct ContractCatalog {
    contracts: BTreeMap<String, OwnerCatalog>,
}

impl ContractCatalog {
    fn insert(
        &mut self,
        id: impl Into<String>,
        owner: OwnerCatalog,
        issues: &mut Vec<ValidationIssue>,
    ) {
        let id = id.into();
        if owner.is_empty() {
            issues.push(ValidationIssue::new(
                &owner.owner,
                "contract_identifier.owner",
                format!("semantic contract {id} has an empty identifier catalog"),
            ));
        }
        if self.contracts.insert(id.clone(), owner).is_some() {
            issues.push(ValidationIssue::new(
                "docs/doc-index.yaml",
                "contract_identifier.owner",
                format!("semantic contract {id} is exposed by more than one owner descriptor"),
            ));
        }
    }

    fn exact_matches(
        &self,
        value: &str,
        predicate: impl Fn(IdentifierCategory) -> bool + Copy,
    ) -> BTreeSet<(String, IdentifierCategory)> {
        self.contracts
            .iter()
            .flat_map(|(contract, owner)| {
                owner
                    .identifiers
                    .iter()
                    .filter_map(move |(category, values)| {
                        if predicate(*category) && values.contains(value) {
                            Some((contract.clone(), *category))
                        } else {
                            None
                        }
                    })
            })
            .collect()
    }

    fn all_identifiers(&self, predicate: impl Fn(IdentifierCategory) -> bool) -> BTreeSet<String> {
        self.contracts
            .values()
            .flat_map(|owner| {
                owner
                    .identifiers
                    .iter()
                    .filter(|(category, _)| predicate(**category))
                    .flat_map(|(_, values)| values.iter().cloned())
            })
            .collect()
    }

    fn scoped_identifiers(
        &self,
        scope: &BTreeSet<String>,
    ) -> BTreeSet<(IdentifierCategory, String)> {
        scope
            .iter()
            .filter_map(|contract| self.contracts.get(contract))
            .flat_map(|owner| {
                owner.identifiers.iter().flat_map(|(category, values)| {
                    values
                        .iter()
                        .cloned()
                        .map(|value| (*category, value))
                        .collect::<Vec<_>>()
                })
            })
            .collect()
    }
}

pub(crate) fn validate_contract_identifiers(
    root: &Path,
    index: &DocIndex,
    issues: &mut Vec<ValidationIssue>,
) {
    if index
        .paired_documents
        .values()
        .all(|paired| paired.contracts.is_empty())
    {
        return;
    }
    let catalog = load_contract_catalog(root, issues);
    validate_descriptor_relationships(&catalog, issues);
    validate_document_contract_scopes(index, &catalog, issues);

    for paired in index
        .paired_documents
        .values()
        .filter(|paired| !paired.contracts.is_empty())
    {
        validate_pair(root, paired, &catalog, issues);
    }
}

fn load_contract_catalog(root: &Path, issues: &mut Vec<ValidationIssue>) -> ContractCatalog {
    let mut catalog = ContractCatalog::default();
    load_public_api_descriptors(&mut catalog, issues);
    load_cli_descriptors(&mut catalog, issues);
    load_protocol_descriptors(&mut catalog, issues);
    load_wire_descriptors(&mut catalog, issues);
    load_json_descriptor_file(
        root,
        DIAGNOSTIC_DESCRIPTOR_PATH,
        ContractDomain::Diagnostic,
        &[
            ("codes", IdentifierCategory::DiagnosticCode),
            ("related_contracts", IdentifierCategory::DiagnosticCode),
        ],
        &mut catalog,
        issues,
    );
    load_json_descriptor_file(
        root,
        CLI_OUTPUT_DESCRIPTOR_PATH,
        ContractDomain::CliOutput,
        &[
            ("properties", IdentifierCategory::CliOutputProperty),
            ("values", IdentifierCategory::CliOutputValue),
            ("schema_names", IdentifierCategory::CliOutputSchema),
        ],
        &mut catalog,
        issues,
    );
    catalog
}

fn load_public_api_descriptors(catalog: &mut ContractCatalog, issues: &mut Vec<ValidationIssue>) {
    for descriptor in volicord_types::contracts::public_json_contract_descriptors() {
        let identifiers = descriptor.identifiers();
        catalog.insert(
            descriptor.id(),
            OwnerCatalog {
                owner: "crates/volicord-types/src/contracts.rs".to_owned(),
                domain: ContractDomain::PublicApi,
                identifiers: BTreeMap::from([
                    (
                        IdentifierCategory::ApiProperty,
                        identifiers.properties().clone(),
                    ),
                    (IdentifierCategory::ApiValue, identifiers.values().clone()),
                    (
                        IdentifierCategory::ApiSchema,
                        identifiers.schema_names().clone(),
                    ),
                ]),
                related_contracts: descriptor.related_contracts().iter().cloned().collect(),
                schema: descriptor.schema().cloned(),
            },
            issues,
        );
    }
}

fn load_cli_descriptors(catalog: &mut ContractCatalog, issues: &mut Vec<ValidationIssue>) {
    for descriptor in volicord_command_model::public_cli_contract_descriptors() {
        catalog.insert(
            descriptor.id(),
            OwnerCatalog {
                owner: "crates/volicord-command-model/src/lib.rs".to_owned(),
                domain: ContractDomain::CliCommand,
                identifiers: BTreeMap::from([
                    (IdentifierCategory::CliSyntax, descriptor.syntax().clone()),
                    (IdentifierCategory::CliValue, descriptor.values().clone()),
                ]),
                related_contracts: descriptor.related_contracts().iter().cloned().collect(),
                schema: None,
            },
            issues,
        );
    }
}

fn load_protocol_descriptors(catalog: &mut ContractCatalog, issues: &mut Vec<ValidationIssue>) {
    for descriptor in volicord_mcp_protocol::protocol_contract_descriptors() {
        catalog.insert(
            descriptor.id(),
            OwnerCatalog {
                owner: "crates/volicord-mcp-protocol/src/lib.rs".to_owned(),
                domain: ContractDomain::Protocol,
                identifiers: BTreeMap::from([(
                    IdentifierCategory::ProtocolIdentifier,
                    descriptor.identifiers().clone(),
                )]),
                related_contracts: descriptor
                    .related_contracts()
                    .iter()
                    .map(|related| (*related).to_owned())
                    .collect(),
                schema: None,
            },
            issues,
        );
    }
}

fn load_wire_descriptors(catalog: &mut ContractCatalog, issues: &mut Vec<ValidationIssue>) {
    for descriptor in volicord_mcp_wire::wire_contract_descriptors() {
        catalog.insert(
            descriptor.id(),
            OwnerCatalog {
                owner: "crates/volicord-mcp-wire/src/contracts.rs".to_owned(),
                domain: ContractDomain::Protocol,
                identifiers: BTreeMap::from([(
                    IdentifierCategory::ProtocolIdentifier,
                    descriptor.identifiers().clone(),
                )]),
                related_contracts: descriptor
                    .related_contracts()
                    .iter()
                    .map(|related| (*related).to_owned())
                    .collect(),
                schema: None,
            },
            issues,
        );
    }
}

fn load_json_descriptor_file(
    root: &Path,
    relative_path: &str,
    domain: ContractDomain,
    fields: &[(&str, IdentifierCategory)],
    catalog: &mut ContractCatalog,
    issues: &mut Vec<ValidationIssue>,
) {
    let contents = match fs::read_to_string(root.join(relative_path)) {
        Ok(contents) => contents,
        Err(error) => {
            issues.push(ValidationIssue::new(
                relative_path,
                "contract_identifier.owner",
                format!("failed to read semantic contract descriptors: {error}"),
            ));
            return;
        }
    };
    let value: Value = match serde_json::from_str(&contents) {
        Ok(value) => value,
        Err(error) => {
            issues.push(ValidationIssue::new(
                relative_path,
                "contract_identifier.owner",
                format!("failed to parse semantic contract descriptors: {error}"),
            ));
            return;
        }
    };
    let Some(descriptors) = value.get("contracts").and_then(Value::as_array) else {
        issues.push(ValidationIssue::new(
            relative_path,
            "contract_identifier.owner",
            "semantic descriptor artifact must contain a contracts array",
        ));
        return;
    };

    for descriptor in descriptors {
        let Some(object) = descriptor.as_object() else {
            issues.push(ValidationIssue::new(
                relative_path,
                "contract_identifier.owner",
                "semantic contract descriptor must be an object",
            ));
            continue;
        };
        let Some(id) = object.get("id").and_then(Value::as_str) else {
            issues.push(ValidationIssue::new(
                relative_path,
                "contract_identifier.owner",
                "semantic contract descriptor must have a string id",
            ));
            continue;
        };
        let mut identifiers = BTreeMap::new();
        for (field, category) in fields {
            if *field == "related_contracts" {
                continue;
            }
            identifiers.insert(
                *category,
                json_string_set(object.get(*field), relative_path, id, field, issues),
            );
        }
        let related_contracts = json_string_set(
            object.get("related_contracts"),
            relative_path,
            id,
            "related_contracts",
            issues,
        );
        catalog.insert(
            id,
            OwnerCatalog {
                owner: relative_path.to_owned(),
                domain,
                identifiers,
                related_contracts,
                schema: None,
            },
            issues,
        );
    }
}

fn json_string_set(
    value: Option<&Value>,
    path: &str,
    contract: &str,
    field: &str,
    issues: &mut Vec<ValidationIssue>,
) -> BTreeSet<String> {
    let Some(values) = value.and_then(Value::as_array) else {
        issues.push(ValidationIssue::new(
            path,
            "contract_identifier.owner",
            format!("semantic contract {contract} field {field} must be an array"),
        ));
        return BTreeSet::new();
    };
    let mut result = BTreeSet::new();
    for value in values {
        let Some(value) = value.as_str().filter(|value| looks_like_identifier(value)) else {
            issues.push(ValidationIssue::new(
                path,
                "contract_identifier.owner",
                format!(
                    "semantic contract {contract} field {field} must contain identifier strings"
                ),
            ));
            continue;
        };
        if !result.insert(value.to_owned()) {
            issues.push(ValidationIssue::new(
                path,
                "contract_identifier.owner",
                format!("semantic contract {contract} repeats {field} value {value}"),
            ));
        }
    }
    result
}

fn validate_descriptor_relationships(catalog: &ContractCatalog, issues: &mut Vec<ValidationIssue>) {
    for (id, owner) in &catalog.contracts {
        for related in &owner.related_contracts {
            if !catalog.contracts.contains_key(related) {
                issues.push(ValidationIssue::new(
                    &owner.owner,
                    "contract_identifier.relationship",
                    format!("semantic contract {id} relates to unknown contract {related}"),
                ));
            }
        }
    }
}

fn validate_document_contract_scopes(
    index: &DocIndex,
    catalog: &ContractCatalog,
    issues: &mut Vec<ValidationIssue>,
) {
    for paired in index.paired_documents.values() {
        for contract in &paired.contracts {
            if !catalog.contracts.contains_key(contract) {
                issues.push(ValidationIssue::new(
                    "docs/doc-index.yaml",
                    "contract_identifier.scope",
                    format!(
                        "document {} references unknown semantic contract {contract}",
                        paired.doc_id
                    ),
                ));
            }
        }
    }
}

fn validate_pair(
    root: &Path,
    paired: &PairedDocument,
    catalog: &ContractCatalog,
    issues: &mut Vec<ValidationIssue>,
) {
    let Some(en) = read_structure(root, &paired.path_en, issues) else {
        return;
    };
    let Some(ko) = read_structure(root, &paired.path_ko, issues) else {
        return;
    };

    let en_validation = validate_language(paired, &en, catalog, &paired.path_en, issues);
    let ko_validation = validate_language(paired, &ko, catalog, &paired.path_ko, issues);
    compare_valid_units(
        paired,
        &en,
        &ko,
        &en_validation,
        &ko_validation,
        catalog,
        issues,
    );
}

fn read_structure(
    root: &Path,
    relative_path: &str,
    issues: &mut Vec<ValidationIssue>,
) -> Option<MarkdownStructure> {
    match fs::read_to_string(root.join(relative_path)) {
        Ok(contents) => Some(markdown::structure(&contents, &[])),
        Err(error) => {
            issues.push(ValidationIssue::new(
                relative_path,
                "contract_identifier.read",
                format!("failed to read paired Markdown: {error}"),
            ));
            None
        }
    }
}

#[derive(Debug, Default)]
struct LanguageValidation {
    valid_units: BTreeSet<MeaningUnitKey>,
    identifiers: BTreeMap<MeaningUnitKey, BTreeSet<(IdentifierCategory, String)>>,
}

fn validate_language(
    paired: &PairedDocument,
    structure: &MarkdownStructure,
    catalog: &ContractCatalog,
    path: &str,
    issues: &mut Vec<ValidationIssue>,
) -> LanguageValidation {
    let scoped_identifiers = catalog.scoped_identifiers(&paired.contracts);
    let mut result = LanguageValidation::default();
    for unit in structure.units() {
        let issue_count = issues.len();
        validate_declared_contracts(paired, unit, catalog, path, issues);
        validate_unit_candidates(paired, unit, catalog, path, issues);
        if issues.len() == issue_count {
            result.valid_units.insert(unit.key.clone());
            result.identifiers.insert(
                unit.key.clone(),
                unit_identifiers(unit, &scoped_identifiers),
            );
        }
    }
    result
}

fn validate_declared_contracts(
    paired: &PairedDocument,
    unit: &MarkdownUnit,
    catalog: &ContractCatalog,
    path: &str,
    issues: &mut Vec<ValidationIssue>,
) {
    for contract in unit.literals.iter().filter_map(declared_contract) {
        if !paired.contracts.contains(contract) || !catalog.contracts.contains_key(contract) {
            issues.push(ValidationIssue::at_line(
                path,
                "contract_identifier.scope",
                Some(unit.line),
                format!(
                    "document {} structural unit `{}` declares contract {contract}, which is outside its exact document scope",
                    paired.doc_id, unit.key
                ),
            ));
        } else if paired.contracts.len() < 2 {
            issues.push(ValidationIssue::at_line(
                path,
                "contract_identifier.scope",
                Some(unit.line),
                format!(
                    "document {} structural unit `{}` declares contract {contract}, but fence-level contract selection is only valid in a document with multiple independent contracts",
                    paired.doc_id, unit.key
                ),
            ));
        }
    }
}

fn validate_unit_candidates(
    paired: &PairedDocument,
    unit: &MarkdownUnit,
    catalog: &ContractCatalog,
    path: &str,
    issues: &mut Vec<ValidationIssue>,
) {
    for literal in &unit.literals {
        match (literal.kind, literal.language.as_deref()) {
            (MarkdownLiteralKind::Inline, None) => {
                validate_inline_literal(paired, unit, literal, catalog, path, issues);
            }
            (MarkdownLiteralKind::Fenced, Some("json" | "yaml" | "yml")) => {
                validate_structured_literal(paired, unit, literal, catalog, path, issues);
            }
            (MarkdownLiteralKind::Fenced, Some("bash" | "console" | "sh" | "shell" | "zsh")) => {
                validate_exact_occurrences(paired, unit, literal, catalog, path, |_| true, issues);
            }
            (MarkdownLiteralKind::Fenced, Some("text")) => {
                validate_exact_occurrences(
                    paired,
                    unit,
                    literal,
                    catalog,
                    path,
                    |category| {
                        matches!(
                            category,
                            IdentifierCategory::CliSyntax
                                | IdentifierCategory::CliValue
                                | IdentifierCategory::DiagnosticCode
                                | IdentifierCategory::ProtocolIdentifier
                        )
                    },
                    issues,
                );
            }
            _ => {}
        }
    }
}

fn validate_inline_literal(
    paired: &PairedDocument,
    unit: &MarkdownUnit,
    literal: &MarkdownLiteral,
    catalog: &ContractCatalog,
    path: &str,
    issues: &mut Vec<ValidationIssue>,
) {
    if unrelated_code_literal(&literal.text) {
        return;
    }
    validate_exact_occurrences(paired, unit, literal, catalog, path, |_| true, issues);

    let domains = document_domains(paired, catalog);
    if looks_like_identifier(&literal.text)
        && literal
            .text
            .chars()
            .any(|character| matches!(character, '_' | '.'))
        && literal
            .text
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_lowercase())
        && catalog.exact_matches(&literal.text, |_| true).is_empty()
        && !is_plural_variant_of_catalog_identifier(
            &literal.text,
            &catalog.all_identifiers(|category| {
                domains.contains(&category.domain())
                    && !matches!(
                        category,
                        IdentifierCategory::CliSyntax | IdentifierCategory::CliValue
                    )
            }),
        )
    {
        let suggestions = nearest_identifiers(
            &literal.text,
            &catalog.all_identifiers(|category| {
                domains.contains(&category.domain())
                    && !matches!(
                        category,
                        IdentifierCategory::CliSyntax | IdentifierCategory::CliValue
                    )
            }),
        )
        .into_iter()
        .filter(|identifier| edit_distance(&literal.text, identifier) == 1)
        .collect::<BTreeSet<_>>();
        if !suggestions.is_empty() {
            report_invalid(
                paired,
                unit,
                literal.line,
                &literal.text,
                &suggestions,
                path,
                issues,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn validate_exact_occurrences(
    paired: &PairedDocument,
    unit: &MarkdownUnit,
    literal: &MarkdownLiteral,
    catalog: &ContractCatalog,
    path: &str,
    predicate: impl Fn(IdentifierCategory) -> bool + Copy,
    issues: &mut Vec<ValidationIssue>,
) {
    let domains = document_domains(paired, catalog);
    let predicate =
        |category: IdentifierCategory| domains.contains(&category.domain()) && predicate(category);
    let all = catalog.all_identifiers(predicate);
    let matched = all
        .iter()
        .filter(|identifier| markdown::contains_exact_identifier(&literal.text, identifier))
        .filter(|identifier| {
            !is_qualified_source_member(&literal.text, identifier)
                || catalog
                    .exact_matches(identifier, predicate)
                    .iter()
                    .any(|(_, category)| {
                        matches!(
                            category,
                            IdentifierCategory::DiagnosticCode
                                | IdentifierCategory::ProtocolIdentifier
                        )
                    })
        })
        .collect::<Vec<_>>();
    for value in matched.iter().copied().filter(|value| {
        !matched.iter().any(|other| {
            other.len() > value.len() && markdown::contains_exact_identifier(other, value)
        })
    }) {
        validate_exact_candidate(
            paired,
            unit,
            literal.line,
            value,
            catalog,
            path,
            predicate,
            issues,
        );
    }
}

fn validate_structured_literal(
    paired: &PairedDocument,
    unit: &MarkdownUnit,
    literal: &MarkdownLiteral,
    catalog: &ContractCatalog,
    path: &str,
    issues: &mut Vec<ValidationIssue>,
) {
    let selected = structured_scope(paired, literal, catalog);
    if selected.is_empty() {
        return;
    }
    let Ok(value) = serde_yaml::from_str::<serde_yaml::Value>(&literal.text) else {
        return;
    };
    let mut keys = BTreeSet::new();
    let mut values = BTreeSet::new();
    collect_structured_tokens(&value, &mut Vec::new(), &mut keys, &mut values);

    for key in keys {
        validate_structured_candidate(
            paired,
            unit,
            literal.line,
            &key.value,
            &key.path,
            &selected,
            catalog,
            path,
            true,
            issues,
        );
    }
    for value in values {
        let matches = catalog.exact_matches(&value, IdentifierCategory::is_structured_value);
        if !matches.is_empty() {
            validate_structured_candidate(
                paired,
                unit,
                literal.line,
                &value,
                &[],
                &selected,
                catalog,
                path,
                false,
                issues,
            );
        }
    }
}

fn structured_scope(
    paired: &PairedDocument,
    literal: &MarkdownLiteral,
    catalog: &ContractCatalog,
) -> BTreeSet<String> {
    if let Some(contract) = declared_contract(literal) {
        return paired
            .contracts
            .contains(contract)
            .then(|| contract.to_owned())
            .into_iter()
            .collect();
    }
    let owners = paired
        .contracts
        .iter()
        .filter_map(|contract| {
            catalog
                .contracts
                .get(contract)
                .map(|owner| (contract, owner))
        })
        .collect::<Vec<_>>();
    if owners
        .iter()
        .all(|(_, owner)| owner.domain.supports_automatic_structured_examples())
    {
        owners
            .into_iter()
            .map(|(contract, _)| contract.clone())
            .collect()
    } else {
        BTreeSet::new()
    }
}

#[allow(clippy::too_many_arguments)]
fn validate_structured_candidate(
    paired: &PairedDocument,
    unit: &MarkdownUnit,
    line: usize,
    value: &str,
    path_to_key: &[String],
    selected: &BTreeSet<String>,
    catalog: &ContractCatalog,
    path: &str,
    key: bool,
    issues: &mut Vec<ValidationIssue>,
) {
    let selected_domains = selected
        .iter()
        .filter_map(|contract| catalog.contracts.get(contract))
        .map(|owner| owner.domain)
        .collect::<BTreeSet<_>>();
    let predicate = |category: IdentifierCategory| {
        selected_domains.contains(&category.domain())
            && if key {
                category.is_structured_key()
            } else {
                category.is_structured_value()
            }
    };
    let selected_owners = selected
        .iter()
        .filter_map(|contract| catalog.contracts.get(contract))
        .collect::<Vec<_>>();
    if key && selected_owners.iter().any(|owner| owner.schema.is_some()) {
        if selected_owners.iter().any(|owner| {
            owner
                .schema
                .as_ref()
                .is_some_and(|schema| schema_accepts_key(schema, path_to_key, value))
                || owner.schema.is_none()
                    && owner
                        .identifiers
                        .get(&IdentifierCategory::ApiProperty)
                        .is_some_and(|identifiers| identifiers.contains(value))
        }) {
            return;
        }
    } else {
        let exact = catalog.exact_matches(value, predicate);
        if exact
            .iter()
            .any(|(contract, _)| selected.contains(contract))
        {
            return;
        }
    }
    let exact = catalog.exact_matches(value, predicate);
    if !exact.is_empty() {
        report_out_of_scope(paired, unit, line, value, &exact, path, issues);
        return;
    }
    if key {
        let suggestions = nearest_identifiers(value, &catalog.all_identifiers(predicate));
        report_invalid(paired, unit, line, value, &suggestions, path, issues);
    }
}

#[allow(clippy::too_many_arguments)]
fn validate_exact_candidate(
    paired: &PairedDocument,
    unit: &MarkdownUnit,
    line: usize,
    value: &str,
    catalog: &ContractCatalog,
    path: &str,
    predicate: impl Fn(IdentifierCategory) -> bool + Copy,
    issues: &mut Vec<ValidationIssue>,
) {
    let matches = catalog.exact_matches(value, predicate);
    if matches
        .iter()
        .any(|(contract, _)| paired.contracts.contains(contract))
    {
        return;
    }
    report_out_of_scope(paired, unit, line, value, &matches, path, issues);
}

fn report_out_of_scope(
    paired: &PairedDocument,
    unit: &MarkdownUnit,
    line: usize,
    value: &str,
    matches: &BTreeSet<(String, IdentifierCategory)>,
    path: &str,
    issues: &mut Vec<ValidationIssue>,
) {
    let owners = matches
        .iter()
        .map(|(contract, category)| format!("{contract} ({})", category.label()))
        .collect::<Vec<_>>()
        .join(", ");
    issues.push(ValidationIssue::at_line(
        path,
        "contract_identifier.out_of_scope",
        Some(line),
        format!(
            "document {} structural unit `{}` uses `{value}`, which belongs to {owners} but not to the document's exact semantic contract scope",
            paired.doc_id, unit.key
        ),
    ));
}

fn document_domains(
    paired: &PairedDocument,
    catalog: &ContractCatalog,
) -> BTreeSet<ContractDomain> {
    paired
        .contracts
        .iter()
        .filter_map(|contract| catalog.contracts.get(contract))
        .map(|owner| owner.domain)
        .collect()
}

fn report_invalid(
    paired: &PairedDocument,
    unit: &MarkdownUnit,
    line: usize,
    value: &str,
    suggestions: &BTreeSet<String>,
    path: &str,
    issues: &mut Vec<ValidationIssue>,
) {
    let suggestion = if suggestions.is_empty() {
        String::new()
    } else {
        format!(
            "; nearest current owner identifier(s): {}",
            format_identifiers(suggestions)
        )
    };
    issues.push(ValidationIssue::at_line(
        path,
        "contract_identifier.invalid",
        Some(line),
        format!(
            "document {} structural unit `{}` uses contract-bound identifier `{value}`, which exists in no current owner descriptor{suggestion}",
            paired.doc_id, unit.key
        ),
    ));
}

fn compare_valid_units(
    paired: &PairedDocument,
    en: &MarkdownStructure,
    ko: &MarkdownStructure,
    en_validation: &LanguageValidation,
    ko_validation: &LanguageValidation,
    catalog: &ContractCatalog,
    issues: &mut Vec<ValidationIssue>,
) {
    let en_units = units_by_key(en);
    let ko_units = units_by_key(ko);
    let keys = en_units
        .keys()
        .chain(ko_units.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    for key in keys {
        if !en_validation.valid_units.contains(&key) || !ko_validation.valid_units.contains(&key) {
            continue;
        }
        let en_identifiers = en_validation
            .identifiers
            .get(&key)
            .cloned()
            .unwrap_or_default();
        let ko_identifiers = ko_validation
            .identifiers
            .get(&key)
            .cloned()
            .unwrap_or_default();
        report_missing_identifiers(
            paired,
            &key,
            en_units.get(&key).copied(),
            ko_units.get(&key).copied(),
            &en_identifiers
                .difference(&ko_identifiers)
                .cloned()
                .collect(),
            "Korean",
            &paired.path_ko,
            en,
            ko,
            catalog,
            issues,
        );
        report_missing_identifiers(
            paired,
            &key,
            en_units.get(&key).copied(),
            ko_units.get(&key).copied(),
            &ko_identifiers
                .difference(&en_identifiers)
                .cloned()
                .collect(),
            "English",
            &paired.path_en,
            en,
            ko,
            catalog,
            issues,
        );
    }
}

fn units_by_key(structure: &MarkdownStructure) -> BTreeMap<MeaningUnitKey, &MarkdownUnit> {
    structure
        .units()
        .map(|unit| (unit.key.clone(), unit))
        .collect()
}

fn unit_identifiers(
    unit: &MarkdownUnit,
    scoped: &BTreeSet<(IdentifierCategory, String)>,
) -> BTreeSet<(IdentifierCategory, String)> {
    let mut identifiers = BTreeSet::new();
    for literal in &unit.literals {
        match (literal.kind, literal.language.as_deref()) {
            (MarkdownLiteralKind::Inline, None) => {
                if unrelated_code_literal(&literal.text) {
                    continue;
                }
                for (category, value) in scoped {
                    if markdown::contains_exact_identifier(&literal.text, value)
                        && (!is_qualified_source_member(&literal.text, value)
                            || matches!(
                                category,
                                IdentifierCategory::DiagnosticCode
                                    | IdentifierCategory::ProtocolIdentifier
                            ))
                    {
                        identifiers.insert((*category, value.clone()));
                    }
                }
            }
            (MarkdownLiteralKind::Fenced, Some("json" | "yaml" | "yml")) => {
                let Ok(value) = serde_yaml::from_str::<serde_yaml::Value>(&literal.text) else {
                    continue;
                };
                let mut keys = BTreeSet::new();
                let mut values = BTreeSet::new();
                collect_structured_tokens(&value, &mut Vec::new(), &mut keys, &mut values);
                for (category, identifier) in scoped {
                    if category.is_structured_key()
                        && keys.iter().any(|key| key.value == *identifier)
                        || category.is_structured_value() && values.contains(identifier)
                    {
                        identifiers.insert((*category, identifier.clone()));
                    }
                }
            }
            (MarkdownLiteralKind::Fenced, Some("bash" | "console" | "sh" | "shell" | "zsh")) => {
                for (category, value) in scoped {
                    if markdown::contains_exact_identifier(&literal.text, value) {
                        identifiers.insert((*category, value.clone()));
                    }
                }
            }
            (MarkdownLiteralKind::Fenced, Some("text")) => {
                for (category, value) in scoped {
                    if matches!(
                        category,
                        IdentifierCategory::CliSyntax
                            | IdentifierCategory::CliValue
                            | IdentifierCategory::DiagnosticCode
                            | IdentifierCategory::ProtocolIdentifier
                    ) && markdown::contains_exact_identifier(&literal.text, value)
                    {
                        identifiers.insert((*category, value.clone()));
                    }
                }
            }
            _ => {}
        }
    }
    identifiers
}

#[allow(clippy::too_many_arguments)]
fn report_missing_identifiers(
    paired: &PairedDocument,
    key: &MeaningUnitKey,
    en_unit: Option<&MarkdownUnit>,
    ko_unit: Option<&MarkdownUnit>,
    missing: &BTreeSet<(IdentifierCategory, String)>,
    language: &str,
    path: &str,
    en: &MarkdownStructure,
    ko: &MarkdownStructure,
    catalog: &ContractCatalog,
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
    let missing = missing
        .iter()
        .map(|(category, value)| format!("`{value}` ({})", category.label()))
        .collect::<Vec<_>>()
        .join(", ");
    let owners = paired
        .contracts
        .iter()
        .filter_map(|contract| {
            catalog
                .contracts
                .get(contract)
                .map(|owner| format!("{contract} ({})", owner.owner))
        })
        .collect::<Vec<_>>()
        .join(", ");
    issues.push(ValidationIssue::at_line(
        path,
        "contract_identifier.missing",
        Some(issue_line),
        format!(
            "document pair {} ({} <-> {}), structural unit `{key}` (English line {en_line}, Korean line {ko_line}), semantic contracts {owners}: {language} meaning unit is missing {missing}",
            paired.doc_id, paired.path_en, paired.path_ko
        ),
    ));
}

fn declared_contract(literal: &MarkdownLiteral) -> Option<&str> {
    literal
        .attributes
        .iter()
        .find_map(|attribute| attribute.strip_prefix("contract="))
}

#[derive(Debug, Clone, Eq, Ord, PartialEq, PartialOrd)]
struct StructuredKey {
    path: Vec<String>,
    value: String,
}

fn collect_structured_tokens(
    value: &serde_yaml::Value,
    path: &mut Vec<String>,
    keys: &mut BTreeSet<StructuredKey>,
    values: &mut BTreeSet<String>,
) {
    match value {
        serde_yaml::Value::Mapping(mapping) => {
            for (key, value) in mapping {
                if let Some(key) = key.as_str() {
                    let key = normalize_structured_key(key).to_owned();
                    keys.insert(StructuredKey {
                        path: path.clone(),
                        value: key.clone(),
                    });
                    path.push(key);
                    collect_structured_tokens(value, path, keys, values);
                    path.pop();
                } else {
                    collect_structured_tokens(value, path, keys, values);
                }
            }
        }
        serde_yaml::Value::Sequence(sequence) => {
            for value in sequence {
                collect_structured_tokens(value, path, keys, values);
            }
        }
        serde_yaml::Value::String(value) => {
            values.insert(value.to_owned());
        }
        _ => {}
    }
}

fn schema_accepts_key(schema: &Value, path: &[String], key: &str) -> bool {
    let mut path = path;
    if path.first().is_some_and(|segment| segment == "params") {
        path = &path[1..];
    }
    if path.is_empty() && matches!(key, "method" | "params") {
        return true;
    }
    schema_node_accepts_key(schema, schema, path, key, &mut BTreeSet::new())
}

fn schema_node_accepts_key(
    root: &Value,
    node: &Value,
    path: &[String],
    key: &str,
    resolving: &mut BTreeSet<String>,
) -> bool {
    if let Some(reference) = node.get("$ref").and_then(Value::as_str) {
        if !resolving.insert(reference.to_owned()) {
            return false;
        }
        let result = resolve_schema_reference(root, reference)
            .is_some_and(|resolved| schema_node_accepts_key(root, resolved, path, key, resolving));
        resolving.remove(reference);
        return result;
    }
    for combinator in ["allOf", "anyOf", "oneOf"] {
        if node
            .get(combinator)
            .and_then(Value::as_array)
            .is_some_and(|branches| {
                branches.iter().any(|branch| {
                    schema_node_accepts_key(root, branch, path, key, &mut resolving.clone())
                })
            })
        {
            return true;
        }
    }

    if let Some((first, remaining)) = path.split_first() {
        if node
            .get("title")
            .and_then(Value::as_str)
            .is_some_and(|title| title == first)
        {
            return schema_node_accepts_key(root, node, remaining, key, resolving);
        }
        if let Some(definition) = root
            .get("definitions")
            .or_else(|| root.get("$defs"))
            .and_then(Value::as_object)
            .and_then(|definitions| definitions.get(first))
        {
            return schema_node_accepts_key(root, definition, remaining, key, resolving);
        }
        if let Some(property) = node
            .get("properties")
            .and_then(Value::as_object)
            .and_then(|properties| properties.get(first))
        {
            return schema_node_accepts_key(root, property, remaining, key, resolving);
        }
        if let Some(items) = node.get("items") {
            return schema_node_accepts_key(root, items, path, key, resolving);
        }
        return false;
    }

    if node
        .get("properties")
        .and_then(Value::as_object)
        .is_some_and(|properties| properties.contains_key(key))
    {
        return true;
    }
    if node
        .get("title")
        .and_then(Value::as_str)
        .is_some_and(|title| title == key)
        || root
            .get("definitions")
            .or_else(|| root.get("$defs"))
            .and_then(Value::as_object)
            .is_some_and(|definitions| definitions.contains_key(key))
    {
        return true;
    }
    node.get("additionalProperties")
        .is_none_or(|additional| additional != &Value::Bool(false))
}

fn resolve_schema_reference<'a>(root: &'a Value, reference: &str) -> Option<&'a Value> {
    let pointer = reference.strip_prefix('#')?;
    root.pointer(pointer)
}

fn normalize_structured_key(key: &str) -> &str {
    key.strip_suffix('?').unwrap_or(key)
}

fn unrelated_code_literal(value: &str) -> bool {
    value.contains('/')
        || value.contains('\\')
        || value.contains("::")
        || [
            ".rs", ".md", ".json", ".yaml", ".yml", ".toml", ".sql", ".html",
        ]
        .iter()
        .any(|suffix| value.ends_with(suffix))
}

fn is_qualified_source_member(literal: &str, identifier: &str) -> bool {
    literal.contains(&format!("{identifier}."))
        || literal.contains(&format!(".{identifier}"))
        || literal.contains(&format!("{identifier}::"))
        || literal.contains(&format!("::{identifier}"))
}

fn is_plural_variant_of_catalog_identifier(
    candidate: &str,
    identifiers: &BTreeSet<String>,
) -> bool {
    identifiers.iter().any(|identifier| {
        candidate == format!("{identifier}s")
            || candidate == format!("{identifier}es")
            || identifier == &format!("{candidate}s")
            || identifier == &format!("{candidate}es")
    })
}

fn looks_like_identifier(value: &str) -> bool {
    !value.is_empty()
        && !value.chars().any(char::is_whitespace)
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.')
        })
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
                    "documented operation categories differ from the current OperationCategory owner; missing: {}; unexpected: {}",
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

    fn owner(
        domain: ContractDomain,
        category: IdentifierCategory,
        identifiers: &[&str],
    ) -> OwnerCatalog {
        OwnerCatalog {
            owner: "synthetic owner".to_owned(),
            domain,
            identifiers: BTreeMap::from([(
                category,
                identifiers
                    .iter()
                    .map(|identifier| (*identifier).to_owned())
                    .collect(),
            )]),
            related_contracts: BTreeSet::new(),
            schema: None,
        }
    }

    fn catalog(entries: &[(&str, OwnerCatalog)]) -> ContractCatalog {
        ContractCatalog {
            contracts: entries
                .iter()
                .map(|(id, owner)| ((*id).to_owned(), owner.clone()))
                .collect(),
        }
    }

    fn validate(
        catalog: &ContractCatalog,
        contracts: &[&str],
        english: &str,
        korean: &str,
    ) -> Vec<ValidationIssue> {
        let root = TempDir::new().expect("fixture root");
        fs::write(root.path().join("en.md"), english).expect("English fixture");
        fs::write(root.path().join("ko.md"), korean).expect("Korean fixture");
        let paired = PairedDocument {
            doc_id: "reference.api.method-alpha".to_owned(),
            path_en: "en.md".to_owned(),
            path_ko: "ko.md".to_owned(),
            contracts: contracts
                .iter()
                .map(|contract| (*contract).to_owned())
                .collect(),
        };
        let mut issues = Vec::new();
        validate_pair(root.path(), &paired, catalog, &mut issues);
        issues.sort();
        issues
    }

    #[test]
    fn another_methods_valid_field_is_out_of_scope() {
        let catalog = catalog(&[
            (
                "api.method.alpha.request",
                owner(
                    ContractDomain::PublicApi,
                    IdentifierCategory::ApiProperty,
                    &["alpha_field"],
                ),
            ),
            (
                "api.method.beta.request",
                owner(
                    ContractDomain::PublicApi,
                    IdentifierCategory::ApiProperty,
                    &["beta_field"],
                ),
            ),
        ]);
        let issues = validate(
            &catalog,
            &["api.method.alpha.request"],
            "# A\n\n```yaml\nbeta_field: value\n```\n",
            "# 가\n\n```yaml\nbeta_field: value\n```\n",
        );

        assert_eq!(issues.len(), 2, "{issues:#?}");
        assert!(issues
            .iter()
            .all(|issue| issue.category() == "contract_identifier.out_of_scope"));
    }

    #[test]
    fn same_misspelling_in_both_languages_fails_before_parity() {
        let catalog = catalog(&[(
            "api.method.alpha.request",
            owner(
                ContractDomain::PublicApi,
                IdentifierCategory::ApiProperty,
                &["state_version"],
            ),
        )]);
        let issues = validate(
            &catalog,
            &["api.method.alpha.request"],
            "# A\n\n`state_vesion`\n",
            "# 가\n\n`state_vesion`\n",
        );

        assert_eq!(issues.len(), 2, "{issues:#?}");
        assert!(issues
            .iter()
            .all(|issue| issue.category() == "contract_identifier.invalid"));
    }

    #[test]
    fn simple_lowercase_values_and_snake_case_properties_are_validated() {
        let mut api = owner(
            ContractDomain::PublicApi,
            IdentifierCategory::ApiProperty,
            &["display_mode"],
        );
        api.identifiers.insert(
            IdentifierCategory::ApiValue,
            ["ready", "blocked"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
        );
        let catalog = catalog(&[("api.method.alpha.request", api)]);
        let issues = validate(
            &catalog,
            &["api.method.alpha.request"],
            "# A\n\n`display_mode` is `ready`.\n",
            "# 가\n\n`display_mode`는 `ready`입니다.\n",
        );

        assert!(issues.is_empty(), "{issues:#?}");
    }

    #[test]
    fn cli_diagnostics_and_protocol_categories_remain_distinct() {
        let catalog = catalog(&[
            (
                "cli.command.alpha",
                owner(
                    ContractDomain::CliCommand,
                    IdentifierCategory::CliValue,
                    &["read-only"],
                ),
            ),
            (
                "diagnostic.platform",
                owner(
                    ContractDomain::Diagnostic,
                    IdentifierCategory::DiagnosticCode,
                    &["platform.target.unsupported"],
                ),
            ),
            (
                "mcp.protocol",
                owner(
                    ContractDomain::Protocol,
                    IdentifierCategory::ProtocolIdentifier,
                    &["inputSchema"],
                ),
            ),
        ]);
        let issues = validate(
            &catalog,
            &["cli.command.alpha", "diagnostic.platform", "mcp.protocol"],
            "# A\n\n`read-only`, `platform.target.unsupported`, `inputSchema`.\n",
            "# 가\n\n`read-only`, `platform.target.unsupported`, `inputSchema`.\n",
        );

        assert!(issues.is_empty(), "{issues:#?}");
    }

    #[test]
    fn mcp_wire_identifiers_resolve_to_the_wire_owner_descriptor() {
        let mut catalog = ContractCatalog::default();
        let mut issues = Vec::new();
        load_protocol_descriptors(&mut catalog, &mut issues);
        load_wire_descriptors(&mut catalog, &mut issues);

        assert!(issues.is_empty(), "{issues:#?}");
        let owner = catalog
            .contracts
            .get("mcp.wire")
            .expect("MCP wire contract");
        assert_eq!(owner.owner, "crates/volicord-mcp-wire/src/contracts.rs");
        assert!(owner
            .identifiers
            .get(&IdentifierCategory::ProtocolIdentifier)
            .is_some_and(|identifiers| identifiers.contains("MCP_UNAVAILABLE")));
        assert!(catalog
            .contracts
            .get("mcp.protocol")
            .and_then(|protocol| {
                protocol
                    .identifiers
                    .get(&IdentifierCategory::ProtocolIdentifier)
            })
            .is_some_and(|identifiers| !identifiers.contains("MCP_UNAVAILABLE")));
    }

    #[test]
    fn unknown_structured_keys_fail_without_a_marker() {
        let catalog = catalog(&[(
            "api.method.alpha.request",
            owner(
                ContractDomain::PublicApi,
                IdentifierCategory::ApiProperty,
                &["known_key"],
            ),
        )]);
        let issues = validate(
            &catalog,
            &["api.method.alpha.request"],
            "# A\n\n```json\n{\"unknown_key\": true}\n```\n",
            "# 가\n\n```json\n{\"unknown_key\": true}\n```\n",
        );

        assert_eq!(issues.len(), 2, "{issues:#?}");
        assert!(issues
            .iter()
            .all(|issue| issue.category() == "contract_identifier.invalid"));
    }

    #[test]
    fn unrelated_paths_and_source_identifiers_are_ignored() {
        let catalog = catalog(&[(
            "api.method.alpha.request",
            owner(
                ContractDomain::PublicApi,
                IdentifierCategory::ApiProperty,
                &["state_version"],
            ),
        )]);
        let issues = validate(
            &catalog,
            &["api.method.alpha.request"],
            "# A\n\nSee `src/state_version.rs` and `crate::state_version`.\n",
            "# 가\n\n`src/state_version.rs`와 `crate::state_version`를 봅니다.\n",
        );

        assert!(issues.is_empty(), "{issues:#?}");
    }

    #[test]
    fn fence_contract_selects_one_of_multiple_legitimate_contracts() {
        let catalog = catalog(&[
            (
                "api.method.alpha.request",
                owner(
                    ContractDomain::PublicApi,
                    IdentifierCategory::ApiProperty,
                    &["alpha_field"],
                ),
            ),
            (
                "api.method.beta.request",
                owner(
                    ContractDomain::PublicApi,
                    IdentifierCategory::ApiProperty,
                    &["beta_field"],
                ),
            ),
        ]);
        let issues = validate(
            &catalog,
            &["api.method.alpha.request", "api.method.beta.request"],
            "# A\n\n```yaml contract=api.method.alpha.request\nalpha_field: value\n```\n",
            "# 가\n\n```yaml contract=api.method.alpha.request\nalpha_field: value\n```\n",
        );

        assert!(issues.is_empty(), "{issues:#?}");
    }

    #[test]
    fn diagnostics_are_deterministic() {
        let catalog = catalog(&[(
            "api.method.alpha.request",
            owner(
                ContractDomain::PublicApi,
                IdentifierCategory::ApiProperty,
                &["state_version"],
            ),
        )]);
        let first = validate(
            &catalog,
            &["api.method.alpha.request"],
            "# A\n\n`state_version`\n",
            "# 가\n\nnone\n",
        );
        let second = validate(
            &catalog,
            &["api.method.alpha.request"],
            "# A\n\n`state_version`\n",
            "# 가\n\nnone\n",
        );

        assert_eq!(first, second);
        assert_eq!(first.len(), 1, "{first:#?}");
        assert_eq!(first[0].category(), "contract_identifier.missing");
    }
}
