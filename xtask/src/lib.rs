use anyhow::{Context, Result};
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use serde_yaml::{Mapping, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};

const DOC_INDEX_PATH: &str = "docs/doc-index.yaml";
const TERMINOLOGY_MAP_PATH: &str = "docs/terminology-map.yaml";

const TOP_LEVEL_REQUIRED: &[&str] = &[
    "version",
    "metadata",
    "language_retrieval",
    "owner_areas",
    "applicability",
    "entry_schema",
    "shared_documents",
    "documents",
];
const TOP_LEVEL_ALLOWED: &[&str] = TOP_LEVEL_REQUIRED;
const CATALOG_ENTRY_ALLOWED: &[&str] = &["description"];
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
    "applies_to",
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
    "applies_to",
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
const LEGACY_FIELDS: &[&str] = &["role", "owner_for", "not_owner_for", "audience"];
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
const REQUIRED_SHARED_PATHS: &[&str] = &[
    "AGENTS.md",
    "docs/AGENTS.md",
    "crates/AGENTS.md",
    "README.md",
    "docs/README.md",
    "docs/doc-index.yaml",
    "docs/terminology-map.yaml",
];
const RETIRED_EXACT_PATHS: &[&str] = &["docs/en/start.md", "docs/ko/start.md"];
const RETIRED_PREFIXES: &[&str] = &[
    "docs/en/use/",
    "docs/ko/use/",
    "docs/en/build/",
    "docs/ko/build/",
];

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ValidationError {
    file: String,
    category: &'static str,
    message: String,
}

impl ValidationError {
    fn new(file: impl Into<String>, category: &'static str, message: impl Into<String>) -> Self {
        Self {
            file: file.into(),
            category,
            message: message.into(),
        }
    }

    pub fn file(&self) -> &str {
        &self.file
    }

    pub fn category(&self) -> &'static str {
        self.category
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl Ord for ValidationError {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (&self.file, self.category, &self.message).cmp(&(
            &other.file,
            other.category,
            &other.message,
        ))
    }
}

impl PartialOrd for ValidationError {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}: {}", self.file, self.category, self.message)
    }
}

#[derive(Debug, Clone)]
pub struct CheckReport {
    errors: Vec<ValidationError>,
}

impl CheckReport {
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn errors(&self) -> &[ValidationError] {
        &self.errors
    }
}

#[derive(Debug, Clone)]
struct DocIndex {
    indexed_paths: BTreeSet<String>,
    paired_paths: BTreeMap<String, (String, String)>,
    path_doc_ids: BTreeMap<String, String>,
    paired_documents: BTreeMap<String, PairedDocument>,
}

#[derive(Debug, Clone)]
struct PairedDocument {
    doc_id: String,
    path_en: String,
    path_ko: String,
}

#[derive(Debug, Clone)]
struct DocEntry {
    doc_id: String,
    depends_on: Vec<String>,
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

#[derive(Debug, Clone)]
struct LinkFailure {
    category: &'static str,
    message: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
struct SemanticLinkKey {
    target: SemanticLinkTarget,
    fragment: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
enum SemanticLinkTarget {
    DocId(String),
    RepositoryPath(String),
}

#[derive(Debug, Clone)]
struct MarkdownAnchors {
    anchors: BTreeSet<String>,
}

#[derive(Default)]
struct AnchorCache {
    files: BTreeMap<String, MarkdownAnchors>,
}

pub fn run_docs_check(root: &Path) -> Result<CheckReport> {
    let root = normalize_existing_root(root)?;
    let doc_index_path = root.join(DOC_INDEX_PATH);
    if !doc_index_path.exists() {
        anyhow::bail!(
            "docs-check must run from the repository root; missing {}",
            DOC_INDEX_PATH
        );
    }

    let mut errors = Vec::new();
    let index = validate_doc_index(&root, &mut errors);

    if let Some(index) = index.as_ref() {
        validate_document_coverage(&root, index, &mut errors);
        validate_markdown_links(&root, index, &mut errors);
        validate_bilingual_link_parity(&root, index, &mut errors);
        validate_terminology_paths(&root, index, &mut errors);
        validate_retired_paths(&root, index, &mut errors);
    }

    errors.sort();
    errors.dedup();

    Ok(CheckReport { errors })
}

fn normalize_existing_root(root: &Path) -> Result<PathBuf> {
    root.canonicalize()
        .with_context(|| format!("failed to resolve repository root {}", root.display()))
}

fn validate_doc_index(root: &Path, errors: &mut Vec<ValidationError>) -> Option<DocIndex> {
    let doc_index = root.join(DOC_INDEX_PATH);
    let contents = match fs::read_to_string(&doc_index) {
        Ok(contents) => contents,
        Err(error) => {
            errors.push(ValidationError::new(
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
            errors.push(ValidationError::new(
                DOC_INDEX_PATH,
                "metadata.yaml",
                format!("failed to parse YAML: {error}"),
            ));
            return None;
        }
    };

    let Some(top) = value.as_mapping() else {
        errors.push(ValidationError::new(
            DOC_INDEX_PATH,
            "metadata.shape",
            "doc index must be a YAML mapping",
        ));
        return None;
    };

    for field in TOP_LEVEL_REQUIRED {
        if mapping_get(top, field).is_none() {
            errors.push(ValidationError::new(
                DOC_INDEX_PATH,
                "metadata.missing_field",
                format!("doc index is missing required top-level field {field}"),
            ));
        }
    }

    for field in top.keys().filter_map(Value::as_str) {
        if !TOP_LEVEL_ALLOWED.contains(&field) {
            errors.push(ValidationError::new(
                DOC_INDEX_PATH,
                "metadata.unknown_field",
                format!("doc index uses unsupported top-level field {field}"),
            ));
        }
    }

    match mapping_get(top, "version").and_then(Value::as_i64) {
        Some(3) => {}
        Some(version) => errors.push(ValidationError::new(
            DOC_INDEX_PATH,
            "metadata.version",
            format!("expected version 3, found {version}"),
        )),
        None => errors.push(ValidationError::new(
            DOC_INDEX_PATH,
            "metadata.version",
            "missing numeric version 3",
        )),
    }

    validate_top_level_mapping(top, "metadata", errors);
    validate_top_level_mapping(top, "language_retrieval", errors);
    validate_top_level_mapping(top, "entry_schema", errors);
    let owner_areas = validate_catalog(top, "owner_areas", errors);
    let applicability = validate_catalog(top, "applicability", errors);

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
        errors,
    );

    for required_path in REQUIRED_SHARED_PATHS {
        if !indexed_paths.contains(*required_path) {
            errors.push(ValidationError::new(
                DOC_INDEX_PATH,
                "coverage.missing_shared_index",
                format!("shared maintained path is not indexed: {required_path}"),
            ));
        }
        if !root.join(required_path).exists() {
            errors.push(ValidationError::new(
                DOC_INDEX_PATH,
                "coverage.missing_shared_path",
                format!("shared maintained path does not exist: {required_path}"),
            ));
        }
    }

    for entry in &entries {
        for depends_on in &entry.depends_on {
            if !doc_ids.contains(depends_on) {
                errors.push(ValidationError::new(
                    DOC_INDEX_PATH,
                    "metadata.invalid_depends_on",
                    format!("{} depends on unknown doc_id {depends_on}", entry.doc_id),
                ));
            }
        }
    }

    Some(DocIndex {
        indexed_paths,
        paired_paths,
        path_doc_ids,
        paired_documents,
    })
}

fn validate_top_level_mapping(top: &Mapping, key: &'static str, errors: &mut Vec<ValidationError>) {
    if let Some(value) = mapping_get(top, key) {
        if !value.is_mapping() {
            errors.push(ValidationError::new(
                DOC_INDEX_PATH,
                "metadata.shape",
                format!("{key} must be a mapping"),
            ));
        }
    }
}

fn validate_catalog(
    top: &Mapping,
    key: &'static str,
    errors: &mut Vec<ValidationError>,
) -> BTreeSet<String> {
    let mut identifiers = BTreeSet::new();
    let Some(value) = mapping_get(top, key) else {
        return identifiers;
    };
    let Some(catalog) = value.as_mapping() else {
        errors.push(ValidationError::new(
            DOC_INDEX_PATH,
            "metadata.shape",
            format!("{key} must be a mapping"),
        ));
        return identifiers;
    };

    if catalog.is_empty() {
        errors.push(ValidationError::new(
            DOC_INDEX_PATH,
            "metadata.catalog",
            format!("{key} must not be empty"),
        ));
    }

    for (identifier, value) in catalog {
        let Some(identifier) = identifier.as_str() else {
            errors.push(ValidationError::new(
                DOC_INDEX_PATH,
                "metadata.catalog",
                format!("{key} identifiers must be strings"),
            ));
            continue;
        };
        if !is_catalog_identifier(identifier) {
            errors.push(ValidationError::new(
                DOC_INDEX_PATH,
                "metadata.catalog",
                format!("{key} identifier {identifier} must use lowercase letters, digits, or underscores"),
            ));
        }
        identifiers.insert(identifier.to_string());

        let Some(entry) = value.as_mapping() else {
            errors.push(ValidationError::new(
                DOC_INDEX_PATH,
                "metadata.catalog",
                format!("{key}.{identifier} must be a mapping"),
            ));
            continue;
        };
        for field in entry.keys().filter_map(Value::as_str) {
            if !CATALOG_ENTRY_ALLOWED.contains(&field) {
                errors.push(ValidationError::new(
                    DOC_INDEX_PATH,
                    "metadata.unknown_field",
                    format!("{key}.{identifier} uses unsupported field {field}"),
                ));
            }
        }
        match mapping_get(entry, "description").and_then(Value::as_str) {
            Some(description) if !description.trim().is_empty() => {}
            Some(_) => errors.push(ValidationError::new(
                DOC_INDEX_PATH,
                "metadata.catalog",
                format!("{key}.{identifier} description must not be empty"),
            )),
            None => errors.push(ValidationError::new(
                DOC_INDEX_PATH,
                "metadata.catalog",
                format!("{key}.{identifier} is missing string description"),
            )),
        }
    }

    identifiers
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
    errors: &mut Vec<ValidationError>,
) {
    let Some(value) = mapping_get(top, key) else {
        errors.push(ValidationError::new(
            DOC_INDEX_PATH,
            "metadata.shape",
            format!("missing {key} sequence"),
        ));
        return;
    };
    let Some(sequence) = value.as_sequence() else {
        errors.push(ValidationError::new(
            DOC_INDEX_PATH,
            "metadata.shape",
            format!("{key} must be a sequence"),
        ));
        return;
    };

    for (index, value) in sequence.iter().enumerate() {
        let label = format!("{key}[{index}]");
        let Some(entry) = value.as_mapping() else {
            errors.push(ValidationError::new(
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
                errors.push(ValidationError::new(
                    DOC_INDEX_PATH,
                    "metadata.missing_field",
                    format!("{label} is missing required field {field}"),
                ));
            }
        }

        for field in entry.keys().filter_map(Value::as_str) {
            if LEGACY_FIELDS.contains(&field) {
                errors.push(ValidationError::new(
                    DOC_INDEX_PATH,
                    "metadata.legacy_field",
                    format!("{label} uses retired version 1 field {field}"),
                ));
            }
            if !allowed.contains(&field) {
                errors.push(ValidationError::new(
                    DOC_INDEX_PATH,
                    "metadata.unknown_field",
                    format!("{label} uses unsupported field {field}"),
                ));
            }
        }

        let doc_id = string_field(entry, "doc_id", &label, errors)
            .unwrap_or_else(|| format!("{key}.{index}"));
        if !doc_ids.insert(doc_id.clone()) {
            errors.push(ValidationError::new(
                DOC_INDEX_PATH,
                "metadata.duplicate_doc_id",
                format!("duplicate doc_id {doc_id}"),
            ));
        }

        let kind = string_field(entry, "kind", &label, errors);
        if let Some(kind) = kind.as_deref() {
            if !KINDS.contains(&kind) {
                errors.push(ValidationError::new(
                    DOC_INDEX_PATH,
                    "metadata.invalid_kind",
                    format!("{doc_id} uses unsupported kind {kind}"),
                ));
            }
        }

        let normative_level = string_field(entry, "normative_level", &label, errors);
        if let Some(normative_level) = normative_level.as_deref() {
            if !NORMATIVE_LEVELS.contains(&normative_level) {
                errors.push(ValidationError::new(
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
                errors.push(ValidationError::new(
                    DOC_INDEX_PATH,
                    "metadata.invalid_translation_policy",
                    format!("{doc_id} uses unsupported translation_policy {translation_policy}"),
                ));
            }
        }

        for list_field in OPTIONAL_FIELDS {
            if let Some(items) = mapping_get(entry, list_field) {
                if sequence_strings(items).is_none() {
                    errors.push(ValidationError::new(
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
                        errors.push(ValidationError::new(
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
                errors.push(ValidationError::new(
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
                errors.push(ValidationError::new(
                    DOC_INDEX_PATH,
                    "metadata.invalid_date_order",
                    format!("{doc_id} has created_on after last_updated_on"),
                ));
            }
            if last_updated_on > last_verified_on {
                errors.push(ValidationError::new(
                    DOC_INDEX_PATH,
                    "metadata.invalid_date_order",
                    format!("{doc_id} has last_updated_on after last_verified_on"),
                ));
            }
        }

        validate_applies_to(entry, &doc_id, applicability, errors);

        let mut paired_document = None;
        let paths = match mode {
            EntryMode::Shared => string_field(entry, "path", &label, errors)
                .into_iter()
                .collect::<Vec<_>>(),
            EntryMode::Paired => {
                let path_en = string_field(entry, "path_en", &label, errors);
                let path_ko = string_field(entry, "path_ko", &label, errors);
                if let (Some(path_en), Some(path_ko)) = (&path_en, &path_ko) {
                    validate_mirrored_pair(&doc_id, path_en, path_ko, errors);
                    paired_paths.insert(path_en.clone(), (path_en.clone(), path_ko.clone()));
                    paired_paths.insert(path_ko.clone(), (path_en.clone(), path_ko.clone()));
                    paired_document = Some(PairedDocument {
                        doc_id: doc_id.clone(),
                        path_en: path_en.clone(),
                        path_ko: path_ko.clone(),
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
    errors: &mut Vec<ValidationError>,
) -> Option<IsoDate> {
    let value = string_field(entry, key, label, errors)?;
    match parse_iso_date(&value) {
        Ok(date) => Some(date),
        Err(error) => {
            errors.push(ValidationError::new(
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
    errors: &mut Vec<ValidationError>,
) {
    let Some(value) = mapping_get(entry, "applies_to") else {
        return;
    };
    let Some(items) = sequence_strings(value) else {
        errors.push(ValidationError::new(
            DOC_INDEX_PATH,
            "metadata.invalid_list",
            format!("{doc_id} field applies_to must be a list of strings"),
        ));
        return;
    };

    if items.is_empty() {
        errors.push(ValidationError::new(
            DOC_INDEX_PATH,
            "metadata.invalid_applies_to",
            format!("{doc_id} field applies_to must not be empty"),
        ));
    }

    let mut seen = BTreeSet::new();
    for item in items {
        if !seen.insert(item.clone()) {
            errors.push(ValidationError::new(
                DOC_INDEX_PATH,
                "metadata.duplicate_applicability",
                format!("{doc_id} repeats applies_to value {item}"),
            ));
        }
        if !applicability.contains(&item) {
            errors.push(ValidationError::new(
                DOC_INDEX_PATH,
                "metadata.invalid_applicability",
                format!("{doc_id} uses unknown applies_to value {item}"),
            ));
        }
    }
}

fn validate_mirrored_pair(
    doc_id: &str,
    path_en: &str,
    path_ko: &str,
    errors: &mut Vec<ValidationError>,
) {
    let en_relative = path_en.strip_prefix("docs/en/");
    let ko_relative = path_ko.strip_prefix("docs/ko/");
    match (en_relative, ko_relative) {
        (Some(en_relative), Some(ko_relative)) if en_relative == ko_relative => {}
        _ => errors.push(ValidationError::new(
            DOC_INDEX_PATH,
            "coverage.unmirrored_pair",
            format!(
                "{doc_id} does not use mirrored language-relative paths: {path_en} <-> {path_ko}"
            ),
        )),
    }
}

fn validate_indexed_path(
    root: &Path,
    doc_id: &str,
    path: &str,
    indexed_paths: &mut BTreeSet<String>,
    errors: &mut Vec<ValidationError>,
) {
    if path.starts_with('/') || path.contains('\\') || path.split('/').any(|part| part == "..") {
        errors.push(ValidationError::new(
            DOC_INDEX_PATH,
            "metadata.invalid_path",
            format!("{doc_id} uses non repository-relative path {path}"),
        ));
        return;
    }

    if !indexed_paths.insert(path.to_string()) {
        errors.push(ValidationError::new(
            DOC_INDEX_PATH,
            "metadata.duplicate_path",
            format!("indexed path appears more than once: {path}"),
        ));
    }

    if !root.join(path).exists() {
        errors.push(ValidationError::new(
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
    errors: &mut Vec<ValidationError>,
) -> Option<String> {
    let value = mapping_get(entry, key)?;
    match value.as_str() {
        Some(value) => Some(value.to_string()),
        None => {
            errors.push(ValidationError::new(
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

fn validate_document_coverage(root: &Path, index: &DocIndex, errors: &mut Vec<ValidationError>) {
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
            errors.push(ValidationError::new(
                &en_path,
                "coverage.missing_pair",
                format!("missing Korean paired file {ko_path}"),
            ));
            continue;
        }
        if !index.paired_paths.contains_key(&en_path) {
            errors.push(ValidationError::new(
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
            errors.push(ValidationError::new(
                &ko_path,
                "coverage.missing_pair",
                format!("missing English paired file {en_path}"),
            ));
            continue;
        }
        if !index.paired_paths.contains_key(&ko_path) {
            errors.push(ValidationError::new(
                &ko_path,
                "coverage.unindexed_pair",
                format!("Korean maintained Markdown file is not indexed with pair {en_path}"),
            ));
        }
    }
}

fn markdown_files_under(
    root: &Path,
    relative_dir: &str,
    errors: &mut Vec<ValidationError>,
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
    errors: &mut Vec<ValidationError>,
) {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) => {
            errors.push(ValidationError::new(
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

fn validate_markdown_links(root: &Path, index: &DocIndex, errors: &mut Vec<ValidationError>) {
    let mut cache = AnchorCache::default();
    for path in index
        .indexed_paths
        .iter()
        .filter(|path| path.ends_with(".md"))
    {
        let absolute_path = root.join(path);
        let contents = match fs::read_to_string(&absolute_path) {
            Ok(contents) => contents,
            Err(error) => {
                errors.push(ValidationError::new(
                    path,
                    "link.read",
                    format!("failed to read Markdown file: {error}"),
                ));
                continue;
            }
        };
        for link in markdown_links(&contents) {
            if is_ignored_link(&link) {
                continue;
            }
            if let Some(retired_path) = resolve_link_target(root, path, &link)
                .ok()
                .and_then(|resolved| retired_match(&resolved.path))
            {
                errors.push(ValidationError::new(
                    path,
                    "retired_path.reference",
                    format!("link {link} points to retired documentation path {retired_path}"),
                ));
            }
            if let Err(failure) = validate_local_target(root, path, &link, &mut cache) {
                errors.push(ValidationError::new(
                    path,
                    failure.category,
                    failure.message,
                ));
            }
        }
    }
}

fn validate_bilingual_link_parity(
    root: &Path,
    index: &DocIndex,
    errors: &mut Vec<ValidationError>,
) {
    for paired in index.paired_documents.values() {
        let en_links = match collect_semantic_links(root, index, &paired.path_en) {
            Ok(links) => links,
            Err(error) => {
                errors.push(ValidationError::new(
                    &paired.path_en,
                    "bilingual_link.read",
                    error,
                ));
                continue;
            }
        };
        let ko_links = match collect_semantic_links(root, index, &paired.path_ko) {
            Ok(links) => links,
            Err(error) => {
                errors.push(ValidationError::new(
                    &paired.path_ko,
                    "bilingual_link.read",
                    error,
                ));
                continue;
            }
        };

        compare_semantic_link_multisets(paired, en_links, ko_links, errors);
    }
}

fn collect_semantic_links(
    root: &Path,
    index: &DocIndex,
    path: &str,
) -> std::result::Result<BTreeMap<SemanticLinkKey, usize>, String> {
    let contents = fs::read_to_string(root.join(path))
        .map_err(|error| format!("failed to read Markdown file: {error}"))?;
    let mut links = BTreeMap::new();
    for link in markdown_reader_links(&contents) {
        if is_ignored_link(&link) {
            continue;
        }
        if let Some(key) = normalize_semantic_link(root, index, path, &link) {
            *links.entry(key).or_insert(0) += 1;
        }
    }
    Ok(links)
}

fn normalize_semantic_link(
    root: &Path,
    index: &DocIndex,
    source: &str,
    link: &str,
) -> Option<SemanticLinkKey> {
    let resolved = resolve_link_target(root, source, link).ok()?;
    let target_absolute = root.join(&resolved.path);
    if !target_absolute.exists() {
        return None;
    }

    let indexed_lookup_path = indexed_target_lookup_path(root, &resolved.path);
    let target = index
        .path_doc_ids
        .get(&indexed_lookup_path)
        .cloned()
        .map(SemanticLinkTarget::DocId)
        .unwrap_or_else(|| SemanticLinkTarget::RepositoryPath(resolved.path));

    Some(SemanticLinkKey {
        target,
        fragment: resolved.fragment,
    })
}

fn indexed_target_lookup_path(root: &Path, path: &str) -> String {
    let absolute = root.join(path);
    if absolute.is_dir() {
        let readme = absolute.join("README.md");
        if readme.exists() {
            return repo_relative(root, &readme);
        }
    }
    path.to_string()
}

fn compare_semantic_link_multisets(
    paired: &PairedDocument,
    en_links: BTreeMap<SemanticLinkKey, usize>,
    ko_links: BTreeMap<SemanticLinkKey, usize>,
    errors: &mut Vec<ValidationError>,
) {
    let mut only_en = multiset_difference(&en_links, &ko_links);
    let mut only_ko = multiset_difference(&ko_links, &en_links);

    report_fragment_mismatches(paired, &mut only_en, &mut only_ko, errors);
    report_target_mismatches(paired, &mut only_en, &mut only_ko, errors);
    report_unpaired_semantic_links(paired, "bilingual_link.only_en", true, only_en, errors);
    report_unpaired_semantic_links(paired, "bilingual_link.only_ko", false, only_ko, errors);
}

fn multiset_difference(
    left: &BTreeMap<SemanticLinkKey, usize>,
    right: &BTreeMap<SemanticLinkKey, usize>,
) -> BTreeMap<SemanticLinkKey, usize> {
    let mut difference = BTreeMap::new();
    for (key, left_count) in left {
        let right_count = right.get(key).copied().unwrap_or(0);
        if *left_count > right_count {
            difference.insert(key.clone(), left_count - right_count);
        }
    }
    difference
}

fn report_fragment_mismatches(
    paired: &PairedDocument,
    only_en: &mut BTreeMap<SemanticLinkKey, usize>,
    only_ko: &mut BTreeMap<SemanticLinkKey, usize>,
    errors: &mut Vec<ValidationError>,
) {
    let en_keys = only_en.keys().cloned().collect::<Vec<_>>();
    for en_key in en_keys {
        while count_for(only_en, &en_key) > 0 {
            let Some(ko_key) = only_ko
                .keys()
                .find(|ko_key| ko_key.target == en_key.target && ko_key.fragment != en_key.fragment)
                .cloned()
            else {
                break;
            };
            let count = count_for(only_en, &en_key).min(count_for(only_ko, &ko_key));
            consume_count(only_en, &en_key, count);
            consume_count(only_ko, &ko_key, count);
            errors.push(ValidationError::new(
                &paired.path_en,
                "bilingual_link.fragment_mismatch",
                format!(
                    "{} has {count} paired local semantic link occurrence(s) to {} but different fragments: English {}, Korean {} ({} <-> {})",
                    paired.doc_id,
                    en_key.target.describe(),
                    describe_fragment(&en_key.fragment),
                    describe_fragment(&ko_key.fragment),
                    paired.path_en,
                    paired.path_ko
                ),
            ));
        }
    }
}

fn report_target_mismatches(
    paired: &PairedDocument,
    only_en: &mut BTreeMap<SemanticLinkKey, usize>,
    only_ko: &mut BTreeMap<SemanticLinkKey, usize>,
    errors: &mut Vec<ValidationError>,
) {
    let en_keys = only_en.keys().cloned().collect::<Vec<_>>();
    for en_key in en_keys {
        while count_for(only_en, &en_key) > 0 {
            let Some(ko_key) = only_ko
                .keys()
                .find(|ko_key| ko_key.fragment == en_key.fragment && ko_key.target != en_key.target)
                .cloned()
            else {
                break;
            };
            let count = count_for(only_en, &en_key).min(count_for(only_ko, &ko_key));
            consume_count(only_en, &en_key, count);
            consume_count(only_ko, &ko_key, count);
            errors.push(ValidationError::new(
                &paired.path_en,
                "bilingual_link.target_mismatch",
                format!(
                    "{} has {count} paired local semantic link occurrence(s) with {} but different normalized targets: English {}, Korean {} ({} <-> {})",
                    paired.doc_id,
                    describe_fragment(&en_key.fragment),
                    en_key.target.describe(),
                    ko_key.target.describe(),
                    paired.path_en,
                    paired.path_ko
                ),
            ));
        }
    }
}

fn report_unpaired_semantic_links(
    paired: &PairedDocument,
    category: &'static str,
    english_surplus: bool,
    links: BTreeMap<SemanticLinkKey, usize>,
    errors: &mut Vec<ValidationError>,
) {
    for (key, count) in links {
        let language = if english_surplus { "English" } else { "Korean" };
        let paired_language = if english_surplus { "Korean" } else { "English" };
        errors.push(ValidationError::new(
            &paired.path_en,
            category,
            format!(
                "{} has {count} more {language} occurrence(s) of local semantic link to {} than {paired_language} ({} <-> {})",
                paired.doc_id,
                key.describe(),
                paired.path_en,
                paired.path_ko
            ),
        ));
    }
}

fn count_for(links: &BTreeMap<SemanticLinkKey, usize>, key: &SemanticLinkKey) -> usize {
    links.get(key).copied().unwrap_or(0)
}

fn consume_count(
    links: &mut BTreeMap<SemanticLinkKey, usize>,
    key: &SemanticLinkKey,
    count: usize,
) {
    if let Some(current) = links.get_mut(key) {
        *current -= count;
        if *current == 0 {
            links.remove(key);
        }
    }
}

impl SemanticLinkKey {
    fn describe(&self) -> String {
        match &self.fragment {
            Some(fragment) => format!("{}#{fragment}", self.target.describe()),
            None => format!("{} without fragment", self.target.describe()),
        }
    }
}

impl SemanticLinkTarget {
    fn describe(&self) -> String {
        match self {
            SemanticLinkTarget::DocId(doc_id) => format!("target {doc_id}"),
            SemanticLinkTarget::RepositoryPath(path) => format!("repository path {path}"),
        }
    }
}

fn describe_fragment(fragment: &Option<String>) -> String {
    match fragment {
        Some(fragment) => format!("#{fragment}"),
        None => "no fragment".to_string(),
    }
}

fn markdown_links(contents: &str) -> Vec<String> {
    markdown_destinations(contents, true)
}

fn markdown_reader_links(contents: &str) -> Vec<String> {
    markdown_destinations(contents, false)
}

fn markdown_destinations(contents: &str, include_images: bool) -> Vec<String> {
    let mut links = Vec::new();
    let parser = Parser::new_ext(contents, markdown_options());
    for event in parser {
        match event {
            Event::Start(Tag::Link { dest_url, .. }) => {
                links.push(dest_url.to_string());
            }
            Event::Start(Tag::Image { dest_url, .. }) if include_images => {
                links.push(dest_url.to_string());
            }
            _ => {}
        }
    }
    links
}

fn validate_local_target(
    root: &Path,
    source: &str,
    link: &str,
    cache: &mut AnchorCache,
) -> std::result::Result<(), LinkFailure> {
    let resolved = resolve_link_target(root, source, link).map_err(|message| LinkFailure {
        category: "link.invalid_target",
        message,
    })?;

    let target_absolute = root.join(&resolved.path);
    if !target_absolute.exists() {
        return Err(LinkFailure {
            category: "link.missing_target",
            message: format!("link {link} resolves to missing target {}", resolved.path),
        });
    }

    if let Some(fragment) = resolved.fragment {
        let anchor_path = if target_absolute.is_dir() {
            let readme = target_absolute.join("README.md");
            if readme.exists() {
                repo_relative(root, &readme)
            } else {
                return Err(LinkFailure {
                    category: "link.missing_fragment",
                    message: format!(
                        "link {link} has fragment #{fragment}, but {} is a directory without README.md",
                        resolved.path
                    ),
                });
            }
        } else {
            resolved.path.clone()
        };

        if !anchor_path.ends_with(".md") {
            return Err(LinkFailure {
                category: "link.missing_fragment",
                message: format!(
                    "link {link} has fragment #{fragment}, but {anchor_path} is not Markdown"
                ),
            });
        }

        let anchors = cache
            .anchors_for(root, &anchor_path)
            .map_err(|message| LinkFailure {
                category: "link.read",
                message,
            })?;
        if !anchors.contains_fragment(&fragment) {
            return Err(LinkFailure {
                category: "link.missing_fragment",
                message: format!(
                    "link {link} resolves to {anchor_path} without fragment #{fragment}"
                ),
            });
        }
    }

    Ok(())
}

#[derive(Debug, Clone)]
struct ResolvedLink {
    path: String,
    fragment: Option<String>,
}

fn resolve_link_target(
    root: &Path,
    source: &str,
    link: &str,
) -> std::result::Result<ResolvedLink, String> {
    let (path_part, fragment) = split_link(link);
    let path_part = percent_decode(&path_part)
        .map_err(|error| format!("link {link} has invalid percent encoding: {error}"))?;
    let fragment = fragment
        .map(|fragment| {
            percent_decode(&fragment).map(|decoded| decoded.trim_start_matches('#').to_string())
        })
        .transpose()
        .map_err(|error| format!("link {link} has invalid fragment percent encoding: {error}"))?;

    let source_parent = Path::new(source).parent().unwrap_or_else(|| Path::new(""));
    let joined = if path_part.is_empty() {
        root.join(source)
    } else if let Some(stripped) = path_part.strip_prefix('/') {
        root.join(stripped)
    } else {
        root.join(source_parent).join(path_part)
    };
    let normalized = normalize_path(&joined);
    let relative = normalized
        .strip_prefix(root)
        .map_err(|_| format!("link {link} resolves outside the repository"))?;

    Ok(ResolvedLink {
        path: path_to_slash(relative),
        fragment,
    })
}

fn split_link(link: &str) -> (String, Option<String>) {
    let without_query = link.split('?').next().unwrap_or(link);
    match without_query.split_once('#') {
        Some((path, fragment)) => (path.to_string(), Some(fragment.to_string())),
        None => (without_query.to_string(), None),
    }
}

impl AnchorCache {
    fn anchors_for(
        &mut self,
        root: &Path,
        path: &str,
    ) -> std::result::Result<&MarkdownAnchors, String> {
        if !self.files.contains_key(path) {
            let contents = fs::read_to_string(root.join(path))
                .map_err(|error| format!("failed to read Markdown target {path}: {error}"))?;
            let anchors = collect_markdown_anchors(&contents);
            self.files.insert(path.to_string(), anchors);
        }
        Ok(self.files.get(path).expect("anchor cache entry inserted"))
    }
}

impl MarkdownAnchors {
    fn contains_fragment(&self, fragment: &str) -> bool {
        self.anchors.contains(fragment)
            || fragment
                .strip_prefix("user-content-")
                .is_some_and(|stripped| self.anchors.contains(stripped))
    }
}

fn collect_markdown_anchors(contents: &str) -> MarkdownAnchors {
    let mut anchors = BTreeSet::new();
    let mut slug_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut heading_text = String::new();
    let mut in_heading = false;

    for event in Parser::new_ext(contents, markdown_options()) {
        match event {
            Event::Start(Tag::Heading { id, .. }) => {
                in_heading = true;
                heading_text.clear();
                if let Some(id) = id {
                    anchors.insert(id.to_string());
                }
            }
            Event::End(TagEnd::Heading(_)) => {
                in_heading = false;
                let base = slugify_heading(&heading_text);
                if !base.is_empty() {
                    let count = slug_counts.entry(base.clone()).or_insert(0);
                    let anchor = if *count == 0 {
                        base
                    } else {
                        format!("{base}-{count}")
                    };
                    *count += 1;
                    anchors.insert(anchor);
                }
            }
            Event::Text(text) | Event::Code(text) if in_heading => {
                heading_text.push_str(&text);
            }
            Event::Html(html) | Event::InlineHtml(html) => {
                for id in html_anchor_ids(&html) {
                    anchors.insert(id);
                }
            }
            _ => {}
        }
    }

    MarkdownAnchors { anchors }
}

fn markdown_options() -> Options {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_HEADING_ATTRIBUTES);
    options
}

fn slugify_heading(heading: &str) -> String {
    let mut slug = String::new();
    let mut previous_dash = false;

    for character in heading.trim().chars() {
        for lower in character.to_lowercase() {
            if lower.is_alphanumeric() {
                slug.push(lower);
                previous_dash = false;
            } else if lower.is_whitespace() || lower == '-' {
                if !previous_dash && !slug.is_empty() {
                    slug.push('-');
                    previous_dash = true;
                }
            } else if lower == '_' {
                slug.push(lower);
                previous_dash = false;
            }
        }
    }

    slug.trim_matches('-').to_string()
}

fn html_anchor_ids(html: &str) -> Vec<String> {
    let mut ids = Vec::new();
    ids.extend(html_attribute_values(html, "id"));
    if html.trim_start().to_ascii_lowercase().starts_with("<a") {
        ids.extend(html_attribute_values(html, "name"));
    }
    ids
}

fn html_attribute_values(html: &str, attribute: &str) -> Vec<String> {
    let lower = html.to_ascii_lowercase();
    let mut values = Vec::new();
    let mut search_start = 0;
    let needle = format!("{attribute}=");

    while let Some(offset) = lower[search_start..].find(&needle) {
        let value_start = search_start + offset + needle.len();
        let Some(quote) = html[value_start..].chars().next() else {
            break;
        };
        if quote != '"' && quote != '\'' {
            search_start = value_start;
            continue;
        }
        let content_start = value_start + quote.len_utf8();
        let Some(end_offset) = html[content_start..].find(quote) else {
            break;
        };
        values.push(html[content_start..content_start + end_offset].to_string());
        search_start = content_start + end_offset + quote.len_utf8();
    }

    values
}

fn validate_terminology_paths(root: &Path, index: &DocIndex, errors: &mut Vec<ValidationError>) {
    let path = root.join(TERMINOLOGY_MAP_PATH);
    let contents = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(error) => {
            errors.push(ValidationError::new(
                TERMINOLOGY_MAP_PATH,
                "terminology.read",
                format!("failed to read terminology map: {error}"),
            ));
            return;
        }
    };
    let value: Value = match serde_yaml::from_str(&contents) {
        Ok(value) => value,
        Err(error) => {
            errors.push(ValidationError::new(
                TERMINOLOGY_MAP_PATH,
                "terminology.yaml",
                format!("failed to parse YAML: {error}"),
            ));
            return;
        }
    };

    let mut mentions = BTreeSet::new();
    collect_yaml_path_mentions(&value, &mut mentions);

    let mut cache = AnchorCache::default();
    for mention in mentions {
        if let Err(failure) = validate_terminology_target(root, index, &mention, &mut cache) {
            errors.push(ValidationError::new(
                TERMINOLOGY_MAP_PATH,
                failure.category,
                failure.message,
            ));
        }
    }
}

fn validate_terminology_target(
    root: &Path,
    index: &DocIndex,
    mention: &str,
    cache: &mut AnchorCache,
) -> std::result::Result<(), LinkFailure> {
    let (path, fragment) = split_link(mention);
    let path = percent_decode(&path).map_err(|error| LinkFailure {
        category: "terminology.invalid_target",
        message: format!("path {mention} has invalid percent encoding: {error}"),
    })?;
    if path.contains('{') || path.contains('}') || path.contains('*') {
        return Ok(());
    }
    if !is_repository_document_path(&path) {
        return Ok(());
    }
    let normalized = normalize_path(&PathBuf::from(&path));
    let path = path_to_slash(&normalized);
    if !root.join(&path).exists() {
        return Err(LinkFailure {
            category: "terminology.missing_target",
            message: format!("path reference does not exist: {mention}"),
        });
    }
    if !index.indexed_paths.contains(&path) {
        return Err(LinkFailure {
            category: "terminology.unindexed_target",
            message: format!("path reference is not indexed in docs/doc-index.yaml: {mention}"),
        });
    }
    if let Some(fragment) = fragment {
        let fragment = percent_decode(&fragment).map_err(|error| LinkFailure {
            category: "terminology.invalid_target",
            message: format!("path {mention} has invalid fragment percent encoding: {error}"),
        })?;
        if path.ends_with(".md") {
            let anchors = cache
                .anchors_for(root, &path)
                .map_err(|message| LinkFailure {
                    category: "terminology.read",
                    message,
                })?;
            if !anchors.contains_fragment(&fragment) {
                return Err(LinkFailure {
                    category: "terminology.missing_fragment",
                    message: format!(
                        "path reference {mention} points to missing fragment #{fragment}"
                    ),
                });
            }
        }
    }
    Ok(())
}

fn validate_retired_paths(root: &Path, index: &DocIndex, errors: &mut Vec<ValidationError>) {
    for path in index
        .indexed_paths
        .iter()
        .filter(|path| path.ends_with(".md") || path.ends_with(".yaml") || path.ends_with(".yml"))
    {
        if path.ends_with(".md") {
            let contents = match fs::read_to_string(root.join(path)) {
                Ok(contents) => contents,
                Err(_) => continue,
            };
            for reference in markdown_retired_references(root, path, &contents) {
                errors.push(ValidationError::new(
                    path,
                    "retired_path.reference",
                    format!("references retired documentation path {reference}"),
                ));
            }
        } else {
            let contents = match fs::read_to_string(root.join(path)) {
                Ok(contents) => contents,
                Err(_) => continue,
            };
            let value: Value = match serde_yaml::from_str(&contents) {
                Ok(value) => value,
                Err(_) => continue,
            };
            let mut mentions = BTreeSet::new();
            collect_yaml_path_mentions(&value, &mut mentions);
            for mention in mentions {
                let resolved = normalize_path(&PathBuf::from(split_link(&mention).0));
                let reference = path_to_slash(&resolved);
                if let Some(retired) = retired_match(&reference) {
                    errors.push(ValidationError::new(
                        path,
                        "retired_path.reference",
                        format!("references retired documentation path {retired}"),
                    ));
                }
            }
        }
    }
}

fn markdown_retired_references(root: &Path, source: &str, contents: &str) -> BTreeSet<String> {
    let mut references = BTreeSet::new();
    let mut in_code_block = false;

    for event in Parser::new_ext(contents, markdown_options()) {
        match event {
            Event::Start(Tag::CodeBlock(_)) => in_code_block = true,
            Event::End(TagEnd::CodeBlock) => in_code_block = false,
            Event::Start(Tag::Link { dest_url, .. })
            | Event::Start(Tag::Image { dest_url, .. }) => {
                if !is_ignored_link(&dest_url) {
                    if let Ok(resolved) = resolve_link_target(root, source, &dest_url) {
                        if let Some(retired) = retired_match(&resolved.path) {
                            references.insert(retired);
                        }
                    }
                }
            }
            Event::Text(text) | Event::Html(text) | Event::InlineHtml(text) if !in_code_block => {
                for mention in path_mentions_in_text(&text) {
                    let resolved = normalize_path(&PathBuf::from(split_link(&mention).0));
                    let reference = path_to_slash(&resolved);
                    if let Some(retired) = retired_match(&reference) {
                        references.insert(retired);
                    }
                }
            }
            _ => {}
        }
    }

    references
}

fn collect_yaml_path_mentions(value: &Value, mentions: &mut BTreeSet<String>) {
    match value {
        Value::String(text) => {
            for mention in path_mentions_in_text(text) {
                mentions.insert(mention);
            }
        }
        Value::Sequence(items) => {
            for item in items {
                collect_yaml_path_mentions(item, mentions);
            }
        }
        Value::Mapping(mapping) => {
            for (key, value) in mapping {
                collect_yaml_path_mentions(key, mentions);
                collect_yaml_path_mentions(value, mentions);
            }
        }
        _ => {}
    }
}

fn path_mentions_in_text(text: &str) -> Vec<String> {
    let prefixes = ["docs/", "AGENTS.md", "README.md", "crates/AGENTS.md"];
    let mut mentions = Vec::new();
    for prefix in prefixes {
        let mut start = 0;
        while let Some(offset) = text[start..].find(prefix) {
            let mention_start = start + offset;
            let mut mention_end = mention_start;
            for (char_offset, character) in text[mention_start..].char_indices() {
                if char_offset == 0 {
                    mention_end = mention_start + character.len_utf8();
                    continue;
                }
                if character.is_whitespace()
                    || matches!(
                        character,
                        ')' | ']' | '}' | '>' | '"' | '\'' | '`' | ',' | ';'
                    )
                {
                    break;
                }
                mention_end = mention_start + char_offset + character.len_utf8();
            }
            let mention = text[mention_start..mention_end]
                .trim_matches(|character: char| {
                    matches!(
                        character,
                        '.' | ':' | ')' | ']' | '}' | '>' | '"' | '\'' | '`'
                    )
                })
                .to_string();
            if !mention.is_empty() {
                mentions.push(mention);
            }
            start = mention_end;
        }
    }
    mentions
}

fn is_repository_document_path(path: &str) -> bool {
    path == "AGENTS.md"
        || path == "README.md"
        || path == "docs/AGENTS.md"
        || path == "crates/AGENTS.md"
        || path.starts_with("docs/")
}

fn retired_match(path: &str) -> Option<String> {
    if RETIRED_EXACT_PATHS.contains(&path) {
        return Some(path.to_string());
    }
    for prefix in RETIRED_PREFIXES {
        if path == prefix.trim_end_matches('/') || path.starts_with(prefix) {
            return Some(path.to_string());
        }
    }
    None
}

fn is_ignored_link(link: &str) -> bool {
    let trimmed = link.trim();
    trimmed.is_empty() || has_uri_scheme(trimmed)
}

fn has_uri_scheme(link: &str) -> bool {
    let Some(colon_index) = link.find(':') else {
        return false;
    };
    let scheme = &link[..colon_index];
    !scheme.is_empty()
        && scheme.chars().enumerate().all(|(index, character)| {
            if index == 0 {
                character.is_ascii_alphabetic()
            } else {
                character.is_ascii_alphanumeric() || matches!(character, '+' | '-' | '.')
            }
        })
}

fn mapping_get<'a>(mapping: &'a Mapping, key: &str) -> Option<&'a Value> {
    mapping.get(Value::String(key.to_string()))
}

fn repo_relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map(path_to_slash)
        .unwrap_or_else(|_| path_to_slash(path))
}

fn path_to_slash(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value.to_string_lossy().to_string()),
            Component::CurDir => None,
            Component::ParentDir => Some("..".to_string()),
            Component::RootDir | Component::Prefix(_) => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(value) => normalized.push(value),
            Component::RootDir => normalized.push(Path::new("/")),
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
        }
    }
    normalized
}

fn percent_decode(value: &str) -> std::result::Result<String, String> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len() {
                return Err("truncated percent escape".to_string());
            }
            let high =
                hex_value(bytes[index + 1]).ok_or_else(|| "invalid percent escape".to_string())?;
            let low =
                hex_value(bytes[index + 2]).ok_or_else(|| "invalid percent escape".to_string())?;
            decoded.push(high << 4 | low);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }

    String::from_utf8(decoded).map_err(|error| error.to_string())
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
