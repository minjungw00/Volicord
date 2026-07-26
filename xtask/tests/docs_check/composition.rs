use super::*;

#[test]
fn focused_validators_compose_without_duplicate_rule_ownership() {
    let fixture = valid_fixture();
    let current = fs::read_to_string(fixture.path().join("docs/en/example.md")).expect("example");
    write(
        fixture.path(),
        "docs/en/example.md",
        &format!("{current}\n```sh cli-example\nvolicord unknown-endpoint\n```\n"),
    );

    let report = report(fixture.path());

    assert_eq!(report.issues().len(), 1, "{:#?}", report.issues());
    assert_eq!(report.issues()[0].category(), "command.invalid_example");
}

#[test]
fn composed_validator_issues_have_deterministic_ordering() {
    let fixture = valid_fixture();
    write(
        fixture.path(),
        "docs/en/example.md",
        "# Overview\n\n[missing](missing.md)\n\n```sh cli-example\nvolicord unknown-endpoint\n```\n",
    );

    let first = report(fixture.path());
    let second = report(fixture.path());

    assert_eq!(first.issues(), second.issues());
    assert!(first
        .issues()
        .windows(2)
        .all(|issues| issues[0] <= issues[1]));
    assert!(has_category(&first, "link.missing_target"));
    assert!(has_category(&first, "command.invalid_example"));
}
