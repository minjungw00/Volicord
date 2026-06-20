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
    write(root, "README.md", "# Harness\n");
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
        "<a id=\"overview\"></a>\n<a id=\"explicit-anchor\"></a>\n# 개요\n",
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
    r#"version: 2
shared_documents:
- doc_id: agents.root
  path: AGENTS.md
  kind: maintenance
  summary: Root rules.
  normative_level: maintenance
- doc_id: agents.docs
  path: docs/AGENTS.md
  kind: maintenance
  summary: Docs rules.
  normative_level: maintenance
- doc_id: agents.crates
  path: crates/AGENTS.md
  kind: maintenance
  summary: Crates rules.
  normative_level: maintenance
- doc_id: readme.root
  path: README.md
  kind: landing
  summary: Root README.
  normative_level: guide
- doc_id: docs.root
  path: docs/README.md
  kind: landing
  summary: Docs README.
  normative_level: guide
- doc_id: docs.doc-index
  path: docs/doc-index.yaml
  kind: maintenance
  summary: Documentation metadata.
  normative_level: maintenance
- doc_id: terminology.map
  path: docs/terminology-map.yaml
  kind: maintenance
  summary: Terminology metadata.
  normative_level: maintenance
documents:
- doc_id: docs.index
  path_en: docs/en/README.md
  path_ko: docs/ko/README.md
  kind: landing
  summary: Language indexes.
  normative_level: guide
  translation_policy: semantic_parity
  journeys:
  - learn
- doc_id: example
  path_en: docs/en/example.md
  path_ko: docs/ko/example.md
  kind: explanation
  summary: Example pair.
  normative_level: guide
  translation_policy: semantic_parity
  journeys:
  - learn
"#
    .to_string()
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
  example:
    category: product
    en: Example
    ko_reference: 예시
    ko_user: 예시
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

#[test]
fn accepts_valid_version_2_metadata() {
    let fixture = valid_fixture();

    let report = report(fixture.path());

    assert!(report.is_ok(), "{:#?}", report.errors());
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

    let report = report(fixture.path());

    assert!(report.is_ok(), "{:#?}", report.errors());
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
