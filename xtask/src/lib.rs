mod architecture;
mod cli_docs;
mod diagnostics;
mod doc_index;
mod document_structure;
mod hygiene;
mod links;
mod markdown;
mod mcp_spec;
mod parity;
mod release_metadata;
mod repository;
mod storage;
mod terminology;
mod workspace_manifests;

use anyhow::Result;
use std::path::Path;

pub use architecture::{
    run_maintainability_report, CoverageHint, FileMetric, MaintainabilityReport, MixedSignalFile,
};
pub use cli_docs::{run_docs_sync, DocsSyncReport};
pub use diagnostics::{CheckReport, ValidationIssue};
pub use mcp_spec::{
    check_mcp_spec_fixture, check_mcp_spec_fixture_with_production_profiles, run_mcp_spec_check,
    run_mcp_spec_sync, McpSpecCheckReport, McpSpecSyncReport,
};
pub use release_metadata::{run_release_version_check, ReleaseVersionReport};

const DOC_INDEX_PATH: &str = "docs/doc-index.yaml";

pub fn run_docs_check(root: &Path) -> Result<CheckReport> {
    let root = repository::normalize_existing_root(root)?;
    let doc_index_path = root.join(DOC_INDEX_PATH);
    if !doc_index_path.exists() {
        anyhow::bail!(
            "docs-check must run from the repository root; missing {}",
            DOC_INDEX_PATH
        );
    }

    let mut issues = Vec::new();
    let index = doc_index::validate_doc_index(&root, &mut issues);

    if let Some(index) = index.as_ref() {
        doc_index::validate_document_coverage(&root, index, &mut issues);
        links::validate_markdown_links(&root, index, &mut issues);
        links::validate_bilingual_link_parity(&root, index, &mut issues);
        terminology::validate_terminology_paths(&root, index, &mut issues);
        let exact_identifiers = terminology::exact_identifier_catalog(
            &root.join("docs/terminology-map.yaml"),
            &mut issues,
        );
        parity::validate_bilingual_structure(&root, index, &exact_identifiers, &mut issues);
        cli_docs::validate_generated_cli_synopsis_regions(&root, index, &mut issues);
        cli_docs::validate_volicord_command_examples(&root, index, &mut issues);
        hygiene::validate_public_document_language(&root, index, &mut issues);
    }
    document_structure::validate_surface_stability_sections(&root, &mut issues);
    architecture::validate_xtask_dependency_boundary(&root.join("xtask/Cargo.toml"), &mut issues);
    hygiene::validate_public_language_claims(&root, &mut issues);
    storage::validate_storage_ddl_sql_blocks(&root, &mut issues);
    document_structure::validate_operation_category_values(&root, &mut issues);

    issues.sort();
    issues.dedup();

    Ok(CheckReport { issues })
}
