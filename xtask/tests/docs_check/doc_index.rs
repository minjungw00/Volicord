use super::*;

#[test]
fn accepts_shared_root_readme_without_korean_readme() {
    let fixture = valid_fixture();

    assert!(!fixture.path().join("README.ko.md").exists());

    let report = report(fixture.path());

    assert!(report.is_ok(), "{:#?}", report.issues());
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

    assert!(report.is_ok(), "{:#?}", report.issues());
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
"#,
    );
    write(fixture.path(), "docs/doc-index.yaml", &index);

    let report = report(fixture.path());

    assert!(report.is_ok(), "{:#?}", report.issues());
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

    assert_eq!(errors.len(), 1, "{:#?}", report.issues());
    assert_eq!(errors[0].path(), "README.ko.md");
    assert!(
        errors[0].message().contains("README.md <-> README.ko.md"),
        "{:#?}",
        report.issues()
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
fn rejects_unsupported_metadata_version() {
    let fixture = valid_fixture();
    let index = valid_doc_index().replace("version: 3", "version: 99");
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
        "default_applicability:\n- sample_workspace\n",
        "default_applicability:\n- unknown_applicability\n",
        1,
    );
    write(fixture.path(), "docs/doc-index.yaml", &index);

    let report = report(fixture.path());

    assert!(has_category(&report, "metadata.invalid_applicability"));
}

#[test]
fn reports_empty_or_duplicate_default_applicability() {
    let fixture = valid_fixture();
    let empty = valid_doc_index().replacen(
        "default_applicability:\n- sample_workspace\n",
        "default_applicability: []\n",
        1,
    );
    write(fixture.path(), "docs/doc-index.yaml", &empty);

    let empty_report = report(fixture.path());

    assert!(has_category(
        &empty_report,
        "metadata.invalid_default_applicability"
    ));

    let duplicate = valid_doc_index().replacen(
        "default_applicability:\n- sample_workspace\n",
        "default_applicability:\n- sample_workspace\n- sample_workspace\n",
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
