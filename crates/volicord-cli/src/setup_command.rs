use std::{
    io,
    io::Write,
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};
use volicord_store::{runtime_home::RuntimeHomeResolutionError, StoreError};

use crate::setup_command::{
    output::{append_interactive_notes, render_setup_output},
    workflow::{command_status, run_setup_workflow, setup_options},
};
#[cfg(test)]
use crate::shell_path::mcp_binary_name;
pub(crate) use crate::shell_path::{is_executable_file, volicord_binary_name};

mod discovery;
mod interactive;
mod linking;
mod output;
mod shell_startup;
mod workflow;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandStatus {
    Complete,
    ActionRequired,
    Failed,
}

impl CommandStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::ActionRequired => "action_required",
            Self::Failed => "failed",
        }
    }

    pub const fn exits_failure(self) -> bool {
        matches!(self, Self::Failed)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutcome {
    pub status: CommandStatus,
    pub output: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetupCommandError {
    Usage(String),
    Runtime(String),
}

impl std::fmt::Display for SetupCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Usage(message) | Self::Runtime(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for SetupCommandError {}

impl From<StoreError> for SetupCommandError {
    fn from(error: StoreError) -> Self {
        Self::Runtime(error.to_string())
    }
}

impl From<RuntimeHomeResolutionError> for SetupCommandError {
    fn from(error: RuntimeHomeResolutionError) -> Self {
        Self::Runtime(error.to_string())
    }
}

impl From<io::Error> for SetupCommandError {
    fn from(error: io::Error) -> Self {
        Self::Runtime(error.to_string())
    }
}

pub trait SetupProcess {
    fn env_var(&self, name: &str) -> Option<std::ffi::OsString>;
    fn current_exe(&self) -> Result<PathBuf, String>;
}

pub struct ProductionSetupProcess;

impl SetupProcess for ProductionSetupProcess {
    fn env_var(&self, name: &str) -> Option<std::ffi::OsString> {
        std::env::var_os(name)
    }

    fn current_exe(&self) -> Result<PathBuf, String> {
        std::env::current_exe()
            .map_err(|error| format!("failed to read current executable: {error}"))
    }
}

pub trait SetupTerminal {
    fn write_str(&mut self, text: &str) -> io::Result<()>;
    fn read_line(&mut self, input: &mut String) -> io::Result<usize>;
}

pub struct StdioSetupTerminal {
    stdin: io::Stdin,
    stdout: io::Stdout,
}

/// Typed input for the reusable setup workflow.
///
/// The executable's supported `init` syntax is declared in [`crate::cli`].
/// This type keeps the lower-level setup workflow free of command-line parsing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SetupWorkflowOptions {
    pub runtime_home: Option<PathBuf>,
    pub link_bin: Option<PathBuf>,
    pub mcp_command: Option<PathBuf>,
    pub json: bool,
}

impl StdioSetupTerminal {
    pub fn new() -> Self {
        Self {
            stdin: io::stdin(),
            stdout: io::stdout(),
        }
    }
}

impl Default for StdioSetupTerminal {
    fn default() -> Self {
        Self::new()
    }
}

impl SetupTerminal for StdioSetupTerminal {
    fn write_str(&mut self, text: &str) -> io::Result<()> {
        self.stdout.write_all(text.as_bytes())?;
        self.stdout.flush()
    }

    fn read_line(&mut self, input: &mut String) -> io::Result<usize> {
        self.stdin.read_line(input)
    }
}

pub struct ClosureSetupProcess<'a, F>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    env_var: &'a F,
}

impl<'a, F> ClosureSetupProcess<'a, F>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    pub fn new(env_var: &'a F) -> Self {
        Self { env_var }
    }
}

impl<F> SetupProcess for ClosureSetupProcess<'_, F>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    fn env_var(&self, name: &str) -> Option<std::ffi::OsString> {
        (self.env_var)(name)
    }

    fn current_exe(&self) -> Result<PathBuf, String> {
        std::env::current_exe()
            .map_err(|error| format!("failed to read current executable: {error}"))
    }
}

pub fn run_setup_command(
    options: SetupWorkflowOptions,
    current_dir: &Path,
    process: &impl SetupProcess,
) -> Result<CommandOutcome, SetupCommandError> {
    run_setup_command_inner(options, current_dir, process, None)
}

pub fn run_setup_command_interactive(
    options: SetupWorkflowOptions,
    current_dir: &Path,
    process: &impl SetupProcess,
    terminal: &mut dyn SetupTerminal,
) -> Result<CommandOutcome, SetupCommandError> {
    run_setup_command_inner(options, current_dir, process, Some(terminal))
}

fn run_setup_command_inner(
    options: SetupWorkflowOptions,
    current_dir: &Path,
    process: &impl SetupProcess,
    terminal: Option<&mut dyn SetupTerminal>,
) -> Result<CommandOutcome, SetupCommandError> {
    let parsed = setup_options(options, current_dir);
    let outcome = run_setup_workflow(parsed, current_dir, process, terminal)?;
    let status = command_status(outcome.report.status);
    let output = append_interactive_notes(
        render_setup_output(
            outcome.output,
            &outcome.report,
            &outcome.runtime_home,
            outcome.installation_profile.as_ref(),
            &outcome.checks,
        )?,
        outcome.output,
        &outcome.interactive_notes,
    );
    Ok(CommandOutcome { status, output })
}

pub(crate) fn runtime_home_id_for_path(path: &Path) -> Result<String, SetupCommandError> {
    let path_text = path.to_str().ok_or_else(|| {
        SetupCommandError::Runtime("Runtime Home path must be valid UTF-8".to_owned())
    })?;
    let digest = Sha256::digest(path_text.as_bytes());
    Ok(format!(
        "runtime_home_{:016x}",
        u64::from_be_bytes([
            digest[0], digest[1], digest[2], digest[3], digest[4], digest[5], digest[6], digest[7],
        ])
    ))
}

fn command_parent(path: &Path) -> PathBuf {
    path.parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

pub(crate) fn path_text(path: &Path) -> String {
    path.display().to_string()
}

pub(crate) fn absolute_path(current_dir: &Path, path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        current_dir.join(path)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::{BTreeMap, VecDeque},
        env,
        ffi::OsString,
        fs,
        io::{self, Write},
    };

    use crate::{
        setup_command::discovery::plan_setup_actions,
        setup_report::{CommandAvailability, SetupActionKind},
        shell_path::PATH_ENV,
    };
    use serde_json::Value;
    use volicord_store::{
        agent_connections::CONNECTION_MODE_WORKFLOW, bootstrap::installation_profile,
        sqlite::registry_db_path,
    };
    use volicord_test_support::TempRuntimeHome;

    #[cfg(unix)]
    use super::interactive::{
        plan_interactive_menu_choices, InteractiveMenuChoice, InteractiveSetupChoice,
    };
    use super::workflow::{OutputFormat, ParsedSetupOptions};
    use super::*;

    #[derive(Debug)]
    struct FakeProcess {
        exe: PathBuf,
        env: BTreeMap<String, OsString>,
    }

    impl SetupProcess for FakeProcess {
        fn env_var(&self, name: &str) -> Option<OsString> {
            self.env.get(name).cloned()
        }

        fn current_exe(&self) -> Result<PathBuf, String> {
            Ok(self.exe.clone())
        }
    }

    fn workflow_options(
        runtime_home: &Path,
        mcp_command: Option<&Path>,
        link_bin: Option<&Path>,
        json: bool,
    ) -> SetupWorkflowOptions {
        SetupWorkflowOptions {
            runtime_home: Some(runtime_home.to_path_buf()),
            link_bin: link_bin.map(Path::to_path_buf),
            mcp_command: mcp_command.map(Path::to_path_buf),
            json,
        }
    }

    #[derive(Debug)]
    enum FakeTerminalInput {
        Line(String),
        #[cfg(unix)]
        MenuChoiceContaining(String),
    }

    impl FakeTerminalInput {
        fn line(line: impl Into<String>) -> Self {
            Self::Line(line.into())
        }

        #[cfg(unix)]
        fn menu_choice_containing(label: impl Into<String>) -> Self {
            Self::MenuChoiceContaining(label.into())
        }
    }

    #[derive(Debug)]
    struct FakeTerminal {
        input: VecDeque<FakeTerminalInput>,
        output: String,
    }

    impl FakeTerminal {
        fn new(lines: &[&str]) -> Self {
            Self {
                input: lines
                    .iter()
                    .map(|line| FakeTerminalInput::line(*line))
                    .collect(),
                output: String::new(),
            }
        }

        #[cfg(unix)]
        fn with_inputs(inputs: Vec<FakeTerminalInput>) -> Self {
            Self {
                input: inputs.into(),
                output: String::new(),
            }
        }

        fn output(&self) -> &str {
            &self.output
        }
    }

    impl SetupTerminal for FakeTerminal {
        fn write_str(&mut self, text: &str) -> io::Result<()> {
            self.output.push_str(text);
            Ok(())
        }

        fn read_line(&mut self, input: &mut String) -> io::Result<usize> {
            let Some(next_input) = self.input.pop_front() else {
                return Ok(0);
            };
            let line = match next_input {
                FakeTerminalInput::Line(line) => line,
                #[cfg(unix)]
                FakeTerminalInput::MenuChoiceContaining(label) => {
                    menu_choice_number_containing(&self.output, &label)
                        .unwrap_or_else(|| panic!("menu choice containing {label:?} not found"))
                        .to_string()
                }
            };
            let line = format!("{line}\n");
            input.push_str(&line);
            Ok(line.len())
        }
    }

    #[cfg(unix)]
    fn menu_choice_number_containing(output: &str, label_fragment: &str) -> Option<usize> {
        output.lines().find_map(|line| {
            let trimmed = line.trim_start();
            let (number, label) = trimmed.split_once(". ")?;
            label
                .contains(label_fragment)
                .then(|| number.parse::<usize>().ok())
                .flatten()
        })
    }

    #[cfg(unix)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum InteractiveChoiceKind {
        LinkOnly,
        LinkAndShell,
        Manual,
        Skip,
        Cancelled,
    }

    #[cfg(unix)]
    fn interactive_choice_kinds(choices: &[InteractiveMenuChoice]) -> Vec<InteractiveChoiceKind> {
        choices
            .iter()
            .map(|choice| match &choice.choice {
                InteractiveSetupChoice::LinkOnly(_) => InteractiveChoiceKind::LinkOnly,
                InteractiveSetupChoice::LinkAndShell { .. } => InteractiveChoiceKind::LinkAndShell,
                InteractiveSetupChoice::Manual { .. } => InteractiveChoiceKind::Manual,
                InteractiveSetupChoice::Skip => InteractiveChoiceKind::Skip,
                InteractiveSetupChoice::Cancelled(_) => InteractiveChoiceKind::Cancelled,
            })
            .collect()
    }

    #[test]
    fn setup_action_planner_reports_stable_action_kinds() -> Result<(), Box<dyn std::error::Error>>
    {
        let fixture = TempRuntimeHome::new("setup-action-planner-kinds")?;
        let home = fixture.path().join("home");
        let local_bin = home.join(".local").join("bin");
        fs::create_dir_all(&home)?;
        let command_path = fixture.path().join("exe").join(volicord_binary_name());
        let process = FakeProcess {
            exe: command_path.clone(),
            env: BTreeMap::from([("HOME".to_owned(), home.clone().into_os_string())]),
        };
        let parsed = ParsedSetupOptions {
            runtime_home: None,
            link_bin: None,
            mcp_command: None,
            output: OutputFormat::Text,
        };
        let commands = vec![CommandAvailability {
            id: "volicord_command".to_owned(),
            command_name: volicord_binary_name(),
            discovered: true,
            discovered_path: Some(path_text(&command_path)),
            discovery_source: Some("test".to_owned()),
            available_on_path: false,
            path_matches_discovered: false,
            discovered_directory_on_path: false,
            path_match: None,
        }];
        let mut actions_required = Vec::new();
        let mut actions_optional = Vec::new();

        plan_setup_actions(
            &commands,
            &parsed,
            &process,
            None,
            &mut actions_required,
            &mut actions_optional,
        );

        assert_eq!(
            actions_required
                .iter()
                .map(|action| action.kind)
                .collect::<Vec<_>>(),
            vec![SetupActionKind::CommandAvailability]
        );
        assert_eq!(
            actions_optional
                .iter()
                .map(|action| action.kind)
                .collect::<Vec<_>>(),
            vec![SetupActionKind::CommandLinks]
        );
        assert_eq!(actions_optional[0].path, Some(path_text(&local_bin)));
        assert!(!local_bin.exists());
        Ok(())
    }

    #[test]
    fn setup_action_planner_uses_home_bin_when_local_bin_is_unavailable(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("setup-action-planner-home-bin")?;
        let home = fixture.path().join("home");
        let home_bin = home.join("bin");
        fs::create_dir_all(&home)?;
        fs::write(home.join(".local"), "not a directory")?;
        let command_path = fixture.path().join("exe").join(volicord_binary_name());
        let process = FakeProcess {
            exe: command_path.clone(),
            env: BTreeMap::from([("HOME".to_owned(), home.clone().into_os_string())]),
        };
        let parsed = ParsedSetupOptions {
            runtime_home: None,
            link_bin: None,
            mcp_command: None,
            output: OutputFormat::Text,
        };
        let commands = vec![CommandAvailability {
            id: "volicord_command".to_owned(),
            command_name: volicord_binary_name(),
            discovered: true,
            discovered_path: Some(path_text(&command_path)),
            discovery_source: Some("test".to_owned()),
            available_on_path: false,
            path_matches_discovered: false,
            discovered_directory_on_path: false,
            path_match: None,
        }];
        let mut actions_required = Vec::new();
        let mut actions_optional = Vec::new();

        plan_setup_actions(
            &commands,
            &parsed,
            &process,
            None,
            &mut actions_required,
            &mut actions_optional,
        );

        assert_eq!(
            actions_optional
                .iter()
                .map(|action| action.kind)
                .collect::<Vec<_>>(),
            vec![SetupActionKind::CommandLinks]
        );
        assert_eq!(actions_optional[0].path, Some(path_text(&home_bin)));
        assert!(actions_optional[0].command.is_none());
        assert!(!home_bin.exists());
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn interactive_menu_plan_prefers_existing_path_link() -> Result<(), Box<dyn std::error::Error>>
    {
        let fixture = TempRuntimeHome::new("setup-menu-path-dir")?;
        let path_dir = fixture.path().join("path-bin");
        let home = fixture.path().join("home");
        let local_bin = home.join(".local").join("bin");
        fs::create_dir_all(&path_dir)?;
        fs::create_dir_all(&local_bin)?;
        let process = FakeProcess {
            exe: fixture.path().join("volicord"),
            env: BTreeMap::from([
                (PATH_ENV.to_owned(), env::join_paths([path_dir.as_path()])?),
                ("HOME".to_owned(), home.into_os_string()),
                ("SHELL".to_owned(), OsString::from("/bin/zsh")),
            ]),
        };
        let selected = [
            fixture.path().join(volicord_binary_name()),
            fixture.path().join(mcp_binary_name()),
        ];

        let menu = plan_interactive_menu_choices(
            &process,
            [selected[0].as_path(), selected[1].as_path()],
        )?;

        assert_eq!(
            interactive_choice_kinds(&menu.choices),
            vec![
                InteractiveChoiceKind::LinkOnly,
                InteractiveChoiceKind::Manual,
                InteractiveChoiceKind::Skip,
            ]
        );
        assert!(menu.choices[0].label.contains("already on PATH"));
        assert!(menu.shell_unavailable.is_none());
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn interactive_menu_plan_orders_shell_update_before_user_bin_only(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("setup-menu-shell-order")?;
        let home = fixture.path().join("home");
        let local_bin = home.join(".local").join("bin");
        fs::create_dir_all(&home)?;
        let process = FakeProcess {
            exe: fixture.path().join("volicord"),
            env: BTreeMap::from([
                ("HOME".to_owned(), home.clone().into_os_string()),
                ("SHELL".to_owned(), OsString::from("/bin/zsh")),
            ]),
        };
        let selected = [
            fixture.path().join(volicord_binary_name()),
            fixture.path().join(mcp_binary_name()),
        ];

        let menu = plan_interactive_menu_choices(
            &process,
            [selected[0].as_path(), selected[1].as_path()],
        )?;

        assert_eq!(
            interactive_choice_kinds(&menu.choices),
            vec![
                InteractiveChoiceKind::LinkAndShell,
                InteractiveChoiceKind::LinkOnly,
                InteractiveChoiceKind::Manual,
                InteractiveChoiceKind::Skip,
            ]
        );
        assert!(menu.choices[0].label.contains("managed PATH block"));
        assert!(menu.choices[0].label.contains("Create "));
        assert!(menu.choices[1].label.contains("PATH still needs an update"));
        assert!(menu.shell_unavailable.is_none());
        assert!(!local_bin.exists());
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn interactive_menu_plan_uses_home_bin_when_local_bin_is_unavailable(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("setup-menu-home-bin")?;
        let home = fixture.path().join("home");
        let home_bin = home.join("bin");
        fs::create_dir_all(&home)?;
        fs::write(home.join(".local"), "not a directory")?;
        let process = FakeProcess {
            exe: fixture.path().join("volicord"),
            env: BTreeMap::from([
                ("HOME".to_owned(), home.clone().into_os_string()),
                ("SHELL".to_owned(), OsString::from("/bin/zsh")),
            ]),
        };
        let selected = [
            fixture.path().join(volicord_binary_name()),
            fixture.path().join(mcp_binary_name()),
        ];

        let menu = plan_interactive_menu_choices(
            &process,
            [selected[0].as_path(), selected[1].as_path()],
        )?;

        assert_eq!(
            interactive_choice_kinds(&menu.choices),
            vec![
                InteractiveChoiceKind::LinkAndShell,
                InteractiveChoiceKind::LinkOnly,
                InteractiveChoiceKind::Manual,
                InteractiveChoiceKind::Skip,
            ]
        );
        match &menu.choices[0].choice {
            InteractiveSetupChoice::LinkAndShell { link_bin, .. } => {
                assert_eq!(link_bin, &home_bin);
            }
            other => panic!("expected link-and-shell choice, got {other:?}"),
        }
        assert!(menu.choices[0].label.contains(&path_text(&home_bin)));
        assert!(menu.choices[1].label.contains("PATH still needs an update"));
        assert!(menu.shell_unavailable.is_none());
        assert!(!home_bin.exists());
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn interactive_menu_plan_keeps_manual_and_skip_when_shell_is_unsupported(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("setup-menu-unsupported-shell")?;
        let home = fixture.path().join("home");
        fs::create_dir_all(home.join(".local").join("bin"))?;
        let process = FakeProcess {
            exe: fixture.path().join("volicord"),
            env: BTreeMap::from([
                ("HOME".to_owned(), home.into_os_string()),
                ("SHELL".to_owned(), OsString::from("/bin/fish")),
            ]),
        };
        let selected = [
            fixture.path().join(volicord_binary_name()),
            fixture.path().join(mcp_binary_name()),
        ];

        let menu = plan_interactive_menu_choices(
            &process,
            [selected[0].as_path(), selected[1].as_path()],
        )?;

        assert_eq!(
            interactive_choice_kinds(&menu.choices),
            vec![
                InteractiveChoiceKind::LinkOnly,
                InteractiveChoiceKind::Manual,
                InteractiveChoiceKind::Skip,
            ]
        );
        assert!(menu
            .shell_unavailable
            .as_deref()
            .is_some_and(|reason| reason.contains("fish is not supported")));
        assert!(menu.choices[1].label.contains("print the PATH command"));
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn setup_interactive_creates_links_in_writable_path_dir(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("setup-interactive-path-dir")?;
        let exe_dir = fixture.path().join("exe");
        let path_dir = fixture.path().join("path-bin");
        let home = fixture.path().join("home");
        fs::create_dir_all(&path_dir)?;
        fs::create_dir_all(&home)?;
        let volicord = write_executable(&exe_dir, &volicord_binary_name())?;
        let mcp = write_executable(&exe_dir, &mcp_binary_name())?;
        let process = FakeProcess {
            exe: volicord.clone(),
            env: BTreeMap::from([
                (PATH_ENV.to_owned(), env::join_paths([path_dir.as_path()])?),
                ("HOME".to_owned(), home.clone().into_os_string()),
                ("SHELL".to_owned(), OsString::from("/bin/zsh")),
            ]),
        };
        let mut terminal =
            FakeTerminal::with_inputs(vec![FakeTerminalInput::menu_choice_containing(
                "already on PATH",
            )]);

        let outcome = run_setup_command_interactive(
            workflow_options(fixture.path(), None, None, false),
            fixture.path(),
            &process,
            &mut terminal,
        )?;

        assert_eq!(outcome.status, CommandStatus::Complete);
        assert!(terminal.output().contains("Choices:"));
        assert_eq!(
            fs::canonicalize(path_dir.join(volicord_binary_name()))?,
            volicord
        );
        assert_eq!(fs::canonicalize(path_dir.join(mcp_binary_name()))?, mcp);
        assert!(!home.join(".zshrc").exists());
        Ok(())
    }

    #[test]
    fn setup_interactive_json_never_prompts() -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("setup-interactive-json")?;
        let bin_dir = fixture.path().join("bin");
        let volicord = write_executable(&bin_dir, &volicord_binary_name())?;
        write_executable(&bin_dir, &mcp_binary_name())?;
        let process = FakeProcess {
            exe: volicord,
            env: BTreeMap::new(),
        };
        let mut terminal = FakeTerminal::new(&[]);

        let outcome = run_setup_command_interactive(
            workflow_options(fixture.path(), None, None, true),
            fixture.path(),
            &process,
            &mut terminal,
        )?;

        assert_eq!(outcome.status, CommandStatus::ActionRequired);
        assert_eq!(terminal.output(), "");
        let value: Value = serde_json::from_str(&outcome.output)?;
        assert_eq!(value["status"], "action_required");
        Ok(())
    }

    #[test]
    fn setup_json_reports_missing_user_bin_action_without_creating_it(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("setup-json-missing-user-bin")?;
        let bin_dir = fixture.path().join("bin");
        let home = fixture.path().join("home");
        let local_bin = home.join(".local").join("bin");
        fs::create_dir_all(&home)?;
        let volicord = write_executable(&bin_dir, &volicord_binary_name())?;
        write_executable(&bin_dir, &mcp_binary_name())?;
        let process = FakeProcess {
            exe: volicord,
            env: BTreeMap::from([("HOME".to_owned(), home.clone().into_os_string())]),
        };

        let outcome = run_setup_command(
            workflow_options(fixture.path(), None, None, true),
            fixture.path(),
            &process,
        )?;

        assert_eq!(outcome.status, CommandStatus::ActionRequired);
        let value: Value = serde_json::from_str(&outcome.output)?;
        assert_eq!(value["status"], "action_required");
        assert!(value["actions_optional"]
            .as_array()
            .expect("actions_optional should be an array")
            .iter()
            .any(|action| action["id"] == "create_command_links"
                && action["path"] == path_text(&local_bin)
                && action.get("command").is_none()));
        assert!(!local_bin.exists());
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn setup_interactive_link_bin_never_prompts() -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("setup-interactive-link-bin")?;
        let bin_dir = fixture.path().join("bin");
        let link_bin = fixture.path().join("links");
        let volicord = write_executable(&bin_dir, &volicord_binary_name())?;
        let mcp = write_executable(&bin_dir, &mcp_binary_name())?;
        let process = FakeProcess {
            exe: volicord,
            env: BTreeMap::from([(PATH_ENV.to_owned(), env::join_paths([link_bin.as_path()])?)]),
        };
        let mut terminal = FakeTerminal::new(&[]);

        let outcome = run_setup_command_interactive(
            workflow_options(fixture.path(), Some(&mcp), Some(&link_bin), false),
            fixture.path(),
            &process,
            &mut terminal,
        )?;

        assert_eq!(outcome.status, CommandStatus::Complete);
        assert_eq!(terminal.output(), "");
        assert!(link_bin.join(volicord_binary_name()).exists());
        assert!(link_bin.join(mcp_binary_name()).exists());
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn setup_interactive_writes_shell_startup_block_idempotently(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("setup-interactive-shell")?;
        let exe_dir = fixture.path().join("exe");
        let home = fixture.path().join("home");
        let link_bin = home.join(".local").join("bin");
        fs::create_dir_all(&home)?;
        let volicord = write_executable(&exe_dir, &volicord_binary_name())?;
        let mcp = write_executable(&exe_dir, &mcp_binary_name())?;
        let process = FakeProcess {
            exe: volicord,
            env: BTreeMap::from([
                ("HOME".to_owned(), home.clone().into_os_string()),
                ("SHELL".to_owned(), OsString::from("/bin/zsh")),
            ]),
        };

        let mut first_terminal = FakeTerminal::with_inputs(vec![
            FakeTerminalInput::menu_choice_containing("managed PATH block"),
            FakeTerminalInput::line("y"),
        ]);
        let first = run_setup_command_interactive(
            workflow_options(fixture.path(), None, None, false),
            fixture.path(),
            &process,
            &mut first_terminal,
        )?;
        assert_eq!(first.status, CommandStatus::ActionRequired);
        assert!(first_terminal.output().contains("Managed block to write"));

        assert_eq!(
            fs::canonicalize(link_bin.join(volicord_binary_name()))?,
            process.exe
        );
        assert_eq!(fs::canonicalize(link_bin.join(mcp_binary_name()))?, mcp);
        let zshrc = home.join(".zshrc");
        let first_text = fs::read_to_string(&zshrc)?;
        assert!(first_text.contains("# BEGIN VOLICORD MANAGED PATH"));
        assert!(first_text.contains("export PATH=\"$HOME/.local/bin:$PATH\""));

        let mut second_terminal = FakeTerminal::with_inputs(vec![
            FakeTerminalInput::menu_choice_containing("managed PATH block"),
            FakeTerminalInput::line("y"),
        ]);
        run_setup_command_interactive(
            workflow_options(fixture.path(), None, None, false),
            fixture.path(),
            &process,
            &mut second_terminal,
        )?;
        let second_text = fs::read_to_string(&zshrc)?;
        assert_eq!(
            second_text.matches("# BEGIN VOLICORD MANAGED PATH").count(),
            1
        );
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn setup_interactive_does_not_add_shell_block_for_unmanaged_link(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("setup-interactive-unsafe-link")?;
        let exe_dir = fixture.path().join("exe");
        let home = fixture.path().join("home");
        let link_bin = home.join(".local/bin");
        fs::create_dir_all(&link_bin)?;
        let volicord = write_executable(&exe_dir, &volicord_binary_name())?;
        write_executable(&link_bin, &volicord_binary_name())?;
        let process = FakeProcess {
            exe: volicord,
            env: BTreeMap::from([
                ("HOME".to_owned(), home.clone().into_os_string()),
                ("SHELL".to_owned(), OsString::from("/bin/zsh")),
            ]),
        };
        let mut terminal = FakeTerminal::with_inputs(vec![
            FakeTerminalInput::menu_choice_containing("managed PATH block"),
            FakeTerminalInput::line("y"),
        ]);

        let outcome = run_setup_command_interactive(
            workflow_options(fixture.path(), None, None, false),
            fixture.path(),
            &process,
            &mut terminal,
        )?;

        assert_eq!(outcome.status, CommandStatus::ActionRequired);
        assert!(terminal.output().contains("Managed block to write"));
        assert!(outcome.output.contains("Move or remove the existing"));
        assert!(!outcome.output.contains("Open a new shell"));
        assert_ne!(
            fs::canonicalize(link_bin.join(volicord_binary_name()))?,
            process.exe
        );
        assert!(!home.join(".zshrc").exists());
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn setup_interactive_unsupported_shell_uses_manual_action(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("setup-interactive-unsupported-shell")?;
        let exe_dir = fixture.path().join("exe");
        let home = fixture.path().join("home");
        fs::create_dir_all(&home)?;
        let volicord = write_executable(&exe_dir, &volicord_binary_name())?;
        write_executable(&exe_dir, &mcp_binary_name())?;
        let process = FakeProcess {
            exe: volicord,
            env: BTreeMap::from([
                ("HOME".to_owned(), home.clone().into_os_string()),
                ("SHELL".to_owned(), OsString::from("/bin/fish")),
            ]),
        };
        let mut terminal =
            FakeTerminal::with_inputs(vec![FakeTerminalInput::menu_choice_containing(
                "print the PATH command",
            )]);

        let outcome = run_setup_command_interactive(
            workflow_options(fixture.path(), None, None, false),
            fixture.path(),
            &process,
            &mut terminal,
        )?;

        assert_eq!(outcome.status, CommandStatus::ActionRequired);
        assert!(terminal
            .output()
            .contains("Shell startup update is unavailable"));
        assert!(terminal.output().contains("Run this command after setup"));
        assert!(!home.join(".config/fish/config.fish").exists());
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn setup_interactive_skip_reports_action_required_without_links(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("setup-interactive-skip")?;
        let exe_dir = fixture.path().join("exe");
        let home = fixture.path().join("home");
        fs::create_dir_all(&home)?;
        let volicord = write_executable(&exe_dir, &volicord_binary_name())?;
        write_executable(&exe_dir, &mcp_binary_name())?;
        let process = FakeProcess {
            exe: volicord.clone(),
            env: BTreeMap::from([
                ("HOME".to_owned(), home.clone().into_os_string()),
                ("SHELL".to_owned(), OsString::from("/bin/zsh")),
            ]),
        };
        let mut terminal =
            FakeTerminal::with_inputs(vec![FakeTerminalInput::menu_choice_containing(
                "Skip command linking",
            )]);

        let outcome = run_setup_command_interactive(
            workflow_options(fixture.path(), None, None, false),
            fixture.path(),
            &process,
            &mut terminal,
        )?;

        assert_eq!(outcome.status, CommandStatus::ActionRequired);
        assert!(terminal.output().contains("Skip command linking for now."));
        assert!(outcome.output.contains("command linking was skipped"));
        assert!(outcome
            .output
            .contains("Status: action_required (not a fatal CLI error)"));
        assert!(outcome
            .output
            .contains("Meaning: installation profile setup needs a named user action"));
        assert!(outcome
            .output
            .contains("Next:\n  1. Make volicord available on PATH"));
        assert!(outcome
            .output
            .contains("This does not prove future shell PATH state"));
        assert!(!outcome.output.contains("runtime_home_state:"));
        assert!(!outcome.output.contains("next_action:"));
        assert!(!home.join(".local").exists());
        assert!(!home.join(".zshrc").exists());
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn setup_interactive_link_only_creates_links_without_shell_startup_when_path_needs_update(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("setup-interactive-link-only")?;
        let exe_dir = fixture.path().join("exe");
        let home = fixture.path().join("home");
        fs::create_dir_all(&home)?;
        let volicord = write_executable(&exe_dir, &volicord_binary_name())?;
        let mcp = write_executable(&exe_dir, &mcp_binary_name())?;
        let process = FakeProcess {
            exe: volicord.clone(),
            env: BTreeMap::from([
                ("HOME".to_owned(), home.clone().into_os_string()),
                ("SHELL".to_owned(), OsString::from("/bin/zsh")),
            ]),
        };
        let mut terminal =
            FakeTerminal::with_inputs(vec![FakeTerminalInput::menu_choice_containing(
                "PATH still needs an update",
            )]);

        let outcome = run_setup_command_interactive(
            workflow_options(fixture.path(), None, None, false),
            fixture.path(),
            &process,
            &mut terminal,
        )?;

        assert_eq!(outcome.status, CommandStatus::ActionRequired);
        assert!(!terminal.output().contains("Managed block to write"));
        let link_bin = home.join(".local/bin");
        assert!(link_bin.is_dir());
        assert_eq!(
            fs::canonicalize(link_bin.join(volicord_binary_name()))?,
            volicord
        );
        assert_eq!(fs::canonicalize(link_bin.join(mcp_binary_name()))?, mcp);
        assert!(!home.join(".zshrc").exists());
        assert!(outcome.output.contains("Add "));
        assert!(outcome.output.contains(".local/bin"));
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn setup_interactive_link_only_creates_home_bin_when_local_bin_is_unavailable(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("setup-interactive-home-bin")?;
        let exe_dir = fixture.path().join("exe");
        let home = fixture.path().join("home");
        let local = home.join(".local");
        let link_bin = home.join("bin");
        fs::create_dir_all(&home)?;
        fs::write(&local, "not a directory")?;
        let volicord = write_executable(&exe_dir, &volicord_binary_name())?;
        let mcp = write_executable(&exe_dir, &mcp_binary_name())?;
        let process = FakeProcess {
            exe: volicord.clone(),
            env: BTreeMap::from([
                ("HOME".to_owned(), home.clone().into_os_string()),
                ("SHELL".to_owned(), OsString::from("/bin/zsh")),
            ]),
        };
        let mut terminal =
            FakeTerminal::with_inputs(vec![FakeTerminalInput::menu_choice_containing(
                "PATH still needs an update",
            )]);

        let outcome = run_setup_command_interactive(
            workflow_options(fixture.path(), None, None, false),
            fixture.path(),
            &process,
            &mut terminal,
        )?;

        assert_eq!(outcome.status, CommandStatus::ActionRequired);
        assert!(terminal.output().contains(&path_text(&link_bin)));
        assert!(!terminal.output().contains("Managed block to write"));
        assert!(link_bin.is_dir());
        assert_eq!(
            fs::canonicalize(link_bin.join(volicord_binary_name()))?,
            volicord
        );
        assert_eq!(fs::canonicalize(link_bin.join(mcp_binary_name()))?, mcp);
        assert!(fs::metadata(&local)?.is_file());
        assert!(!home.join(".zshrc").exists());
        assert!(outcome.output.contains("Add "));
        assert!(outcome.output.contains(&path_text(&link_bin)));
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn setup_interactive_declined_shell_startup_update_leaves_files_unchanged(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("setup-interactive-decline-shell")?;
        let exe_dir = fixture.path().join("exe");
        let home = fixture.path().join("home");
        fs::create_dir_all(&home)?;
        let zshrc = home.join(".zshrc");
        let original_zshrc = "export PATH=\"$HOME/bin:$PATH\"\n";
        fs::write(&zshrc, original_zshrc)?;
        let volicord = write_executable(&exe_dir, &volicord_binary_name())?;
        write_executable(&exe_dir, &mcp_binary_name())?;
        let process = FakeProcess {
            exe: volicord.clone(),
            env: BTreeMap::from([
                ("HOME".to_owned(), home.clone().into_os_string()),
                ("SHELL".to_owned(), OsString::from("/bin/zsh")),
            ]),
        };
        let mut terminal = FakeTerminal::with_inputs(vec![
            FakeTerminalInput::menu_choice_containing("managed PATH block"),
            FakeTerminalInput::line("n"),
        ]);

        let outcome = run_setup_command_interactive(
            workflow_options(fixture.path(), None, None, false),
            fixture.path(),
            &process,
            &mut terminal,
        )?;

        assert_eq!(outcome.status, CommandStatus::ActionRequired);
        assert!(terminal.output().contains("Managed block to write"));
        assert!(outcome
            .output
            .contains("shell startup update was not approved"));
        assert_eq!(fs::read_to_string(&zshrc)?, original_zshrc);
        assert!(!home.join(".local").exists());
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn setup_interactive_eof_cancels_command_linking() -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("setup-interactive-eof")?;
        let exe_dir = fixture.path().join("exe");
        let home = fixture.path().join("home");
        fs::create_dir_all(&home)?;
        let volicord = write_executable(&exe_dir, &volicord_binary_name())?;
        write_executable(&exe_dir, &mcp_binary_name())?;
        let process = FakeProcess {
            exe: volicord,
            env: BTreeMap::from([
                ("HOME".to_owned(), home.clone().into_os_string()),
                ("SHELL".to_owned(), OsString::from("/bin/zsh")),
            ]),
        };
        let mut terminal = FakeTerminal::new(&[]);

        let outcome = run_setup_command_interactive(
            workflow_options(fixture.path(), None, None, false),
            fixture.path(),
            &process,
            &mut terminal,
        )?;

        assert_eq!(outcome.status, CommandStatus::ActionRequired);
        assert!(outcome.output.contains("setup prompt cancelled"));
        assert!(!home.join(".local").exists());
        assert!(!home.join(".zshrc").exists());
        Ok(())
    }

    #[test]
    fn setup_records_explicit_mcp_command() -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("setup-explicit")?;
        let bin_dir = fixture.path().join("bin");
        let volicord = write_executable(&bin_dir, &volicord_binary_name())?;
        let mcp = write_executable(&bin_dir, "custom-volicord")?;
        let process = FakeProcess {
            exe: volicord,
            env: BTreeMap::new(),
        };

        let outcome = run_setup_command(
            workflow_options(fixture.path(), Some(&mcp), None, true),
            fixture.path(),
            &process,
        )?;

        assert_eq!(outcome.status, CommandStatus::ActionRequired);
        let value: Value = serde_json::from_str(&outcome.output)?;
        assert_eq!(value["status"], "action_required");
        assert_eq!(
            value["setup_report"]["installation_profile"]["status"],
            "complete"
        );
        assert!(value["checks"]
            .as_array()
            .expect("checks should be an array")
            .iter()
            .any(|check| {
                check["id"] == "volicord_mcp_command"
                    && check["status"] == "passed"
                    && check["details"]["path"] == path_text(&mcp)
                    && check["details"]["source"] == "explicit"
            }));
        assert!(value["actions_required"]
            .as_array()
            .expect("actions_required should be an array")
            .iter()
            .any(|action| action["id"] == "make_volicord_command_available"));
        let profile = installation_profile(fixture.path())?.expect("profile should be stored");
        assert_eq!(profile.volicord_mcp_command, path_text(&mcp));
        assert_eq!(profile.default_connection_mode, CONNECTION_MODE_WORKFLOW);
        assert!(registry_db_path(fixture.path()).exists());
        Ok(())
    }

    #[test]
    fn setup_uses_volicord_as_default_mcp_launch_command() -> Result<(), Box<dyn std::error::Error>>
    {
        let fixture = TempRuntimeHome::new("setup-default-mcp")?;
        let bin_dir = fixture.path().join("bin");
        let volicord = write_executable(&bin_dir, &volicord_binary_name())?;
        let process = FakeProcess {
            exe: volicord.clone(),
            env: BTreeMap::new(),
        };

        run_setup_command(
            workflow_options(fixture.path(), None, None, false),
            fixture.path(),
            &process,
        )?;

        let profile = installation_profile(fixture.path())?.expect("profile should be stored");
        assert_eq!(profile.volicord_mcp_command, path_text(&volicord));
        Ok(())
    }

    #[test]
    fn setup_keeps_default_mcp_launch_bound_to_current_volicord(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("setup-path")?;
        let exe_dir = fixture.path().join("exe");
        let path_dir = fixture.path().join("path-bin");
        let volicord = write_executable(&exe_dir, &volicord_binary_name())?;
        write_executable(&path_dir, &volicord_binary_name())?;
        let process = FakeProcess {
            exe: volicord.clone(),
            env: BTreeMap::from([(PATH_ENV.to_owned(), env::join_paths([path_dir.as_path()])?)]),
        };

        run_setup_command(
            workflow_options(fixture.path(), None, None, false),
            fixture.path(),
            &process,
        )?;

        let profile = installation_profile(fixture.path())?.expect("profile should be stored");
        assert_eq!(profile.volicord_mcp_command, path_text(&volicord));
        Ok(())
    }

    #[test]
    fn setup_json_does_not_require_separate_mcp_executable(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("setup-single-executable")?;
        let bin_dir = fixture.path().join("bin");
        let volicord = write_executable(&bin_dir, &volicord_binary_name())?;
        let process = FakeProcess {
            exe: volicord.clone(),
            env: BTreeMap::new(),
        };

        let outcome = run_setup_command(
            workflow_options(fixture.path(), None, None, true),
            fixture.path(),
            &process,
        )?;

        assert_eq!(outcome.status, CommandStatus::ActionRequired);
        let value: Value = serde_json::from_str(&outcome.output)?;
        assert_eq!(value["status"], "action_required");
        assert_eq!(
            value["setup_report"]["installation_profile"]["status"],
            "complete"
        );
        let profile = installation_profile(fixture.path())?.expect("profile should be stored");
        assert_eq!(profile.volicord_mcp_command, path_text(&volicord));
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn setup_creates_requested_links() -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("setup-links")?;
        let bin_dir = fixture.path().join("bin");
        let link_bin = fixture.path().join("links");
        let volicord = write_executable(&bin_dir, &volicord_binary_name())?;
        let mcp = write_executable(&bin_dir, &mcp_binary_name())?;
        let process = FakeProcess {
            exe: volicord.clone(),
            env: BTreeMap::from([(PATH_ENV.to_owned(), env::join_paths([link_bin.as_path()])?)]),
        };

        let outcome = run_setup_command(
            workflow_options(fixture.path(), Some(&mcp), Some(&link_bin), true),
            fixture.path(),
            &process,
        )?;

        assert_eq!(outcome.status, CommandStatus::Complete);
        let value: Value = serde_json::from_str(&outcome.output)?;
        assert_eq!(value["status"], "complete");
        assert!(value["actions_performed"]
            .as_array()
            .expect("actions_performed should be an array")
            .iter()
            .any(|action| action["id"] == "create_volicord_link"));
        assert_eq!(
            fs::canonicalize(link_bin.join(volicord_binary_name()))?,
            volicord
        );
        assert_eq!(fs::canonicalize(link_bin.join(mcp_binary_name()))?, mcp);
        let profile = installation_profile(fixture.path())?.expect("profile should be stored");
        let metadata: Value = serde_json::from_str(&profile.metadata_json)?;
        assert_eq!(metadata["link_bin"], path_text(&link_bin));
        assert_eq!(metadata["link_bin_requested"], true);
        assert_eq!(metadata["link_results"]["volicord"], "created");
        assert!(metadata["link_results"]["volicord_mcp"].is_null());
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn setup_link_bin_reports_path_action_without_prompting(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("setup-links-path-action")?;
        let bin_dir = fixture.path().join("bin");
        let link_bin = fixture.path().join("links");
        let volicord = write_executable(&bin_dir, &volicord_binary_name())?;
        let mcp = write_executable(&bin_dir, &mcp_binary_name())?;
        let process = FakeProcess {
            exe: volicord.clone(),
            env: BTreeMap::new(),
        };

        let outcome = run_setup_command(
            workflow_options(fixture.path(), Some(&mcp), Some(&link_bin), true),
            fixture.path(),
            &process,
        )?;

        assert_eq!(outcome.status, CommandStatus::ActionRequired);
        let value: Value = serde_json::from_str(&outcome.output)?;
        assert_eq!(value["status"], "action_required");
        assert!(value["actions_required"]
            .as_array()
            .expect("actions_required should be an array")
            .iter()
            .any(|action| action["id"] == "add_link_bin_to_path"));
        assert_eq!(
            fs::canonicalize(link_bin.join(volicord_binary_name()))?,
            volicord
        );
        assert_eq!(fs::canonicalize(link_bin.join(mcp_binary_name()))?, mcp);
        Ok(())
    }

    #[test]
    fn setup_link_bin_error_still_saves_profile_when_possible(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("setup-link-bin-file")?;
        let bin_dir = fixture.path().join("bin");
        let link_bin = fixture.path().join("not-a-directory");
        fs::write(&link_bin, "not a directory")?;
        let volicord = write_executable(&bin_dir, &volicord_binary_name())?;
        let mcp = write_executable(&bin_dir, &mcp_binary_name())?;
        let process = FakeProcess {
            exe: volicord,
            env: BTreeMap::new(),
        };

        let outcome = run_setup_command(
            workflow_options(fixture.path(), Some(&mcp), Some(&link_bin), true),
            fixture.path(),
            &process,
        )?;

        assert_eq!(outcome.status, CommandStatus::ActionRequired);
        let value: Value = serde_json::from_str(&outcome.output)?;
        assert_eq!(
            value["setup_report"]["installation_profile"]["status"],
            "complete"
        );
        assert!(value["checks"]
            .as_array()
            .expect("checks should be an array")
            .iter()
            .any(|check| check["id"] == "link_bin"
                && check["summary"] == "link directory could not be created"
                && check["details"]["detail"]
                    .as_str()
                    .is_some_and(|detail| !detail.is_empty())));
        assert!(value["actions_required"]
            .as_array()
            .expect("actions_required should be an array")
            .iter()
            .any(|action| action["id"] == "repair_link_bin"));
        assert!(!value["actions_required"]
            .as_array()
            .expect("actions_required should be an array")
            .iter()
            .any(|action| action["id"] == "add_link_bin_to_path"));
        assert!(!link_bin.join(volicord_binary_name()).exists());
        assert!(!link_bin.join(mcp_binary_name()).exists());
        assert!(installation_profile(fixture.path())?.is_some());
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn setup_link_bin_probe_failure_reports_repair_action() -> Result<(), Box<dyn std::error::Error>>
    {
        use std::os::unix::fs::PermissionsExt;

        let fixture = TempRuntimeHome::new("setup-link-bin-probe-fails")?;
        let bin_dir = fixture.path().join("bin");
        let link_bin = fixture.path().join("links");
        fs::create_dir_all(&link_bin)?;
        let mut permissions = fs::metadata(&link_bin)?.permissions();
        permissions.set_mode(0o555);
        fs::set_permissions(&link_bin, permissions)?;
        if crate::shell_path::path_directory_is_verified_writable(&link_bin) {
            restore_writable_dir(&link_bin)?;
            return Ok(());
        }

        let volicord = write_executable(&bin_dir, &volicord_binary_name())?;
        let mcp = write_executable(&bin_dir, &mcp_binary_name())?;
        let process = FakeProcess {
            exe: volicord,
            env: BTreeMap::new(),
        };

        let outcome = run_setup_command(
            workflow_options(fixture.path(), Some(&mcp), Some(&link_bin), true),
            fixture.path(),
            &process,
        );
        restore_writable_dir(&link_bin)?;
        let outcome = outcome?;

        assert_eq!(outcome.status, CommandStatus::ActionRequired);
        let value: Value = serde_json::from_str(&outcome.output)?;
        assert!(value["checks"]
            .as_array()
            .expect("checks should be an array")
            .iter()
            .any(|check| check["id"] == "link_bin"
                && check["summary"] == "link directory is not writable"));
        assert!(value["actions_required"]
            .as_array()
            .expect("actions_required should be an array")
            .iter()
            .any(|action| action["id"] == "repair_link_bin"));
        assert!(!link_bin.join(volicord_binary_name()).exists());
        assert!(!link_bin.join(mcp_binary_name()).exists());
        assert_eq!(fs::read_dir(&link_bin)?.count(), 0);
        Ok(())
    }

    fn write_executable(dir: &Path, name: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
        fs::create_dir_all(dir)?;
        let path = dir.join(name);
        let mut file = fs::File::create(&path)?;
        writeln!(file, "#!/bin/sh")?;
        make_executable(&path)?;
        Ok(path)
    }

    #[cfg(unix)]
    fn make_executable(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(path)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions)?;
        Ok(())
    }

    #[cfg(not(unix))]
    fn make_executable(_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    #[cfg(unix)]
    fn restore_writable_dir(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(path)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions)?;
        Ok(())
    }

    #[test]
    fn runtime_home_id_is_stable_for_same_path() {
        let path = Path::new("/tmp/volicord-id-test");

        assert_eq!(
            runtime_home_id_for_path(path).unwrap(),
            runtime_home_id_for_path(path).unwrap()
        );
    }

    #[test]
    fn installation_profile_can_be_read_through_store_after_setup(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("setup-sql")?;
        let bin_dir = fixture.path().join("bin");
        let volicord = write_executable(&bin_dir, &volicord_binary_name())?;
        let mcp = write_executable(&bin_dir, &mcp_binary_name())?;
        let process = FakeProcess {
            exe: volicord,
            env: BTreeMap::new(),
        };

        run_setup_command(
            workflow_options(fixture.path(), Some(&mcp), None, false),
            fixture.path(),
            &process,
        )?;

        assert!(installation_profile(fixture.path())?.is_some());
        Ok(())
    }
}
