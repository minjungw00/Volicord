mod architecture;
mod artifact_hygiene;
mod cli_docs;
mod contract_docs;
mod contract_identifiers;
mod diagnostics;
mod doc_index;
mod document_structure;
mod links;
mod markdown;
mod mcp_spec;
mod owner_route;
mod parity;
mod release_metadata;
mod repository;
mod source_bundle;
mod storage;
mod structured_parser;
mod terminology;
mod validation;
mod workspace_manifests;

use anyhow::Result;
use std::path::Path;

pub use architecture::{
    derive_workspace_package_inputs, run_architecture_check, run_maintainability_report,
    CoverageHint, FileMetric, MaintainabilityReport, MixedSignalFile, WorkspacePackageInput,
};
pub use cli_docs::{run_docs_sync, DocsSyncReport};
pub use diagnostics::{CheckReport, ValidationIssue};
pub use mcp_spec::{
    check_mcp_spec_fixture, check_mcp_spec_fixture_with_production_profiles, run_mcp_spec_check,
    run_mcp_spec_sync, McpSpecCheckReport, McpSpecSyncReport,
};
pub use owner_route::{run_owner_route, OwnerRouteReport};
pub use release_metadata::{run_release_version_check, ReleaseVersionReport};
pub use source_bundle::{create_source_bundle, validate_source_bundle, SourceBundleReport};
pub use validation::{
    current_linux_validation_plan, run_validation, CurrentValidationCommand,
    CurrentValidationCommandKind, CurrentValidationPlan, ValidationProfile, ValidationRunSummary,
};

const DOC_INDEX_PATH: &str = "docs/doc-index.yaml";
const GENERATED_DOC_REGION_NOTICE: &str =
    "<!-- This region is generated from maintained sources; do not edit it directly. -->";

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
        parity::validate_bilingual_structure(&root, index, &mut issues);
        contract_identifiers::validate_contract_identifiers(&root, index, &mut issues);
        contract_identifiers::validate_operation_category_values(&root, index, &mut issues);
        contract_docs::validate_generated_contract_tables(&root, index, &mut issues);
        cli_docs::validate_generated_cli_synopsis_regions(&root, index, &mut issues);
        architecture::validate_generated_architecture_regions(&root, index, &mut issues);
        cli_docs::validate_volicord_command_examples(&root, index, &mut issues);
        document_structure::validate_architecture_design_documents(&root, index, &mut issues);
        document_structure::validate_surface_stability_sections(&root, index, &mut issues);
        storage::validate_baseline_ref_contract(&root, index, &mut issues);
        storage::validate_storage_ddl_sql_blocks(&root, index, &mut issues);
    }
    if root.join("docs/owner-routing.yaml").exists() {
        owner_route::validate_owner_routing(&root, &mut issues);
    }
    artifact_hygiene::validate_tracked_artifacts(&root, &mut issues);

    issues.sort();
    issues.dedup();

    Ok(CheckReport { issues })
}
