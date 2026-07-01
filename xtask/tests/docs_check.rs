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
  volicord_workspace_0_1:
    description: Volicord workspace 0.1.
  doc_index_schema_v3:
    description: Documentation index schema v3.
  terminology_map_v1:
    description: Terminology map v1.
entry_schema: {}
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
  applies_to:
  - volicord_workspace_0_1
- doc_id: agents.docs
  path: docs/AGENTS.md
  kind: maintenance
  summary: Docs rules.
  normative_level: maintenance
  owner_area: repository_guidance
  created_on: '2026-06-20'
  last_updated_on: '2026-06-20'
  last_verified_on: '2026-06-23'
  applies_to:
  - volicord_workspace_0_1
- doc_id: agents.crates
  path: crates/AGENTS.md
  kind: maintenance
  summary: Crates rules.
  normative_level: maintenance
  owner_area: repository_guidance
  created_on: '2026-06-20'
  last_updated_on: '2026-06-20'
  last_verified_on: '2026-06-23'
  applies_to:
  - volicord_workspace_0_1
- doc_id: readme.root
  path: README.md
  kind: landing
  summary: Root README.
  normative_level: guide
  owner_area: onboarding
  created_on: '2026-06-20'
  last_updated_on: '2026-06-20'
  last_verified_on: '2026-06-23'
  applies_to:
  - volicord_workspace_0_1
- doc_id: docs.root
  path: docs/README.md
  kind: landing
  summary: Docs README.
  normative_level: guide
  owner_area: onboarding
  created_on: '2026-06-20'
  last_updated_on: '2026-06-20'
  last_verified_on: '2026-06-23'
  applies_to:
  - volicord_workspace_0_1
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
  - doc_index_schema_v3
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
  - terminology_map_v1
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
  applies_to:
  - volicord_workspace_0_1
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
  applies_to:
  - volicord_workspace_0_1
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
  applies_to:
  - volicord_workspace_0_1
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
  applies_to:
  - volicord_workspace_0_1
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
        .errors()
        .iter()
        .any(|error| error.category() == category)
}

fn category_errors<'a>(
    report: &'a xtask::CheckReport,
    category: &str,
) -> Vec<&'a xtask::ValidationError> {
    report
        .errors()
        .iter()
        .filter(|error| error.category() == category)
        .collect()
}

#[test]
fn accepts_valid_version_3_metadata() {
    let fixture = valid_fixture();

    let report = report(fixture.path());

    assert!(report.is_ok(), "{:#?}", report.errors());
}

#[test]
fn accepts_shared_root_readme_without_korean_readme() {
    let fixture = valid_fixture();

    assert!(!fixture.path().join("README.ko.md").exists());

    let report = report(fixture.path());

    assert!(report.is_ok(), "{:#?}", report.errors());
}

#[test]
fn accepts_registered_root_readme_pair() {
    let fixture = valid_fixture();
    write(fixture.path(), "README.ko.md", "# Volicord Korean\n");
    write(
        fixture.path(),
        "docs/doc-index.yaml",
        &valid_doc_index_with_root_readme_pair(),
    );

    let report = report(fixture.path());

    assert!(report.is_ok(), "{:#?}", report.errors());
}

#[test]
fn accepts_normal_mirrored_docs_pair() {
    let fixture = valid_fixture();
    write(fixture.path(), "docs/en/extra.md", "# Extra\n");
    write(fixture.path(), "docs/ko/extra.md", "# Extra\n");
    let mut index = valid_doc_index();
    index.push_str(
        r#"- doc_id: extra
  path_en: docs/en/extra.md
  path_ko: docs/ko/extra.md
  kind: explanation
  summary: Extra mirrored pair.
  normative_level: guide
  translation_policy: semantic_parity
  owner_area: developer_documentation
  created_on: '2026-06-20'
  last_updated_on: '2026-06-20'
  last_verified_on: '2026-06-23'
  applies_to:
  - volicord_workspace_0_1
"#,
    );
    write(fixture.path(), "docs/doc-index.yaml", &index);

    let report = report(fixture.path());

    assert!(report.is_ok(), "{:#?}", report.errors());
}

#[test]
fn rejects_arbitrary_root_level_pair() {
    let fixture = valid_fixture();
    write(fixture.path(), "GUIDE.md", "# Guide\n");
    write(fixture.path(), "GUIDE.ko.md", "# Guide Korean\n");
    let mut index = valid_doc_index();
    index.push_str(
        r#"- doc_id: guide.root
  path_en: GUIDE.md
  path_ko: GUIDE.ko.md
  kind: explanation
  summary: Arbitrary root pair.
  normative_level: guide
  translation_policy: semantic_parity
  owner_area: developer_documentation
  created_on: '2026-06-20'
  last_updated_on: '2026-06-20'
  last_verified_on: '2026-06-23'
  applies_to:
  - volicord_workspace_0_1
"#,
    );
    write(fixture.path(), "docs/doc-index.yaml", &index);

    let report = report(fixture.path());

    assert!(has_category(&report, "coverage.unmirrored_pair"));
}

#[test]
fn rejects_reversed_root_readme_pair() {
    let fixture = valid_fixture();
    write(fixture.path(), "README.ko.md", "# Volicord Korean\n");
    let index = valid_doc_index()
        .replace(root_readme_shared_entry(), "")
        .replace("path_en: docs/en/README.md", "path_en: README.ko.md")
        .replace("path_ko: docs/ko/README.md", "path_ko: README.md");
    write(fixture.path(), "docs/doc-index.yaml", &index);

    let report = report(fixture.path());

    assert!(has_category(&report, "coverage.unmirrored_pair"));
}

#[test]
fn reports_unindexed_korean_root_readme() {
    let fixture = valid_fixture();
    write(fixture.path(), "README.ko.md", "# Volicord Korean\n");

    let report = report(fixture.path());
    let errors = category_errors(&report, "coverage.unindexed_pair");

    assert_eq!(errors.len(), 1, "{:#?}", report.errors());
    assert_eq!(errors[0].file(), "README.ko.md");
    assert!(
        errors[0].message().contains("README.md <-> README.ko.md"),
        "{:#?}",
        report.errors()
    );
}

#[test]
fn reports_missing_file_in_registered_root_readme_pair() {
    let fixture = valid_fixture();
    write(
        fixture.path(),
        "docs/doc-index.yaml",
        &valid_doc_index_with_root_readme_pair(),
    );

    let report = report(fixture.path());

    assert!(has_category(&report, "metadata.missing_path"));
}

#[test]
fn reports_bilingual_link_mismatch_in_registered_root_readme_pair() {
    let fixture = valid_fixture();
    write(
        fixture.path(),
        "README.md",
        "# Volicord\n\n[Docs](docs/en/README.md)\n",
    );
    write(fixture.path(), "README.ko.md", "# Volicord Korean\n");
    write(
        fixture.path(),
        "docs/doc-index.yaml",
        &valid_doc_index_with_root_readme_pair(),
    );

    let report = report(fixture.path());

    assert!(has_category(&report, "bilingual_link.only_en"));
}

#[test]
fn rejects_version_2_metadata() {
    let fixture = valid_fixture();
    let index = valid_doc_index().replace("version: 3", "version: 2");
    write(fixture.path(), "docs/doc-index.yaml", &index);

    let report = report(fixture.path());

    assert!(has_category(&report, "metadata.version"));
}

#[test]
fn reports_missing_maintenance_fields() {
    let fixture = valid_fixture();
    let index = valid_doc_index().replacen("  owner_area: repository_guidance\n", "", 1);
    write(fixture.path(), "docs/doc-index.yaml", &index);

    let report = report(fixture.path());

    assert!(has_category(&report, "metadata.missing_field"));
}

#[test]
fn reports_unknown_owner_area() {
    let fixture = valid_fixture();
    let index = valid_doc_index().replacen(
        "  owner_area: repository_guidance\n",
        "  owner_area: missing_area\n",
        1,
    );
    write(fixture.path(), "docs/doc-index.yaml", &index);

    let report = report(fixture.path());

    assert!(has_category(&report, "metadata.invalid_owner_area"));
}

#[test]
fn reports_unknown_applicability_identifier() {
    let fixture = valid_fixture();
    let index = valid_doc_index().replacen(
        "  - volicord_workspace_0_1\n",
        "  - unknown_applicability\n",
        1,
    );
    write(fixture.path(), "docs/doc-index.yaml", &index);

    let report = report(fixture.path());

    assert!(has_category(&report, "metadata.invalid_applicability"));
}

#[test]
fn reports_empty_or_duplicate_applicability() {
    let fixture = valid_fixture();
    let empty = valid_doc_index().replacen(
        "  applies_to:\n  - volicord_workspace_0_1\n",
        "  applies_to: []\n",
        1,
    );
    write(fixture.path(), "docs/doc-index.yaml", &empty);

    let empty_report = report(fixture.path());

    assert!(has_category(&empty_report, "metadata.invalid_applies_to"));

    let duplicate = valid_doc_index().replacen(
        "  applies_to:\n  - volicord_workspace_0_1\n",
        "  applies_to:\n  - volicord_workspace_0_1\n  - volicord_workspace_0_1\n",
        1,
    );
    write(fixture.path(), "docs/doc-index.yaml", &duplicate);

    let duplicate_report = report(fixture.path());

    assert!(has_category(
        &duplicate_report,
        "metadata.duplicate_applicability"
    ));
}

#[test]
fn reports_invalid_date_syntax() {
    let fixture = valid_fixture();
    let index = valid_doc_index().replacen(
        "  created_on: '2026-06-20'\n",
        "  created_on: '2026/06/20'\n",
        1,
    );
    write(fixture.path(), "docs/doc-index.yaml", &index);

    let report = report(fixture.path());

    assert!(has_category(&report, "metadata.invalid_date_syntax"));
}

#[test]
fn reports_invalid_calendar_date() {
    let fixture = valid_fixture();
    let index = valid_doc_index().replacen(
        "  created_on: '2026-06-20'\n",
        "  created_on: '2026-02-30'\n",
        1,
    );
    write(fixture.path(), "docs/doc-index.yaml", &index);

    let report = report(fixture.path());

    assert!(has_category(&report, "metadata.invalid_date_calendar"));
}

#[test]
fn reports_invalid_date_ordering() {
    let fixture = valid_fixture();
    let index = valid_doc_index().replacen(
        "  created_on: '2026-06-20'\n",
        "  created_on: '2026-06-24'\n",
        1,
    );
    write(fixture.path(), "docs/doc-index.yaml", &index);

    let report = report(fixture.path());

    assert!(has_category(&report, "metadata.invalid_date_order"));
}

#[test]
fn reports_unknown_top_level_or_entry_fields() {
    let fixture = valid_fixture();
    let mut index = valid_doc_index().replacen(
        "  normative_level: maintenance\n",
        "  normative_level: maintenance\n  unexpected_entry_field: true\n",
        1,
    );
    index.push_str("unexpected_top_level: true\n");
    write(fixture.path(), "docs/doc-index.yaml", &index);

    let report = report(fixture.path());

    assert!(has_category(&report, "metadata.unknown_field"));
}

#[test]
fn reports_duplicate_doc_id() {
    let fixture = valid_fixture();
    write(fixture.path(), "docs/en/duplicate.md", "# Duplicate\n");
    write(
        fixture.path(),
        "docs/ko/duplicate.md",
        "<a id=\"duplicate\"></a>\n# 중복\n",
    );
    let mut index = valid_doc_index();
    index.push_str(
        r#"- doc_id: example
  path_en: docs/en/duplicate.md
  path_ko: docs/ko/duplicate.md
  kind: explanation
  summary: Duplicate id.
  normative_level: guide
  translation_policy: semantic_parity
  owner_area: developer_documentation
  created_on: '2026-06-20'
  last_updated_on: '2026-06-20'
  last_verified_on: '2026-06-23'
  applies_to:
  - volicord_workspace_0_1
"#,
    );
    write(fixture.path(), "docs/doc-index.yaml", &index);

    let report = report(fixture.path());

    assert!(has_category(&report, "metadata.duplicate_doc_id"));
}

#[test]
fn reports_missing_paired_path() {
    let fixture = valid_fixture();
    write(fixture.path(), "docs/en/orphan.md", "# Orphan\n");

    let report = report(fixture.path());

    assert!(has_category(&report, "coverage.missing_pair"));
}

#[test]
fn reports_invalid_depends_on() {
    let fixture = valid_fixture();
    let index = valid_doc_index().replace(
        "  journeys:\n  - learn\n",
        "  journeys:\n  - learn\n  depends_on:\n  - missing.doc\n",
    );
    write(fixture.path(), "docs/doc-index.yaml", &index);

    let report = report(fixture.path());

    assert!(has_category(&report, "metadata.invalid_depends_on"));
}

#[test]
fn reports_invalid_kind_or_journey() {
    let fixture = valid_fixture();
    let index = valid_doc_index()
        .replace("  kind: explanation\n", "  kind: mystery\n")
        .replace("  - learn\n", "  - wander\n");
    write(fixture.path(), "docs/doc-index.yaml", &index);

    let report = report(fixture.path());

    assert!(has_category(&report, "metadata.invalid_kind"));
    assert!(has_category(&report, "metadata.invalid_journey"));
}

#[test]
fn reports_broken_relative_link() {
    let fixture = valid_fixture();
    write(
        fixture.path(),
        "docs/en/example.md",
        "# Overview\n\n[Missing](missing.md)\n",
    );

    let report = report(fixture.path());

    assert!(has_category(&report, "link.missing_target"));
}

#[test]
fn accepts_valid_local_fragment() {
    let fixture = valid_fixture();

    let report = report(fixture.path());

    assert!(report.is_ok(), "{:#?}", report.errors());
}

#[test]
fn reports_missing_fragment() {
    let fixture = valid_fixture();
    write(
        fixture.path(),
        "docs/en/example.md",
        "# Overview\n\n[Missing fragment](#missing-fragment)\n",
    );

    let report = report(fixture.path());

    assert!(has_category(&report, "link.missing_fragment"));
}

#[test]
fn ignores_links_inside_fenced_code() {
    let fixture = valid_fixture();
    write(
        fixture.path(),
        "docs/en/example.md",
        "# Overview\n\n```md\n[Missing](missing.md)\n```\n",
    );

    let report = report(fixture.path());

    assert!(
        !has_category(&report, "link.missing_target"),
        "{:#?}",
        report.errors()
    );
}

#[test]
fn accepts_explicit_html_anchor() {
    let fixture = valid_fixture();
    write(
        fixture.path(),
        "docs/en/example.md",
        "# Overview\n\n<a id=\"explicit-anchor\"></a>\n\n[Anchor](#explicit-anchor)\n",
    );
    write(
        fixture.path(),
        "docs/ko/example.md",
        "<a id=\"overview\"></a>\n<a id=\"explicit-anchor\"></a>\n# 개요\n\n[앵커](#explicit-anchor)\n",
    );

    let report = report(fixture.path());

    assert!(report.is_ok(), "{:#?}", report.errors());
}

#[test]
fn accepts_language_specific_paths_to_same_doc_id() {
    let fixture = valid_fixture();
    write(
        fixture.path(),
        "docs/en/example.md",
        "# Overview\n\n[Language index](README.md)\n",
    );
    write(
        fixture.path(),
        "docs/ko/example.md",
        "<a id=\"overview\"></a>\n# 개요\n\n[언어 색인](README.md)\n",
    );

    let report = report(fixture.path());

    assert!(report.is_ok(), "{:#?}", report.errors());
}

#[test]
fn reports_bilingual_link_only_in_english() {
    let fixture = valid_fixture();
    write(
        fixture.path(),
        "docs/en/example.md",
        "# Overview\n\n[Language index](README.md)\n",
    );
    write(
        fixture.path(),
        "docs/ko/example.md",
        "<a id=\"overview\"></a>\n# 개요\n",
    );

    let report = report(fixture.path());

    assert!(has_category(&report, "bilingual_link.only_en"));
}

#[test]
fn reports_bilingual_link_different_maintained_target() {
    let fixture = valid_fixture();
    write(
        fixture.path(),
        "docs/en/example.md",
        "# Overview\n\n[Language index](README.md)\n",
    );
    write(
        fixture.path(),
        "docs/ko/example.md",
        "<a id=\"overview\"></a>\n# 개요\n\n[예시](example.md)\n",
    );

    let report = report(fixture.path());

    assert!(has_category(&report, "bilingual_link.target_mismatch"));
}

#[test]
fn reports_bilingual_link_different_fragment_on_same_target() {
    let fixture = valid_fixture();
    write(
        fixture.path(),
        "docs/en/example.md",
        "# Overview\n\n<a id=\"explicit-anchor\"></a>\n\n[Anchor](#explicit-anchor)\n",
    );
    write(
        fixture.path(),
        "docs/ko/example.md",
        "<a id=\"overview\"></a>\n<a id=\"explicit-anchor\"></a>\n# 개요\n\n[앵커](#overview)\n",
    );

    let report = report(fixture.path());

    assert!(has_category(&report, "bilingual_link.fragment_mismatch"));
}

#[test]
fn accepts_english_heading_anchor_with_explicit_korean_anchor() {
    let fixture = valid_fixture();
    write(
        fixture.path(),
        "docs/en/example.md",
        "# Overview\n\n[Self](#overview)\n",
    );
    write(
        fixture.path(),
        "docs/ko/example.md",
        "<a id=\"overview\"></a>\n# 개요\n\n[자체](#overview)\n",
    );

    let report = report(fixture.path());

    assert!(report.is_ok(), "{:#?}", report.errors());
}

#[test]
fn ignores_external_links_for_bilingual_parity() {
    let fixture = valid_fixture();
    write(
        fixture.path(),
        "docs/en/example.md",
        "# Overview\n\n[External](https://example.com/path)\n",
    );
    write(
        fixture.path(),
        "docs/ko/example.md",
        "<a id=\"overview\"></a>\n# 개요\n",
    );

    let report = report(fixture.path());

    assert!(report.is_ok(), "{:#?}", report.errors());
}

#[test]
fn ignores_image_links_for_bilingual_parity() {
    let fixture = valid_fixture();
    write(fixture.path(), "docs/en/figure.png", "");
    write(
        fixture.path(),
        "docs/en/example.md",
        "# Overview\n\n![Diagram](figure.png)\n",
    );
    write(
        fixture.path(),
        "docs/ko/example.md",
        "<a id=\"overview\"></a>\n# 개요\n",
    );

    let report = report(fixture.path());

    assert!(report.is_ok(), "{:#?}", report.errors());
}

#[test]
fn ignores_fenced_code_links_for_bilingual_parity() {
    let fixture = valid_fixture();
    write(
        fixture.path(),
        "docs/en/example.md",
        "# Overview\n\n```md\n[Language index](README.md)\n```\n",
    );
    write(
        fixture.path(),
        "docs/ko/example.md",
        "<a id=\"overview\"></a>\n# 개요\n",
    );

    let report = report(fixture.path());

    assert!(report.is_ok(), "{:#?}", report.errors());
}

#[test]
fn accepts_shared_document_links_for_bilingual_parity() {
    let fixture = valid_fixture();
    write(
        fixture.path(),
        "docs/en/example.md",
        "# Overview\n\n[Repository README](../../README.md)\n",
    );
    write(
        fixture.path(),
        "docs/ko/example.md",
        "<a id=\"overview\"></a>\n# 개요\n\n[저장소 README](../../README.md)\n",
    );

    let report = report(fixture.path());

    assert!(report.is_ok(), "{:#?}", report.errors());
}

#[test]
fn accepts_non_indexed_repository_file_links_for_bilingual_parity() {
    let fixture = valid_fixture();
    write(fixture.path(), "support.txt", "fixture support\n");
    write(
        fixture.path(),
        "docs/en/example.md",
        "# Overview\n\n[Support file](../../support.txt)\n",
    );
    write(
        fixture.path(),
        "docs/ko/example.md",
        "<a id=\"overview\"></a>\n# 개요\n\n[지원 파일](../../support.txt)\n",
    );

    let report = report(fixture.path());

    assert!(report.is_ok(), "{:#?}", report.errors());
}

#[test]
fn reports_repeated_bilingual_links_deterministically() {
    let fixture = valid_fixture();
    write(
        fixture.path(),
        "docs/en/example.md",
        "# Overview\n\n[Language index](README.md)\n\n[Again](README.md)\n",
    );
    write(
        fixture.path(),
        "docs/ko/example.md",
        "<a id=\"overview\"></a>\n# 개요\n\n[언어 색인](README.md)\n",
    );

    let report = report(fixture.path());
    let errors = category_errors(&report, "bilingual_link.only_en");

    assert_eq!(errors.len(), 1, "{:#?}", report.errors());
    assert!(
        errors[0].message().contains("1 more English occurrence"),
        "{:#?}",
        report.errors()
    );
    assert!(
        errors[0].message().contains("docs.index"),
        "{:#?}",
        report.errors()
    );
}

#[test]
fn reports_retired_path_references() {
    let fixture = valid_fixture();
    write(
        fixture.path(),
        "docs/en/example.md",
        "# Overview\n\n[Old guide](use/old.md)\n",
    );

    let report = report(fixture.path());

    assert!(has_category(&report, "retired_path.reference"));
}

#[test]
fn reports_terminology_map_path_failure() {
    let fixture = valid_fixture();
    let terminology = valid_terminology_map()
        .replace("docs/en/example.md#overview", "docs/en/missing.md#overview");
    write(fixture.path(), "docs/terminology-map.yaml", &terminology);

    let report = report(fixture.path());

    assert!(has_category(&report, "terminology.missing_target"));
}

#[test]
fn reports_required_terminology_role_failure() {
    let fixture = valid_fixture();
    let terminology = valid_terminology_map().replace(
        r#"  project_selector:
    category: identifier
    roles:
      - mcp_public_selector
"#,
        r#"  project_selector:
    category: identifier
"#,
    );
    write(fixture.path(), "docs/terminology-map.yaml", &terminology);

    let report = report(fixture.path());
    let errors = category_errors(&report, "terminology.missing_role");

    assert_eq!(errors.len(), 1, "{:#?}", report.errors());
    assert!(
        errors[0].message().contains("project_selector"),
        "{:#?}",
        report.errors()
    );
}

#[test]
fn reports_invalid_terminology_role_value() {
    let fixture = valid_fixture();
    let terminology =
        valid_terminology_map().replace("      - mcp_public_selector", "      - public_id");
    write(fixture.path(), "docs/terminology-map.yaml", &terminology);

    let report = report(fixture.path());
    let errors = category_errors(&report, "terminology.invalid_role");

    assert_eq!(errors.len(), 1, "{:#?}", report.errors());
    assert!(
        errors[0].message().contains("public_id"),
        "{:#?}",
        report.errors()
    );
}

#[test]
fn accepts_sensitive_identifiers_in_document_prose_when_map_roles_are_valid() {
    let fixture = valid_fixture();
    write(
        fixture.path(),
        "docs/en/example.md",
        "# Overview\n\nA diagnostic can mention `connection_id` and `project_id`, while a public MCP call can use `project_selector`.\n",
    );
    write(
        fixture.path(),
        "docs/ko/example.md",
        "<a id=\"overview\"></a>\n# 개요\n\n진단에는 `connection_id`와 `project_id`가 나올 수 있고, 공개 MCP 호출은 `project_selector`를 사용할 수 있습니다.\n",
    );

    let report = report(fixture.path());

    assert!(report.is_ok(), "{:#?}", report.errors());
}

#[test]
fn accepts_supported_volicord_shell_command_examples() {
    let fixture = valid_fixture();
    let commands = r#"```sh
./target/debug/volicord setup
volicord setup
volicord setup --json
volicord setup --mcp-command /path/to/volicord
volicord setup --link-bin /path/to/bin
volicord setup --home /path/to/runtime-home
./target/debug/volicord mcp --version
./target/debug/volicord mcp --help
volicord mcp --stdio --connection CONNECTION_ID
volicord mcp --check --connection CONNECTION_ID
volicord mcp --check --connection CONNECTION_ID --project PROJECT_ID
volicord init --host codex --repo /path/to/repo --mode mcp-only
volicord init --host claude-code --repo /path/to/repo --mode guarded --allow-degraded
./target/debug/volicord init --host codex --repo /path/to/repo --dry-run
volicord connect codex --read-only
volicord export mcp-config --output /tmp/volicord.mcp.json
volicord connection mode codex workflow
volicord user status --task active
```
"#;
    write(
        fixture.path(),
        "docs/en/example.md",
        &format!("# Overview\n\n{commands}"),
    );
    write(
        fixture.path(),
        "docs/ko/example.md",
        &format!("<a id=\"overview\"></a>\n# 개요\n\n{commands}"),
    );

    let report = report(fixture.path());

    assert!(report.is_ok(), "{:#?}", report.errors());
}

#[test]
fn rejects_mcp_command_on_connect_command_examples() {
    let fixture = valid_fixture();
    write(
        fixture.path(),
        "docs/en/example.md",
        "# Overview\n\n```sh\nvolicord connect codex --mcp-command ./target/debug/volicord\n```\n",
    );

    let report = report(fixture.path());
    let errors = category_errors(&report, "command.invalid_example");

    assert_eq!(errors.len(), 1, "{:#?}", report.errors());
    assert!(
        errors[0].message().contains("--mcp-command"),
        "{:#?}",
        report.errors()
    );
    assert!(
        errors[0].message().contains("volicord connect"),
        "{:#?}",
        report.errors()
    );
}

#[test]
fn rejects_link_bin_on_connect_command_examples() {
    let fixture = valid_fixture();
    write(
        fixture.path(),
        "docs/en/example.md",
        "# Overview\n\n```sh\nvolicord connect codex --link-bin /path/to/bin\n```\n",
    );

    let report = report(fixture.path());
    let errors = category_errors(&report, "command.invalid_example");

    assert_eq!(errors.len(), 1, "{:#?}", report.errors());
    assert!(
        errors[0].message().contains("--link-bin"),
        "{:#?}",
        report.errors()
    );
    assert!(
        errors[0].message().contains("volicord connect"),
        "{:#?}",
        report.errors()
    );
}

#[test]
fn ignores_unsupported_volicord_commands_in_prose() {
    let fixture = valid_fixture();
    write(
        fixture.path(),
        "docs/en/example.md",
        "# Overview\n\nA diagnostic can mention `connection_id`, and prose can name `volicord connect codex --mcp-command ./target/debug/volicord` or `volicord connect codex --link-bin /path/to/bin` without becoming an executable example.\n",
    );

    let report = report(fixture.path());

    assert!(
        !has_category(&report, "command.invalid_example"),
        "{:#?}",
        report.errors()
    );
}
