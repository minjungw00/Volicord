use std::fmt::Write;

const REPOSITORY_BOUNDARY: &str = "xtask maintains the Volicord source repository. It does not validate a Volicord Runtime Home, Agent Connection, Product Repository workflow, or product correctness.";

struct CommandHelp {
    name: &'static str,
    summary: &'static str,
    usage: &'static str,
    details: &'static str,
    options: &'static [&'static str],
}

const COMMANDS: &[CommandHelp] = &[
    CommandHelp {
        name: "owner-route",
        summary: "Resolve maintenance owners and validation classes for changed repository paths.",
        usage: "cargo run --locked -p xtask -- owner-route --changed [--base REVISION] [--json]",
        details: "Reports repository instructions, owners, packages, and validation classes without running their checks.",
        options: &[
            "--changed        Include committed, staged, unstaged, and untracked changes.",
            "--base REVISION Compare committed changes after REVISION; omit it for working-tree changes only.",
            "--json           Emit the report as JSON.",
        ],
    },
    CommandHelp {
        name: "validate",
        summary: "Plan and run repository-owned validation with durable results.",
        usage: "cargo run --locked -p xtask -- validate <focused|final> --base REVISION [--json]",
        details: "Use focused for intermediate commits. Use final once after the complete series; it owns the exact workspace aggregate and bounded diagnostics.",
        options: &[
            "--base REVISION Use REVISION as the explicit change-series base.",
            "--json           Emit the completed run summary as JSON.",
        ],
    },
    CommandHelp {
        name: "ci-base",
        summary: "Resolve and verify the event-specific CI change-series base.",
        usage: "cargo run --locked -p xtask -- ci-base --event-name EVENT --event-path PATH --head REVISION [--github-output PATH]",
        details: "Selects the pull-request base SHA, valid push before SHA, or required manual input, then verifies a reachable nonempty ancestor range.",
        options: &[
            "--event-name EVENT   Select pull_request, push, or workflow_dispatch handling.",
            "--event-path PATH    Read the GitHub event JSON payload from PATH.",
            "--head REVISION      Resolve and verify the checked-out head revision.",
            "--github-output PATH Append the resolved base as a GitHub Actions step output.",
        ],
    },
    CommandHelp {
        name: "validation-plan",
        summary: "Inspect the current Linux repository-validation command plan without executing it.",
        usage: "cargo run --locked -p xtask -- validation-plan [--json]",
        details: "Shows the typed command membership and ordering shared by final validation and the main Linux CI job.",
        options: &["--json           Emit the plan as JSON."],
    },
    CommandHelp {
        name: "architecture-check",
        summary: "Check the maintained Rust workspace package and dependency architecture.",
        usage: "cargo run --locked -p xtask -- architecture-check",
        details: "Compares Cargo workspace metadata with the repository-owned architecture model.",
        options: &[],
    },
    CommandHelp {
        name: "docs-check",
        summary: "Check maintained documentation structure and generated-region drift.",
        usage: "cargo run --locked -p xtask -- docs-check",
        details: "Validates documentation metadata, routes, links, terminology, bilingual structure, examples, and generated regions.",
        options: &[],
    },
    CommandHelp {
        name: "docs-sync",
        summary: "Regenerate marked documentation regions from maintained sources.",
        usage: "cargo run --locked -p xtask -- docs-sync",
        details: "Updates generator-owned regions only; review the resulting documentation diff before committing it.",
        options: &[],
    },
    CommandHelp {
        name: "maintainability-report",
        summary: "Render diagnostic maintainability signals for repository review.",
        usage: "cargo run --locked -p xtask -- maintainability-report",
        details: "Reports source-size, mixed-responsibility, and test-coverage hints without making an acceptance decision.",
        options: &[],
    },
    CommandHelp {
        name: "mcp-spec-check",
        summary: "Check the integrity of pinned MCP specification fixtures offline.",
        usage: "cargo run --locked -p xtask -- mcp-spec-check",
        details: "Checks the repository fixture manifest, immutable references, schemas, checksums, and supported-profile parity.",
        options: &[],
    },
    CommandHelp {
        name: "mcp-spec-sync",
        summary: "Refresh pinned MCP specification fixtures as an explicit maintenance action.",
        usage: "cargo run --locked -p xtask -- mcp-spec-sync",
        details: "Resolves and downloads recorded upstream revisions, validates the complete candidate, and then replaces the fixture.",
        options: &[],
    },
    CommandHelp {
        name: "release-version-check",
        summary: "Check workspace and optional release-tag version consistency.",
        usage: "cargo run --locked -p xtask -- release-version-check [--tag TAG]",
        details: "Validates repository release metadata before packaging or publication.",
        options: &["--tag TAG        Require TAG to match the workspace release version."],
    },
    CommandHelp {
        name: "source-bundle",
        summary: "Create and validate a deterministic source archive from one committed Git tree.",
        usage: "cargo run --locked -p xtask -- source-bundle --output PATH [--commit COMMIT]",
        details: "This is a maintenance and release-packaging command, not an ordinary installation step.",
        options: &[
            "--output PATH    Write the completed ZIP to PATH.",
            "--commit COMMIT Select COMMIT instead of HEAD.",
        ],
    },
    CommandHelp {
        name: "source-bundle-validate",
        summary: "Compare a source archive with the complete selected Git tree.",
        usage: "cargo run --locked -p xtask -- source-bundle-validate --input PATH [--commit COMMIT]",
        details: "Checks archive paths, types, modes, link targets, contents, and deterministic metadata for maintenance or release review.",
        options: &[
            "--input PATH     Read the source ZIP from PATH.",
            "--commit COMMIT Select COMMIT instead of HEAD.",
        ],
    },
];

pub(crate) fn requested_help(args: &[String]) -> Option<Result<String, String>> {
    match args {
        [option] if is_help(option) => Some(Ok(render_top_level())),
        [command, option] if is_help(option) => Some(render_command(command)),
        [command] if command == "help" => Some(Ok(render_top_level())),
        [help, command] if help == "help" => Some(render_command(command)),
        _ => None,
    }
}

pub(crate) fn short_usage() -> &'static str {
    "usage: cargo run --locked -p xtask -- <COMMAND> [OPTIONS]\nRun `cargo run --locked -p xtask -- --help` for repository-maintenance help."
}

fn is_help(value: &str) -> bool {
    matches!(value, "-h" | "--help")
}

fn render_top_level() -> String {
    let mut output = String::from("Volicord repository maintenance\n\n");
    writeln!(output, "{REPOSITORY_BOUNDARY}").expect("writing to a String cannot fail");
    output.push_str(
        "\nUsage:\n  cargo run --locked -p xtask -- <COMMAND> [OPTIONS]\n  cargo run --locked -p xtask -- <COMMAND> --help\n\nCommands:\n",
    );
    for command in COMMANDS {
        writeln!(output, "  {:<25} {}", command.name, command.summary)
            .expect("writing to a String cannot fail");
    }
    output.push_str("\nUse `<COMMAND> --help` for command-specific maintenance help.\n");
    output
}

fn render_command(name: &str) -> Result<String, String> {
    let Some(command) = COMMANDS.iter().find(|command| command.name == name) else {
        return Err(format!(
            "unknown xtask command `{name}`; run `cargo run --locked -p xtask -- --help`"
        ));
    };

    let mut output = format!(
        "{} — {}\n\nUsage:\n  {}\n\nPurpose:\n  {}\n",
        command.name, command.summary, command.usage, command.details
    );
    output.push_str("\nOptions:\n");
    for option in command.options {
        writeln!(output, "  {option}").expect("writing to a String cannot fail");
    }
    output.push_str("  -h, --help      Print this command-specific help.\n");
    output.push_str("\nRepository boundary:\n  ");
    output.push_str(REPOSITORY_BOUNDARY);
    output.push('\n');
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_maintenance_command_has_specific_help_and_the_repository_boundary() {
        for command in COMMANDS {
            let help = render_command(command.name).expect("known command help");
            assert!(help.contains(command.usage));
            assert!(help.contains(REPOSITORY_BOUNDARY));
        }
    }

    #[test]
    fn unknown_command_help_is_actionable() {
        let error = render_command("not-a-command").expect_err("unknown command");
        assert!(error.contains("unknown xtask command `not-a-command`"));
        assert!(error.contains("--help"));
    }
}
