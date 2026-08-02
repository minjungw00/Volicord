use crate::diagnostics::ValidationIssue;
use crate::doc_index::DocIndex;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

const STORAGE_REGISTRY_SQL_PATH: &str = "crates/volicord-store/src/schema/registry.sql";
const STORAGE_PROJECT_SQL_PATH: &str = "crates/volicord-store/src/schema/project.sql";
const STORAGE_DDL_DOC_ID: &str = "reference.storage-ddl";

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
