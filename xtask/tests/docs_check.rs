use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use tempfile::TempDir;

fn valid_fixture() -> TempDir {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path();
    for dir in [
        "crates",
        "docs/en",
        "docs/ko",
        "docs/en/reference",
        "docs/ko/reference",
    ] {
        fs::create_dir_all(root.join(dir)).expect("create fixture dir");
    }

    write(root, "AGENTS.md", "# Root Agent Rules\n");
    write(root, "docs/AGENTS.md", "# Docs Agent Rules\n");
    write(root, "crates/AGENTS.md", "# Crates Agent Rules\n");
    write(
        root,
        "Cargo.toml",
        "[workspace.package]\nversion = \"1.2.3\"\n",
    );
    write(
        root,
        "xtask/Cargo.toml",
        "[package]\nname = \"documentation-checker\"\nversion = \"1.2.3\"\n",
    );
    write(root, "README.md", "# Volicord\n");
    write(root, "docs/README.md", "# Documentation\n");
    write(root, "docs/en/README.md", "# English Docs\n");
    write(
        root,
        "docs/ko/README.md",
        "<a id=\"english-docs\"></a>\n# 한국어 문서\n",
    );
    write(
        root,
        "docs/en/example.md",
        "# Overview\n\n<a id=\"explicit-anchor\"></a>\n\nSee [self](#overview), [anchor](#explicit-anchor), and [README](README.md).\n",
    );
    write(
        root,
        "docs/ko/example.md",
        "<a id=\"overview\"></a>\n<a id=\"explicit-anchor\"></a>\n# 개요\n\n[자체](#overview), [앵커](#explicit-anchor), [README](README.md)를 참조합니다.\n",
    );

    write(root, "docs/doc-index.yaml", &valid_doc_index());
    write(root, "docs/terminology-map.yaml", &valid_terminology_map());

    temp
}

fn write(root: &Path, path: &str, contents: &str) {
    if let Some(parent) = root.join(path).parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(root.join(path), contents).expect("write fixture file");
}

fn valid_doc_index() -> String {
    r#"version: 3
metadata: {}
language_retrieval: {}
owner_areas:
  repository_guidance:
    description: Repository guidance.
  documentation_maintenance:
    description: Documentation maintenance.
  onboarding:
    description: Onboarding.
  developer_documentation:
    description: Developer documentation.
applicability:
  sample_workspace:
    description: Current sample workspace package version.
    version_source: workspace_package
  doc_index_schema:
    description: Current documentation index schema.
    version_source: doc_index_schema
  terminology_map_schema:
    description: Current terminology map schema.
    version_source: terminology_map_schema
default_applicability:
- sample_workspace
entry_schema:
  applicability_fields:
    description: Current applicability description.
    version_source: Current owning source.
  default_applicability: Current root defaults.
  shared_required:
  - doc_id
  - path
  - kind
  - summary
  - normative_level
  - owner_area
  - created_on
  - last_updated_on
  - last_verified_on
  paired_required:
  - doc_id
  - path_en
  - path_ko
  - kind
  - summary
  - normative_level
  - translation_policy
  - owner_area
  - created_on
  - last_updated_on
  - last_verified_on
  optional:
  - primary_audience
  - journeys
  - canonical_for
  - depends_on
  - contracts
  maintenance_fields:
    owner_area: Current maintenance owner.
    created_on: Current creation date.
    last_updated_on: Current content date.
    last_verified_on: Current verification date.
    applies_to: Additional applicability values.
  kinds:
  - landing
  - tutorial
  - how_to
  - explanation
  - reference
  - maintenance
  reader_journeys:
  - evaluate
  - install
  - operate
  - learn
  - implement
  - maintain
  normative_levels:
  - contract
  - guide
  - example
  - maintenance
  translation_policies:
  - semantic_parity
shared_documents:
- doc_id: agents.root
  path: AGENTS.md
  kind: maintenance
  summary: Root rules.
  normative_level: maintenance
  owner_area: repository_guidance
  created_on: '2026-06-20'
  last_updated_on: '2026-06-20'
  last_verified_on: '2026-06-23'
- doc_id: agents.docs
  path: docs/AGENTS.md
  kind: maintenance
  summary: Docs rules.
  normative_level: maintenance
  owner_area: repository_guidance
  created_on: '2026-06-20'
  last_updated_on: '2026-06-20'
  last_verified_on: '2026-06-23'
- doc_id: agents.crates
  path: crates/AGENTS.md
  kind: maintenance
  summary: Crates rules.
  normative_level: maintenance
  owner_area: repository_guidance
  created_on: '2026-06-20'
  last_updated_on: '2026-06-20'
  last_verified_on: '2026-06-23'
- doc_id: readme.root
  path: README.md
  kind: landing
  summary: Root README.
  normative_level: guide
  owner_area: onboarding
  created_on: '2026-06-20'
  last_updated_on: '2026-06-20'
  last_verified_on: '2026-06-23'
- doc_id: docs.root
  path: docs/README.md
  kind: landing
  summary: Docs README.
  normative_level: guide
  owner_area: onboarding
  created_on: '2026-06-20'
  last_updated_on: '2026-06-20'
  last_verified_on: '2026-06-23'
- doc_id: docs.doc-index
  path: docs/doc-index.yaml
  kind: maintenance
  summary: Documentation metadata.
  normative_level: maintenance
  owner_area: documentation_maintenance
  created_on: '2026-06-20'
  last_updated_on: '2026-06-20'
  last_verified_on: '2026-06-23'
  applies_to:
  - doc_index_schema
- doc_id: terminology.map
  path: docs/terminology-map.yaml
  kind: maintenance
  summary: Terminology metadata.
  normative_level: maintenance
  owner_area: documentation_maintenance
  created_on: '2026-06-20'
  last_updated_on: '2026-06-20'
  last_verified_on: '2026-06-23'
  applies_to:
  - terminology_map_schema
documents:
- doc_id: docs.index
  path_en: docs/en/README.md
  path_ko: docs/ko/README.md
  kind: landing
  summary: Language indexes.
  normative_level: guide
  translation_policy: semantic_parity
  owner_area: onboarding
  created_on: '2026-06-20'
  last_updated_on: '2026-06-20'
  last_verified_on: '2026-06-23'
  journeys:
  - learn
- doc_id: example
  path_en: docs/en/example.md
  path_ko: docs/ko/example.md
  kind: explanation
  summary: Example pair.
  normative_level: guide
  translation_policy: semantic_parity
  owner_area: developer_documentation
  created_on: '2026-06-20'
  last_updated_on: '2026-06-20'
  last_verified_on: '2026-06-23'
  journeys:
  - learn
"#
    .to_string()
}

fn valid_doc_index_with_root_readme_pair() -> String {
    let mut index = valid_doc_index().replace(root_readme_shared_entry(), "");
    index.push_str(root_readme_paired_entry());
    index
}

fn install_admin_cli_fixture(root: &Path) {
    let owner = "# Administrative CLI\n\n## Command Model\n\n<!-- BEGIN GENERATED: volicord-cli-synopses -->\n<!-- END GENERATED: volicord-cli-synopses -->\n";
    write(root, "docs/en/reference/admin-cli.md", owner);
    write(root, "docs/ko/reference/admin-cli.md", owner);

    let mut index = valid_doc_index();
    index.push_str(
        r#"- doc_id: reference.admin-cli
  path_en: docs/en/reference/admin-cli.md
  path_ko: docs/ko/reference/admin-cli.md
  kind: reference
  summary: Administrative CLI.
  normative_level: contract
  translation_policy: semantic_parity
  owner_area: developer_documentation
  created_on: '2026-06-20'
  last_updated_on: '2026-06-20'
  last_verified_on: '2026-06-23'
"#,
    );
    write(root, "docs/doc-index.yaml", &index);
}

fn root_readme_shared_entry() -> &'static str {
    r#"- doc_id: readme.root
  path: README.md
  kind: landing
  summary: Root README.
  normative_level: guide
  owner_area: onboarding
  created_on: '2026-06-20'
  last_updated_on: '2026-06-20'
  last_verified_on: '2026-06-23'
"#
}

fn root_readme_paired_entry() -> &'static str {
    r#"- doc_id: readme.root
  path_en: README.md
  path_ko: README.ko.md
  kind: landing
  summary: Root README pair.
  normative_level: guide
  translation_policy: semantic_parity
  owner_area: onboarding
  created_on: '2026-06-20'
  last_updated_on: '2026-06-20'
  last_verified_on: '2026-06-23'
"#
}

fn valid_terminology_map() -> String {
    r##"version: 1
related_documents:
  index:
    en: "docs/en/example.md#overview"
    ko: "docs/ko/example.md#overview"
related_metadata:
  doc_index: "docs/doc-index.yaml"
terms:
  volicord_runtime_home:
    category: product_label
    roles:
      - public_user_term
    en: Volicord Runtime Home
    aliases_en:
      - Runtime Home
    ko_reference: Volicord Runtime Home
    ko_user: 런타임 홈
    primary_owner:
      en: "docs/en/example.md#overview"
      ko: "docs/ko/example.md#overview"
    related_references: []
  connection_internal_id:
    category: identifier
    roles:
      - storage_internal_identifier
    en: connection_internal_id
    ko_reference: "`connection_internal_id`"
    ko_user: "`connection_internal_id`"
    primary_owner:
      en: "docs/en/example.md#overview"
      ko: "docs/ko/example.md#overview"
    related_references: []
  project_internal_id:
    category: identifier
    roles:
      - storage_internal_identifier
    en: project_internal_id
    ko_reference: "`project_internal_id`"
    ko_user: "`project_internal_id`"
    primary_owner:
      en: "docs/en/example.md#overview"
      ko: "docs/ko/example.md#overview"
    related_references: []
  connection_id:
    category: identifier
    roles:
      - mcp_process_binding
      - diagnostic_field
    en: connection_id
    ko_reference: "`connection_id`"
    ko_user: "`connection_id`"
    primary_owner:
      en: "docs/en/example.md#overview"
      ko: "docs/ko/example.md#overview"
    related_references: []
  project_id:
    category: identifier
    roles:
      - diagnostic_field
    en: project_id
    ko_reference: "`project_id`"
    ko_user: "`project_id`"
    primary_owner:
      en: "docs/en/example.md#overview"
      ko: "docs/ko/example.md#overview"
    related_references: []
  project_selector:
    category: identifier
    roles:
      - mcp_public_selector
    en: project_selector
    ko_reference: "`project_selector`"
    ko_user: "`project_selector`"
    primary_owner:
      en: "docs/en/example.md#overview"
      ko: "docs/ko/example.md#overview"
    related_references: []
  installation_profile:
    category: storage_record
    roles:
      - storage_record
    en: installation_profile
    ko_reference: "`installation_profile`"
    ko_user: "`installation_profile`"
    primary_owner:
      en: "docs/en/example.md#overview"
      ko: "docs/ko/example.md#overview"
    related_references: []
"##
    .to_string()
}

fn report(root: &Path) -> xtask::CheckReport {
    xtask::run_docs_check(root).expect("docs check runs")
}

fn has_category(report: &xtask::CheckReport, category: &str) -> bool {
    report
        .issues()
        .iter()
        .any(|error| error.category() == category)
}

fn category_errors<'a>(
    report: &'a xtask::CheckReport,
    category: &str,
) -> Vec<&'a xtask::ValidationIssue> {
    report
        .issues()
        .iter()
        .filter(|error| error.category() == category)
        .collect()
}

fn index_admin_cli_surface_doc(root: &Path) {
    let mut index = valid_doc_index();
    index.push_str(
        r#"- doc_id: reference.admin-cli
  path_en: docs/en/reference/admin-cli.md
  path_ko: docs/ko/reference/admin-cli.md
  kind: reference
  summary: Administrative CLI reference.
  normative_level: contract
  translation_policy: semantic_parity
  owner_area: developer_documentation
  created_on: '2026-06-20'
  last_updated_on: '2026-06-20'
  last_verified_on: '2026-06-23'
"#,
    );
    write(root, "docs/doc-index.yaml", &index);
}

fn index_architecture_design_pair(root: &Path) {
    let mut index = valid_doc_index();
    index.push_str(
        r#"- doc_id: architecture-guide.design.example-boundary
  path_en: docs/en/architecture-guide/design/example-boundary.md
  path_ko: docs/ko/architecture-guide/design/example-boundary.md
  kind: explanation
  summary: Example architecture boundary.
  normative_level: guide
  translation_policy: semantic_parity
  owner_area: developer_documentation
  created_on: '2026-06-20'
  last_updated_on: '2026-06-20'
  last_verified_on: '2026-06-23'
"#,
    );
    write(root, "docs/doc-index.yaml", &index);
}

fn install_operation_category_fixture(root: &Path, en_values: &[String], ko_values: &[String]) {
    write(
        root,
        "docs/en/reference/api/schema-value-sets.md",
        &operation_category_owner("# API schema value sets", "Value", en_values),
    );
    write(
        root,
        "docs/ko/reference/api/schema-value-sets.md",
        &operation_category_owner("# API 스키마 값 집합", "값", ko_values),
    );

    let mut index = valid_doc_index();
    index.push_str(
        r#"- doc_id: reference.api.schema-value-sets
  path_en: docs/en/reference/api/schema-value-sets.md
  path_ko: docs/ko/reference/api/schema-value-sets.md
  kind: reference
  summary: API schema value sets.
  normative_level: contract
  translation_policy: semantic_parity
  owner_area: developer_documentation
  created_on: '2026-06-20'
  last_updated_on: '2026-06-20'
  last_verified_on: '2026-06-23'
"#,
    );
    write(root, "docs/doc-index.yaml", &index);
}

fn operation_category_owner(title: &str, value_heading: &str, values: &[String]) -> String {
    let mut rows = String::new();
    for value in values {
        writeln!(&mut rows, "| `{value}` | Description. |")
            .expect("writing to a String cannot fail");
    }
    format!(
        "{title}\n\n<a id=\"operation-category-values\"></a>\n## Operation category values\n\n| {value_heading} | Note |\n|---|---|\n{rows}"
    )
}

#[path = "docs_check/architecture.rs"]
mod architecture;
#[path = "docs_check/artifact_hygiene.rs"]
mod artifact_hygiene;
#[path = "docs_check/cli_docs.rs"]
mod cli_docs;
#[path = "docs_check/cli_generation.rs"]
mod cli_generation;
#[path = "docs_check/composition.rs"]
mod composition;
#[path = "docs_check/contract_docs.rs"]
mod contract_docs;
#[path = "docs_check/contract_identifiers.rs"]
mod contract_identifiers;
#[path = "docs_check/doc_index.rs"]
mod doc_index;
#[path = "docs_check/doc_index_applicability.rs"]
mod doc_index_applicability;
#[path = "docs_check/document_structure.rs"]
mod document_structure;
#[path = "docs_check/links.rs"]
mod links;
#[path = "docs_check/terminology.rs"]
mod terminology;
