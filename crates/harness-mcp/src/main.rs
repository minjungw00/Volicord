#![forbid(unsafe_code)]

use std::{fmt, process};

fn main() {
    match dispatch_args(std::env::args()) {
        Ok(McpCommand::Stdio) => {
            if let Err(error) = harness_mcp::run_stdio_from_env() {
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
        Ok(McpCommand::Check) => match harness_mcp::run_preflight_check_from_env() {
            Ok(report) => print!("{report}"),
            Err(error) => {
                eprintln!("error: {error}");
                process::exit(1);
            }
        },
        Err(error) => {
            eprintln!("error: {error}\n\n{}", usage());
            process::exit(2);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum McpCommand {
    Stdio,
    Help,
    Version,
    Check,
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
        [] => Ok(McpCommand::Stdio),
        [option] if option == "-h" || option == "--help" => Ok(McpCommand::Help),
        [option] if option == "-V" || option == "--version" => Ok(McpCommand::Version),
        [option] if option == "--check" => Ok(McpCommand::Check),
        [first, rest @ ..]
            if is_mode_option(first)
                && rest.iter().any(|option| is_mode_option(option.as_str())) =>
        {
            Err(McpCommandError(
                "cannot combine harness-mcp command-line modes".to_owned(),
            ))
        }
        [first, extra, ..] if is_mode_option(first) => {
            Err(McpCommandError(format!("unexpected argument: {extra}")))
        }
        [option, ..] if option.starts_with('-') => {
            Err(McpCommandError(format!("unknown option: {option}")))
        }
        [argument, ..] => Err(McpCommandError(format!("unexpected argument: {argument}"))),
    }
}

fn is_mode_option(option: &str) -> bool {
    matches!(option, "-h" | "--help" | "-V" | "--version" | "--check")
}

fn usage() -> String {
    "Usage:\n  harness-mcp\n  harness-mcp --check\n  harness-mcp --help\n  harness-mcp --version\n\nEnvironment:\n  HARNESS_PROJECT_ID           Required project binding for stdio and --check\n  HARNESS_SURFACE_ID           Required surface binding for stdio and --check\n  HARNESS_HOME                 Optional Runtime Home path (default: $HOME/.harness)\n  HARNESS_SURFACE_INSTANCE_ID  Optional explicit surface instance binding\n\nNo arguments starts the line-delimited MCP stdio loop.\n"
        .to_owned()
}

fn version() -> String {
    format!("harness-mcp {}\n", env!("CARGO_PKG_VERSION"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_argument_dispatch_selects_stdio() {
        assert_eq!(
            dispatch_args(["harness-mcp"]).expect("no args should dispatch"),
            McpCommand::Stdio
        );
    }

    #[test]
    fn help_and_version_do_not_require_environment() {
        assert_eq!(
            dispatch_args(["harness-mcp", "--help"]).expect("help should dispatch"),
            McpCommand::Help
        );
        assert!(usage().contains("HARNESS_PROJECT_ID"));
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
    fn positional_arguments_are_usage_classified() {
        let error = dispatch_args(["harness-mcp", "serve"])
            .expect_err("positional arguments should be rejected");

        assert_eq!(error.to_string(), "unexpected argument: serve");
    }
}
