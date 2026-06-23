#![forbid(unsafe_code)]

use std::{fmt, process};

fn main() {
    match dispatch_args(std::env::args()) {
        Ok(McpCommand::Stdio { integration_id }) => {
            if let Err(error) = volicord_mcp::run_stdio_from_env(&integration_id) {
                eprintln!("error: {error}");
                process::exit(1);
            }
        }
        Ok(McpCommand::Help) => {
            print!("{}", usage());
        }
        Ok(McpCommand::Version) => {
            print!("{}", version());
        }
        Ok(McpCommand::Check {
            integration_id,
            project_id,
        }) => {
            match volicord_mcp::run_preflight_check_from_env(&integration_id, project_id.as_deref())
            {
                Ok(report) => print!("{report}"),
                Err(error) => {
                    eprintln!("error: {error}");
                    process::exit(1);
                }
            }
        }
        Err(error) => {
            eprintln!("error: {error}\n\n{}", usage());
            process::exit(2);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum McpCommand {
    Stdio {
        integration_id: String,
    },
    Help,
    Version,
    Check {
        integration_id: String,
        project_id: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct McpCommandError(String);

impl fmt::Display for McpCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for McpCommandError {}

fn dispatch_args<I, S>(args: I) -> Result<McpCommand, McpCommandError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let args = args.into_iter().map(Into::into).collect::<Vec<_>>();
    let options = &args[1..];
    match options {
        [option] if option == "-h" || option == "--help" => Ok(McpCommand::Help),
        [option] if option == "-V" || option == "--version" => Ok(McpCommand::Version),
        _ => parse_operational_options(options),
    }
}

fn parse_operational_options(options: &[String]) -> Result<McpCommand, McpCommandError> {
    let mut check = false;
    let mut integration_id = None;
    let mut project_id = None;
    let mut index = 0;

    while index < options.len() {
        match options[index].as_str() {
            "--check" => {
                if check {
                    return Err(McpCommandError(
                        "--check was supplied more than once".to_owned(),
                    ));
                }
                check = true;
                index += 1;
            }
            "--integration" => {
                if integration_id.is_some() {
                    return Err(McpCommandError(
                        "--integration was supplied more than once".to_owned(),
                    ));
                }
                index += 1;
                let value = options
                    .get(index)
                    .ok_or_else(|| McpCommandError("--integration requires a value".to_owned()))?;
                if value.starts_with('-') {
                    return Err(McpCommandError("--integration requires a value".to_owned()));
                }
                integration_id = Some(value.clone());
                index += 1;
            }
            "--project" => {
                if project_id.is_some() {
                    return Err(McpCommandError(
                        "--project was supplied more than once".to_owned(),
                    ));
                }
                index += 1;
                let value = options
                    .get(index)
                    .ok_or_else(|| McpCommandError("--project requires a value".to_owned()))?;
                if value.starts_with('-') {
                    return Err(McpCommandError("--project requires a value".to_owned()));
                }
                project_id = Some(value.clone());
                index += 1;
            }
            "-h" | "--help" | "-V" | "--version" => {
                return Err(McpCommandError(
                    "cannot combine harness-mcp command-line modes".to_owned(),
                ))
            }
            option if option.starts_with('-') => {
                return Err(McpCommandError(format!("unknown option: {option}")));
            }
            argument => return Err(McpCommandError(format!("unexpected argument: {argument}"))),
        }
    }

    if project_id.is_some() && !check {
        return Err(McpCommandError(
            "--project is only valid with --check".to_owned(),
        ));
    }
    let integration_id = integration_id.ok_or_else(|| {
        McpCommandError("--integration is required for integration-bound startup".to_owned())
    })?;

    if check {
        Ok(McpCommand::Check {
            integration_id,
            project_id,
        })
    } else {
        Ok(McpCommand::Stdio { integration_id })
    }
}

fn usage() -> String {
    "Usage:\n  harness-mcp --integration <integration_id>\n  harness-mcp --check --integration <integration_id>\n  harness-mcp --check --integration <integration_id> --project <project_id>\n  harness-mcp --help\n  harness-mcp --version\n\nEnvironment:\n  HARNESS_HOME                 Optional Runtime Home path (default: $HOME/.harness)\n\nThe selected Agent Integration Profile supplies the MCP surface binding. Project selection happens per public Harness tool call.\n"
        .to_owned()
}

fn version() -> String {
    format!("harness-mcp {}\n", env!("CARGO_PKG_VERSION"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_argument_dispatch_is_usage_classified() {
        let error = dispatch_args(["harness-mcp"]).expect_err("no args should be rejected");

        assert_eq!(
            error.to_string(),
            "--integration is required for integration-bound startup"
        );
    }

    #[test]
    fn help_and_version_do_not_require_environment() {
        assert_eq!(
            dispatch_args(["harness-mcp", "--help"]).expect("help should dispatch"),
            McpCommand::Help
        );
        assert!(usage().contains("--integration <integration_id>"));
        assert!(!usage().contains("HARNESS_PROJECT_ID"));
        assert_eq!(
            version(),
            format!("harness-mcp {}\n", env!("CARGO_PKG_VERSION"))
        );
        assert_eq!(
            dispatch_args(["harness-mcp", "-V"]).expect("version should dispatch"),
            McpCommand::Version
        );
    }

    #[test]
    fn integration_startup_forms_dispatch() {
        assert_eq!(
            dispatch_args(["harness-mcp", "--integration", "agent_main"])
                .expect("integration stdio should dispatch"),
            McpCommand::Stdio {
                integration_id: "agent_main".to_owned()
            }
        );
        assert_eq!(
            dispatch_args(["harness-mcp", "--check", "--integration", "agent_main"])
                .expect("integration check should dispatch"),
            McpCommand::Check {
                integration_id: "agent_main".to_owned(),
                project_id: None
            }
        );
        assert_eq!(
            dispatch_args([
                "harness-mcp",
                "--check",
                "--integration",
                "agent_main",
                "--project",
                "project_a",
            ])
            .expect("project check should dispatch"),
            McpCommand::Check {
                integration_id: "agent_main".to_owned(),
                project_id: Some("project_a".to_owned())
            }
        );
    }

    #[test]
    fn invalid_option_is_usage_classified() {
        let error = dispatch_args(["harness-mcp", "--bogus"])
            .expect_err("unknown option should be a usage error");

        assert_eq!(error.to_string(), "unknown option: --bogus");
    }

    #[test]
    fn combined_modes_are_usage_classified() {
        let error = dispatch_args(["harness-mcp", "--check", "--version"])
            .expect_err("combined modes should be rejected");

        assert_eq!(
            error.to_string(),
            "cannot combine harness-mcp command-line modes"
        );
    }

    #[test]
    fn missing_integration_is_usage_classified() {
        let error = dispatch_args(["harness-mcp", "--project", "project_a"])
            .expect_err("project without check should be rejected");

        assert_eq!(error.to_string(), "--project is only valid with --check");

        let error = dispatch_args(["harness-mcp", "--check", "--project", "project_a"])
            .expect_err("check without integration should be rejected");

        assert_eq!(
            error.to_string(),
            "--integration is required for integration-bound startup"
        );

        let error = dispatch_args(["harness-mcp", "--check"])
            .expect_err("check without integration should be rejected");

        assert_eq!(
            error.to_string(),
            "--integration is required for integration-bound startup"
        );
    }

    #[test]
    fn positional_arguments_are_usage_classified() {
        let error = dispatch_args(["harness-mcp", "serve"])
            .expect_err("positional arguments should be rejected");

        assert_eq!(error.to_string(), "unexpected argument: serve");
    }
}
