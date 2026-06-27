#![forbid(unsafe_code)]

use std::{
    collections::BTreeMap,
    env, fmt, fs,
    path::{Path, PathBuf},
    process,
    time::{SystemTime, UNIX_EPOCH},
};

use volicord_cli::{
    agent_command::{agent_usage, run_agent_command, AgentCommandError, ProductionAgentProcess},
    registration::ADMIN_METADATA_JSON,
    user_command::{run_user_command, user_usage, UserCommandError},
};
use volicord_store::bootstrap::{
    initialize_runtime_home, list_projects, register_project, ProjectRegistration,
    ACTIVE_PROJECT_STATUS,
};
use volicord_store::runtime_home::{resolve_runtime_home, RuntimeHomeResolutionError};

type CliOptions = BTreeMap<String, Vec<String>>;

fn main() {
    let args = env::args().collect::<Vec<_>>();
    let current_dir = match env::current_dir() {
        Ok(path) => path,
        Err(error) => {
            eprintln!("error: failed to read current directory: {error}");
            process::exit(1);
        }
    };

    match run_cli(args, |name| env::var_os(name), &current_dir) {
        Ok(output) => print!("{output}"),
        Err(CliError::Usage(message)) => {
            eprintln!("{message}");
            process::exit(2);
        }
        Err(CliError::Runtime(message)) => {
            eprintln!("error: {message}");
            process::exit(1);
        }
        Err(CliError::FailureOutput(output)) => {
            print!("{output}");
            process::exit(1);
        }
    }
}

fn run_cli<I, S, F>(args: I, env_var: F, current_dir: &Path) -> Result<String, CliError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    let args = args.into_iter().map(Into::into).collect::<Vec<_>>();
    let command = args.get(1).map(String::as_str).unwrap_or("--help");

    match command {
        "-h" | "--help" | "help" => Ok(usage()),
        "-V" | "--version" => {
            if args.len() == 2 {
                Ok(version())
            } else {
                Err(CliError::usage(format!(
                    "unexpected argument for {command}\n\n{}",
                    usage()
                )))
            }
        }
        "init" => command_init(&args[2..], env_var, current_dir),
        "agent" => {
            let mut agent_process = ProductionAgentProcess;
            run_agent_command(&args[2..], current_dir, &mut agent_process).map_err(CliError::from)
        }
        "user" => run_user_command(&args[2..], env_var, current_dir).map_err(CliError::from),
        "project" => command_project(&args[2..], env_var, current_dir),
        other => Err(CliError::usage(format!(
            "unknown command: {other}\n\n{}",
            usage()
        ))),
    }
}

fn command_init<F>(args: &[String], env_var: F, current_dir: &Path) -> Result<String, CliError>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    let options = parse_options(args, &["runtime-home-id"])?;
    let runtime_home = resolve_runtime_home(env_var, current_dir)?;
    let runtime_home_id = options
        .value("runtime-home-id")
        .unwrap_or_else(|| generated_id("runtime_home"));
    let record = initialize_runtime_home(&runtime_home, &runtime_home_id, ADMIN_METADATA_JSON)?;

    Ok(format!(
        "runtime_home initialized\nruntime_home: {}\nruntime_home_id: {}\nregistry_db: {}\n",
        display_path(&record.runtime_home),
        record.runtime_home_id,
        display_path(&record.registry_db_path)
    ))
}

fn command_project<F>(args: &[String], env_var: F, current_dir: &Path) -> Result<String, CliError>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    let Some(subcommand) = args.first().map(String::as_str) else {
        return Err(CliError::usage(project_usage()));
    };
    let runtime_home = resolve_runtime_home(env_var, current_dir)?;

    match subcommand {
        "register" => {
            let options = parse_options(&args[1..], &["project-id", "repo-root", "status"])?;
            let project_id = required_option(&options, "project-id")?;
            let repo_root = required_option(&options, "repo-root")?;
            let status = options
                .value("status")
                .unwrap_or_else(|| ACTIVE_PROJECT_STATUS.to_owned());
            let repo_root = canonical_existing_dir(current_dir, repo_root, "repo-root")?;

            let record = register_project(
                &runtime_home,
                ProjectRegistration {
                    project_id,
                    repo_root,
                    project_home: None,
                    status,
                    metadata_json: ADMIN_METADATA_JSON.to_owned(),
                },
            )?;

            Ok(format!(
                "project registered\nproject_id: {}\nrepo_root: {}\nproject_home: {}\nstate_db: {}\nstatus: {}\n",
                record.project_id,
                display_path(&record.repo_root),
                display_path(&record.project_home),
                display_path(&record.state_db_path),
                record.status
            ))
        }
        "list" => {
            let options = parse_options(&args[1..], &[])?;
            reject_options(&options)?;
            let projects = list_projects(&runtime_home)?;
            let mut output = String::from("project_id\trepo_root\tproject_home\tstatus\n");
            for project in projects {
                output.push_str(&format!(
                    "{}\t{}\t{}\t{}\n",
                    project.project_id,
                    display_path(&project.repo_root),
                    display_path(&project.project_home),
                    project.status
                ));
            }
            Ok(output)
        }
        "-h" | "--help" | "help" => Ok(project_usage()),
        other => Err(CliError::usage(format!(
            "unknown project command: {other}\n\n{}",
            project_usage()
        ))),
    }
}

fn parse_options(args: &[String], allowed: &[&str]) -> Result<CliOptions, CliError> {
    parse_options_with_repeatable(args, allowed, &[])
}

fn parse_options_with_repeatable(
    args: &[String],
    allowed: &[&str],
    repeatable: &[&str],
) -> Result<CliOptions, CliError> {
    let mut options = BTreeMap::new();
    let mut index = 0;

    while index < args.len() {
        let token = &args[index];
        if token == "-h" || token == "--help" || token == "help" {
            return Err(CliError::usage(usage()));
        }
        if !token.starts_with("--") {
            return Err(CliError::usage(format!("unexpected argument: {token}")));
        }

        let without_prefix = &token[2..];
        let (name, value) = if let Some((name, value)) = without_prefix.split_once('=') {
            (name.to_owned(), value.to_owned())
        } else {
            index += 1;
            let Some(value) = args.get(index) else {
                return Err(CliError::usage(format!(
                    "missing value for --{without_prefix}"
                )));
            };
            (without_prefix.to_owned(), value.clone())
        };

        if !allowed.iter().any(|allowed_name| *allowed_name == name) {
            return Err(CliError::usage(format!("unknown option: --{name}")));
        }
        let values = options.entry(name.clone()).or_insert_with(Vec::new);
        if !values.is_empty()
            && !repeatable
                .iter()
                .any(|repeatable_name| *repeatable_name == name)
        {
            return Err(CliError::usage(format!("duplicate option: --{name}")));
        }
        values.push(value);

        index += 1;
    }

    Ok(options)
}

trait CliOptionsExt {
    fn value(&self, name: &str) -> Option<String>;
    fn value_ref(&self, name: &str) -> Option<&String>;
}

impl CliOptionsExt for CliOptions {
    fn value(&self, name: &str) -> Option<String> {
        self.value_ref(name).cloned()
    }

    fn value_ref(&self, name: &str) -> Option<&String> {
        self.get(name).and_then(|values| values.first())
    }
}

fn required_option(options: &CliOptions, name: &str) -> Result<String, CliError> {
    options
        .value_ref(name)
        .filter(|value| !value.trim().is_empty())
        .cloned()
        .ok_or_else(|| CliError::usage(format!("missing required option: --{name}")))
}

fn reject_options(options: &CliOptions) -> Result<(), CliError> {
    if options.is_empty() {
        Ok(())
    } else {
        let names = options
            .keys()
            .map(|name| format!("--{name}"))
            .collect::<Vec<_>>()
            .join(", ");
        Err(CliError::usage(format!("unexpected option(s): {names}")))
    }
}

fn canonical_existing_dir(
    current_dir: &Path,
    value: String,
    field: &'static str,
) -> Result<PathBuf, CliError> {
    let path = absolute_path(current_dir, PathBuf::from(value));
    let path = fs::canonicalize(&path)
        .map_err(|error| CliError::runtime(format!("{field} is not accessible: {error}")))?;
    if path.is_dir() {
        Ok(path)
    } else {
        Err(CliError::runtime(format!("{field} must be a directory")))
    }
}

fn absolute_path(current_dir: &Path, path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        current_dir.join(path)
    }
}

fn generated_id(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!("{prefix}_{nanos}_{}", process::id())
}

fn display_path(path: &Path) -> String {
    path.display().to_string()
}

fn usage() -> String {
    format!(
        "Usage:\n  volicord --help\n  volicord --version\n  volicord init [--runtime-home-id ID]\n  {}\n  {}\n  {}\n\nEnvironment:\n  VOLICORD_HOME  Override Runtime Home path (default: $HOME/.volicord)\n\nAgent Connection commands manage local MCP host connections. User Channel commands record local user judgments.\nThese are local administrative commands, not public Volicord API methods.\n",
        agent_usage().trim_end(),
        user_usage().trim_end(),
        project_usage().trim_end()
    )
}

fn version() -> String {
    format!("volicord {}\n", env!("CARGO_PKG_VERSION"))
}

fn project_usage() -> String {
    "volicord project register --project-id ID --repo-root PATH [--status active]\nvolicord project list\n"
        .to_owned()
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CliError {
    Usage(String),
    Runtime(String),
    FailureOutput(String),
}

impl CliError {
    fn usage(message: impl Into<String>) -> Self {
        Self::Usage(message.into())
    }

    fn runtime(message: impl Into<String>) -> Self {
        Self::Runtime(message.into())
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(message) | Self::Runtime(message) | Self::FailureOutput(message) => {
                formatter.write_str(message)
            }
        }
    }
}

impl std::error::Error for CliError {}

impl From<volicord_store::StoreError> for CliError {
    fn from(error: volicord_store::StoreError) -> Self {
        Self::Runtime(error.to_string())
    }
}

impl From<RuntimeHomeResolutionError> for CliError {
    fn from(error: RuntimeHomeResolutionError) -> Self {
        Self::Runtime(error.to_string())
    }
}

impl From<AgentCommandError> for CliError {
    fn from(error: AgentCommandError) -> Self {
        match error {
            AgentCommandError::Usage(message) => Self::Usage(message),
            AgentCommandError::Runtime(message) => Self::Runtime(message),
            AgentCommandError::FailureOutput(output) => Self::FailureOutput(output),
        }
    }
}

impl From<UserCommandError> for CliError {
    fn from(error: UserCommandError) -> Self {
        match error {
            UserCommandError::Usage(message) => Self::Usage(message),
            UserCommandError::Runtime(message) => Self::Runtime(message),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        ffi::OsString,
        fs,
        path::{Path, PathBuf},
    };

    use rusqlite::Connection;
    use volicord_store::sqlite::{project_state_db_path, registry_db_path};
    use volicord_test_support::TempRuntimeHome;

    use super::*;

    #[test]
    fn version_does_not_require_runtime_home_environment() {
        let output = run_cli(
            ["volicord", "--version"],
            |_| None,
            Path::new(env!("CARGO_MANIFEST_DIR")),
        )
        .expect("version should not need Runtime Home");

        assert_eq!(output, format!("volicord {}\n", env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn short_version_is_exact_alias() {
        let output = run_cli(
            ["volicord", "-V"],
            |_| None,
            Path::new(env!("CARGO_MANIFEST_DIR")),
        )
        .expect("short version should not need Runtime Home");

        assert_eq!(output, version());
    }

    #[test]
    fn help_mentions_version_discovery() {
        let output = run_cli(
            ["volicord", "--help"],
            |_| None,
            Path::new(env!("CARGO_MANIFEST_DIR")),
        )
        .expect("help should not need Runtime Home");

        assert!(output.contains("volicord --version"));
    }

    #[test]
    fn unknown_top_level_command_is_usage_error() {
        let error = run_cli(
            ["volicord", "not-a-real-command"],
            |_| None,
            Path::new(env!("CARGO_MANIFEST_DIR")),
        )
        .expect_err("unknown command should be a usage error");

        assert_eq!(
            error,
            CliError::Usage(format!(
                "unknown command: not-a-real-command\n\n{}",
                usage()
            ))
        );
    }

    #[test]
    fn init_respects_volicord_home_override() {
        let runtime_home = TempRuntimeHome::new("cli-init").expect("temp runtime home");
        let output = run_with_home(
            runtime_home.path(),
            ["volicord", "init", "--runtime-home-id", "runtime_home_test"],
        )
        .expect("init should succeed");

        assert!(output.contains("runtime_home initialized\n"));
        assert!(output.contains("runtime_home_id: runtime_home_test\n"));
        assert!(registry_db_path(runtime_home.path()).exists());
    }

    #[test]
    fn default_runtime_home_uses_user_home() {
        let runtime_home = resolve_runtime_home(
            |name| {
                if name == "HOME" {
                    Some(OsString::from("/tmp/volicord-cli-home"))
                } else {
                    None
                }
            },
            Path::new(env!("CARGO_MANIFEST_DIR")),
        )
        .expect("default runtime home should resolve");

        assert_eq!(
            runtime_home,
            PathBuf::from("/tmp/volicord-cli-home/.volicord")
        );
    }

    #[test]
    fn init_resolves_runtime_home_with_shared_resolver() {
        let current_dir = TempRuntimeHome::new("cli-cwd").expect("temp current dir");
        let runtime_home = resolve_runtime_home(
            |name| {
                if name == "VOLICORD_HOME" {
                    Some(OsString::from("shared-runtime"))
                } else {
                    None
                }
            },
            current_dir.path(),
        )
        .expect("shared resolver should resolve relative Runtime Home");

        let output = run_cli(
            [
                "volicord",
                "init",
                "--runtime-home-id",
                "runtime_home_shared",
            ],
            |name| {
                if name == "VOLICORD_HOME" {
                    Some(OsString::from("shared-runtime"))
                } else {
                    None
                }
            },
            current_dir.path(),
        )
        .expect("init should use shared Runtime Home resolution");

        assert!(output.contains(&format!("runtime_home: {}\n", display_path(&runtime_home))));
        assert!(registry_db_path(runtime_home).exists());
    }

    #[test]
    fn runtime_home_resolution_errors_are_runtime_errors() {
        let error = run_cli(
            ["volicord", "init"],
            |name| {
                if name == "VOLICORD_HOME" {
                    Some(OsString::new())
                } else {
                    None
                }
            },
            Path::new(env!("CARGO_MANIFEST_DIR")),
        )
        .expect_err("empty VOLICORD_HOME should fail");

        assert!(matches!(error, CliError::Runtime(_)));
        assert!(error
            .to_string()
            .contains("VOLICORD_HOME must not be empty"));
    }

    #[test]
    fn project_register_creates_registry_and_project_state() {
        let runtime_home = TempRuntimeHome::new("cli-project").expect("temp runtime home");
        run_with_home(
            runtime_home.path(),
            [
                "volicord",
                "init",
                "--runtime-home-id",
                "runtime_home_project",
            ],
        )
        .expect("init should succeed");

        let output = run_with_home(
            runtime_home.path(),
            [
                "volicord",
                "project",
                "register",
                "--project-id",
                "project_alpha",
                "--repo-root",
                ".",
            ],
        )
        .expect("project register should succeed");

        assert!(output.contains("project registered\n"));
        assert!(output.contains("project_id: project_alpha\n"));
        assert!(project_state_db_path(runtime_home.path(), "project_alpha").exists());

        let conn = Connection::open(project_state_db_path(runtime_home.path(), "project_alpha"))
            .expect("state database should open");
        let state_version: i64 = conn
            .query_row(
                "SELECT state_version FROM project_state WHERE project_id = 'project_alpha'",
                [],
                |row| row.get(0),
            )
            .expect("project state row should exist");
        assert_eq!(state_version, 0);
    }

    #[test]
    fn project_register_rejects_repository_under_runtime_home_without_project_state() {
        let runtime_home = TempRuntimeHome::new("cli-project-boundary").expect("temp runtime home");
        run_with_home(
            runtime_home.path(),
            [
                "volicord",
                "init",
                "--runtime-home-id",
                "runtime_home_project_boundary",
            ],
        )
        .expect("init should succeed");
        let repo_root = runtime_home.path().join("product-repo");
        fs::create_dir_all(&repo_root).expect("repo fixture should be created");

        let error = run_with_home(
            runtime_home.path(),
            [
                "volicord",
                "project",
                "register",
                "--project-id",
                "project_boundary",
                "--repo-root",
                repo_root.to_str().expect("utf8 path"),
            ],
        )
        .expect_err("project register should reject Product Repository inside Runtime Home");

        assert!(matches!(error, CliError::Runtime(_)));
        assert!(error
            .to_string()
            .contains("Product Repository must not be inside Volicord Runtime Home"));
        assert!(list_projects(runtime_home.path())
            .expect("registry inspection should still work")
            .is_empty());
        assert!(!project_state_db_path(runtime_home.path(), "project_boundary").exists());
    }

    #[test]
    fn project_list_uses_deterministic_order() {
        let runtime_home = TempRuntimeHome::new("cli-project-list").expect("temp runtime home");
        run_with_home(
            runtime_home.path(),
            [
                "volicord",
                "init",
                "--runtime-home-id",
                "runtime_home_project_list",
            ],
        )
        .expect("init should succeed");

        for project_id in ["project_b", "project_a"] {
            run_with_home(
                runtime_home.path(),
                [
                    "volicord",
                    "project",
                    "register",
                    "--project-id",
                    project_id,
                    "--repo-root",
                    ".",
                ],
            )
            .expect("project register should succeed");
        }

        let output = run_with_home(runtime_home.path(), ["volicord", "project", "list"])
            .expect("project list should succeed");
        let lines = output.lines().collect::<Vec<_>>();
        assert_eq!(lines[0], "project_id\trepo_root\tproject_home\tstatus");
        assert!(lines[1].starts_with("project_a\t"));
        assert!(lines[2].starts_with("project_b\t"));
    }

    #[test]
    fn project_register_requires_initialized_runtime_home() {
        let runtime_home = TempRuntimeHome::new("cli-uninitialized").expect("temp runtime home");
        let error = run_with_home(
            runtime_home.path(),
            [
                "volicord",
                "project",
                "register",
                "--project-id",
                "project_missing_runtime",
                "--repo-root",
                ".",
            ],
        )
        .expect_err("project register should require init");

        assert!(matches!(error, CliError::Runtime(_)));
        assert!(error.to_string().contains("runtime_home not found"));
    }

    fn run_with_home<const N: usize>(
        runtime_home: &Path,
        args: [&str; N],
    ) -> Result<String, CliError> {
        run_cli(
            args,
            |name| {
                if name == "VOLICORD_HOME" {
                    Some(OsString::from(runtime_home))
                } else {
                    None
                }
            },
            Path::new(env!("CARGO_MANIFEST_DIR")),
        )
    }
}
