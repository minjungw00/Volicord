use crate::diagnostics::ValidationIssue;
use crate::doc_index::{ContractId, DocIndex, PairedDocument};
use crate::markdown::{
    self, MarkdownLiteral, MarkdownLiteralKind, MarkdownStructure, MarkdownUnit, MeaningUnitKey,
};
use crate::structured_parser::{self, StructuredParseError};
use jsonschema::{error::ValidationErrorKind, Draft, JSONSchema};
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
const CURRENT_JSON_SCHEMA_DIALECT: &str = "http://json-schema.org/draft-07/schema#";

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

#[derive(Debug, Clone)]
struct OwnerCatalog {
    owner: String,
    domain: ContractDomain,
    identifiers: BTreeMap<IdentifierCategory, BTreeSet<String>>,
    related_contracts: BTreeSet<String>,
    example_schemas: BTreeMap<String, Value>,
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

    fn scoped_identifiers<'a>(
        &self,
        scope: impl Iterator<Item = &'a ContractId>,
    ) -> BTreeSet<(IdentifierCategory, String)> {
        scope
            .filter_map(|contract| self.contracts.get(contract.as_str()))
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
        .all(|paired| paired.contract_bindings().next().is_none())
    {
        return;
    }
    let catalog = load_contract_catalog(root, issues);
    validate_descriptor_relationships(&catalog, issues);

    for paired in index
        .paired_documents
        .values()
        .filter(|paired| paired.contract_bindings().next().is_some())
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
                example_schemas: descriptor.example_schemas().clone(),
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
                example_schemas: BTreeMap::new(),
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
                example_schemas: BTreeMap::new(),
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
                example_schemas: descriptor.example_schemas().clone(),
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
                example_schemas: json_example_schemas(object, relative_path, id, issues),
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

fn json_example_schemas(
    object: &serde_json::Map<String, Value>,
    path: &str,
    contract: &str,
    issues: &mut Vec<ValidationIssue>,
) -> BTreeMap<String, Value> {
    let Some(value) = object.get("example_schemas") else {
        return BTreeMap::new();
    };
    let Some(schemas) = value.as_object() else {
        issues.push(ValidationIssue::new(
            path,
            "contract_example.schema_owner",
            format!("semantic contract {contract} field example_schemas must be an object"),
        ));
        return BTreeMap::new();
    };
    let mut result = BTreeMap::new();
    for (shape, schema) in schemas {
        if shape.is_empty() || !schema.is_object() {
            issues.push(ValidationIssue::new(
                path,
                "contract_example.schema_owner",
                format!(
                    "semantic contract {contract} example_schemas entries require non-empty shape names and object schemas"
                ),
            ));
            continue;
        }
        result.insert(shape.clone(), schema.clone());
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

    let en_validation = validate_language(paired, &en, catalog, &paired.path_en, "English", issues);
    let ko_validation = validate_language(paired, &ko, catalog, &paired.path_ko, "Korean", issues);
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
    language: &str,
    issues: &mut Vec<ValidationIssue>,
) -> LanguageValidation {
    let scoped_identifiers = catalog.scoped_identifiers(paired.contract_ids());
    let mut result = LanguageValidation::default();
    for unit in structure.units() {
        let issue_count = issues.len();
        validate_declared_contracts(paired, unit, catalog, path, issues);
        let structured_instances =
            validate_unit_candidates(paired, unit, catalog, path, language, issues);
        if issues.len() == issue_count {
            result.valid_units.insert(unit.key.clone());
            result.identifiers.insert(
                unit.key.clone(),
                unit_identifiers(unit, &scoped_identifiers, &structured_instances),
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
        if !paired.contains_contract(contract) || !catalog.contracts.contains_key(contract) {
            issues.push(ValidationIssue::at_line(
                path,
                "contract_identifier.scope",
                Some(unit.line),
                format!(
                    "document {} structural unit `{}` declares contract {contract}, which is outside its exact document scope",
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
    language: &str,
    issues: &mut Vec<ValidationIssue>,
) -> BTreeMap<usize, Value> {
    let mut structured_instances = BTreeMap::new();
    for (literal_index, literal) in unit.literals.iter().enumerate() {
        match (literal.kind, literal.language.as_deref()) {
            (MarkdownLiteralKind::Inline, None) => {
                validate_inline_literal(paired, unit, literal, catalog, path, issues);
            }
            (MarkdownLiteralKind::Fenced, Some("json" | "yaml" | "yml")) => {
                if let Some(instance) = validate_structured_literal(
                    paired, unit, literal, catalog, path, language, issues,
                ) {
                    structured_instances.insert(literal_index, instance);
                }
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
    structured_instances
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
    language: &str,
    issues: &mut Vec<ValidationIssue>,
) -> Option<Value> {
    let shapes = declared_attributes(literal, "shape=");
    if shapes.len() != 1 || shapes[0].is_empty() {
        issues.push(ValidationIssue::at_line(
            path,
            "contract_example.owner",
            Some(literal.line),
            format!(
                "{language} document {} structural unit `{}` has a structured example that requires exactly one non-empty shape= selector",
                paired.doc_id, unit.key,
            ),
        ));
        return None;
    }
    let shape = shapes[0];
    let contract_selectors = declared_attributes(literal, "contract=");
    if contract_selectors.len() > 1
        || contract_selectors
            .iter()
            .any(|contract| contract.is_empty())
    {
        issues.push(ValidationIssue::at_line(
            path,
            "contract_example.owner",
            Some(literal.line),
            format!(
                "{language} document {} structural unit `{}` has a structured example with invalid or repeated contract= selectors",
                paired.doc_id, unit.key,
            ),
        ));
        return None;
    }
    let candidates = if let Some(contract) = contract_selectors.first().copied() {
        if !paired.contains_contract(contract) {
            return None;
        }
        vec![contract]
    } else {
        paired
            .contract_ids()
            .filter(|contract| {
                catalog
                    .contracts
                    .get(contract.as_str())
                    .is_some_and(|owner| owner.example_schemas.contains_key(shape))
            })
            .map(ContractId::as_str)
            .collect::<Vec<_>>()
    };
    let contract = match candidates.as_slice() {
        [contract] => *contract,
        [] => {
            issues.push(ValidationIssue::at_line(
                path,
                "contract_example.owner",
                Some(literal.line),
                format!(
                    "{language} document {} structural unit `{}` shape `{shape}` resolves to no exact semantic contract",
                    paired.doc_id, unit.key
                ),
            ));
            return None;
        }
        _ => {
            issues.push(ValidationIssue::at_line(
                path,
                "contract_example.owner",
                Some(literal.line),
                format!(
                    "{language} document {} structural unit `{}` shape `{shape}` is ambiguous across semantic contracts {}; add one exact contract= selector",
                    paired.doc_id,
                    unit.key,
                    candidates.join(", ")
                ),
            ));
            return None;
        }
    };
    let owner = catalog.contracts.get(contract)?;
    let Some(schema) = owner.example_schemas.get(shape) else {
        issues.push(ValidationIssue::at_line(
            path,
            "contract_example.owner",
            Some(literal.line),
            format!(
                "{language} document {} structural unit `{}` semantic contract `{contract}` does not expose example shape `{shape}`",
                paired.doc_id, unit.key
            ),
        ));
        return None;
    };
    let instance = match parse_structured_instance(literal) {
        Ok(instance) => instance,
        Err(StructuredParseError::DuplicateKey {
            instance_path,
            key,
            first,
            repeated,
        }) => {
            let first_document_line = literal.line + first.line - 1;
            let repeated_document_line = literal.line + repeated.line - 1;
            let key =
                serde_json::to_string(&key).unwrap_or_else(|_| "\"<unprintable key>\"".to_owned());
            issues.push(ValidationIssue::at_line(
                path,
                "contract_example.parse",
                Some(repeated_document_line),
                format!(
                    "{language} document {} structural unit `{}` semantic contract `{contract}` shape `{shape}` instance path `{instance_path}` has duplicate key {key}; first occurrence at structured source line {}, column {} (document line {first_document_line}), repeated occurrence at structured source line {}, column {} (document line {repeated_document_line})",
                    paired.doc_id,
                    unit.key,
                    first.line,
                    first.column,
                    repeated.line,
                    repeated.column,
                ),
            ));
            return None;
        }
        Err(StructuredParseError::Invalid {
            instance_path,
            position,
            message,
        }) => {
            let issue_line = position
                .map(|position| literal.line + position.line - 1)
                .unwrap_or(literal.line);
            let position = position.map_or_else(
                || "at an unavailable structured source position".to_owned(),
                |position| {
                    format!(
                        "at structured source line {}, column {} (document line {})",
                        position.line,
                        position.column,
                        literal.line + position.line - 1
                    )
                },
            );
            issues.push(ValidationIssue::at_line(
                path,
                "contract_example.parse",
                Some(issue_line),
                format!(
                    "{language} document {} structural unit `{}` semantic contract `{contract}` shape `{shape}` instance path `{instance_path}` is not a JSON-compatible {} instance {position}: {message}",
                    paired.doc_id,
                    unit.key,
                    literal.language.as_deref().unwrap_or("structured"),
                ),
            ));
            return None;
        }
    };
    if let Some(dialect) = schema.get("$schema") {
        if dialect.as_str() != Some(CURRENT_JSON_SCHEMA_DIALECT) {
            issues.push(ValidationIssue::at_line(
                path,
                "contract_example.schema_owner",
                Some(literal.line),
                format!(
                    "{language} document {} structural unit `{}` semantic contract `{contract}` shape `{shape}` owner {} declares unsupported JSON Schema dialect {}; expected `{CURRENT_JSON_SCHEMA_DIALECT}`",
                    paired.doc_id,
                    unit.key,
                    owner.owner,
                    bounded_json(dialect)
                ),
            ));
            return None;
        }
    }
    let compiled = match JSONSchema::options()
        .with_draft(Draft::Draft7)
        .compile(schema)
    {
        Ok(compiled) => compiled,
        Err(error) => {
            issues.push(ValidationIssue::at_line(
                path,
                "contract_example.schema_owner",
                Some(literal.line),
                format!(
                    "{language} document {} structural unit `{}` semantic contract `{contract}` shape `{shape}` owner {} has an invalid generated JSON Schema at {}: {}",
                    paired.doc_id, unit.key, owner.owner, error.schema_path, error
                ),
            ));
            return None;
        }
    };
    if let Err(validation_errors) = compiled.validate(&instance) {
        let mut diagnostics = validation_errors
            .map(|error| {
                (
                    error.instance_path.to_string(),
                    error.schema_path.to_string(),
                    schema_expectation(&error.kind),
                    bounded_actual(error.instance.as_ref()),
                )
            })
            .collect::<Vec<_>>();
        diagnostics.sort();
        diagnostics.dedup();
        for (instance_path, schema_path, expected, actual) in diagnostics {
            let instance_path = if instance_path.is_empty() {
                "/"
            } else {
                &instance_path
            };
            let schema_path = if schema_path.is_empty() {
                "/"
            } else {
                &schema_path
            };
            issues.push(ValidationIssue::at_line(
                path,
                "contract_example.schema",
                Some(literal.line),
                format!(
                    "{language} document {} structural unit `{}` semantic contract `{contract}` shape `{shape}` instance path `{instance_path}` violates schema rule `{schema_path}`: expected {expected}; actual {actual}",
                    paired.doc_id, unit.key
                ),
            ));
        }
    }
    Some(instance)
}

fn parse_structured_instance(literal: &MarkdownLiteral) -> Result<Value, StructuredParseError> {
    structured_parser::parse(
        literal.language.as_deref().unwrap_or("structured"),
        &literal.text,
    )
}

fn schema_expectation(kind: &ValidationErrorKind) -> String {
    let expected = match kind {
        ValidationErrorKind::AdditionalItems { limit } => {
            format!("at most {limit} array items")
        }
        ValidationErrorKind::AdditionalProperties { unexpected }
        | ValidationErrorKind::UnevaluatedProperties { unexpected } => {
            format!(
                "no unknown properties; unexpected {}",
                unexpected.join(", ")
            )
        }
        ValidationErrorKind::Constant { expected_value } => {
            format!("const {}", bounded_json(expected_value))
        }
        ValidationErrorKind::Enum { options } => {
            format!("one of {}", bounded_json(options))
        }
        ValidationErrorKind::Required { property } => {
            format!("required property {}", bounded_json(property))
        }
        ValidationErrorKind::Type { kind } => format!("type {kind:?}"),
        ValidationErrorKind::MinItems { limit } => format!("at least {limit} array items"),
        ValidationErrorKind::MaxItems { limit } => format!("at most {limit} array items"),
        ValidationErrorKind::MinLength { limit } => format!("string length at least {limit}"),
        ValidationErrorKind::MaxLength { limit } => format!("string length at most {limit}"),
        ValidationErrorKind::Minimum { limit } => format!("number >= {}", bounded_json(limit)),
        ValidationErrorKind::Maximum { limit } => format!("number <= {}", bounded_json(limit)),
        ValidationErrorKind::ExclusiveMinimum { limit } => {
            format!("number > {}", bounded_json(limit))
        }
        ValidationErrorKind::ExclusiveMaximum { limit } => {
            format!("number < {}", bounded_json(limit))
        }
        ValidationErrorKind::Pattern { pattern } => format!("string matching `{pattern}`"),
        ValidationErrorKind::AnyOf => "a value accepted by one `anyOf` branch".to_owned(),
        ValidationErrorKind::OneOfNotValid => {
            "a value accepted by exactly one `oneOf` branch".to_owned()
        }
        ValidationErrorKind::OneOfMultipleValid => {
            "a value accepted by only one `oneOf` branch".to_owned()
        }
        other => bounded_text(&format!("{other:?}"), 160),
    };
    bounded_text(&expected, 160)
}

fn bounded_actual(value: &Value) -> String {
    match value {
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => bounded_json(value),
        Value::Array(values) => format!("array with {} item(s)", values.len()),
        Value::Object(values) => format!("object with {} propertie(s)", values.len()),
    }
}

fn bounded_json(value: &Value) -> String {
    bounded_text(
        &serde_json::to_string(value).unwrap_or_else(|_| "<unprintable>".to_owned()),
        120,
    )
}

fn bounded_text(value: &str, limit: usize) -> String {
    let mut bounded = value.chars().take(limit).collect::<String>();
    if value.chars().count() > limit {
        bounded.push('…');
    }
    bounded
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
        .any(|(contract, _)| paired.contains_contract(contract))
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
        .contract_ids()
        .filter_map(|contract| catalog.contracts.get(contract.as_str()))
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
    structured_instances: &BTreeMap<usize, Value>,
) -> BTreeSet<(IdentifierCategory, String)> {
    let mut identifiers = BTreeSet::new();
    for (literal_index, literal) in unit.literals.iter().enumerate() {
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
                let Some(value) = structured_instances.get(&literal_index) else {
                    continue;
                };
                let mut keys = BTreeSet::new();
                let mut values = BTreeSet::new();
                collect_structured_tokens(value, &mut Vec::new(), &mut keys, &mut values);
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
        .contract_ids()
        .filter_map(|contract| {
            catalog
                .contracts
                .get(contract.as_str())
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
        .filter(|contract| !contract.is_empty())
}

fn declared_attributes<'a>(literal: &'a MarkdownLiteral, prefix: &str) -> Vec<&'a str> {
    literal
        .attributes
        .iter()
        .filter_map(|attribute| attribute.strip_prefix(prefix))
        .collect()
}

#[derive(Debug, Clone, Eq, Ord, PartialEq, PartialOrd)]
struct StructuredKey {
    path: Vec<String>,
    value: String,
}

fn collect_structured_tokens(
    value: &Value,
    path: &mut Vec<String>,
    keys: &mut BTreeSet<StructuredKey>,
    values: &mut BTreeSet<String>,
) {
    match value {
        Value::Object(mapping) => {
            for (key, value) in mapping {
                let key = normalize_structured_key(key).to_owned();
                keys.insert(StructuredKey {
                    path: path.clone(),
                    value: key.clone(),
                });
                path.push(key);
                collect_structured_tokens(value, path, keys, values);
                path.pop();
            }
        }
        Value::Array(sequence) => {
            for value in sequence {
                collect_structured_tokens(value, path, keys, values);
            }
        }
        Value::String(value) => {
            values.insert(value.to_owned());
        }
        _ => {}
    }
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
    use crate::doc_index::{DocumentContractBinding, DocumentContractRole};
    use serde_json::json;
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
            example_schemas: BTreeMap::new(),
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

    fn schema_owner(shape: &str, schema: Value) -> OwnerCatalog {
        let identifiers = volicord_types::contracts::identifiers_from_json_schema(&schema);
        OwnerCatalog {
            owner: "synthetic schema owner".to_owned(),
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
            related_contracts: BTreeSet::new(),
            example_schemas: BTreeMap::from([(shape.to_owned(), schema)]),
        }
    }

    fn request_schema() -> Value {
        json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "additionalProperties": false,
            "required": ["request_only", "nested", "items", "state", "nonnull"],
            "properties": {
                "request_only": {"type": "string"},
                "nested": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["known"],
                    "properties": {"known": {"type": "boolean"}}
                },
                "items": {"type": "array", "items": {"type": "integer"}},
                "state": {"enum": ["ready", "blocked"]},
                "nonnull": {"type": "string"}
            }
        })
    }

    fn response_schema() -> Value {
        json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "additionalProperties": false,
            "required": ["response_only", "outcome"],
            "properties": {
                "response_only": {"type": "integer"},
                "outcome": {"const": "complete"}
            }
        })
    }

    fn structured_catalog() -> ContractCatalog {
        catalog(&[
            (
                "api.method.alpha.request",
                schema_owner("params", request_schema()),
            ),
            (
                "api.method.alpha.response",
                schema_owner("result_body", response_schema()),
            ),
        ])
    }

    fn validate(
        catalog: &ContractCatalog,
        contracts: &[&str],
        english: &str,
        korean: &str,
    ) -> Vec<ValidationIssue> {
        validate_for_document(
            catalog,
            "reference.api.method-alpha",
            contracts,
            english,
            korean,
        )
    }

    fn validate_for_document(
        catalog: &ContractCatalog,
        doc_id: &str,
        contracts: &[&str],
        english: &str,
        korean: &str,
    ) -> Vec<ValidationIssue> {
        let root = TempDir::new().expect("fixture root");
        fs::write(root.path().join("en.md"), english).expect("English fixture");
        fs::write(root.path().join("ko.md"), korean).expect("Korean fixture");
        let paired = PairedDocument {
            doc_id: doc_id.to_owned(),
            path_en: "en.md".to_owned(),
            path_ko: "ko.md".to_owned(),
            contract_bindings: contracts
                .iter()
                .map(|contract| DocumentContractBinding {
                    contract_id: ContractId::parse((*contract).to_owned())
                        .expect("synthetic contract id"),
                    role: DocumentContractRole::SupportingContract,
                })
                .collect(),
        };
        let mut issues = Vec::new();
        validate_pair(root.path(), &paired, catalog, &mut issues);
        issues.sort();
        issues
    }

    #[test]
    fn identifier_validation_consumes_resolved_bindings_independent_of_document_route() {
        let catalog = structured_catalog();
        let issues = validate_for_document(
            &catalog,
            "reference.api.method-route-that-names-no-contract",
            &["api.method.alpha.request"],
            "# A\n\n`request_only`\n",
            "# 가\n\n`request_only`\n",
        );

        assert!(issues.is_empty(), "{issues:#?}");
    }

    #[test]
    fn structured_example_validation_uses_the_same_resolved_binding_set() {
        let catalog = structured_catalog();
        let document = "# A\n\n```json shape=params\n{\"request_only\":\"value\",\"nested\":{\"known\":true},\"items\":[1],\"state\":\"ready\",\"nonnull\":\"value\"}\n```\n";
        let issues = validate_for_document(
            &catalog,
            "reference.api.method-route-that-names-no-contract",
            &["api.method.alpha.request"],
            document,
            document,
        );

        assert!(issues.is_empty(), "{issues:#?}");
    }

    #[test]
    fn response_only_property_fails_in_a_request_example() {
        let catalog = structured_catalog();
        let issues = validate(
            &catalog,
            &["api.method.alpha.request", "api.method.alpha.response"],
            "# A\n\n```json shape=params\n{\"response_only\": 1}\n```\n",
            "# 가\n\n```json shape=params\n{\"response_only\": 1}\n```\n",
        );

        assert!(!issues.is_empty(), "{issues:#?}");
        assert!(issues
            .iter()
            .all(|issue| issue.category() == "contract_example.schema"));
        assert!(issues
            .iter()
            .any(|issue| issue.message().contains("response_only")));
    }

    #[test]
    fn request_only_property_fails_in_a_response_example() {
        let catalog = structured_catalog();
        let issues = validate(
            &catalog,
            &["api.method.alpha.request", "api.method.alpha.response"],
            "# A\n\n```json shape=result_body\n{\"request_only\":\"value\",\"outcome\":\"complete\"}\n```\n",
            "# 가\n\n```json shape=result_body\n{\"request_only\":\"value\",\"outcome\":\"complete\"}\n```\n",
        );

        assert!(issues
            .iter()
            .any(|issue| issue.message().contains("request_only")));
        assert!(issues
            .iter()
            .all(|issue| issue.category() == "contract_example.schema"));
    }

    #[test]
    fn schema_validation_enforces_nested_required_type_array_enum_and_null_rules() {
        let catalog = structured_catalog();
        let cases = [
            (
                "unknown nested property",
                "{\"request_only\":\"value\",\"nested\":{\"known\":true,\"extra\":1},\"items\":[1],\"state\":\"ready\",\"nonnull\":\"value\"}",
                "extra",
            ),
            (
                "missing required property",
                "{\"request_only\":\"value\",\"nested\":{\"known\":true},\"items\":[1],\"state\":\"ready\"}",
                "nonnull",
            ),
            (
                "wrong scalar type",
                "{\"request_only\":4,\"nested\":{\"known\":true},\"items\":[1],\"state\":\"ready\",\"nonnull\":\"value\"}",
                "request_only",
            ),
            (
                "wrong array element type",
                "{\"request_only\":\"value\",\"nested\":{\"known\":true},\"items\":[\"bad\"],\"state\":\"ready\",\"nonnull\":\"value\"}",
                "/items/0",
            ),
            (
                "invalid enum value",
                "{\"request_only\":\"value\",\"nested\":{\"known\":true},\"items\":[1],\"state\":\"reday\",\"nonnull\":\"value\"}",
                "reday",
            ),
            (
                "null in a non-null field",
                "{\"request_only\":\"value\",\"nested\":{\"known\":true},\"items\":[1],\"state\":\"ready\",\"nonnull\":null}",
                "nonnull",
            ),
        ];

        for (label, instance, expected) in cases {
            let document = format!("# A\n\n```json shape=params\n{instance}\n```\n");
            let issues = validate(
                &catalog,
                &["api.method.alpha.request", "api.method.alpha.response"],
                &document,
                &document,
            );
            assert!(!issues.is_empty(), "{label}: {issues:#?}");
            assert!(
                issues
                    .iter()
                    .any(|issue| issue.message().contains(expected)),
                "{label}: {issues:#?}"
            );
            assert!(issues
                .iter()
                .all(|issue| issue.category() == "contract_example.schema"));
        }
    }

    #[test]
    fn same_invalid_enum_in_both_languages_fails_before_parity() {
        let catalog = structured_catalog();
        let document = "# A\n\n```yaml shape=params\nrequest_only: value\nnested:\n  known: true\nitems: [1]\nstate: reday\nnonnull: value\n```\n";
        let issues = validate(
            &catalog,
            &["api.method.alpha.request", "api.method.alpha.response"],
            document,
            document,
        );

        assert_eq!(issues.len(), 2, "{issues:#?}");
        assert!(issues
            .iter()
            .all(|issue| issue.category() == "contract_example.schema"));
        assert!(issues.iter().all(|issue| issue.message().contains("reday")));
    }

    #[test]
    fn valid_yaml_is_converted_to_json_and_validated() {
        let catalog = structured_catalog();
        let document = "# A\n\n```yaml shape=params\nrequest_only: value\nnested:\n  known: true\nitems: [1, 2]\nstate: ready\nnonnull: value\n```\n";
        let issues = validate(
            &catalog,
            &["api.method.alpha.request", "api.method.alpha.response"],
            document,
            document,
        );

        assert!(issues.is_empty(), "{issues:#?}");
    }

    #[test]
    fn duplicate_key_diagnostic_identifies_owner_path_and_source_locations() {
        let catalog = structured_catalog();
        let english = "# A\n\n```json shape=params\n{\n  \"outer\": [\n    {\n      \"entry\": \"first\",\n      \"entry\": \"second\"\n    }\n  ]\n}\n```\n";
        let korean = "# 가\n\n```json shape=params\n{\"request_only\":\"value\",\"nested\":{\"known\":true},\"items\":[1],\"state\":\"ready\",\"nonnull\":\"value\"}\n```\n";
        let issues = validate(&catalog, &["api.method.alpha.request"], english, korean);

        assert_eq!(issues.len(), 1, "{issues:#?}");
        let issue = &issues[0];
        assert_eq!(issue.path(), "en.md");
        assert_eq!(issue.line(), Some(8));
        assert_eq!(issue.category(), "contract_example.parse");
        for expected in [
            "English document reference.api.method-alpha",
            "structural unit `",
            "semantic contract `api.method.alpha.request`",
            "shape `params`",
            "instance path `/outer/0`",
            "duplicate key \"entry\"",
            "first occurrence at structured source line 4, column 7 (document line 7)",
            "repeated occurrence at structured source line 5, column 7 (document line 8)",
        ] {
            assert!(issue.message().contains(expected), "{issue:#?}");
        }
    }

    #[test]
    fn duplicate_keys_are_rejected_independently_in_each_language() {
        let catalog = structured_catalog();
        let valid = "# A\n\n```yaml shape=params\nrequest_only: value\nnested:\n  known: true\nitems: [1]\nstate: ready\nnonnull: value\n```\n";
        let duplicate = "# A\n\n```yaml shape=params\nentry: first\nentry: second\n```\n";

        for (english, korean, expected_path, expected_language) in [
            (duplicate, valid, "en.md", "English"),
            (valid, duplicate, "ko.md", "Korean"),
        ] {
            let issues = validate(&catalog, &["api.method.alpha.request"], english, korean);
            assert_eq!(issues.len(), 1, "{issues:#?}");
            assert_eq!(issues[0].path(), expected_path);
            assert_eq!(issues[0].category(), "contract_example.parse");
            assert!(
                issues[0].message().contains(expected_language),
                "{issues:#?}"
            );
        }
    }

    #[test]
    fn matching_duplicate_failures_in_both_languages_are_not_hidden_by_parity() {
        let catalog = structured_catalog();
        let english =
            "# A\n\n```json shape=params\n{\"entry\":\"first\",\"entry\":\"second\"}\n```\n";
        let korean = "# 가\n\n```yaml shape=params\nentry: first\nentry: second\n```\n";
        let issues = validate(&catalog, &["api.method.alpha.request"], english, korean);

        assert_eq!(issues.len(), 2, "{issues:#?}");
        assert!(issues
            .iter()
            .all(|issue| issue.category() == "contract_example.parse"));
        assert!(issues
            .iter()
            .all(|issue| issue.message().contains("duplicate key")));
        assert_eq!(
            issues
                .iter()
                .map(|issue| issue.path())
                .collect::<BTreeSet<_>>(),
            BTreeSet::from(["en.md", "ko.md"])
        );
    }

    #[test]
    fn reader_facing_schema_notation_is_not_treated_as_an_instance() {
        let catalog = structured_catalog();
        let document = "# A\n\n```schema\nExampleShape:\n  illustrative_field: string\n```\n";
        let issues = validate(
            &catalog,
            &["api.method.alpha.request", "api.method.alpha.response"],
            document,
            document,
        );

        assert!(issues.is_empty(), "{issues:#?}");
    }

    #[test]
    fn non_json_yaml_construct_is_rejected() {
        let catalog = structured_catalog();
        let document = "# A\n\n```yaml shape=params\nrequest_only: !private value\n```\n";
        let issues = validate(
            &catalog,
            &["api.method.alpha.request", "api.method.alpha.response"],
            document,
            document,
        );

        assert_eq!(issues.len(), 2, "{issues:#?}");
        assert!(issues
            .iter()
            .all(|issue| issue.category() == "contract_example.parse"));
        assert!(issues
            .iter()
            .all(|issue| issue.message().contains("YAML tag")));
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
        assert!(owner
            .example_schemas
            .contains_key("mcp_request.volicord.status"));
        assert!(owner
            .example_schemas
            .contains_key("mcp_response.volicord.status"));
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
    fn cli_output_descriptor_artifact_exposes_its_exact_instance_shape() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask has a workspace root");
        let mut issues = Vec::new();
        let catalog = load_contract_catalog(root, &mut issues);

        assert!(issues.is_empty(), "{issues:#?}");
        let owner = catalog
            .contracts
            .get("cli.output.inbox")
            .expect("CLI inbox output contract");
        assert!(owner.example_schemas.contains_key("cli_output"));
    }

    #[test]
    fn structured_example_without_an_exact_shape_fails() {
        let catalog = structured_catalog();
        let issues = validate(
            &catalog,
            &["api.method.alpha.request", "api.method.alpha.response"],
            "# A\n\n```json\n{\"unknown_key\": true}\n```\n",
            "# 가\n\n```json\n{\"unknown_key\": true}\n```\n",
        );

        assert_eq!(issues.len(), 2, "{issues:#?}");
        assert!(issues
            .iter()
            .all(|issue| issue.category() == "contract_example.owner"));
    }

    #[test]
    fn structured_example_with_repeated_shape_or_contract_selectors_fails() {
        let catalog = structured_catalog();
        for document in [
            "# A\n\n```json shape=params shape=params\n{}\n```\n",
            "# A\n\n```json contract=api.method.alpha.request contract=api.method.alpha.request shape=params\n{}\n```\n",
        ] {
            let issues = validate(
                &catalog,
                &["api.method.alpha.request", "api.method.alpha.response"],
                document,
                document,
            );

            assert_eq!(issues.len(), 2, "{issues:#?}");
            assert!(issues
                .iter()
                .all(|issue| issue.category() == "contract_example.owner"));
        }
    }

    #[test]
    fn a_shape_shared_by_multiple_contracts_requires_an_exact_contract_selector() {
        let schema = json!({
            "type": "object",
            "additionalProperties": false,
            "required": ["value"],
            "properties": {"value": {"type": "string"}}
        });
        let catalog = catalog(&[
            (
                "api.method.alpha.request",
                schema_owner("params", schema.clone()),
            ),
            ("api.method.beta.request", schema_owner("params", schema)),
        ]);
        let document = "# A\n\n```yaml shape=params\nvalue: alpha\n```\n";
        let issues = validate(
            &catalog,
            &["api.method.alpha.request", "api.method.beta.request"],
            document,
            document,
        );

        assert_eq!(issues.len(), 2, "{issues:#?}");
        assert!(issues
            .iter()
            .all(|issue| issue.message().contains("is ambiguous")));
    }

    #[test]
    fn schema_validation_enforces_const_values() {
        let catalog = structured_catalog();
        let document =
            "# A\n\n```json shape=result_body\n{\"response_only\":1,\"outcome\":\"pending\"}\n```\n";
        let issues = validate(
            &catalog,
            &["api.method.alpha.request", "api.method.alpha.response"],
            document,
            document,
        );

        assert_eq!(issues.len(), 2, "{issues:#?}");
        assert!(issues.iter().all(|issue| {
            issue.category() == "contract_example.schema"
                && issue.message().contains("const")
                && issue.message().contains("pending")
        }));
    }

    #[test]
    fn invalid_generated_schema_is_reported_as_an_owner_error() {
        let catalog = catalog(&[(
            "api.method.alpha.request",
            schema_owner("params", json!({"type": 42})),
        )]);
        let document = "# A\n\n```json shape=params\n{}\n```\n";
        let issues = validate(&catalog, &["api.method.alpha.request"], document, document);

        assert!(!issues.is_empty(), "{issues:#?}");
        assert!(issues
            .iter()
            .all(|issue| issue.category() == "contract_example.schema_owner"));
        assert!(issues
            .iter()
            .all(|issue| issue.message().contains("invalid generated JSON Schema")));
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
        let schema = json!({
            "type": "object",
            "additionalProperties": false,
            "required": ["value"],
            "properties": {"value": {"type": "string"}}
        });
        let catalog = catalog(&[
            (
                "api.method.alpha.request",
                schema_owner("params", schema.clone()),
            ),
            ("api.method.beta.request", schema_owner("params", schema)),
        ]);
        let issues = validate(
            &catalog,
            &["api.method.alpha.request", "api.method.beta.request"],
            "# A\n\n```yaml contract=api.method.alpha.request shape=params\nvalue: alpha\n```\n",
            "# 가\n\n```yaml contract=api.method.alpha.request shape=params\nvalue: alpha\n```\n",
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

    #[test]
    fn schema_diagnostics_are_deterministic_and_focused() {
        let catalog = structured_catalog();
        let document = "# A\n\n```json shape=params\n{\"request_only\":false,\"nested\":{\"known\":true},\"items\":[1],\"state\":\"ready\",\"nonnull\":\"value\"}\n```\n";
        let first = validate(
            &catalog,
            &["api.method.alpha.request", "api.method.alpha.response"],
            document,
            document,
        );
        let second = validate(
            &catalog,
            &["api.method.alpha.request", "api.method.alpha.response"],
            document,
            document,
        );

        assert_eq!(first, second);
        assert_eq!(first.len(), 2, "{first:#?}");
        assert!(first.iter().all(|issue| {
            issue
                .message()
                .contains("semantic contract `api.method.alpha.request`")
                && issue.message().contains("shape `params`")
                && issue.message().contains("instance path `/request_only`")
                && issue.message().contains("expected type")
                && issue.message().contains("actual false")
        }));
    }
}
