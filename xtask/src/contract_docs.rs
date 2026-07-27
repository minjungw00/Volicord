use crate::diagnostics::ValidationIssue;
use crate::doc_index::{DocIndex, PairedDocument};
use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::ops::Range;
use std::path::Path;
use volicord_types::contracts::{public_json_contract_descriptors, JsonContractDescriptor};

const METHOD_DOC_PREFIX: &str = "reference.api.method-";

#[derive(Copy, Clone)]
enum Language {
    English,
    Korean,
}

pub(crate) fn sync_generated_contract_tables(root: &Path, index: &DocIndex) -> Result<Vec<String>> {
    let descriptors = request_descriptors();
    let mut updated_paths = Vec::new();
    for paired in method_documents(index) {
        let requests = document_requests(paired, &descriptors)?;
        for (path, language) in [
            (&paired.path_en, Language::English),
            (&paired.path_ko, Language::Korean),
        ] {
            let absolute = root.join(path);
            let contents = fs::read_to_string(&absolute)
                .with_context(|| format!("failed to read generated contract owner at {path}"))?;
            let expected = render_contract_tables(&requests, language)?;
            let range = contract_table_range(&contents, &requests, language)
                .with_context(|| format!("invalid generated contract table region in {path}"))?;
            if contents[range.clone()] == expected {
                continue;
            }
            let mut updated = String::with_capacity(contents.len() + expected.len());
            updated.push_str(&contents[..range.start]);
            updated.push_str(&expected);
            updated.push_str(&contents[range.end..]);
            fs::write(&absolute, updated)
                .with_context(|| format!("failed to update generated contract owner at {path}"))?;
            updated_paths.push(path.clone());
        }
    }
    Ok(updated_paths)
}

pub(crate) fn validate_generated_contract_tables(
    root: &Path,
    index: &DocIndex,
    issues: &mut Vec<ValidationIssue>,
) {
    let descriptors = request_descriptors();
    for paired in method_documents(index) {
        let requests = match document_requests(paired, &descriptors) {
            Ok(requests) => requests,
            Err(error) => {
                issues.push(ValidationIssue::new(
                    "docs/doc-index.yaml",
                    "generated_contract.owner",
                    error.to_string(),
                ));
                continue;
            }
        };
        for (path, language) in [
            (&paired.path_en, Language::English),
            (&paired.path_ko, Language::Korean),
        ] {
            let contents = match fs::read_to_string(root.join(path)) {
                Ok(contents) => contents,
                Err(error) => {
                    issues.push(ValidationIssue::new(
                        path,
                        "generated_contract.read",
                        format!("failed to read generated contract owner: {error}"),
                    ));
                    continue;
                }
            };
            let expected = match render_contract_tables(&requests, language) {
                Ok(expected) => expected,
                Err(error) => {
                    issues.push(ValidationIssue::new(
                        path,
                        "generated_contract.owner",
                        error.to_string(),
                    ));
                    continue;
                }
            };
            let range = match contract_table_range(&contents, &requests, language) {
                Ok(range) => range,
                Err(error) => {
                    issues.push(ValidationIssue::new(
                        path,
                        "generated_contract.region",
                        error.to_string(),
                    ));
                    continue;
                }
            };
            if contents[range] != expected {
                issues.push(ValidationIssue::new(
                    path,
                    "generated_contract.drift",
                    "generated request field tables differ from their semantic contract descriptors; run `cargo run -p xtask -- docs-sync`",
                ));
            }
        }
    }
}

fn request_descriptors() -> BTreeMap<String, JsonContractDescriptor> {
    public_json_contract_descriptors()
        .into_iter()
        .filter(|descriptor| {
            descriptor.id().starts_with("api.method.") && descriptor.id().ends_with(".request")
        })
        .map(|descriptor| (descriptor.id().to_owned(), descriptor))
        .collect()
}

fn method_documents(index: &DocIndex) -> impl Iterator<Item = &PairedDocument> {
    index
        .paired_documents
        .values()
        .filter(|paired| paired.doc_id.starts_with(METHOD_DOC_PREFIX))
}

fn document_requests<'a>(
    paired: &PairedDocument,
    descriptors: &'a BTreeMap<String, JsonContractDescriptor>,
) -> Result<Vec<&'a JsonContractDescriptor>> {
    let route = paired
        .doc_id
        .strip_prefix(METHOD_DOC_PREFIX)
        .context("method document route has no method suffix")?
        .replace('-', "_");
    let methods = if route == "close_task" {
        vec!["check_close", "close_task"]
    } else {
        vec![route.as_str()]
    };
    methods
        .into_iter()
        .map(|method| {
            let id = format!("api.method.{method}.request");
            descriptors.get(&id).with_context(|| {
                format!("method document {} has no descriptor {id}", paired.doc_id)
            })
        })
        .collect()
}

fn render_contract_tables(
    descriptors: &[&JsonContractDescriptor],
    language: Language,
) -> Result<String> {
    let mut output = String::new();
    for (index, descriptor) in descriptors.iter().enumerate() {
        if index > 0 {
            output.push('\n');
        }
        let schema = descriptor.schema().with_context(|| {
            format!(
                "semantic contract {} has no exact request schema",
                descriptor.id()
            )
        })?;
        let title = schema
            .get("title")
            .and_then(Value::as_str)
            .with_context(|| {
                format!("semantic contract {} has no schema title", descriptor.id())
            })?;
        match language {
            Language::English => {
                output.push_str("### `");
                output.push_str(title);
                output.push_str("` fields\n\n");
                output.push_str("| Field | Required | Nullable | Type |\n");
                output.push_str("|---|---|---|---|\n");
            }
            Language::Korean => {
                output.push_str("### `");
                output.push_str(title);
                output.push_str("` 필드\n\n");
                output.push_str("| 필드 | 필수 | Null 허용 | 형식 |\n");
                output.push_str("|---|---|---|---|\n");
            }
        }
        let required = schema
            .get("required")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect::<BTreeSet<_>>();
        let properties = schema
            .get("properties")
            .and_then(Value::as_object)
            .with_context(|| {
                format!(
                    "semantic contract {} has no object properties",
                    descriptor.id()
                )
            })?;
        for (field, property) in properties {
            let is_required = required.contains(field.as_str());
            let nullable = schema_allows_null(property);
            output.push_str("| `");
            output.push_str(field);
            output.push_str("` | ");
            output.push_str(match (language, is_required) {
                (Language::English, true) => "yes",
                (Language::English, false) => "no",
                (Language::Korean, true) => "예",
                (Language::Korean, false) => "아니요",
            });
            output.push_str(" | ");
            output.push_str(match (language, nullable) {
                (Language::English, true) => "yes",
                (Language::English, false) => "no",
                (Language::Korean, true) => "예",
                (Language::Korean, false) => "아니요",
            });
            output.push_str(" | `");
            output.push_str(&schema_type(property));
            output.push_str("` |\n");
        }
    }
    output.pop();
    Ok(output)
}

fn schema_allows_null(schema: &Value) -> bool {
    schema.get("type").is_some_and(|kind| {
        kind == "null"
            || kind
                .as_array()
                .is_some_and(|kinds| kinds.iter().any(|kind| kind == "null"))
    }) || ["anyOf", "oneOf"].iter().any(|key| {
        schema
            .get(key)
            .and_then(Value::as_array)
            .is_some_and(|branches| branches.iter().any(schema_allows_null))
    })
}

fn schema_type(schema: &Value) -> String {
    if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
        return reference.rsplit('/').next().unwrap_or(reference).to_owned();
    }
    if let Some(items) = schema.get("items") {
        return format!("{}[]", schema_type(items));
    }
    for combinator in ["anyOf", "oneOf"] {
        if let Some(branches) = schema.get(combinator).and_then(Value::as_array) {
            let types = branches
                .iter()
                .filter(|branch| !schema_allows_only_null(branch))
                .map(schema_type)
                .collect::<BTreeSet<_>>();
            if !types.is_empty() {
                return types.into_iter().collect::<Vec<_>>().join(" | ");
            }
        }
    }
    if let Some(kind) = schema.get("type").and_then(Value::as_str) {
        return kind.to_owned();
    }
    if let Some(kinds) = schema.get("type").and_then(Value::as_array) {
        let kinds = kinds
            .iter()
            .filter_map(Value::as_str)
            .filter(|kind| *kind != "null")
            .collect::<BTreeSet<_>>();
        if !kinds.is_empty() {
            return kinds.into_iter().collect::<Vec<_>>().join(" | ");
        }
    }
    if schema.get("properties").is_some() {
        return "object".to_owned();
    }
    "value".to_owned()
}

fn schema_allows_only_null(schema: &Value) -> bool {
    schema.get("type").is_some_and(|kind| kind == "null")
}

fn contract_table_range(
    contents: &str,
    descriptors: &[&JsonContractDescriptor],
    language: Language,
) -> Result<Range<usize>> {
    let first_title = descriptors
        .first()
        .and_then(|descriptor| descriptor.schema())
        .and_then(|schema| schema.get("title"))
        .and_then(Value::as_str)
        .context("request contract has no schema title")?;
    let heading = match language {
        Language::English => format!("### `{first_title}` fields"),
        Language::Korean => format!("### `{first_title}` 필드"),
    };
    let start = contents
        .find(&heading)
        .with_context(|| format!("missing generated request table heading {heading}"))?;
    let bytes = contents.as_bytes();
    let mut end = start;
    for line in contents[start..].split_inclusive('\n') {
        let trimmed = line.trim_end_matches(['\r', '\n']);
        let belongs = end == start
            || trimmed.is_empty()
            || trimmed.starts_with('|')
            || descriptors.iter().any(|descriptor| {
                let title = descriptor
                    .schema()
                    .and_then(|schema| schema.get("title"))
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                trimmed == format!("### `{title}` fields")
                    || trimmed == format!("### `{title}` 필드")
            });
        if !belongs {
            break;
        }
        end += line.len();
    }
    while end > start && bytes[end - 1] == b'\n' {
        end -= 1;
    }
    Ok(start..end)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn generated_request_table_drift_is_detected_deterministically() {
        let schema = json!({
            "title": "AlphaRequest",
            "type": "object",
            "required": ["snake_case"],
            "properties": {
                "optional_value": {"type": ["string", "null"]},
                "snake_case": {"type": "string"}
            }
        });
        let descriptor = JsonContractDescriptor::from_owner_schema(
            "api.method.alpha.request",
            schema,
            Default::default(),
            Vec::new(),
        );
        let expected =
            render_contract_tables(&[&descriptor], Language::English).expect("generated table");
        let drifted = expected.replacen("`snake_case`", "`snake_caze`", 1);
        let range = contract_table_range(&drifted, &[&descriptor], Language::English)
            .expect("generated region");

        assert_ne!(&drifted[range], expected);
        assert_eq!(
            expected,
            render_contract_tables(&[&descriptor], Language::English).expect("deterministic table")
        );
    }

    #[test]
    fn generated_response_table_drift_is_detected_deterministically() {
        let schema = json!({
            "title": "AlphaResult",
            "type": "object",
            "required": ["outcome", "items"],
            "properties": {
                "items": {"type": "array", "items": {"type": "integer"}},
                "outcome": {"enum": ["complete", "blocked"]}
            }
        });
        let descriptor = JsonContractDescriptor::from_owner_schema(
            "api.method.alpha.response",
            schema,
            Default::default(),
            Vec::new(),
        );
        let expected =
            render_contract_tables(&[&descriptor], Language::Korean).expect("generated table");
        let drifted = expected.replacen("`outcome`", "`outcame`", 1);
        let range = contract_table_range(&drifted, &[&descriptor], Language::Korean)
            .expect("generated region");

        assert_ne!(&drifted[range], expected);
        assert_eq!(
            expected,
            render_contract_tables(&[&descriptor], Language::Korean)
                .expect("deterministic response table")
        );
    }
}
