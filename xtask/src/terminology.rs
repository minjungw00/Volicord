use crate::diagnostics::ValidationIssue;
use crate::doc_index::{DocIndex, TERMINOLOGY_MAP_PATH};
use crate::links::{split_link, AnchorCache, LinkFailure};
use crate::repository::{normalize_path, path_to_slash};
use serde_yaml::{Mapping, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

fn mapping_get<'a>(mapping: &'a Mapping, key: &str) -> Option<&'a Value> {
    mapping.get(Value::String(key.to_string()))
}

fn is_repository_document_path(path: &str) -> bool {
    path == "AGENTS.md"
        || path == "README.md"
        || path == "docs/AGENTS.md"
        || path == "crates/AGENTS.md"
        || path.starts_with("docs/")
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

const TERMINOLOGY_ALLOWED_ROLES: &[&str] = &[
    "public_user_term",
    "storage_internal_identifier",
    "storage_record",
    "mcp_process_binding",
    "diagnostic_field",
    "mcp_public_selector",
];
const REQUIRED_TERMINOLOGY_ROLES: &[RequiredTerminologyRoles] = &[
    RequiredTerminologyRoles {
        term_key: "connection_internal_id",
        display: "connection_internal_id",
        roles: &["storage_internal_identifier"],
    },
    RequiredTerminologyRoles {
        term_key: "project_internal_id",
        display: "project_internal_id",
        roles: &["storage_internal_identifier"],
    },
    RequiredTerminologyRoles {
        term_key: "connection_id",
        display: "connection_id",
        roles: &["mcp_process_binding", "diagnostic_field"],
    },
    RequiredTerminologyRoles {
        term_key: "project_id",
        display: "project_id",
        roles: &["diagnostic_field"],
    },
    RequiredTerminologyRoles {
        term_key: "project_selector",
        display: "project_selector",
        roles: &["mcp_public_selector"],
    },
    RequiredTerminologyRoles {
        term_key: "installation_profile",
        display: "installation_profile",
        roles: &["storage_record"],
    },
    RequiredTerminologyRoles {
        term_key: "volicord_runtime_home",
        display: "Volicord Runtime Home",
        roles: &["public_user_term"],
    },
];

struct RequiredTerminologyRoles {
    term_key: &'static str,
    display: &'static str,
    roles: &'static [&'static str],
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ExactIdentifierCatalog {
    pub(crate) identifiers: BTreeSet<String>,
}

pub(crate) fn validate_terminology_paths(
    root: &Path,
    index: &DocIndex,
    errors: &mut Vec<ValidationIssue>,
) {
    let path = root.join(TERMINOLOGY_MAP_PATH);
    let contents = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(error) => {
            errors.push(ValidationIssue::new(
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
            errors.push(ValidationIssue::new(
                TERMINOLOGY_MAP_PATH,
                "terminology.yaml",
                format!("failed to parse YAML: {error}"),
            ));
            return;
        }
    };

    validate_terminology_roles(&value, errors);

    let mut mentions = BTreeSet::new();
    collect_yaml_path_mentions(&value, &mut mentions);

    let mut cache = AnchorCache::default();
    for mention in mentions {
        if let Err(failure) = validate_terminology_target(root, index, &mention, &mut cache) {
            errors.push(ValidationIssue::new(
                TERMINOLOGY_MAP_PATH,
                failure.category,
                failure.message,
            ));
        }
    }
}

pub(crate) fn exact_identifier_catalog(
    terminology_path: &Path,
    issues: &mut Vec<ValidationIssue>,
) -> ExactIdentifierCatalog {
    let contents = match fs::read_to_string(terminology_path) {
        Ok(contents) => contents,
        Err(error) => {
            issues.push(ValidationIssue::new(
                TERMINOLOGY_MAP_PATH,
                "identifier_parity.terminology_read",
                format!("failed to read exact-identifier catalog: {error}"),
            ));
            return ExactIdentifierCatalog::default();
        }
    };
    let value: Value = match serde_yaml::from_str(&contents) {
        Ok(value) => value,
        Err(error) => {
            issues.push(ValidationIssue::new(
                TERMINOLOGY_MAP_PATH,
                "identifier_parity.terminology_yaml",
                format!("failed to parse exact-identifier catalog: {error}"),
            ));
            return ExactIdentifierCatalog::default();
        }
    };
    let Some(top) = value.as_mapping() else {
        return ExactIdentifierCatalog::default();
    };

    let mut catalog = ExactIdentifierCatalog::default();
    if let Some(global) = mapping_get(top, "identifier_preservation")
        .and_then(Value::as_mapping)
        .and_then(|preservation| mapping_get(preservation, "identifiers"))
    {
        extend_identifier_sequence(
            global,
            "identifier_preservation.identifiers",
            &mut catalog.identifiers,
            issues,
        );
    }

    if let Some(terms) = mapping_get(top, "terms").and_then(Value::as_mapping) {
        for (term_key, entry) in terms {
            let term_key = term_key.as_str().unwrap_or("<non-string>");
            let Some(entry) = entry.as_mapping() else {
                continue;
            };
            if let Some(values) = mapping_get(entry, "preserve_as_identifier") {
                extend_identifier_sequence(
                    values,
                    &format!("terms.{term_key}.preserve_as_identifier"),
                    &mut catalog.identifiers,
                    issues,
                );
            }
            if mapping_get(entry, "preserve_identifier").and_then(Value::as_bool) == Some(true) {
                if let Some(identifier) = mapping_get(entry, "en")
                    .and_then(Value::as_str)
                    .and_then(normalize_identifier_literal)
                {
                    catalog.identifiers.insert(identifier.to_owned());
                }
            }
        }
    }

    catalog
}

fn extend_identifier_sequence(
    value: &Value,
    field: &str,
    identifiers: &mut BTreeSet<String>,
    issues: &mut Vec<ValidationIssue>,
) {
    let Some(values) = value.as_sequence() else {
        issues.push(ValidationIssue::new(
            TERMINOLOGY_MAP_PATH,
            "identifier_parity.terminology_shape",
            format!("{field} must be a list of exact identifier strings"),
        ));
        return;
    };
    for value in values {
        let Some(identifier) = value.as_str().and_then(normalize_identifier_literal) else {
            issues.push(ValidationIssue::new(
                TERMINOLOGY_MAP_PATH,
                "identifier_parity.terminology_shape",
                format!("{field} must contain only non-empty exact identifier strings"),
            ));
            continue;
        };
        identifiers.insert(identifier.to_owned());
    }
}

fn normalize_identifier_literal(value: &str) -> Option<&str> {
    let value = value.trim();
    let value = value
        .strip_prefix('`')
        .and_then(|value| value.strip_suffix('`'))
        .unwrap_or(value);
    (!value.is_empty()).then_some(value)
}

fn validate_terminology_roles(value: &Value, errors: &mut Vec<ValidationIssue>) {
    let Some(top) = value.as_mapping() else {
        errors.push(ValidationIssue::new(
            TERMINOLOGY_MAP_PATH,
            "terminology.shape",
            "terminology map must be a YAML mapping",
        ));
        return;
    };
    let Some(terms) = mapping_get(top, "terms") else {
        errors.push(ValidationIssue::new(
            TERMINOLOGY_MAP_PATH,
            "terminology.missing_terms",
            "terminology map is missing terms",
        ));
        return;
    };
    let Some(terms) = terms.as_mapping() else {
        errors.push(ValidationIssue::new(
            TERMINOLOGY_MAP_PATH,
            "terminology.shape",
            "terminology map terms must be a mapping",
        ));
        return;
    };

    let mut role_map = BTreeMap::new();
    for (term_key, entry) in terms {
        let Some(term_key) = term_key.as_str() else {
            continue;
        };
        let Some(entry) = entry.as_mapping() else {
            continue;
        };
        let Some(roles_value) = mapping_get(entry, "roles") else {
            continue;
        };

        let mut roles = BTreeSet::new();
        match roles_value.as_sequence() {
            Some(sequence) if !sequence.is_empty() => {
                for role in sequence {
                    let Some(role) = role.as_str() else {
                        errors.push(ValidationIssue::new(
                            TERMINOLOGY_MAP_PATH,
                            "terminology.invalid_role",
                            format!("{term_key} role values must be strings"),
                        ));
                        continue;
                    };
                    if !TERMINOLOGY_ALLOWED_ROLES.contains(&role) {
                        errors.push(ValidationIssue::new(
                            TERMINOLOGY_MAP_PATH,
                            "terminology.invalid_role",
                            format!("{term_key} uses unsupported terminology role {role}"),
                        ));
                    }
                    if !roles.insert(role.to_string()) {
                        errors.push(ValidationIssue::new(
                            TERMINOLOGY_MAP_PATH,
                            "terminology.invalid_role",
                            format!("{term_key} repeats terminology role {role}"),
                        ));
                    }
                }
            }
            Some(_) => errors.push(ValidationIssue::new(
                TERMINOLOGY_MAP_PATH,
                "terminology.invalid_role",
                format!("{term_key} roles must not be empty"),
            )),
            None => errors.push(ValidationIssue::new(
                TERMINOLOGY_MAP_PATH,
                "terminology.invalid_role",
                format!("{term_key} roles must be a list"),
            )),
        }
        role_map.insert(term_key.to_string(), roles);
    }

    for required in REQUIRED_TERMINOLOGY_ROLES {
        let Some(entry) = mapping_get(terms, required.term_key) else {
            errors.push(ValidationIssue::new(
                TERMINOLOGY_MAP_PATH,
                "terminology.missing_required_term",
                format!("required terminology term {} is missing", required.display),
            ));
            continue;
        };
        if !entry.is_mapping() {
            errors.push(ValidationIssue::new(
                TERMINOLOGY_MAP_PATH,
                "terminology.shape",
                format!(
                    "required terminology term {} must be a mapping",
                    required.display
                ),
            ));
            continue;
        }
        let Some(roles) = role_map.get(required.term_key) else {
            errors.push(ValidationIssue::new(
                TERMINOLOGY_MAP_PATH,
                "terminology.missing_role",
                format!(
                    "required terminology term {} is missing roles metadata",
                    required.display
                ),
            ));
            continue;
        };
        for role in required.roles {
            if !roles.contains(*role) {
                errors.push(ValidationIssue::new(
                    TERMINOLOGY_MAP_PATH,
                    "terminology.missing_role",
                    format!(
                        "required terminology term {} is missing role {}",
                        required.display, role
                    ),
                ));
            }
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
