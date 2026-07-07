use std::{
    io,
    path::{Path, PathBuf},
};

use crate::{
    setup_report::CommandAvailability,
    shell_path::{path_directory_is_on_path, PATH_ENV},
};

use super::{
    discovery::{command_availability_summary, suggested_link_bin_candidate},
    shell_startup::{
        selected_command_dirs, shell_path_command, shell_path_command_for_selected_dirs,
        shell_startup_plan, ShellStartupPlan,
    },
    SetupCommandError, SetupProcess, SetupTerminal,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum InteractiveSetupChoice {
    LinkOnly(PathBuf),
    LinkAndShell {
        link_bin: PathBuf,
        shell: ShellStartupPlan,
    },
    Manual {
        link_bin: Option<PathBuf>,
        command: String,
    },
    Skip,
    Cancelled(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct InteractiveMenuChoice {
    pub(super) number: usize,
    pub(super) label: String,
    pub(super) choice: InteractiveSetupChoice,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct InteractiveMenuPlan {
    pub(super) choices: Vec<InteractiveMenuChoice>,
    pub(super) shell_unavailable: Option<String>,
}

pub(super) fn prompt_command_availability_choice(
    terminal: &mut dyn SetupTerminal,
    process: &impl SetupProcess,
    commands: &[CommandAvailability],
    selected_paths: [&Path; 2],
) -> Result<InteractiveSetupChoice, SetupCommandError> {
    terminal.write_str(
        "Volicord setup can help make these commands available on PATH for future shells and MCP hosts.\n",
    )?;
    for command in commands {
        if !command.selected_path_ready() {
            terminal.write_str(&format!(
                "- {}: {}\n",
                command.command_name,
                command_availability_summary(command)
            ))?;
        }
    }

    let menu = plan_interactive_menu_choices(process, selected_paths)?;

    if let Some(reason) = &menu.shell_unavailable {
        terminal.write_str(&format!("Shell startup update is unavailable: {reason}\n"))?;
    }
    terminal.write_str("Choices:\n")?;
    for choice in &menu.choices {
        terminal.write_str(&format!("  {}. {}\n", choice.number, choice.label))?;
    }

    let skip_number = menu
        .choices
        .iter()
        .find(|choice| matches!(choice.choice, InteractiveSetupChoice::Skip))
        .map(|choice| choice.number)
        .unwrap_or(menu.choices.len());
    loop {
        terminal.write_str(&format!("Choice [{skip_number}]: "))?;
        let Some(input) = read_prompt_line(terminal)? else {
            return Ok(InteractiveSetupChoice::Cancelled(
                "setup prompt cancelled; no command links or shell startup files were changed"
                    .to_owned(),
            ));
        };
        let selected_number = if input.trim().is_empty() {
            skip_number
        } else if let Ok(number) = input.trim().parse::<usize>() {
            number
        } else {
            terminal.write_str("Enter the number of one setup choice.\n")?;
            continue;
        };
        let Some(choice) = menu
            .choices
            .iter()
            .find(|choice| choice.number == selected_number)
            .map(|choice| choice.choice.clone())
        else {
            terminal.write_str("Enter one of the listed setup choice numbers.\n")?;
            continue;
        };
        return confirm_interactive_choice(terminal, choice);
    }
}

pub(super) fn plan_interactive_menu_choices(
    process: &impl SetupProcess,
    selected_paths: [&Path; 2],
) -> Result<InteractiveMenuPlan, SetupCommandError> {
    let path_env = process.env_var(PATH_ENV);
    let link_candidate = suggested_link_bin_candidate(process);
    let mut shell_unavailable = None;
    let mut choices = Vec::new();
    if let Some(link_candidate) = link_candidate {
        let link_bin = link_candidate.path().to_path_buf();
        let requires_creation = link_candidate.requires_creation();
        let link_bin_on_path = path_directory_is_on_path(path_env.as_deref(), &link_bin);
        if link_bin_on_path {
            push_menu_choice(
                &mut choices,
                link_only_label(&link_bin, requires_creation, "already on PATH"),
                InteractiveSetupChoice::LinkOnly(link_bin.clone()),
            );
        } else {
            match shell_startup_plan(process, &link_bin) {
                Ok(plan) => push_menu_choice(
                    &mut choices,
                    link_and_shell_label(&link_bin, requires_creation, &plan.target_file),
                    InteractiveSetupChoice::LinkAndShell {
                        link_bin: link_bin.clone(),
                        shell: plan,
                    },
                ),
                Err(reason) => shell_unavailable = Some(reason),
            }
            push_menu_choice(
                &mut choices,
                link_only_label(&link_bin, requires_creation, "PATH still needs an update"),
                InteractiveSetupChoice::LinkOnly(link_bin.clone()),
            );
        }

        push_menu_choice(
            &mut choices,
            manual_link_label(&link_bin, requires_creation),
            InteractiveSetupChoice::Manual {
                link_bin: Some(link_bin.clone()),
                command: shell_path_command(process, &link_bin)?,
            },
        );
    } else {
        let command =
            shell_path_command_for_selected_dirs(process, &selected_command_dirs(selected_paths))?;
        push_menu_choice(
            &mut choices,
            "Print the PATH command without modifying files.".to_owned(),
            InteractiveSetupChoice::Manual {
                link_bin: None,
                command,
            },
        );
    }
    push_menu_choice(
        &mut choices,
        "Skip command linking for now.".to_owned(),
        InteractiveSetupChoice::Skip,
    );

    Ok(InteractiveMenuPlan {
        choices,
        shell_unavailable,
    })
}

fn link_and_shell_label(link_bin: &Path, requires_creation: bool, target_file: &Path) -> String {
    if requires_creation {
        format!(
            "Create {}, create links, and add a managed PATH block to {}.",
            link_bin.display(),
            target_file.display()
        )
    } else {
        format!(
            "Create links and add a managed PATH block to {}.",
            target_file.display()
        )
    }
}

fn link_only_label(link_bin: &Path, requires_creation: bool, path_status: &str) -> String {
    if requires_creation {
        format!(
            "Create {} and command links; {path_status}.",
            link_bin.display()
        )
    } else {
        format!(
            "Create command links in {}; {path_status}.",
            link_bin.display()
        )
    }
}

fn manual_link_label(link_bin: &Path, requires_creation: bool) -> String {
    if requires_creation {
        format!(
            "Create {}, create links, and print the PATH command.",
            link_bin.display()
        )
    } else {
        format!(
            "Create command links in {} and print the PATH command.",
            link_bin.display()
        )
    }
}

fn confirm_interactive_choice(
    terminal: &mut dyn SetupTerminal,
    choice: InteractiveSetupChoice,
) -> Result<InteractiveSetupChoice, SetupCommandError> {
    match choice {
        InteractiveSetupChoice::LinkAndShell { link_bin, shell } => {
            terminal.write_str(&format!(
                "Shell startup file:\n  {}\n\nManaged block to write:\n{}",
                shell.target_file.display(),
                shell.block
            ))?;
            terminal.write_str("Write this managed block? [y/N]: ")?;
            let Some(answer) = read_prompt_line(terminal)? else {
                return Ok(InteractiveSetupChoice::Cancelled(
                    "setup prompt cancelled; no command links or shell startup files were changed"
                        .to_owned(),
                ));
            };
            if is_yes(&answer) {
                Ok(InteractiveSetupChoice::LinkAndShell { link_bin, shell })
            } else {
                Ok(InteractiveSetupChoice::Cancelled(
                    "shell startup update was not approved; no command links or shell startup files were changed"
                        .to_owned(),
                ))
            }
        }
        InteractiveSetupChoice::Manual { link_bin, command } => {
            terminal.write_str(&format!("Run this command after setup:\n  {command}\n"))?;
            Ok(InteractiveSetupChoice::Manual { link_bin, command })
        }
        other => Ok(other),
    }
}

fn push_menu_choice(
    choices: &mut Vec<InteractiveMenuChoice>,
    label: String,
    choice: InteractiveSetupChoice,
) {
    choices.push(InteractiveMenuChoice {
        number: choices.len() + 1,
        label,
        choice,
    });
}

fn read_prompt_line(terminal: &mut dyn SetupTerminal) -> Result<Option<String>, SetupCommandError> {
    let mut input = String::new();
    match terminal.read_line(&mut input) {
        Ok(0) => Ok(None),
        Ok(_) => Ok(Some(input.trim().to_owned())),
        Err(error) if error.kind() == io::ErrorKind::Interrupted => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn is_yes(input: &str) -> bool {
    matches!(input.trim().to_ascii_lowercase().as_str(), "y" | "yes")
}
