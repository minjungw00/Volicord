use crate::diagnostics::ValidationIssue;
use crate::repository::repo_relative;
use crate::workspace_manifests::{
    read_toml_document, workspace_package_version, workspace_rust_version,
};
use serde_yaml::{Mapping, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use volicord_mcp_protocol::ProtocolRegistry;

fn mapping_get<'a>(mapping: &'a Mapping, key: &str) -> Option<&'a Value> {
    mapping.get(Value::String(key.to_string()))
}

const DOC_INDEX_PATH: &str = "docs/doc-index.yaml";
pub(crate) const TERMINOLOGY_MAP_PATH: &str = "docs/terminology-map.yaml";
const CURRENT_DOC_INDEX_VERSION: i64 = 3;
const TOP_LEVEL_REQUIRED: &[&str] = &[
    "version",
    "metadata",
    "language_retrieval",
    "owner_areas",
    "applicability",
    "contract_sources",
    "default_applicability",
    "entry_schema",
    "shared_documents",
    "documents",
];
const TOP_LEVEL_ALLOWED: &[&str] = TOP_LEVEL_REQUIRED;
const CATALOG_ENTRY_ALLOWED: &[&str] = &["description"];
const APPLICABILITY_ENTRY_ALLOWED: &[&str] = &["description", "version_source"];
const CONTRACT_SOURCE_ENTRY_ALLOWED: &[&str] =
    &["description", "kind", "owner", "document_selectors"];
const CONTRACT_SOURCE_KINDS: &[&str] = &[
    "public_json_schemas",
    "command_model",
    "diagnostic_registry",
    "protocol_registry",
];
const APPLICABILITY_VERSION_SOURCES: &[&str] = &[
    "workspace_package",
    "workspace_rust",
    "mcp_production_registry",
    "doc_index_schema",
    "terminology_map_schema",
];
const ENTRY_SCHEMA_FIELDS: &[&str] = &[
    "applicability_fields",
    "default_applicability",
    "shared_required",
    "paired_required",
    "optional",
    "maintenance_fields",
    "kinds",
    "reader_journeys",
    "normative_levels",
    "translation_policies",
];
const APPLICABILITY_SCHEMA_FIELDS: &[&str] = &["description", "version_source"];
const MAINTENANCE_SCHEMA_FIELDS: &[&str] = &[
    "owner_area",
    "created_on",
    "last_updated_on",
    "last_verified_on",
    "applies_to",
];
const SHARED_REQUIRED: &[&str] = &[
    "doc_id",
    "path",
    "kind",
    "summary",
    "normative_level",
    "owner_area",
    "created_on",
    "last_updated_on",
    "last_verified_on",
];
const PAIRED_REQUIRED: &[&str] = &[
    "doc_id",
    "path_en",
    "path_ko",
    "kind",
    "summary",
    "normative_level",
    "translation_policy",
    "owner_area",
    "created_on",
    "last_updated_on",
    "last_verified_on",
];
const OPTIONAL_FIELDS: &[&str] = &[
    "primary_audience",
    "journeys",
    "canonical_for",
    "depends_on",
];
const SHARED_ALLOWED: &[&str] = &[
    "doc_id",
    "path",
    "kind",
    "summary",
    "normative_level",
    "owner_area",
    "created_on",
    "last_updated_on",
    "last_verified_on",
    "applies_to",
    "primary_audience",
    "journeys",
    "canonical_for",
    "depends_on",
];
const PAIRED_ALLOWED: &[&str] = &[
    "doc_id",
    "path_en",
    "path_ko",
    "kind",
    "summary",
    "normative_level",
    "translation_policy",
    "owner_area",
    "created_on",
    "last_updated_on",
    "last_verified_on",
    "applies_to",
    "primary_audience",
    "journeys",
    "canonical_for",
    "depends_on",
];
const KINDS: &[&str] = &[
    "landing",
    "tutorial",
    "how_to",
    "explanation",
    "reference",
    "maintenance",
];
const READER_JOURNEYS: &[&str] = &[
    "evaluate",
    "install",
    "operate",
    "learn",
    "implement",
    "maintain",
];
const NORMATIVE_LEVELS: &[&str] = &["contract", "guide", "example", "maintenance"];
const TRANSLATION_POLICIES: &[&str] = &["semantic_parity"];
const ROOT_README_EN_PATH: &str = "README.md";
const ROOT_README_KO_PATH: &str = "README.ko.md";
const REQUIRED_SHARED_PATHS: &[&str] = &[
    "AGENTS.md",
    "docs/AGENTS.md",
    "crates/AGENTS.md",
    ROOT_README_EN_PATH,
    "docs/README.md",
    "docs/doc-index.yaml",
    "docs/terminology-map.yaml",
];
#[derive(Debug, Clone)]
pub(crate) struct DocIndex {
    pub(crate) indexed_paths: BTreeSet<String>,
    pub(crate) paired_paths: BTreeMap<String, (String, String)>,
    pub(crate) path_doc_ids: BTreeMap<String, String>,
    pub(crate) paired_documents: BTreeMap<String, PairedDocument>,
    pub(crate) contract_sources: BTreeMap<String, ContractSource>,
}

#[derive(Debug, Clone)]
pub(crate) struct PairedDocument {
    pub(crate) doc_id: String,
    pub(crate) path_en: String,
    pub(crate) path_ko: String,
    pub(crate) contract_sources: BTreeSet<String>,
}

#[derive(Debug, Clone)]
struct DocEntry {
    doc_id: String,
    depends_on: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct ContractSource {
    pub(crate) id: String,
    pub(crate) kind: String,
    pub(crate) owner: String,
    selectors: Vec<String>,
}

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct IsoDate {
    year: u16,
    month: u8,
    day: u8,
}

#[derive(Debug, Clone)]
struct DateError {
    category: &'static str,
    message: String,
}

pub(crate) fn validate_doc_index(
    root: &Path,
    errors: &mut Vec<ValidationIssue>,
) -> Option<DocIndex> {
    let doc_index = root.join(DOC_INDEX_PATH);
    let contents = match fs::read_to_string(&doc_index) {
        Ok(contents) => contents,
        Err(error) => {
            errors.push(ValidationIssue::new(
                DOC_INDEX_PATH,
                "metadata.read",
                format!("failed to read doc index: {error}"),
            ));
            return None;
        }
    };

    let value: Value = match serde_yaml::from_str(&contents) {
        Ok(value) => value,
        Err(error) => {
            errors.push(ValidationIssue::new(
                DOC_INDEX_PATH,
                "metadata.yaml",
                format!("failed to parse YAML: {error}"),
            ));
            return None;
        }
    };

    let Some(top) = value.as_mapping() else {
        errors.push(ValidationIssue::new(
            DOC_INDEX_PATH,
            "metadata.shape",
            "doc index must be a YAML mapping",
        ));
        return None;
    };

    for field in TOP_LEVEL_REQUIRED {
        if mapping_get(top, field).is_none() {
            errors.push(ValidationIssue::new(
                DOC_INDEX_PATH,
                "metadata.missing_field",
                format!("doc index is missing required top-level field {field}"),
            ));
        }
    }

    for field in top.keys() {
        let Some(field) = field.as_str() else {
            errors.push(ValidationIssue::new(
                DOC_INDEX_PATH,
                "metadata.unknown_field",
                "doc index top-level field names must be strings",
            ));
            continue;
        };
        if !TOP_LEVEL_ALLOWED.contains(&field) {
            errors.push(ValidationIssue::new(
                DOC_INDEX_PATH,
                "metadata.unknown_field",
                format!("doc index uses unsupported top-level field {field}"),
            ));
        }
    }

    match mapping_get(top, "version").and_then(Value::as_i64) {
        Some(CURRENT_DOC_INDEX_VERSION) => {}
        Some(version) => errors.push(ValidationIssue::new(
            DOC_INDEX_PATH,
            "metadata.version",
            format!(
                "expected current version {CURRENT_DOC_INDEX_VERSION}, found unsupported version {version}"
            ),
        )),
        None => errors.push(ValidationIssue::new(
            DOC_INDEX_PATH,
            "metadata.version",
            format!("missing numeric current version {CURRENT_DOC_INDEX_VERSION}"),
        )),
    }

    validate_top_level_mapping(top, "metadata", errors);
    validate_top_level_mapping(top, "language_retrieval", errors);
    validate_entry_schema(top, errors);
    let owner_areas = validate_catalog(top, "owner_areas", CATALOG_ENTRY_ALLOWED, errors);
    let applicability = validate_catalog(top, "applicability", APPLICABILITY_ENTRY_ALLOWED, errors);
    let contract_sources = validate_contract_sources(top, errors);
    validate_applicability_sources(root, top, errors);
    let default_applicability = validate_default_applicability(top, &applicability, errors);

    let mut entries = Vec::new();
    let mut doc_ids = BTreeSet::new();
    let mut indexed_paths = BTreeSet::new();
    let mut paired_paths = BTreeMap::new();
    let mut path_doc_ids = BTreeMap::new();
    let mut paired_documents = BTreeMap::new();

    validate_entries(
        root,
        top,
        "shared_documents",
        EntryMode::Shared,
        &mut entries,
        &mut doc_ids,
        &mut indexed_paths,
        &mut paired_paths,
        &mut path_doc_ids,
        &mut paired_documents,
        &owner_areas,
        &applicability,
        &default_applicability,
        errors,
    );
    validate_entries(
        root,
        top,
        "documents",
        EntryMode::Paired,
        &mut entries,
        &mut doc_ids,
        &mut indexed_paths,
        &mut paired_paths,
        &mut path_doc_ids,
        &mut paired_documents,
        &owner_areas,
        &applicability,
        &default_applicability,
        errors,
    );

    for required_path in REQUIRED_SHARED_PATHS {
        if !indexed_paths.contains(*required_path) {
            errors.push(ValidationIssue::new(
                DOC_INDEX_PATH,
                "coverage.missing_shared_index",
                format!("shared maintained path is not indexed: {required_path}"),
            ));
        }
        if !root.join(required_path).exists() {
            errors.push(ValidationIssue::new(
                DOC_INDEX_PATH,
                "coverage.missing_shared_path",
                format!("shared maintained path does not exist: {required_path}"),
            ));
        }
    }

    for entry in &entries {
        for depends_on in &entry.depends_on {
            if !doc_ids.contains(depends_on) {
                errors.push(ValidationIssue::new(
                    DOC_INDEX_PATH,
                    "metadata.invalid_depends_on",
                    format!("{} depends on unknown doc_id {depends_on}", entry.doc_id),
                ));
            }
        }
    }

    resolve_contract_sources(&contract_sources, &mut paired_documents, errors);

    Some(DocIndex {
        indexed_paths,
        paired_paths,
        path_doc_ids,
        paired_documents,
        contract_sources: contract_sources
            .into_iter()
            .map(|source| (source.id.clone(), source))
            .collect(),
    })
}

fn validate_top_level_mapping(top: &Mapping, key: &'static str, errors: &mut Vec<ValidationIssue>) {
    if let Some(value) = mapping_get(top, key) {
        if !value.is_mapping() {
            errors.push(ValidationIssue::new(
                DOC_INDEX_PATH,
                "metadata.shape",
                format!("{key} must be a mapping"),
            ));
        }
    }
}

fn validate_entry_schema(top: &Mapping, errors: &mut Vec<ValidationIssue>) {
    let Some(value) = mapping_get(top, "entry_schema") else {
        return;
    };
    let Some(schema) = value.as_mapping() else {
        errors.push(ValidationIssue::new(
            DOC_INDEX_PATH,
            "metadata.entry_schema",
            "entry_schema must be a mapping",
        ));
        return;
    };

    validate_exact_mapping_fields(
        schema,
        "entry_schema",
        ENTRY_SCHEMA_FIELDS,
        "metadata.entry_schema",
        errors,
    );
    validate_description_mapping(
        schema,
        "applicability_fields",
        APPLICABILITY_SCHEMA_FIELDS,
        errors,
    );
    validate_nonempty_schema_description(schema, "entry_schema", "default_applicability", errors);
    validate_schema_sequence(schema, "shared_required", SHARED_REQUIRED, errors);
    validate_schema_sequence(schema, "paired_required", PAIRED_REQUIRED, errors);
    validate_schema_sequence(schema, "optional", OPTIONAL_FIELDS, errors);
    validate_description_mapping(
        schema,
        "maintenance_fields",
        MAINTENANCE_SCHEMA_FIELDS,
        errors,
    );
    validate_schema_sequence(schema, "kinds", KINDS, errors);
    validate_schema_sequence(schema, "reader_journeys", READER_JOURNEYS, errors);
    validate_schema_sequence(schema, "normative_levels", NORMATIVE_LEVELS, errors);
    validate_schema_sequence(schema, "translation_policies", TRANSLATION_POLICIES, errors);
}

fn validate_exact_mapping_fields(
    mapping: &Mapping,
    label: &str,
    expected: &[&str],
    category: &'static str,
    errors: &mut Vec<ValidationIssue>,
) {
    let actual = mapping
        .keys()
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    let expected = expected.iter().copied().collect::<BTreeSet<_>>();
    for missing in expected.difference(&actual) {
        errors.push(ValidationIssue::new(
            DOC_INDEX_PATH,
            category,
            format!("{label} is missing current field {missing}"),
        ));
    }
    for unknown in actual.difference(&expected) {
        errors.push(ValidationIssue::new(
            DOC_INDEX_PATH,
            category,
            format!("{label} uses unsupported field {unknown}"),
        ));
    }
    if mapping.keys().any(|field| field.as_str().is_none()) {
        errors.push(ValidationIssue::new(
            DOC_INDEX_PATH,
            category,
            format!("{label} field names must be strings"),
        ));
    }
}

fn validate_description_mapping(
    schema: &Mapping,
    key: &str,
    expected: &[&str],
    errors: &mut Vec<ValidationIssue>,
) {
    let Some(value) = mapping_get(schema, key) else {
        return;
    };
    let Some(mapping) = value.as_mapping() else {
        errors.push(ValidationIssue::new(
            DOC_INDEX_PATH,
            "metadata.entry_schema",
            format!("entry_schema.{key} must be a mapping"),
        ));
        return;
    };
    validate_exact_mapping_fields(
        mapping,
        &format!("entry_schema.{key}"),
        expected,
        "metadata.entry_schema",
        errors,
    );
    let label = format!("entry_schema.{key}");
    for field in expected {
        validate_nonempty_schema_description(mapping, &label, field, errors);
    }
}

fn validate_nonempty_schema_description(
    mapping: &Mapping,
    label: &str,
    key: &str,
    errors: &mut Vec<ValidationIssue>,
) {
    if mapping_get(mapping, key)
        .and_then(Value::as_str)
        .is_none_or(|description| description.trim().is_empty())
    {
        errors.push(ValidationIssue::new(
            DOC_INDEX_PATH,
            "metadata.entry_schema",
            format!("{label}.{key} must be a non-empty string"),
        ));
    }
}

fn validate_schema_sequence(
    schema: &Mapping,
    key: &str,
    expected: &[&str],
    errors: &mut Vec<ValidationIssue>,
) {
    let Some(value) = mapping_get(schema, key) else {
        return;
    };
    let Some(sequence) = value.as_sequence() else {
        errors.push(ValidationIssue::new(
            DOC_INDEX_PATH,
            "metadata.entry_schema",
            format!("entry_schema.{key} must be a list"),
        ));
        return;
    };
    let actual = sequence
        .iter()
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    let expected = expected.iter().copied().collect::<BTreeSet<_>>();
    if sequence.iter().any(|item| item.as_str().is_none())
        || actual.len() != sequence.len()
        || actual != expected
    {
        errors.push(ValidationIssue::new(
            DOC_INDEX_PATH,
            "metadata.entry_schema",
            format!(
                "entry_schema.{key} must declare exactly the current values: {}",
                expected.into_iter().collect::<Vec<_>>().join(", ")
            ),
        ));
    }
}

fn validate_catalog(
    top: &Mapping,
    key: &'static str,
    allowed_fields: &[&str],
    errors: &mut Vec<ValidationIssue>,
) -> BTreeSet<String> {
    let mut identifiers = BTreeSet::new();
    let Some(value) = mapping_get(top, key) else {
        return identifiers;
    };
    let Some(catalog) = value.as_mapping() else {
        errors.push(ValidationIssue::new(
            DOC_INDEX_PATH,
            "metadata.shape",
            format!("{key} must be a mapping"),
        ));
        return identifiers;
    };

    if catalog.is_empty() {
        errors.push(ValidationIssue::new(
            DOC_INDEX_PATH,
            "metadata.catalog",
            format!("{key} must not be empty"),
        ));
    }

    for (identifier, value) in catalog {
        let Some(identifier) = identifier.as_str() else {
            errors.push(ValidationIssue::new(
                DOC_INDEX_PATH,
                "metadata.catalog",
                format!("{key} identifiers must be strings"),
            ));
            continue;
        };
        if !is_catalog_identifier(identifier) {
            errors.push(ValidationIssue::new(
                DOC_INDEX_PATH,
                "metadata.catalog",
                format!("{key} identifier {identifier} must use lowercase letters, digits, or underscores"),
            ));
        }
        identifiers.insert(identifier.to_string());

        let Some(entry) = value.as_mapping() else {
            errors.push(ValidationIssue::new(
                DOC_INDEX_PATH,
                "metadata.catalog",
                format!("{key}.{identifier} must be a mapping"),
            ));
            continue;
        };
        for field in entry.keys().filter_map(Value::as_str) {
            if !allowed_fields.contains(&field) {
                errors.push(ValidationIssue::new(
                    DOC_INDEX_PATH,
                    "metadata.unknown_field",
                    format!("{key}.{identifier} uses unsupported field {field}"),
                ));
            }
        }
        match mapping_get(entry, "description").and_then(Value::as_str) {
            Some(description) if !description.trim().is_empty() => {}
            Some(_) => errors.push(ValidationIssue::new(
                DOC_INDEX_PATH,
                "metadata.catalog",
                format!("{key}.{identifier} description must not be empty"),
            )),
            None => errors.push(ValidationIssue::new(
                DOC_INDEX_PATH,
                "metadata.catalog",
                format!("{key}.{identifier} is missing string description"),
            )),
        }
    }

    identifiers
}

fn validate_contract_sources(
    top: &Mapping,
    errors: &mut Vec<ValidationIssue>,
) -> Vec<ContractSource> {
    let Some(value) = mapping_get(top, "contract_sources") else {
        return Vec::new();
    };
    let Some(catalog) = value.as_mapping() else {
        errors.push(ValidationIssue::new(
            DOC_INDEX_PATH,
            "metadata.contract_sources",
            "contract_sources must be a mapping",
        ));
        return Vec::new();
    };
    if catalog.is_empty() {
        errors.push(ValidationIssue::new(
            DOC_INDEX_PATH,
            "metadata.contract_sources",
            "contract_sources must not be empty",
        ));
    }

    let mut sources = Vec::new();
    let mut seen_kinds = BTreeSet::new();
    for (id, value) in catalog {
        let Some(id) = id.as_str() else {
            errors.push(ValidationIssue::new(
                DOC_INDEX_PATH,
                "metadata.contract_sources",
                "contract source identifiers must be strings",
            ));
            continue;
        };
        if !is_semantic_applicability_identifier(id) {
            errors.push(ValidationIssue::new(
                DOC_INDEX_PATH,
                "metadata.contract_sources",
                format!(
                    "contract source identifier {id} must use stable lowercase semantic words separated by underscores"
                ),
            ));
        }
        let Some(entry) = value.as_mapping() else {
            errors.push(ValidationIssue::new(
                DOC_INDEX_PATH,
                "metadata.contract_sources",
                format!("contract_sources.{id} must be a mapping"),
            ));
            continue;
        };
        validate_exact_mapping_fields(
            entry,
            &format!("contract_sources.{id}"),
            CONTRACT_SOURCE_ENTRY_ALLOWED,
            "metadata.contract_sources",
            errors,
        );
        validate_nonempty_contract_source_string(entry, id, "description", errors);
        let kind = validate_nonempty_contract_source_string(entry, id, "kind", errors);
        let owner = validate_nonempty_contract_source_string(entry, id, "owner", errors);
        if let Some(kind) = kind.as_deref() {
            if !CONTRACT_SOURCE_KINDS.contains(&kind) {
                errors.push(ValidationIssue::new(
                    DOC_INDEX_PATH,
                    "metadata.contract_sources",
                    format!("contract_sources.{id}.kind {kind} is unsupported"),
                ));
            } else if !seen_kinds.insert(kind.to_owned()) {
                errors.push(ValidationIssue::new(
                    DOC_INDEX_PATH,
                    "metadata.contract_sources",
                    format!(
                        "contract source kind {kind} is assigned more than once; each current contract has one catalog"
                    ),
                ));
            }
        }

        let selectors = mapping_get(entry, "document_selectors")
            .and_then(sequence_strings)
            .unwrap_or_else(|| {
                errors.push(ValidationIssue::new(
                    DOC_INDEX_PATH,
                    "metadata.contract_sources",
                    format!(
                        "contract_sources.{id}.document_selectors must be a non-empty list of strings"
                    ),
                ));
                Vec::new()
            });
        if selectors.is_empty() {
            errors.push(ValidationIssue::new(
                DOC_INDEX_PATH,
                "metadata.contract_sources",
                format!(
                    "contract_sources.{id}.document_selectors must be a non-empty list of strings"
                ),
            ));
        }
        let mut seen_selectors = BTreeSet::new();
        for selector in &selectors {
            if !valid_document_selector(selector) {
                errors.push(ValidationIssue::new(
                    DOC_INDEX_PATH,
                    "metadata.contract_sources",
                    format!(
                        "contract_sources.{id} uses invalid document selector {selector}; use an exact doc_id or a trailing .* family selector"
                    ),
                ));
            }
            if !seen_selectors.insert(selector) {
                errors.push(ValidationIssue::new(
                    DOC_INDEX_PATH,
                    "metadata.contract_sources",
                    format!("contract_sources.{id} repeats document selector {selector}"),
                ));
            }
        }

        if let (Some(kind), Some(owner)) = (kind, owner) {
            sources.push(ContractSource {
                id: id.to_owned(),
                kind,
                owner,
                selectors,
            });
        }
    }
    sources
}

fn validate_nonempty_contract_source_string(
    entry: &Mapping,
    source_id: &str,
    field: &str,
    errors: &mut Vec<ValidationIssue>,
) -> Option<String> {
    match mapping_get(entry, field).and_then(Value::as_str) {
        Some(value) if !value.trim().is_empty() => Some(value.to_owned()),
        _ => {
            errors.push(ValidationIssue::new(
                DOC_INDEX_PATH,
                "metadata.contract_sources",
                format!("contract_sources.{source_id}.{field} must be a non-empty string"),
            ));
            None
        }
    }
}

fn valid_document_selector(selector: &str) -> bool {
    let selector = selector.strip_suffix(".*").unwrap_or(selector);
    !selector.is_empty()
        && !selector.starts_with('.')
        && !selector.ends_with('.')
        && selector.split('.').all(|part| {
            !part.is_empty()
                && part.chars().all(|character| {
                    character.is_ascii_lowercase()
                        || character.is_ascii_digit()
                        || matches!(character, '_' | '-')
                })
        })
}

fn resolve_contract_sources(
    sources: &[ContractSource],
    paired_documents: &mut BTreeMap<String, PairedDocument>,
    errors: &mut Vec<ValidationIssue>,
) {
    for source in sources {
        for selector in &source.selectors {
            let matching_ids = paired_documents
                .keys()
                .filter(|doc_id| document_selector_matches(selector, doc_id))
                .cloned()
                .collect::<Vec<_>>();
            if matching_ids.is_empty() {
                errors.push(ValidationIssue::new(
                    DOC_INDEX_PATH,
                    "metadata.contract_sources",
                    format!(
                        "contract_sources.{} selector {selector} does not resolve to a paired document",
                        source.id
                    ),
                ));
                continue;
            }
            for doc_id in matching_ids {
                paired_documents
                    .get_mut(&doc_id)
                    .expect("selected paired document exists")
                    .contract_sources
                    .insert(source.id.clone());
            }
        }
    }
}

fn document_selector_matches(selector: &str, doc_id: &str) -> bool {
    selector
        .strip_suffix(".*")
        .map_or(doc_id == selector, |prefix| {
            doc_id
                .strip_prefix(prefix)
                .is_some_and(|suffix| suffix.starts_with('.'))
        })
}

fn validate_applicability_sources(root: &Path, top: &Mapping, errors: &mut Vec<ValidationIssue>) {
    let Some(applicability) = mapping_get(top, "applicability").and_then(Value::as_mapping) else {
        return;
    };

    let manifest = read_toml_document(&root.join("Cargo.toml"), "root Cargo.toml");
    let terminology_version = read_yaml_version(&root.join(TERMINOLOGY_MAP_PATH));
    let doc_index_version = mapping_get(top, "version").and_then(Value::as_i64);
    let production_profiles = ProtocolRegistry::production()
        .oldest_to_newest()
        .map(|profile| profile.revision().as_str())
        .collect::<Vec<_>>();
    let mut seen_sources = BTreeSet::new();

    for (identifier, value) in applicability {
        let (Some(identifier), Some(entry)) = (identifier.as_str(), value.as_mapping()) else {
            continue;
        };
        if !is_semantic_applicability_identifier(identifier) {
            errors.push(ValidationIssue::new(
                DOC_INDEX_PATH,
                "metadata.invalid_applicability_identifier",
                format!(
                    "applicability identifier {identifier} must use stable lowercase semantic words separated by underscores"
                ),
            ));
        }

        let Some(version_source) = mapping_get(entry, "version_source").and_then(Value::as_str)
        else {
            errors.push(ValidationIssue::new(
                DOC_INDEX_PATH,
                "metadata.invalid_applicability_source",
                format!(
                    "applicability.{identifier} must declare a string version_source owned by the current repository"
                ),
            ));
            continue;
        };
        if !APPLICABILITY_VERSION_SOURCES.contains(&version_source) {
            errors.push(ValidationIssue::new(
                DOC_INDEX_PATH,
                "metadata.invalid_applicability_source",
                format!(
                    "applicability.{identifier}.version_source {version_source} is unsupported; expected one of {}",
                    APPLICABILITY_VERSION_SOURCES.join(", ")
                ),
            ));
            continue;
        }
        if !seen_sources.insert(version_source) {
            errors.push(ValidationIssue::new(
                DOC_INDEX_PATH,
                "metadata.duplicate_applicability_source",
                format!(
                    "version_source {version_source} is assigned to more than one applicability entry"
                ),
            ));
        }

        let resolved = match version_source {
            "workspace_package" => manifest
                .as_ref()
                .ok()
                .and_then(workspace_package_version)
                .map(str::to_owned),
            "workspace_rust" => manifest
                .as_ref()
                .ok()
                .and_then(workspace_rust_version)
                .map(str::to_owned),
            "mcp_production_registry" => {
                (!production_profiles.is_empty()).then(|| production_profiles.join(", "))
            }
            "doc_index_schema" => doc_index_version.map(|version| version.to_string()),
            "terminology_map_schema" => terminology_version
                .as_ref()
                .ok()
                .and_then(|version| *version)
                .map(|version| version.to_string()),
            _ => None,
        };
        if resolved.as_deref().is_none_or(str::is_empty) {
            errors.push(ValidationIssue::new(
                DOC_INDEX_PATH,
                "metadata.unresolved_applicability_source",
                format!(
                    "applicability.{identifier}.version_source {version_source} does not resolve from its owning file or registry"
                ),
            ));
        }
    }

    if let Err(error) = manifest {
        errors.push(ValidationIssue::new(
            "Cargo.toml",
            "metadata.applicability_source_read",
            format!("failed to resolve workspace applicability sources: {error:#}"),
        ));
    }
    if let Err(error) = terminology_version {
        errors.push(ValidationIssue::new(
            TERMINOLOGY_MAP_PATH,
            "metadata.applicability_source_read",
            format!("failed to resolve terminology map schema version: {error}"),
        ));
    }
}

fn validate_default_applicability(
    top: &Mapping,
    applicability: &BTreeSet<String>,
    errors: &mut Vec<ValidationIssue>,
) -> BTreeSet<String> {
    let Some(value) = mapping_get(top, "default_applicability") else {
        return BTreeSet::new();
    };
    let Some(items) = sequence_strings(value) else {
        errors.push(ValidationIssue::new(
            DOC_INDEX_PATH,
            "metadata.invalid_default_applicability",
            "default_applicability must be a non-empty list of applicability identifiers",
        ));
        return BTreeSet::new();
    };
    if items.is_empty() {
        errors.push(ValidationIssue::new(
            DOC_INDEX_PATH,
            "metadata.invalid_default_applicability",
            "default_applicability must not be empty",
        ));
    }

    let mut defaults = BTreeSet::new();
    for item in items {
        if !defaults.insert(item.clone()) {
            errors.push(ValidationIssue::new(
                DOC_INDEX_PATH,
                "metadata.duplicate_applicability",
                format!("default_applicability repeats value {item}"),
            ));
        }
        if !applicability.contains(&item) {
            errors.push(ValidationIssue::new(
                DOC_INDEX_PATH,
                "metadata.invalid_applicability",
                format!("default_applicability uses unknown value {item}"),
            ));
        }
    }
    defaults
}

fn read_yaml_version(path: &Path) -> Result<Option<i64>, String> {
    let contents = fs::read_to_string(path).map_err(|error| error.to_string())?;
    let value: Value = serde_yaml::from_str(&contents).map_err(|error| error.to_string())?;
    Ok(value
        .as_mapping()
        .and_then(|top| mapping_get(top, "version"))
        .and_then(Value::as_i64))
}

fn is_semantic_applicability_identifier(identifier: &str) -> bool {
    !identifier.is_empty()
        && !identifier.starts_with('_')
        && !identifier.ends_with('_')
        && !identifier.contains("__")
        && identifier
            .chars()
            .all(|character| character.is_ascii_lowercase() || character == '_')
}

fn is_catalog_identifier(identifier: &str) -> bool {
    !identifier.is_empty()
        && identifier.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
        })
}

#[derive(Copy, Clone)]
enum EntryMode {
    Shared,
    Paired,
}

#[allow(clippy::too_many_arguments)]
fn validate_entries(
    root: &Path,
    top: &Mapping,
    key: &'static str,
    mode: EntryMode,
    entries: &mut Vec<DocEntry>,
    doc_ids: &mut BTreeSet<String>,
    indexed_paths: &mut BTreeSet<String>,
    paired_paths: &mut BTreeMap<String, (String, String)>,
    path_doc_ids: &mut BTreeMap<String, String>,
    paired_documents: &mut BTreeMap<String, PairedDocument>,
    owner_areas: &BTreeSet<String>,
    applicability: &BTreeSet<String>,
    default_applicability: &BTreeSet<String>,
    errors: &mut Vec<ValidationIssue>,
) {
    let Some(value) = mapping_get(top, key) else {
        errors.push(ValidationIssue::new(
            DOC_INDEX_PATH,
            "metadata.shape",
            format!("missing {key} sequence"),
        ));
        return;
    };
    let Some(sequence) = value.as_sequence() else {
        errors.push(ValidationIssue::new(
            DOC_INDEX_PATH,
            "metadata.shape",
            format!("{key} must be a sequence"),
        ));
        return;
    };

    for (index, value) in sequence.iter().enumerate() {
        let label = format!("{key}[{index}]");
        let Some(entry) = value.as_mapping() else {
            errors.push(ValidationIssue::new(
                DOC_INDEX_PATH,
                "metadata.entry_shape",
                format!("{label} must be a mapping"),
            ));
            continue;
        };

        let required = match mode {
            EntryMode::Shared => SHARED_REQUIRED,
            EntryMode::Paired => PAIRED_REQUIRED,
        };
        let allowed = match mode {
            EntryMode::Shared => SHARED_ALLOWED,
            EntryMode::Paired => PAIRED_ALLOWED,
        };

        for field in required {
            if mapping_get(entry, field).is_none() {
                errors.push(ValidationIssue::new(
                    DOC_INDEX_PATH,
                    "metadata.missing_field",
                    format!("{label} is missing required field {field}"),
                ));
            }
        }

        for field in entry.keys().filter_map(Value::as_str) {
            if !allowed.contains(&field) {
                errors.push(ValidationIssue::new(
                    DOC_INDEX_PATH,
                    "metadata.unknown_field",
                    format!("{label} uses unsupported field {field}"),
                ));
            }
        }

        let doc_id = string_field(entry, "doc_id", &label, errors)
            .unwrap_or_else(|| format!("{key}.{index}"));
        if !doc_ids.insert(doc_id.clone()) {
            errors.push(ValidationIssue::new(
                DOC_INDEX_PATH,
                "metadata.duplicate_doc_id",
                format!("duplicate doc_id {doc_id}"),
            ));
        }

        let kind = string_field(entry, "kind", &label, errors);
        if let Some(kind) = kind.as_deref() {
            if !KINDS.contains(&kind) {
                errors.push(ValidationIssue::new(
                    DOC_INDEX_PATH,
                    "metadata.invalid_kind",
                    format!("{doc_id} uses unsupported kind {kind}"),
                ));
            }
        }

        let normative_level = string_field(entry, "normative_level", &label, errors);
        if let Some(normative_level) = normative_level.as_deref() {
            if !NORMATIVE_LEVELS.contains(&normative_level) {
                errors.push(ValidationIssue::new(
                    DOC_INDEX_PATH,
                    "metadata.invalid_normative_level",
                    format!("{doc_id} uses unsupported normative_level {normative_level}"),
                ));
            }
        }

        let translation_policy = mapping_get(entry, "translation_policy")
            .and_then(|_| string_field(entry, "translation_policy", &label, errors));
        if let Some(translation_policy) = translation_policy.as_deref() {
            if !TRANSLATION_POLICIES.contains(&translation_policy) {
                errors.push(ValidationIssue::new(
                    DOC_INDEX_PATH,
                    "metadata.invalid_translation_policy",
                    format!("{doc_id} uses unsupported translation_policy {translation_policy}"),
                ));
            }
        }

        for list_field in OPTIONAL_FIELDS {
            if let Some(items) = mapping_get(entry, list_field) {
                if sequence_strings(items).is_none() {
                    errors.push(ValidationIssue::new(
                        DOC_INDEX_PATH,
                        "metadata.invalid_list",
                        format!("{doc_id} field {list_field} must be a list of strings"),
                    ));
                }
            }
        }

        if let Some(journeys_value) = mapping_get(entry, "journeys") {
            if let Some(journeys) = sequence_strings(journeys_value) {
                for journey in journeys {
                    if !READER_JOURNEYS.contains(&journey.as_str()) {
                        errors.push(ValidationIssue::new(
                            DOC_INDEX_PATH,
                            "metadata.invalid_journey",
                            format!("{doc_id} uses unsupported journey {journey}"),
                        ));
                    }
                }
            }
        }

        let owner_area = string_field(entry, "owner_area", &label, errors);
        if let Some(owner_area) = owner_area.as_deref() {
            if !owner_areas.contains(owner_area) {
                errors.push(ValidationIssue::new(
                    DOC_INDEX_PATH,
                    "metadata.invalid_owner_area",
                    format!("{doc_id} uses unknown owner_area {owner_area}"),
                ));
            }
        }

        let created_on = date_field(entry, "created_on", &label, errors);
        let last_updated_on = date_field(entry, "last_updated_on", &label, errors);
        let last_verified_on = date_field(entry, "last_verified_on", &label, errors);
        if let (Some(created_on), Some(last_updated_on), Some(last_verified_on)) =
            (created_on, last_updated_on, last_verified_on)
        {
            if created_on > last_updated_on {
                errors.push(ValidationIssue::new(
                    DOC_INDEX_PATH,
                    "metadata.invalid_date_order",
                    format!("{doc_id} has created_on after last_updated_on"),
                ));
            }
            if last_updated_on > last_verified_on {
                errors.push(ValidationIssue::new(
                    DOC_INDEX_PATH,
                    "metadata.invalid_date_order",
                    format!("{doc_id} has last_updated_on after last_verified_on"),
                ));
            }
        }

        validate_applies_to(entry, &doc_id, applicability, default_applicability, errors);

        let mut paired_document = None;
        let paths = match mode {
            EntryMode::Shared => string_field(entry, "path", &label, errors)
                .into_iter()
                .collect::<Vec<_>>(),
            EntryMode::Paired => {
                let path_en = string_field(entry, "path_en", &label, errors);
                let path_ko = string_field(entry, "path_ko", &label, errors);
                if let (Some(path_en), Some(path_ko)) = (&path_en, &path_ko) {
                    validate_paired_paths(&doc_id, path_en, path_ko, errors);
                    paired_paths.insert(path_en.clone(), (path_en.clone(), path_ko.clone()));
                    paired_paths.insert(path_ko.clone(), (path_en.clone(), path_ko.clone()));
                    paired_document = Some(PairedDocument {
                        doc_id: doc_id.clone(),
                        path_en: path_en.clone(),
                        path_ko: path_ko.clone(),
                        contract_sources: BTreeSet::new(),
                    });
                }
                path_en.into_iter().chain(path_ko).collect::<Vec<_>>()
            }
        };

        for path in &paths {
            validate_indexed_path(root, &doc_id, path, indexed_paths, errors);
            path_doc_ids.insert(path.clone(), doc_id.clone());
        }

        if let Some(paired_document) = paired_document {
            paired_documents.insert(doc_id.clone(), paired_document);
        }

        let depends_on = mapping_get(entry, "depends_on")
            .and_then(sequence_strings)
            .unwrap_or_default();

        entries.push(DocEntry { doc_id, depends_on });
    }
}

fn date_field(
    entry: &Mapping,
    key: &str,
    label: &str,
    errors: &mut Vec<ValidationIssue>,
) -> Option<IsoDate> {
    let value = string_field(entry, key, label, errors)?;
    match parse_iso_date(&value) {
        Ok(date) => Some(date),
        Err(error) => {
            errors.push(ValidationIssue::new(
                DOC_INDEX_PATH,
                error.category,
                format!("{label} field {key} {message}", message = error.message),
            ));
            None
        }
    }
}

fn parse_iso_date(value: &str) -> std::result::Result<IsoDate, DateError> {
    if value.len() != 10
        || value.as_bytes().get(4) != Some(&b'-')
        || value.as_bytes().get(7) != Some(&b'-')
        || !value
            .chars()
            .enumerate()
            .all(|(index, character)| matches!(index, 4 | 7) || character.is_ascii_digit())
    {
        return Err(DateError {
            category: "metadata.invalid_date_syntax",
            message: format!("must use YYYY-MM-DD, found {value}"),
        });
    }

    let year = value[0..4].parse::<u16>().map_err(|_| DateError {
        category: "metadata.invalid_date_syntax",
        message: format!("must use YYYY-MM-DD, found {value}"),
    })?;
    let month = value[5..7].parse::<u8>().map_err(|_| DateError {
        category: "metadata.invalid_date_syntax",
        message: format!("must use YYYY-MM-DD, found {value}"),
    })?;
    let day = value[8..10].parse::<u8>().map_err(|_| DateError {
        category: "metadata.invalid_date_syntax",
        message: format!("must use YYYY-MM-DD, found {value}"),
    })?;

    if year == 0 || month == 0 || month > 12 {
        return Err(DateError {
            category: "metadata.invalid_date_calendar",
            message: format!("is not a valid calendar date: {value}"),
        });
    }
    let max_day = days_in_month(year, month);
    if day == 0 || day > max_day {
        return Err(DateError {
            category: "metadata.invalid_date_calendar",
            message: format!("is not a valid calendar date: {value}"),
        });
    }

    Ok(IsoDate { year, month, day })
}

fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap_year(year: u16) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn validate_applies_to(
    entry: &Mapping,
    doc_id: &str,
    applicability: &BTreeSet<String>,
    default_applicability: &BTreeSet<String>,
    errors: &mut Vec<ValidationIssue>,
) {
    let Some(value) = mapping_get(entry, "applies_to") else {
        return;
    };
    let Some(items) = sequence_strings(value) else {
        errors.push(ValidationIssue::new(
            DOC_INDEX_PATH,
            "metadata.invalid_list",
            format!("{doc_id} field applies_to must be a list of strings"),
        ));
        return;
    };

    if items.is_empty() {
        errors.push(ValidationIssue::new(
            DOC_INDEX_PATH,
            "metadata.invalid_applies_to",
            format!("{doc_id} field applies_to must not be empty"),
        ));
    }

    let mut seen = BTreeSet::new();
    for item in items {
        if !seen.insert(item.clone()) {
            errors.push(ValidationIssue::new(
                DOC_INDEX_PATH,
                "metadata.duplicate_applicability",
                format!("{doc_id} repeats applies_to value {item}"),
            ));
        }
        if !applicability.contains(&item) {
            errors.push(ValidationIssue::new(
                DOC_INDEX_PATH,
                "metadata.invalid_applicability",
                format!("{doc_id} uses unknown applies_to value {item}"),
            ));
        }
        if default_applicability.contains(&item) {
            errors.push(ValidationIssue::new(
                DOC_INDEX_PATH,
                "metadata.redundant_applicability",
                format!("{doc_id} repeats default_applicability value {item} in applies_to"),
            ));
        }
    }
}

fn validate_paired_paths(
    doc_id: &str,
    path_en: &str,
    path_ko: &str,
    errors: &mut Vec<ValidationIssue>,
) {
    if is_mirrored_docs_pair(path_en, path_ko) || is_root_readme_pair(path_en, path_ko) {
        return;
    }

    errors.push(ValidationIssue::new(
        DOC_INDEX_PATH,
        "coverage.unmirrored_pair",
        format!("{doc_id} does not use mirrored language-relative paths: {path_en} <-> {path_ko}"),
    ));
}

fn is_mirrored_docs_pair(path_en: &str, path_ko: &str) -> bool {
    let en_relative = path_en.strip_prefix("docs/en/");
    let ko_relative = path_ko.strip_prefix("docs/ko/");
    matches!((en_relative, ko_relative), (Some(en), Some(ko)) if en == ko)
}

fn is_root_readme_pair(path_en: &str, path_ko: &str) -> bool {
    path_en == ROOT_README_EN_PATH && path_ko == ROOT_README_KO_PATH
}

fn validate_indexed_path(
    root: &Path,
    doc_id: &str,
    path: &str,
    indexed_paths: &mut BTreeSet<String>,
    errors: &mut Vec<ValidationIssue>,
) {
    if path.starts_with('/') || path.contains('\\') || path.split('/').any(|part| part == "..") {
        errors.push(ValidationIssue::new(
            DOC_INDEX_PATH,
            "metadata.invalid_path",
            format!("{doc_id} uses non repository-relative path {path}"),
        ));
        return;
    }

    if !indexed_paths.insert(path.to_string()) {
        errors.push(ValidationIssue::new(
            DOC_INDEX_PATH,
            "metadata.duplicate_path",
            format!("indexed path appears more than once: {path}"),
        ));
    }

    if !root.join(path).exists() {
        errors.push(ValidationIssue::new(
            DOC_INDEX_PATH,
            "metadata.missing_path",
            format!("{doc_id} indexed path does not exist: {path}"),
        ));
    }
}

fn string_field(
    entry: &Mapping,
    key: &str,
    label: &str,
    errors: &mut Vec<ValidationIssue>,
) -> Option<String> {
    let value = mapping_get(entry, key)?;
    match value.as_str() {
        Some(value) => Some(value.to_string()),
        None => {
            errors.push(ValidationIssue::new(
                DOC_INDEX_PATH,
                "metadata.invalid_field_type",
                format!("{label} field {key} must be a string"),
            ));
            None
        }
    }
}

fn sequence_strings(value: &Value) -> Option<Vec<String>> {
    value
        .as_sequence()?
        .iter()
        .map(|item| item.as_str().map(ToOwned::to_owned))
        .collect()
}

pub(crate) fn validate_document_coverage(
    root: &Path,
    index: &DocIndex,
    errors: &mut Vec<ValidationIssue>,
) {
    let en_files = markdown_files_under(root, "docs/en", errors);
    let ko_files = markdown_files_under(root, "docs/ko", errors);
    let ko_set: BTreeSet<_> = ko_files.iter().cloned().collect();
    let en_set: BTreeSet<_> = en_files.iter().cloned().collect();

    for en_path in en_files {
        let Some(relative) = en_path.strip_prefix("docs/en/") else {
            continue;
        };
        let ko_path = format!("docs/ko/{relative}");
        if !ko_set.contains(&ko_path) {
            errors.push(ValidationIssue::new(
                &en_path,
                "coverage.missing_pair",
                format!("missing Korean paired file {ko_path}"),
            ));
            continue;
        }
        if !index.paired_paths.contains_key(&en_path) {
            errors.push(ValidationIssue::new(
                &en_path,
                "coverage.unindexed_pair",
                format!("English maintained Markdown file is not indexed with pair {ko_path}"),
            ));
        }
    }

    for ko_path in ko_files {
        let Some(relative) = ko_path.strip_prefix("docs/ko/") else {
            continue;
        };
        let en_path = format!("docs/en/{relative}");
        if !en_set.contains(&en_path) {
            errors.push(ValidationIssue::new(
                &ko_path,
                "coverage.missing_pair",
                format!("missing English paired file {en_path}"),
            ));
            continue;
        }
        if !index.paired_paths.contains_key(&ko_path) {
            errors.push(ValidationIssue::new(
                &ko_path,
                "coverage.unindexed_pair",
                format!("Korean maintained Markdown file is not indexed with pair {en_path}"),
            ));
        }
    }

    validate_root_readme_pair_coverage(root, index, errors);
}

fn validate_root_readme_pair_coverage(
    root: &Path,
    index: &DocIndex,
    errors: &mut Vec<ValidationIssue>,
) {
    if !root.join(ROOT_README_KO_PATH).exists() {
        return;
    }

    let indexed_as_root_pair = matches!(
        index.paired_paths.get(ROOT_README_KO_PATH),
        Some((path_en, path_ko)) if is_root_readme_pair(path_en, path_ko)
    );
    if !indexed_as_root_pair {
        errors.push(ValidationIssue::new(
            ROOT_README_KO_PATH,
            "coverage.unindexed_pair",
            format!(
                "{ROOT_README_KO_PATH} must be indexed with root README pair {ROOT_README_EN_PATH} <-> {ROOT_README_KO_PATH}"
            ),
        ));
    }
}

fn markdown_files_under(
    root: &Path,
    relative_dir: &str,
    errors: &mut Vec<ValidationIssue>,
) -> Vec<String> {
    let mut files = Vec::new();
    collect_markdown_files(root, &root.join(relative_dir), &mut files, errors);
    files.sort();
    files
}

fn collect_markdown_files(
    root: &Path,
    dir: &Path,
    files: &mut Vec<String>,
    errors: &mut Vec<ValidationIssue>,
) {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) => {
            errors.push(ValidationIssue::new(
                repo_relative(root, dir),
                "coverage.read_dir",
                format!("failed to read documentation directory: {error}"),
            ));
            return;
        }
    };

    let mut entries = entries.filter_map(Result::ok).collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect_markdown_files(root, &path, files, errors);
        } else if path.extension().is_some_and(|extension| extension == "md") {
            files.push(repo_relative(root, &path));
        }
    }
}
