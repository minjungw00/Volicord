mod current_plan;
mod plan;
mod runner;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub use current_plan::{
    current_linux_validation_plan, CurrentValidationCommand, CurrentValidationCommandKind,
    CurrentValidationPlan,
};
pub use runner::run_validation;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationProfile {
    Focused,
    Final,
}

impl ValidationProfile {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "focused" => Some(Self::Focused),
            "final" => Some(Self::Final),
            _ => None,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Focused => "focused",
            Self::Final => "final",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationStatus {
    Pending,
    Passed,
    Failed,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandStatus {
    Pending,
    Passed,
    Failed,
    Skipped,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CommandInvocation {
    pub program: String,
    pub args: Vec<String>,
    pub working_directory: String,
}

impl CommandInvocation {
    fn render(&self) -> String {
        std::iter::once(self.program.as_str())
            .chain(self.args.iter().map(String::as_str))
            .map(shlex::try_quote)
            .collect::<std::result::Result<Vec<_>, _>>()
            .map(|parts| parts.join(" "))
            .unwrap_or_else(|_| {
                std::iter::once(self.program.as_str())
                    .chain(self.args.iter().map(String::as_str))
                    .collect::<Vec<_>>()
                    .join(" ")
            })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ValidationCommandResult {
    pub id: String,
    pub label: String,
    pub invocation: CommandInvocation,
    pub status: CommandStatus,
    pub decomposed: bool,
    pub aggregate_attempt: Option<u8>,
    pub started_at_unix_ms: Option<u64>,
    pub finished_at_unix_ms: Option<u64>,
    pub exit_code: Option<i32>,
    pub stdout_path: Option<String>,
    pub stderr_path: Option<String>,
    pub result_path: Option<String>,
    pub error: Option<String>,
    pub skipped_reason: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct ValidationCategories {
    pub passed: Vec<String>,
    pub failed: Vec<String>,
    pub decomposed: Vec<String>,
    pub skipped: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ValidationRunSummary {
    pub run_id: String,
    pub summary_path: String,
    pub profile: ValidationProfile,
    pub base_revision: String,
    pub head_revision: String,
    pub changed_paths: Vec<String>,
    pub changed_packages: Vec<String>,
    pub validation_classes: Vec<String>,
    pub started_at_unix_ms: u64,
    pub finished_at_unix_ms: Option<u64>,
    pub status: ValidationStatus,
    pub exact_aggregate_attempts: u8,
    pub exact_aggregate_failed: bool,
    pub aggregate_diagnostic: Option<String>,
    pub commands: Vec<ValidationCommandResult>,
    pub categories: ValidationCategories,
}

impl ValidationRunSummary {
    pub fn is_success(&self) -> bool {
        self.status == ValidationStatus::Passed
    }

    pub fn render_human(&self) -> String {
        let mut output = String::new();
        output.push_str(&format!("profile result: {:?}\n", self.status).to_lowercase());
        output.push_str(&format!("profile: {}\n", self.profile.label()));
        output.push_str(&format!("run id: {}\n", self.run_id));
        output.push_str(&format!("summary: {}\n", self.summary_path));
        output.push_str(&format!("base revision: {}\n", self.base_revision));
        output.push_str(&format!("head revision: {}\n", self.head_revision));
        output.push_str(&format!(
            "exact aggregate attempts: {}\n",
            self.exact_aggregate_attempts
        ));
        output.push_str(&format!(
            "exact aggregate failed: {}\n",
            self.exact_aggregate_failed
        ));
        output.push_str(&format!(
            "aggregate diagnostic: {}\n",
            self.aggregate_diagnostic.as_deref().unwrap_or("none")
        ));
        render_values(&mut output, "changed paths", &self.changed_paths);
        render_values(&mut output, "changed packages", &self.changed_packages);
        render_values(&mut output, "validation classes", &self.validation_classes);
        render_category(&mut output, "passed", &self.categories.passed, self);
        render_category(&mut output, "failed", &self.categories.failed, self);
        render_category(&mut output, "decomposed", &self.categories.decomposed, self);
        render_category(&mut output, "skipped", &self.categories.skipped, self);
        output
    }

    pub(crate) fn refresh_categories_and_status(&mut self, finished: bool) {
        self.categories = ValidationCategories {
            passed: self
                .commands
                .iter()
                .filter(|command| command.status == CommandStatus::Passed)
                .map(|command| command.id.clone())
                .collect(),
            failed: self
                .commands
                .iter()
                .filter(|command| command.status == CommandStatus::Failed)
                .map(|command| command.id.clone())
                .collect(),
            decomposed: self
                .commands
                .iter()
                .filter(|command| command.decomposed)
                .map(|command| command.id.clone())
                .collect(),
            skipped: self
                .commands
                .iter()
                .filter(|command| command.status == CommandStatus::Skipped)
                .map(|command| command.id.clone())
                .collect(),
        };
        self.status = if !self.categories.failed.is_empty()
            || (finished
                && self
                    .commands
                    .iter()
                    .any(|command| command.status != CommandStatus::Passed))
        {
            ValidationStatus::Failed
        } else if finished {
            ValidationStatus::Passed
        } else {
            ValidationStatus::Pending
        };
    }
}

fn render_values(output: &mut String, heading: &str, values: &[String]) {
    output.push_str(heading);
    output.push_str(":\n");
    if values.is_empty() {
        output.push_str("- none\n");
    } else {
        for value in values {
            output.push_str("- ");
            output.push_str(value);
            output.push('\n');
        }
    }
}

fn render_category(
    output: &mut String,
    heading: &str,
    ids: &[String],
    summary: &ValidationRunSummary,
) {
    output.push_str(heading);
    output.push_str(":\n");
    if ids.is_empty() {
        output.push_str("- none\n");
        return;
    }
    for id in ids {
        let command = summary
            .commands
            .iter()
            .find(|command| command.id == *id)
            .expect("summary category IDs come from commands");
        output.push_str(&format!(
            "- {}: {} [{}] exit={}; {}\n",
            command.id,
            command.label,
            command.invocation.render(),
            command
                .exit_code
                .map_or_else(|| "none".to_owned(), |code| code.to_string()),
            command
                .skipped_reason
                .as_deref()
                .or(command.error.as_deref())
                .unwrap_or("recorded")
        ));
    }
}

pub(crate) fn repository_relative(root: &Path, path: &Path) -> Result<String> {
    Ok(path
        .strip_prefix(root)
        .with_context(|| format!("{} is outside {}", path.display(), root.display()))?
        .to_string_lossy()
        .replace('\\', "/"))
}

use anyhow::Context as _;
