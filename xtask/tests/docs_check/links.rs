use super::*;

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

    assert!(report.is_ok(), "{:#?}", report.issues());
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
        report.issues()
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

    assert!(report.is_ok(), "{:#?}", report.issues());
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

    assert!(report.is_ok(), "{:#?}", report.issues());
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

    assert!(report.is_ok(), "{:#?}", report.issues());
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

    assert!(report.is_ok(), "{:#?}", report.issues());
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

    assert!(report.is_ok(), "{:#?}", report.issues());
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

    assert!(report.is_ok(), "{:#?}", report.issues());
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

    assert!(report.is_ok(), "{:#?}", report.issues());
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

    assert!(report.is_ok(), "{:#?}", report.issues());
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

    assert_eq!(errors.len(), 1, "{:#?}", report.issues());
    assert!(
        errors[0].message().contains("1 more English occurrence"),
        "{:#?}",
        report.issues()
    );
    assert!(
        errors[0].message().contains("docs.index"),
        "{:#?}",
        report.issues()
    );
}
