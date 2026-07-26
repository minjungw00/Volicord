use super::*;

#[test]
fn rejects_write_readiness_public_document_term() {
    let fixture = valid_fixture();
    write(
        fixture.path(),
        "docs/en/example.md",
        "# Overview\n\nUse the write-readiness boundary before writing.\n",
    );

    let report = report(fixture.path());
    let errors = category_errors(&report, "public_language.write_ticket_term");

    assert_eq!(errors.len(), 1, "{:#?}", report.issues());
    assert!(
        errors[0].message().contains("write-ticket"),
        "{:#?}",
        report.issues()
    );
}

#[test]
fn rejects_ambiguous_host_support_claims_in_english_and_korean_documents() {
    let fixture = valid_fixture();
    write(
        fixture.path(),
        "docs/en/example.md",
        "# Overview\n\nConnect a supported agent host.\nVolicord supports Codex.\n",
    );
    write(
        fixture.path(),
        "docs/ko/example.md",
        "<a id=\"overview\"></a>\n<a id=\"explicit-anchor\"></a>\n# 개요\n\n지원되는 에이전트 호스트를 연결합니다.\n지원되는 관리 호스트를 연결합니다.\n지원 호스트가 준비됐습니다.\n관리 호스트가 지원됩니다.\nCodex를 지원합니다.\n지원되는 Agent Connection을 만듭니다.\nAgent Connection이 지원됩니다.\nAgent Connection 지원을 사용할 수 있습니다.\n지원되는 `record` 프로필입니다.\n`record` 프로필이 지원됩니다.\nrecord·detective 프로필을 지원합니다.\n`record`·`detective` 프로필을 지원합니다.\n`--profile record`를 지원합니다.\n",
    );

    let report = report(fixture.path());
    let errors = category_errors(&report, "public_language.ambiguous_host_support_claim");

    assert_eq!(errors.len(), 15, "{:#?}", report.issues());
    assert!(
        errors
            .iter()
            .any(|error| error.path() == "docs/en/example.md")
            && errors
                .iter()
                .any(|error| error.path() == "docs/ko/example.md"),
        "{:#?}",
        report.issues()
    );
}

#[test]
fn ignores_ambiguous_host_support_claims_inside_document_code_fences() {
    let fixture = valid_fixture();
    write(
        fixture.path(),
        "docs/en/example.md",
        "# Overview\n\n```text\nsupported host\n```\n",
    );

    let report = report(fixture.path());

    assert!(
        category_errors(&report, "public_language.ambiguous_host_support_claim").is_empty(),
        "{:#?}",
        report.issues()
    );
}
#[test]
fn reports_unqualified_public_language_security_claims() {
    let fixture = valid_fixture();
    write(
        fixture.path(),
        "crates/volicord-cli/src/connection_command.rs",
        "pub const MESSAGE: &str = \"Volicord is secure.\";\n",
    );

    let report = report(fixture.path());

    assert!(has_category(&report, "public_language.security_claim"));
}

#[test]
fn reports_unqualified_public_language_claims_in_nested_cli_source() {
    let fixture = valid_fixture();
    write(
        fixture.path(),
        "crates/volicord-cli/src/connection_command/output/text.rs",
        "pub const MESSAGE: &str = \"Volicord output is protected.\";\n",
    );

    let report = report(fixture.path());
    let errors = category_errors(&report, "public_language.security_claim");

    assert_eq!(errors.len(), 1, "{:#?}", report.issues());
    assert_eq!(
        errors[0].path(),
        "crates/volicord-cli/src/connection_command/output/text.rs"
    );
    assert!(
        errors[0].message().contains("protected"),
        "{:#?}",
        report.issues()
    );
}

#[test]
fn reports_ambiguous_host_support_claims_in_nested_cli_source_once_per_line() {
    let fixture = valid_fixture();
    write(
        fixture.path(),
        "crates/volicord-cli/src/connection_command/output/text.rs",
        "pub const MESSAGE: &str = \"Prepare a supported agent host, not a supported host.\";\n",
    );

    let report = report(fixture.path());
    let errors = category_errors(&report, "public_language.ambiguous_host_support_claim");

    assert_eq!(errors.len(), 1, "{:#?}", report.issues());
    assert_eq!(
        errors[0].path(),
        "crates/volicord-cli/src/connection_command/output/text.rs"
    );
    assert!(
        errors[0].message().contains("support_status") && errors[0].message().contains("verified"),
        "{:#?}",
        report.issues()
    );
}

#[test]
fn reports_each_ambiguous_host_support_grammar_class_in_public_cli_source() {
    let fixture = valid_fixture();
    let claims = [
        "supported managed host",
        "supported agent-hosts",
        "supported-host detection",
        "managed coding-agent host support for Codex and Claude Code",
        "supported Agent Connection",
        "support for Agent Connection",
        "Agent Connection support is available",
        "Agent Connection is supported",
        "supported managed connection hosts",
        "managed host is supported",
        "agent-hosts are supported",
        "host is supported",
        "supports Codex",
        "supports Claude Code",
        "supports both Codex and Claude Code",
        "Codex is supported",
        "Codex support is available",
        "Claude Code is fully supported",
        "Claude Code support is available",
        "Codex and Claude Code are supported",
        "supports the record profile",
        "supported `record` profile",
        "supports the `record` profile",
        "`record` profile is supported",
        "supports `--profile record`",
        "detective profile is supported",
        "supported detective host configuration",
        "Record and Detective profiles are supported",
        "`record` and `detective` profiles are supported",
        "supports the Record and Detective profiles",
    ];
    write(
        fixture.path(),
        "crates/volicord-cli/src/doctor_command.rs",
        &claims
            .iter()
            .enumerate()
            .map(|(index, claim)| format!("pub const CLAIM_{index}: &str = \"{claim}\";\n"))
            .collect::<String>(),
    );

    let report = report(fixture.path());
    let errors = category_errors(&report, "public_language.ambiguous_host_support_claim");

    assert_eq!(errors.len(), claims.len(), "{:#?}", report.issues());
    assert!(errors
        .iter()
        .all(|error| { error.path() == "crates/volicord-cli/src/doctor_command.rs" }));
}

#[test]
fn reports_ambiguous_connection_claim_in_generic_host_output_source() {
    let fixture = valid_fixture();
    write(
        fixture.path(),
        "crates/volicord-cli/src/host_integration/generic.rs",
        "pub const MESSAGE: &str = \"Configure after a supported Agent Connection exists.\";\n",
    );

    let report = report(fixture.path());
    let errors = category_errors(&report, "public_language.ambiguous_host_support_claim");

    assert_eq!(errors.len(), 1, "{:#?}", report.issues());
    assert_eq!(
        errors[0].path(),
        "crates/volicord-cli/src/host_integration/generic.rs"
    );
}

#[test]
fn reports_ambiguous_claim_in_builtin_host_adapter_output_source() {
    let fixture = valid_fixture();
    write(
        fixture.path(),
        "crates/volicord-cli/src/host_integration/codex/adapter.rs",
        "pub const MESSAGE: &str = \"The Agent Connection is supported.\";\n",
    );

    let report = report(fixture.path());
    let errors = category_errors(&report, "public_language.ambiguous_host_support_claim");

    assert_eq!(errors.len(), 1, "{:#?}", report.issues());
    assert_eq!(
        errors[0].path(),
        "crates/volicord-cli/src/host_integration/codex/adapter.rs"
    );
}

#[test]
fn permits_negative_and_typed_host_support_language() {
    let fixture = valid_fixture();
    write(
        fixture.path(),
        "crates/volicord-cli/src/connection_command.rs",
        "pub const MESSAGE: &str = \"unsupported host; unsupported managed host; unsupported agent-host; supported hostname; notsupported host; support_status=unsupported_by_host; only verified establishes the feature claim; 미지원 호스트; 미지원 관리 호스트; Codex를 지원하지 않습니다\";\n",
    );

    let report = report(fixture.path());

    assert!(
        category_errors(&report, "public_language.ambiguous_host_support_claim").is_empty(),
        "{:#?}",
        report.issues()
    );
}
