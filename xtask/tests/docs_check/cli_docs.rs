use super::*;

#[test]
fn parses_a_canonical_example_for_every_public_cli_endpoint() {
    let fixture = valid_fixture();
    let invocations =
        volicord_command_model::canonical_public_invocations().expect("canonical invocations");
    let commands = invocations
        .iter()
        .map(|invocation| invocation.arguments().join(" "))
        .collect::<Vec<_>>()
        .join("\n");
    let examples = format!("```sh cli-example\n{commands}\n```\n");
    write(
        fixture.path(),
        "docs/en/example.md",
        &format!("# Overview\n\n{examples}"),
    );
    write(
        fixture.path(),
        "docs/ko/example.md",
        &format!("<a id=\"overview\"></a>\n# 개요\n\n{examples}"),
    );

    let report = report(fixture.path());

    assert!(report.is_ok(), "{:#?}", report.issues());
}

#[test]
fn accepts_inline_supported_init_profile_examples() {
    let fixture = valid_fixture();
    let commands = r#"```sh cli-example
volicord init --host codex --repo /path/to/repo --profile=record
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

    assert!(report.is_ok(), "{:#?}", report.issues());
}

#[test]
fn rejects_generic_unknown_commands_and_options_in_cli_examples() {
    let fixture = valid_fixture();
    write(
        fixture.path(),
        "docs/en/example.md",
        "# Overview\n\n```sh cli-example\nvolicord not-a-command\nvolicord doctor --not-an-option\n```\n",
    );

    let report = report(fixture.path());
    let errors = category_errors(&report, "command.invalid_example");

    assert_eq!(errors.len(), 2, "{:#?}", report.issues());
    assert!(errors.iter().any(|error| error
        .message()
        .contains("unrecognized subcommand 'not-a-command'")));
    assert!(errors
        .iter()
        .any(|error| error.message().contains("--not-an-option")));
}

#[test]
fn ignores_unmarked_shell_fences_text_fences_and_displayed_output() {
    let fixture = valid_fixture();
    write(
        fixture.path(),
        "docs/en/example.md",
        "# Overview\n\n```sh\nvolicord not-a-command\n```\n\n```text\nvolicord doctor --not-an-option\n```\n\nOutput: `volicord not-a-command`\n",
    );

    let report = report(fixture.path());
    assert!(
        category_errors(&report, "command.invalid_example").is_empty(),
        "{:#?}",
        report.issues()
    );
}
