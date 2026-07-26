use crate::diagnostics::ValidationIssue;
use crate::doc_index::DocIndex;
use std::fs;
use std::path::Path;

const STORAGE_REGISTRY_SQL_PATH: &str = "crates/volicord-store/src/schema/registry.sql";
const STORAGE_PROJECT_SQL_PATH: &str = "crates/volicord-store/src/schema/project.sql";
const STORAGE_DDL_DOC_ID: &str = "reference.storage-ddl";

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
    if after_block.find(&end_marker).is_some() {
        Some(block.to_owned())
    } else {
        None
    }
}

fn normalize_canonical_sql_block(sql: &str) -> String {
    let normalized = sql.replace("\r\n", "\n");
    format!("{}\n", normalized.trim_end())
}
