use crate::diagnostics::ValidationIssue;
use crate::doc_index::DocIndex;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use volicord_types::ids::BaselineRef;

const STORAGE_REGISTRY_SQL_PATH: &str = "crates/volicord-store/src/schema/registry.sql";
const STORAGE_PROJECT_SQL_PATH: &str = "crates/volicord-store/src/schema/project.sql";
const STORAGE_DDL_DOC_ID: &str = "reference.storage-ddl";
const STATE_SCHEMA_DOC_ID: &str = "reference.api.schema-state";
const BASELINE_REF_DOC_BEGIN: &str = "<!-- BEGIN GENERATED: BaselineRef canonical scalar -->";
const BASELINE_REF_DOC_END: &str = "<!-- END GENERATED: BaselineRef canonical scalar -->";

pub(crate) fn sync_baseline_ref_contract(root: &Path, index: &DocIndex) -> Result<Vec<String>> {
    let project_path = root.join(STORAGE_PROJECT_SQL_PATH);
    let project_contents = fs::read_to_string(&project_path).with_context(|| {
        format!("failed to read canonical storage SQL at {STORAGE_PROJECT_SQL_PATH}")
    })?;
    let generated_project = render_baseline_ref_sql_regions(&project_contents)
        .context("invalid generated BaselineRef regions in canonical project SQL")?;

    let mut updated_paths = Vec::new();
    if generated_project != project_contents {
        fs::write(&project_path, generated_project).with_context(|| {
            format!("failed to update canonical storage SQL at {STORAGE_PROJECT_SQL_PATH}")
        })?;
        updated_paths.push(STORAGE_PROJECT_SQL_PATH.to_owned());
    }

    let owner = index
        .paired_documents
        .get(STATE_SCHEMA_DOC_ID)
        .with_context(|| {
            format!("docs/doc-index.yaml is missing paired owner {STATE_SCHEMA_DOC_ID}")
        })?;
    for (relative, language) in [(&owner.path_en, "en"), (&owner.path_ko, "ko")] {
        let path = root.join(relative);
        let contents = fs::read_to_string(&path)
            .with_context(|| format!("failed to read state schema owner at {relative}"))?;
        let generated = generated_baseline_ref_doc_region(language);
        let updated = replace_marked_region(
            &contents,
            BASELINE_REF_DOC_BEGIN,
            BASELINE_REF_DOC_END,
            &generated,
        )
        .with_context(|| format!("invalid BaselineRef generated region in {relative}"))?;
        if updated != contents {
            fs::write(&path, updated)
                .with_context(|| format!("failed to update state schema owner at {relative}"))?;
            updated_paths.push(relative.to_string());
        }
    }
    Ok(updated_paths)
}

pub(crate) fn validate_baseline_ref_contract(
    root: &Path,
    index: &DocIndex,
    errors: &mut Vec<ValidationIssue>,
) {
    let project_path = root.join(STORAGE_PROJECT_SQL_PATH);
    match fs::read_to_string(&project_path) {
        Ok(contents) => match render_baseline_ref_sql_regions(&contents) {
            Ok(generated) if generated == contents => {}
            Ok(_) => errors.push(ValidationIssue::new(
                STORAGE_PROJECT_SQL_PATH,
                "baseline_ref.generated_sql_drift",
                "generated BaselineRef SQLite predicates differ from the canonical scalar specification; run `cargo run -p xtask -- docs-sync`",
            )),
            Err(error) => errors.push(ValidationIssue::new(
                STORAGE_PROJECT_SQL_PATH,
                "baseline_ref.generated_sql_region",
                error.to_string(),
            )),
        },
        Err(error) => errors.push(ValidationIssue::new(
            STORAGE_PROJECT_SQL_PATH,
            "baseline_ref.generated_sql_read",
            format!("failed to read canonical project SQL: {error}"),
        )),
    }

    let Some(owner) = index.paired_documents.get(STATE_SCHEMA_DOC_ID) else {
        errors.push(ValidationIssue::new(
            "docs/doc-index.yaml",
            "baseline_ref.generated_docs_owner",
            format!("missing paired owner {STATE_SCHEMA_DOC_ID}"),
        ));
        return;
    };
    for (relative, language) in [(&owner.path_en, "en"), (&owner.path_ko, "ko")] {
        match fs::read_to_string(root.join(relative)) {
            Ok(contents) => {
                let expected = generated_baseline_ref_doc_region(language);
                match replace_marked_region(
                    &contents,
                    BASELINE_REF_DOC_BEGIN,
                    BASELINE_REF_DOC_END,
                    &expected,
                ) {
                    Ok(generated) if generated == contents => {}
                    Ok(_) => errors.push(ValidationIssue::new(
                        relative.to_string(),
                        "baseline_ref.generated_docs_drift",
                        "generated BaselineRef scalar contract differs from its type-owned specification; run `cargo run -p xtask -- docs-sync`",
                    )),
                    Err(error) => errors.push(ValidationIssue::new(
                        relative.to_string(),
                        "baseline_ref.generated_docs_region",
                        error.to_string(),
                    )),
                }
            }
            Err(error) => errors.push(ValidationIssue::new(
                relative.to_string(),
                "baseline_ref.generated_docs_read",
                format!("failed to read state schema owner: {error}"),
            )),
        }
    }
}

fn render_baseline_ref_sql_regions(contents: &str) -> Result<String> {
    let spec = BaselineRef::spec();
    let regions = [
        (
            "BaselineRef non-null baseline_ref",
            spec.sqlite_non_null_predicate("baseline_ref"),
            2,
        ),
        (
            "BaselineRef required baseline_ref",
            spec.sqlite_required_predicate("baseline_ref"),
            1,
        ),
        (
            "BaselineRef nullable baseline_ref",
            spec.sqlite_nullable_predicate("baseline_ref"),
            1,
        ),
        (
            "BaselineRef non-null applied_baseline_ref",
            spec.sqlite_non_null_predicate("applied_baseline_ref"),
            1,
        ),
    ];
    let mut rendered = contents.to_owned();
    for (label, expected, expected_count) in regions {
        rendered = replace_all_sql_regions(&rendered, label, &expected, expected_count)?;
    }
    Ok(rendered)
}

fn replace_all_sql_regions(
    contents: &str,
    label: &str,
    expected: &str,
    expected_count: usize,
) -> Result<String> {
    let begin = format!("-- BEGIN GENERATED: {label}");
    let end = format!("-- END GENERATED: {label}");
    let mut rendered = contents.to_owned();
    let mut search_start = 0;
    let mut count = 0;
    while let Some(relative_begin) = rendered[search_start..].find(&begin) {
        let begin_at = search_start + relative_begin;
        let line_start = rendered[..begin_at]
            .rfind('\n')
            .map_or(0, |index| index + 1);
        let indentation = rendered[line_start..begin_at].to_owned();
        if !indentation.chars().all(|character| character == ' ') {
            anyhow::bail!("generated SQL marker {begin} must be line-indented with spaces");
        }
        let body_start = rendered[begin_at..]
            .find('\n')
            .map(|offset| begin_at + offset + 1)
            .with_context(|| format!("generated SQL marker {begin} has no body"))?;
        let relative_end = rendered[body_start..]
            .find(&end)
            .with_context(|| format!("generated SQL marker {begin} has no matching end"))?;
        let end_at = body_start + relative_end;
        let end_line_start = rendered[..end_at].rfind('\n').map_or(0, |index| index + 1);
        if rendered[end_line_start..end_at] != indentation {
            anyhow::bail!("generated SQL marker {end} indentation differs from its begin marker");
        }
        let generated_body = expected
            .lines()
            .map(|line| format!("{indentation}{line}\n"))
            .collect::<String>();
        rendered.replace_range(body_start..end_line_start, &generated_body);
        search_start = body_start + generated_body.len() + indentation.len() + end.len();
        count += 1;
    }
    if count != expected_count {
        anyhow::bail!(
            "generated SQL region {begin} occurred {count} time(s); expected {expected_count}"
        );
    }
    Ok(rendered)
}

fn generated_baseline_ref_doc_region(language: &str) -> String {
    let spec = BaselineRef::spec();
    let alphabet = String::from_utf8_lossy(spec.allowed_ascii_bytes);
    let forbidden = spec
        .forbidden_complete_values
        .iter()
        .map(|value| format!("`{value}`"))
        .collect::<Vec<_>>()
        .join(", ");
    let examples = spec
        .examples
        .iter()
        .map(|value| format!("`{value}`"))
        .collect::<Vec<_>>()
        .join(", ");
    let forbidden_value = spec
        .forbidden_complete_values
        .first()
        .copied()
        .unwrap_or("none");
    let generated_notice =
        "<!-- Generated by `cargo run -p xtask -- docs-sync`; do not edit this region. -->";
    let body = if language == "ko" {
        format!(
            "{generated_notice}\n- null이 아닌 `{name}`는 UTF-8 문자열이지만 허용되는 값은 다음 byte-level 정규 scalar 계약과 정확히 일치합니다.\n  - byte 길이: `{minimum}..={maximum}`\n  - 허용 ASCII byte 전체: `{alphabet}`\n  - JSON Schema pattern: `{pattern}`\n  - 금지되는 완전한 값: {forbidden}\n  - 정규 예시: {examples}\n- `{name} | null`로 명시된 위치만 실제 JSON `null`을 허용합니다. String `{forbidden_value}`은 null sentinel이 아니며 유효하지 않습니다.\n- Rust parse, `TryFrom`, `FromStr`, serde, MCP 의미 validator, 생성 JSON Schema, SQLite predicate, Store decoder, conformance corpus, 이 문서 region은 모두 같은 type-owned specification에서 파생됩니다.",
            name = spec.semantic_name,
            minimum = spec.minimum_length,
            maximum = spec.maximum_length,
            pattern = spec.json_schema_pattern(),
            forbidden_value = forbidden_value,
        )
    } else {
        format!(
            "{generated_notice}\n- A non-null `{name}` is encoded as a UTF-8 string, but its accepted value set is exactly this byte-level canonical scalar contract.\n  - byte length: `{minimum}..={maximum}`\n  - complete allowed ASCII byte alphabet: `{alphabet}`\n  - JSON Schema pattern: `{pattern}`\n  - forbidden complete values: {forbidden}\n  - canonical examples: {examples}\n- Only a position explicitly typed as `{name} | null` accepts actual JSON `null`. The string `{forbidden_value}` is not a null sentinel and is invalid.\n- Rust parsing, `TryFrom`, `FromStr`, serde, MCP semantic validation, generated JSON Schema, SQLite predicates, Store decoding, the conformance corpus, and this documentation region all derive from the same type-owned specification.",
            name = spec.semantic_name,
            minimum = spec.minimum_length,
            maximum = spec.maximum_length,
            pattern = spec.json_schema_pattern(),
            forbidden_value = forbidden_value,
        )
    };
    format!("{BASELINE_REF_DOC_BEGIN}\n{body}\n{BASELINE_REF_DOC_END}")
}

fn replace_marked_region(contents: &str, begin: &str, end: &str, expected: &str) -> Result<String> {
    let begin_at = contents
        .find(begin)
        .with_context(|| format!("missing {begin}"))?;
    let after_begin = begin_at + begin.len();
    let end_offset = contents[after_begin..]
        .find(end)
        .with_context(|| format!("missing {end}"))?;
    let end_at = after_begin + end_offset + end.len();
    let mut updated = String::with_capacity(contents.len() + expected.len());
    updated.push_str(&contents[..begin_at]);
    updated.push_str(expected);
    updated.push_str(&contents[end_at..]);
    Ok(updated)
}

pub(crate) fn sync_storage_ddl_sql_blocks(root: &Path, index: &DocIndex) -> Result<Vec<String>> {
    let Some(owner) = index.paired_documents.get(STORAGE_DDL_DOC_ID) else {
        return Ok(Vec::new());
    };
    let schema_sources = [
        (
            "registry",
            fs::read_to_string(root.join(STORAGE_REGISTRY_SQL_PATH)).with_context(|| {
                format!("failed to read canonical storage SQL at {STORAGE_REGISTRY_SQL_PATH}")
            })?,
        ),
        (
            "project",
            fs::read_to_string(root.join(STORAGE_PROJECT_SQL_PATH)).with_context(|| {
                format!("failed to read canonical storage SQL at {STORAGE_PROJECT_SQL_PATH}")
            })?,
        ),
    ];

    let mut updated_paths = Vec::new();
    for relative in [&owner.path_en, &owner.path_ko] {
        let path = root.join(relative);
        let contents = fs::read_to_string(&path)
            .with_context(|| format!("failed to read Storage DDL owner at {relative}"))?;
        let mut updated = contents.clone();
        for (label, sql) in &schema_sources {
            updated = replace_canonical_storage_sql_block(
                &updated,
                label,
                &normalize_canonical_sql_block(sql),
            )
            .with_context(|| format!("invalid canonical {label} SQL block in {relative}"))?;
        }
        if updated != contents {
            fs::write(&path, updated)
                .with_context(|| format!("failed to update Storage DDL owner at {relative}"))?;
            updated_paths.push(relative.to_string());
        }
    }
    Ok(updated_paths)
}

pub(crate) fn validate_storage_ddl_sql_blocks(
    root: &Path,
    index: &DocIndex,
    errors: &mut Vec<ValidationIssue>,
) {
    let schema_sources = [
        ("registry", STORAGE_REGISTRY_SQL_PATH),
        ("project", STORAGE_PROJECT_SQL_PATH),
    ];
    let Some(owner) = index.paired_documents.get(STORAGE_DDL_DOC_ID) else {
        return;
    };
    let doc_paths = [&owner.path_en, &owner.path_ko];
    if !schema_sources
        .iter()
        .any(|(_, relative_path)| root.join(relative_path).exists())
        && !doc_paths.iter().any(|path| root.join(path).exists())
    {
        return;
    }

    for (label, schema_path) in schema_sources {
        let expected = match fs::read_to_string(root.join(schema_path)) {
            Ok(contents) => normalize_canonical_sql_block(&contents),
            Err(error) => {
                errors.push(ValidationIssue::new(
                    schema_path,
                    "storage_ddl_sql.read",
                    format!("failed to read canonical storage SQL: {error}"),
                ));
                continue;
            }
        };

        for doc_path in doc_paths {
            let contents = match fs::read_to_string(root.join(doc_path)) {
                Ok(contents) => contents,
                Err(error) => {
                    errors.push(ValidationIssue::new(
                        doc_path,
                        "storage_ddl_sql.read",
                        format!("failed to read Storage DDL document: {error}"),
                    ));
                    continue;
                }
            };
            match extract_canonical_storage_sql_block(&contents, label) {
                Some(actual) if normalize_canonical_sql_block(&actual) == expected => {}
                Some(_) => errors.push(ValidationIssue::new(
                    doc_path,
                    "storage_ddl_sql.drift",
                    format!("canonical {label} SQL block differs from {schema_path}"),
                )),
                None => errors.push(ValidationIssue::new(
                    doc_path,
                    "storage_ddl_sql.missing",
                    format!("missing canonical {label} SQL block"),
                )),
            }
        }
    }
}

fn extract_canonical_storage_sql_block(contents: &str, label: &str) -> Option<String> {
    let start_marker = format!("<!-- canonical-storage-sql: {label} start -->");
    let end_marker = format!("<!-- canonical-storage-sql: {label} end -->");
    let start = contents.find(&start_marker)? + start_marker.len();
    let after_start = &contents[start..];
    let fence_start = after_start.find("```sql")? + "```sql".len();
    let after_fence = &after_start[fence_start..];
    let after_fence = after_fence.strip_prefix('\n').unwrap_or(after_fence);
    let fence_end = after_fence.find("```")?;
    let block = &after_fence[..fence_end];
    let after_block = &after_fence[fence_end + "```".len()..];
    if after_block.contains(&end_marker) {
        Some(block.to_owned())
    } else {
        None
    }
}

fn replace_canonical_storage_sql_block(
    contents: &str,
    label: &str,
    expected: &str,
) -> Result<String> {
    let start_marker = format!("<!-- canonical-storage-sql: {label} start -->");
    let end_marker = format!("<!-- canonical-storage-sql: {label} end -->");
    let marker_start = contents
        .find(&start_marker)
        .with_context(|| format!("missing {start_marker}"))?;
    let marker_end = marker_start + start_marker.len();
    let end_marker_start = contents[marker_end..]
        .find(&end_marker)
        .map(|offset| marker_end + offset)
        .with_context(|| format!("missing {end_marker}"))?;
    let fence_start = contents[marker_end..end_marker_start]
        .find("```sql")
        .map(|offset| marker_end + offset + "```sql".len())
        .with_context(|| format!("missing SQL fence after {start_marker}"))?;
    let body_start = if contents[fence_start..].starts_with("\r\n") {
        fence_start + 2
    } else if contents[fence_start..].starts_with('\n') {
        fence_start + 1
    } else {
        anyhow::bail!("SQL fence after {start_marker} must end with a newline");
    };
    let body_end = contents[body_start..end_marker_start]
        .find("```")
        .map(|offset| body_start + offset)
        .with_context(|| format!("missing closing SQL fence before {end_marker}"))?;

    let mut updated = String::with_capacity(contents.len() + expected.len());
    updated.push_str(&contents[..body_start]);
    updated.push_str(expected);
    updated.push_str(&contents[body_end..]);
    Ok(updated)
}

fn normalize_canonical_sql_block(sql: &str) -> String {
    let normalized = sql.replace("\r\n", "\n");
    format!("{}\n", normalized.trim_end())
}
