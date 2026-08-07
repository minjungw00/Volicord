use serde::Serialize;

use super::plan::RUN_DIRECTORY_PLACEHOLDER;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CurrentValidationCommandKind {
    Process,
    ExactAggregate,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CurrentValidationCommand {
    pub id: String,
    pub label: String,
    pub program: String,
    pub args: Vec<String>,
    pub kind: CurrentValidationCommandKind,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CurrentValidationPlan {
    pub owner: String,
    pub platform: String,
    pub commands: Vec<CurrentValidationCommand>,
}

impl CurrentValidationPlan {
    pub fn render_human(&self) -> String {
        let mut output = format!(
            "current repository validation plan\nowner: {}\nplatform: {}\ncommands:\n",
            self.owner, self.platform
        );
        for command in &self.commands {
            output.push_str(&format!(
                "- {}: {} [{} {}]\n",
                command.id,
                command.label,
                command.program,
                command.args.join(" ")
            ));
        }
        output
    }
}

pub fn current_linux_validation_plan() -> CurrentValidationPlan {
    current_validation_plan("linux", "target/debug/volicord")
}

pub(crate) fn current_platform_validation_plan() -> CurrentValidationPlan {
    if cfg!(windows) {
        current_validation_plan("windows", "target/debug/volicord.exe")
    } else if cfg!(target_os = "macos") {
        current_validation_plan("macos", "target/debug/volicord")
    } else {
        current_linux_validation_plan()
    }
}

fn current_validation_plan(platform: &str, binary: &str) -> CurrentValidationPlan {
    let source_bundle_output = format!("{RUN_DIRECTORY_PLACEHOLDER}/volicord-source.zip");
    CurrentValidationPlan {
        owner: "xtask::validation::current_plan".to_owned(),
        platform: platform.to_owned(),
        commands: vec![
            process(
                "rust-formatting",
                "Rust formatting",
                "cargo",
                &["fmt", "--all", "--check"],
            ),
            process(
                "workspace-architecture",
                "workspace architecture",
                "cargo",
                &["run", "--locked", "-p", "xtask", "--", "architecture-check"],
            ),
            process(
                "workspace-lint",
                "workspace lint",
                "cargo",
                &[
                    "clippy",
                    "--locked",
                    "--workspace",
                    "--all-targets",
                    "--all-features",
                    "--",
                    "-D",
                    "warnings",
                ],
            ),
            process(
                "documentation",
                "documentation owners and generated drift",
                "cargo",
                &["run", "--locked", "-p", "xtask", "--", "docs-check"],
            ),
            process_owned(
                "source-bundle",
                "committed source bundle",
                "cargo",
                vec![
                    "run".to_owned(),
                    "--locked".to_owned(),
                    "-p".to_owned(),
                    "xtask".to_owned(),
                    "--".to_owned(),
                    "source-bundle".to_owned(),
                    "--output".to_owned(),
                    source_bundle_output,
                ],
                CurrentValidationCommandKind::Process,
            ),
            process(
                "mcp-spec",
                "pinned MCP specification fixtures",
                "cargo",
                &["run", "--locked", "-p", "xtask", "--", "mcp-spec-check"],
            ),
            process(
                "maintainability",
                "maintainability report",
                "cargo",
                &[
                    "run",
                    "--locked",
                    "-p",
                    "xtask",
                    "--",
                    "maintainability-report",
                ],
            ),
            process(
                "mcp-protocol-conformance",
                "registry-driven MCP protocol conformance",
                "cargo",
                &[
                    "test",
                    "--locked",
                    "-p",
                    "volicord-mcp",
                    "--test",
                    "protocol_conformance",
                ],
            ),
            process(
                "public-contract-snapshots",
                "public contract snapshots",
                "cargo",
                &[
                    "test",
                    "--locked",
                    "-p",
                    "volicord-integration-tests",
                    "--test",
                    "public_contract_snapshots",
                ],
            ),
            process(
                "storage-ddl-contract",
                "storage DDL contract",
                "cargo",
                &[
                    "test",
                    "--locked",
                    "-p",
                    "volicord-store",
                    "--test",
                    "storage_ddl_contract",
                ],
            ),
            process(
                "mcp-stdio-contract",
                "MCP stdio contract",
                "cargo",
                &[
                    "test",
                    "--locked",
                    "-p",
                    "volicord-cli",
                    "--test",
                    "mcp_transport",
                ],
            ),
            process(
                "mcp-agent-connection-contract",
                "MCP Agent Connection contract",
                "cargo",
                &[
                    "test",
                    "--locked",
                    "-p",
                    "volicord-integration-tests",
                    "--test",
                    "mcp_connection",
                ],
            ),
            process(
                "local-volicord-build",
                "local Volicord binary build",
                "cargo",
                &[
                    "build",
                    "--locked",
                    "-p",
                    "volicord-cli",
                    "--bin",
                    "volicord",
                ],
            ),
            process(
                "local-volicord-smoke",
                "local Volicord binary smoke",
                "cargo",
                &[
                    "run",
                    "--locked",
                    "-p",
                    "volicord-release-smoke",
                    "--",
                    "--bin",
                    binary,
                ],
            ),
            process(
                "release-integrity",
                "release integrity",
                "cargo",
                &[
                    "test",
                    "--locked",
                    "-p",
                    "volicord-release-integrity-tests",
                    "--all-targets",
                    "--all-features",
                ],
            ),
            process_owned(
                "exact-workspace-aggregate",
                "exact workspace aggregate",
                "cargo",
                strings(&[
                    "test",
                    "--locked",
                    "--workspace",
                    "--all-targets",
                    "--all-features",
                ]),
                CurrentValidationCommandKind::ExactAggregate,
            ),
        ],
    }
}

fn process(id: &str, label: &str, program: &str, args: &[&str]) -> CurrentValidationCommand {
    process_owned(
        id,
        label,
        program,
        strings(args),
        CurrentValidationCommandKind::Process,
    )
}

fn process_owned(
    id: &str,
    label: &str,
    program: &str,
    args: Vec<String>,
    kind: CurrentValidationCommandKind,
) -> CurrentValidationCommand {
    CurrentValidationCommand {
        id: id.to_owned(),
        label: label.to_owned(),
        program: program.to_owned(),
        args,
        kind,
    }
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_linux_plan_has_stable_order_and_release_boundaries() {
        let plan = current_linux_validation_plan();
        assert_eq!(
            plan.commands
                .iter()
                .map(|command| command.id.as_str())
                .collect::<Vec<_>>(),
            [
                "rust-formatting",
                "workspace-architecture",
                "workspace-lint",
                "documentation",
                "source-bundle",
                "mcp-spec",
                "maintainability",
                "mcp-protocol-conformance",
                "public-contract-snapshots",
                "storage-ddl-contract",
                "mcp-stdio-contract",
                "mcp-agent-connection-contract",
                "local-volicord-build",
                "local-volicord-smoke",
                "release-integrity",
                "exact-workspace-aggregate",
            ]
        );
        assert_eq!(
            plan.commands
                .iter()
                .filter(|command| command.kind == CurrentValidationCommandKind::ExactAggregate)
                .count(),
            1
        );
        let build = plan
            .commands
            .iter()
            .position(|command| command.id == "local-volicord-build")
            .expect("local binary build");
        let smoke = plan
            .commands
            .iter()
            .position(|command| command.id == "local-volicord-smoke")
            .expect("local binary smoke");
        assert!(build < smoke);
        assert!(plan.commands.iter().any(|command| {
            command.id == "source-bundle"
                && command
                    .args
                    .iter()
                    .any(|argument| argument.contains(RUN_DIRECTORY_PLACEHOLDER))
        }));
    }

    #[test]
    fn ci_delegates_its_linux_checks_to_the_shared_final_plan_once() {
        let workflow: serde_yaml::Value =
            serde_yaml::from_str(include_str!("../../../.github/workflows/ci.yml"))
                .expect("parse CI workflow");
        let steps = workflow["jobs"]["checks"]["steps"]
            .as_sequence()
            .expect("Linux CI steps");
        let invocations = steps
            .iter()
            .filter_map(|step| step["run"].as_str())
            .filter(|run| {
                run.contains("cargo run")
                    && run.contains("-p xtask")
                    && run.contains("validate final")
                    && run.contains("--base HEAD")
            })
            .count();
        assert_eq!(invocations, 1);
        for duplicated in [
            "cargo fmt --check",
            "cargo clippy",
            "source-bundle --output",
            "cargo test --locked --workspace",
            "-p volicord-release-smoke",
        ] {
            assert!(steps
                .iter()
                .filter_map(|step| step["run"].as_str())
                .all(|run| !run.contains(duplicated)));
        }
    }

    #[test]
    fn ci_runs_bounded_mutation_lease_stability_outside_the_shared_final_plan() {
        let workflow: serde_yaml::Value =
            serde_yaml::from_str(include_str!("../../../.github/workflows/ci.yml"))
                .expect("parse CI workflow");
        let job = &workflow["jobs"]["mutation-lease-stability"];
        assert_eq!(job["runs-on"].as_str(), Some("ubuntu-24.04"));
        assert_eq!(job["timeout-minutes"].as_u64(), Some(15));
        let steps = job["steps"].as_sequence().expect("stability CI steps");
        let stability_steps = steps
            .iter()
            .filter(|step| {
                step["run"].as_str().is_some_and(|run| {
                    run.contains("cargo test --locked -p volicord-platform-fs")
                        && run.contains("--test mutation_lease_process")
                })
            })
            .collect::<Vec<_>>();
        assert_eq!(stability_steps.len(), 1);
        let stability = stability_steps[0];
        assert_eq!(
            stability["env"]["MUTATION_LEASE_STABILITY_ITERATIONS"].as_str(),
            Some("20")
        );
        let run = stability["run"].as_str().expect("stability command");
        assert!(
            run.contains("while [ \"$iteration\" -le \"$MUTATION_LEASE_STABILITY_ITERATIONS\" ]")
        );
        assert!(run.contains("iteration-${iteration}.log"));
        assert!(run.contains("set -o pipefail"));
        assert!(run.contains("exit \"$status\""));
        assert!(!run.contains("--test-threads=1"));
        assert!(!run.contains("RUST_TEST_THREADS"));
        assert!(!run.contains("until cargo test"));

        let upload = steps
            .iter()
            .find(|step| step["uses"].as_str() == Some("actions/upload-artifact@v4"))
            .expect("failing stability log upload");
        assert_eq!(upload["if"].as_str(), Some("failure()"));
        assert_eq!(
            upload["with"]["path"].as_str(),
            Some("target/mutation-lease-stability/")
        );

        assert!(current_linux_validation_plan()
            .commands
            .iter()
            .all(|command| {
                command.id != "mutation-lease-stability"
                    && !command.args.iter().any(|argument| {
                        argument.contains("mutation_lease_process")
                            || argument.contains("MUTATION_LEASE_STABILITY_ITERATIONS")
                    })
            }));
    }

    #[test]
    fn current_plan_has_machine_readable_and_human_renderings() {
        let plan = current_linux_validation_plan();
        let json = serde_json::to_string_pretty(&plan).expect("serialize current plan");
        let human = plan.render_human();
        for command in &plan.commands {
            assert!(json.contains(&command.id));
            assert!(human.contains(&command.id));
        }
    }
}
