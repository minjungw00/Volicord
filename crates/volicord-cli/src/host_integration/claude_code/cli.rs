use std::{path::PathBuf, process::Command};

use crate::host_integration::{HostScope, ManagedServerEntry};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandInvocation {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    pub success: bool,
    pub status_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

pub trait CommandRunner {
    fn run(&mut self, invocation: &CommandInvocation) -> Result<CommandOutput, String>;
}

#[derive(Debug, Default, Clone)]
pub struct ProductionCommandRunner;

impl CommandRunner for ProductionCommandRunner {
    fn run(&mut self, invocation: &CommandInvocation) -> Result<CommandOutput, String> {
        let mut command = Command::new(&invocation.program);
        command.args(&invocation.args);
        if let Some(cwd) = &invocation.cwd {
            command.current_dir(cwd);
        }
        let output = command.output().map_err(|error| {
            format!(
                "failed to run {} {}: {error}",
                invocation.program,
                invocation.args.join(" ")
            )
        })?;
        Ok(CommandOutput {
            success: output.status.success(),
            status_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}

pub(super) fn build_add_command(
    program: &str,
    scope: HostScope,
    server_name: &str,
    entry: &ManagedServerEntry,
    cwd: Option<PathBuf>,
) -> CommandInvocation {
    let mut args = vec!["mcp".to_owned(), "add".to_owned()];
    for (key, value) in &entry.env {
        args.push("--env".to_owned());
        args.push(format!("{key}={value}"));
    }
    args.extend([
        "--transport".to_owned(),
        "stdio".to_owned(),
        "--scope".to_owned(),
        scope.as_str().to_owned(),
        server_name.to_owned(),
        "--".to_owned(),
        entry.command.clone(),
    ]);
    args.extend(entry.args.clone());
    CommandInvocation {
        program: program.to_owned(),
        args,
        cwd,
    }
}

pub(super) fn build_get_command(
    program: &str,
    server_name: &str,
    cwd: Option<PathBuf>,
) -> CommandInvocation {
    CommandInvocation {
        program: program.to_owned(),
        args: vec!["mcp".to_owned(), "get".to_owned(), server_name.to_owned()],
        cwd,
    }
}

pub(super) fn build_remove_command(
    program: &str,
    scope: HostScope,
    server_name: &str,
    cwd: Option<PathBuf>,
) -> CommandInvocation {
    CommandInvocation {
        program: program.to_owned(),
        args: vec![
            "mcp".to_owned(),
            "remove".to_owned(),
            "--scope".to_owned(),
            scope.as_str().to_owned(),
            server_name.to_owned(),
        ],
        cwd,
    }
}
