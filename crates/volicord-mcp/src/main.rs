#![forbid(unsafe_code)]

use std::{fmt, process};

fn main() {
    match dispatch_args(std::env::args()) {
        Ok(McpCommand::Stdio { connection_id }) => {
            if let Err(error) = volicord_mcp::run_stdio_from_env(&connection_id) {
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
            connection_id,
            project_id,
        }) => {
            match volicord_mcp::run_preflight_check_from_env(&connection_id, project_id.as_deref())
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
        connection_id: String,
    },
    Help,
    Version,
    Check {
        connection_id: String,
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
    let mut connection_id = None;
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
            "--connection" => {
                if connection_id.is_some() {
                    return Err(McpCommandError(
                        "--connection was supplied more than once".to_owned(),
                    ));
                }
                index += 1;
                let value = options
                    .get(index)
                    .ok_or_else(|| McpCommandError("--connection requires a value".to_owned()))?;
                if value.starts_with('-') {
                    return Err(McpCommandError("--connection requires a value".to_owned()));
                }
                connection_id = Some(value.clone());
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
                    "cannot combine volicord-mcp command-line modes".to_owned(),
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
    let connection_id = connection_id.ok_or_else(|| {
        McpCommandError("--connection is required for connection-bound startup".to_owned())
    })?;

    if check {
        Ok(McpCommand::Check {
            connection_id,
            project_id,
        })
    } else {
        Ok(McpCommand::Stdio { connection_id })
    }
}

fn usage() -> String {
    "Usage:\n  volicord-mcp --connection <connection_id>\n  volicord-mcp --check --connection <connection_id>\n  volicord-mcp --check --connection <connection_id> --project <project_id>\n  volicord-mcp --help\n  volicord-mcp --version\n\nEnvironment:\n  VOLICORD_HOME                 Optional Runtime Home path (default: $HOME/.volicord)\n\nThe selected Agent Connection supplies the MCP process binding. Project selection happens per public Volicord tool call.\n"
        .to_owned()
}

fn version() -> String {
    format!("volicord-mcp {}\n", env!("CARGO_PKG_VERSION"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_argument_dispatch_is_usage_classified() {
        let error = dispatch_args(["volicord-mcp"]).expect_err("no args should be rejected");

        assert_eq!(
            error.to_string(),
            "--connection is required for connection-bound startup"
        );
    }

    #[test]
    fn help_and_version_do_not_require_environment() {
        assert_eq!(
            dispatch_args(["volicord-mcp", "--help"]).expect("help should dispatch"),
            McpCommand::Help
        );
        assert!(usage().contains("--connection <connection_id>"));
        assert!(!usage().contains("VOLICORD_PROJECT_ID"));
        assert_eq!(
            version(),
            format!("volicord-mcp {}\n", env!("CARGO_PKG_VERSION"))
        );
        assert_eq!(
            dispatch_args(["volicord-mcp", "-V"]).expect("version should dispatch"),
            McpCommand::Version
        );
    }

    #[test]
    fn connection_startup_forms_dispatch() {
        assert_eq!(
            dispatch_args(["volicord-mcp", "--connection", "agent_main"])
                .expect("connection stdio should dispatch"),
            McpCommand::Stdio {
                connection_id: "agent_main".to_owned()
            }
        );
        assert_eq!(
            dispatch_args(["volicord-mcp", "--check", "--connection", "agent_main"])
                .expect("connection check should dispatch"),
            McpCommand::Check {
                connection_id: "agent_main".to_owned(),
                project_id: None
            }
        );
        assert_eq!(
            dispatch_args([
                "volicord-mcp",
                "--check",
                "--connection",
                "agent_main",
                "--project",
                "project_a",
            ])
            .expect("project check should dispatch"),
            McpCommand::Check {
                connection_id: "agent_main".to_owned(),
                project_id: Some("project_a".to_owned())
            }
        );
    }

    #[test]
    fn invalid_option_is_usage_classified() {
        let error = dispatch_args(["volicord-mcp", "--bogus"])
            .expect_err("unknown option should be a usage error");

        assert_eq!(error.to_string(), "unknown option: --bogus");
    }

    #[test]
    fn combined_modes_are_usage_classified() {
        let error = dispatch_args(["volicord-mcp", "--check", "--version"])
            .expect_err("combined modes should be rejected");

        assert_eq!(
            error.to_string(),
            "cannot combine volicord-mcp command-line modes"
        );
    }

    #[test]
    fn missing_connection_is_usage_classified() {
        let error = dispatch_args(["volicord-mcp", "--project", "project_a"])
            .expect_err("project without check should be rejected");

        assert_eq!(error.to_string(), "--project is only valid with --check");

        let error = dispatch_args(["volicord-mcp", "--check", "--project", "project_a"])
            .expect_err("check without connection should be rejected");

        assert_eq!(
            error.to_string(),
            "--connection is required for connection-bound startup"
        );

        let error = dispatch_args(["volicord-mcp", "--check"])
            .expect_err("check without connection should be rejected");

        assert_eq!(
            error.to_string(),
            "--connection is required for connection-bound startup"
        );

        let error = dispatch_args(["volicord-mcp", "--connection"])
            .expect_err("missing connection value should be rejected");

        assert_eq!(error.to_string(), "--connection requires a value");
    }

    #[test]
    fn positional_arguments_are_usage_classified() {
        let error = dispatch_args(["volicord-mcp", "serve"])
            .expect_err("positional arguments should be rejected");

        assert_eq!(error.to_string(), "unexpected argument: serve");
    }
}
