use super::plan::{
    assign_dynamic_id, build_validation_plan, process_owned, CommandKind, CommandSpec,
    ValidationPlan, RUN_DIRECTORY_PLACEHOLDER,
};
use super::{
    repository_relative, CommandStatus, ValidationCategories, ValidationCommandResult,
    ValidationProfile, ValidationRunSummary, ValidationStatus,
};
use crate::owner_route::run_owner_route;
use crate::repository::normalize_existing_root;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static RUN_SEQUENCE: AtomicU64 = AtomicU64::new(0);
static ATOMIC_WRITE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub fn run_validation(
    root: &Path,
    profile: ValidationProfile,
    base: &str,
) -> Result<ValidationRunSummary> {
    let root = normalize_existing_root(root)?;
    let route = run_owner_route(&root, Some(base))?;
    let plan = build_validation_plan(&root, profile, route)?;
    let run_id = new_run_id();
    let run_directory = root
        .join("target")
        .join("volicord-validation")
        .join(&run_id);
    fs::create_dir_all(run_directory.join("commands"))?;
    let mut executor = RealProcessExecutor;
    run_plan_with_executor(
        &root,
        &run_directory,
        run_id,
        profile,
        plan,
        &mut executor,
        true,
    )
}

trait ProcessExecutor {
    fn execute(
        &mut self,
        invocation: &super::CommandInvocation,
        stdout_path: &Path,
        stderr_path: &Path,
    ) -> ExecutionOutcome;
}

struct RealProcessExecutor;

impl ProcessExecutor for RealProcessExecutor {
    fn execute(
        &mut self,
        invocation: &super::CommandInvocation,
        stdout_path: &Path,
        stderr_path: &Path,
    ) -> ExecutionOutcome {
        match execute_process(invocation, stdout_path, stderr_path) {
            Ok(code) => ExecutionOutcome {
                exit_code: code,
                error: code
                    .is_none()
                    .then(|| "process terminated without an exit code".to_owned()),
            },
            Err(error) => {
                let message = format!("{error:#}");
                let _ = append_and_sync(stderr_path, &format!("{message}\n"));
                ExecutionOutcome {
                    exit_code: None,
                    error: Some(message),
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
struct ExecutionOutcome {
    exit_code: Option<i32>,
    error: Option<String>,
}

fn execute_process(
    invocation: &super::CommandInvocation,
    stdout_path: &Path,
    stderr_path: &Path,
) -> Result<Option<i32>> {
    let stdout = File::create(stdout_path)
        .with_context(|| format!("failed to create {}", stdout_path.display()))?;
    let stderr = File::create(stderr_path)
        .with_context(|| format!("failed to create {}", stderr_path.display()))?;
    let status = Command::new(&invocation.program)
        .current_dir(&invocation.working_directory)
        .args(&invocation.args)
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .status()
        .with_context(|| format!("failed to execute {}", invocation.program))?;
    sync_file(stdout_path)?;
    sync_file(stderr_path)?;
    Ok(status.code())
}

fn run_plan_with_executor(
    root: &Path,
    run_directory: &Path,
    run_id: String,
    profile: ValidationProfile,
    mut plan: ValidationPlan,
    executor: &mut dyn ProcessExecutor,
    progress: bool,
) -> Result<ValidationRunSummary> {
    resolve_run_directory(&mut plan.commands, run_directory);
    let summary_path = run_directory.join("summary.json");
    let mut summary = ValidationRunSummary {
        run_id,
        summary_path: repository_relative(root, &summary_path)?,
        profile,
        base_revision: plan.base_revision,
        head_revision: plan.head_revision,
        changed_paths: plan.changed_paths,
        changed_packages: plan.changed_packages,
        validation_classes: plan.validation_classes,
        started_at_unix_ms: now_unix_ms(),
        finished_at_unix_ms: None,
        status: ValidationStatus::Pending,
        exact_aggregate_attempts: 0,
        exact_aggregate_failed: false,
        aggregate_diagnostic: None,
        commands: plan
            .commands
            .iter()
            .map(|spec| result_from_spec(root, run_directory, spec))
            .collect::<Result<_>>()?,
        categories: ValidationCategories::default(),
    };
    checkpoint_summary(&summary_path, &mut summary, false)?;
    let lifecycle = initialize_run_lifecycle(run_directory, &summary)?;
    if progress {
        eprint!("{}", run_discovery_message(&summary));
    }

    let aggregate_index = plan
        .commands
        .iter()
        .position(|spec| matches!(spec.kind, CommandKind::ExactAggregate));
    let ordinary_end = aggregate_index.unwrap_or(plan.commands.len());
    for index in 0..ordinary_end {
        execute_spec(
            root,
            run_directory,
            &summary_path,
            &plan.commands[index],
            index,
            &mut summary,
            executor,
            progress,
        )?;
        if summary.commands[index].status == CommandStatus::Failed {
            let reason = format!("stopped after {}", summary.commands[index].id);
            skip_pending(&summary_path, &mut summary, &reason)?;
            return finish_summary(&summary_path, &lifecycle, summary);
        }
    }

    if let Some(index) = aggregate_index {
        execute_aggregate_policy(
            root,
            run_directory,
            &summary_path,
            &mut plan.commands,
            index,
            &mut summary,
            executor,
            progress,
        )?;
    }
    finish_summary(&summary_path, &lifecycle, summary)
}

#[allow(clippy::too_many_arguments)]
fn execute_aggregate_policy(
    root: &Path,
    run_directory: &Path,
    summary_path: &Path,
    specs: &mut Vec<CommandSpec>,
    aggregate_index: usize,
    summary: &mut ValidationRunSummary,
    executor: &mut dyn ProcessExecutor,
    progress: bool,
) -> Result<()> {
    summary.exact_aggregate_attempts = 1;
    execute_spec(
        root,
        run_directory,
        summary_path,
        &specs[aggregate_index],
        aggregate_index,
        summary,
        executor,
        progress,
    )?;
    if summary.commands[aggregate_index].status == CommandStatus::Passed {
        return Ok(());
    }
    summary.exact_aggregate_failed = true;
    checkpoint_summary(summary_path, summary, false)?;

    let failure = match failed_target(root, &summary.commands[aggregate_index])? {
        FailedTargetAnalysis::Single(failure) => failure,
        analysis => {
            record_aggregate_diagnostic(
                summary_path,
                summary,
                format!(
                    "first exact aggregate failure {}; isolated diagnostics and retry were not started",
                    analysis.description()
                ),
            )?;
            return Ok(());
        }
    };
    if summary.changed_packages.contains(&failure.package) {
        record_aggregate_diagnostic(
            summary_path,
            summary,
            format!(
                "first exact aggregate failure identified changed package {}; unchanged-package diagnostics were not started",
                failure.package
            ),
        )?;
        return Ok(());
    }

    let isolated = isolated_target_spec(root, &failure);
    let full_package = full_package_spec(root, &failure.package, "unchanged failing package");
    let isolated_index = append_dynamic_spec(root, run_directory, specs, summary, isolated)?;
    let package_index = append_dynamic_spec(root, run_directory, specs, summary, full_package)?;
    checkpoint_summary(summary_path, summary, false)?;
    execute_spec(
        root,
        run_directory,
        summary_path,
        &specs[isolated_index],
        isolated_index,
        summary,
        executor,
        progress,
    )?;
    execute_spec(
        root,
        run_directory,
        summary_path,
        &specs[package_index],
        package_index,
        summary,
        executor,
        progress,
    )?;
    if summary.commands[isolated_index].status != CommandStatus::Passed
        || summary.commands[package_index].status != CommandStatus::Passed
    {
        return Ok(());
    }

    let mut retry = specs[aggregate_index].clone();
    retry.label = "exact workspace aggregate retry".to_owned();
    retry.aggregate_attempt = Some(2);
    let retry_index = append_dynamic_spec(root, run_directory, specs, summary, retry)?;
    summary.exact_aggregate_attempts = 2;
    checkpoint_summary(summary_path, summary, false)?;
    execute_spec(
        root,
        run_directory,
        summary_path,
        &specs[retry_index],
        retry_index,
        summary,
        executor,
        progress,
    )?;
    if summary.commands[retry_index].status == CommandStatus::Passed {
        record_aggregate_diagnostic(
            summary_path,
            summary,
            format!(
                "exact aggregate retry passed after isolated diagnostics for {}",
                failure.package
            ),
        )?;
        return Ok(());
    }
    summary.exact_aggregate_failed = true;
    let retry_failure = match failed_target(root, &summary.commands[retry_index])? {
        FailedTargetAnalysis::Single(retry_failure) => retry_failure,
        analysis => {
            record_aggregate_diagnostic(
                summary_path,
                summary,
                format!(
                    "second exact aggregate failure {}; decomposition stopped without reusing the first failure target {}",
                    analysis.description(),
                    failure.package
                ),
            )?;
            return Ok(());
        }
    };
    if summary.changed_packages.contains(&retry_failure.package) {
        record_aggregate_diagnostic(
            summary_path,
            summary,
            format!(
                "second exact aggregate failure identified changed package {}; decomposition stopped",
                retry_failure.package
            ),
        )?;
        return Ok(());
    }
    if retry_failure.package != failure.package {
        record_aggregate_diagnostic(
            summary_path,
            summary,
            format!(
                "second exact aggregate failure identified different package {}; first failure identified {}; decomposition stopped",
                retry_failure.package, failure.package
            ),
        )?;
        return Ok(());
    }
    record_aggregate_diagnostic(
        summary_path,
        summary,
        format!(
            "second exact aggregate failure matched unchanged package {}; starting decomposition",
            retry_failure.package
        ),
    )?;
    let excluded = workspace_excluding_spec(root, &retry_failure.package);
    let package = full_package_spec(
        root,
        &retry_failure.package,
        "package after second aggregate-only failure",
    );
    let excluded_index = append_dynamic_spec(root, run_directory, specs, summary, excluded)?;
    let package_index = append_dynamic_spec(root, run_directory, specs, summary, package)?;
    checkpoint_summary(summary_path, summary, false)?;
    execute_spec(
        root,
        run_directory,
        summary_path,
        &specs[excluded_index],
        excluded_index,
        summary,
        executor,
        progress,
    )?;
    execute_spec(
        root,
        run_directory,
        summary_path,
        &specs[package_index],
        package_index,
        summary,
        executor,
        progress,
    )?;
    Ok(())
}

fn record_aggregate_diagnostic(
    summary_path: &Path,
    summary: &mut ValidationRunSummary,
    diagnostic: String,
) -> Result<()> {
    summary.aggregate_diagnostic = Some(diagnostic);
    checkpoint_summary(summary_path, summary, false)
}

#[allow(clippy::too_many_arguments)]
fn execute_spec(
    root: &Path,
    run_directory: &Path,
    summary_path: &Path,
    spec: &CommandSpec,
    index: usize,
    summary: &mut ValidationRunSummary,
    executor: &mut dyn ProcessExecutor,
    progress: bool,
) -> Result<()> {
    let stdout_path = run_directory
        .join("commands")
        .join(format!("{}.stdout.log", spec.id));
    let stderr_path = run_directory
        .join("commands")
        .join(format!("{}.stderr.log", spec.id));
    let result_path = run_directory
        .join("commands")
        .join(format!("{}.json", spec.id));
    let started = now_unix_ms();
    {
        let record = &mut summary.commands[index];
        record.started_at_unix_ms = Some(started);
        record.status = CommandStatus::Pending;
    }
    File::create(&stdout_path)?.sync_all()?;
    File::create(&stderr_path)?.sync_all()?;
    write_command_result(&result_path, &summary.commands[index])?;
    checkpoint_summary(summary_path, summary, false)?;
    if progress {
        eprintln!("validation [{}] starting: {}", spec.id, spec.label);
    }

    let outcome = match &spec.kind {
        CommandKind::Internal {
            stdout,
            stderr,
            exit_code,
        } => {
            write_and_sync(&stdout_path, stdout)?;
            write_and_sync(&stderr_path, stderr)?;
            ExecutionOutcome {
                exit_code: Some(*exit_code),
                error: None,
            }
        }
        CommandKind::Process | CommandKind::ExactAggregate => {
            executor.execute(&spec.invocation, &stdout_path, &stderr_path)
        }
    };
    let passed = outcome.exit_code == Some(0) && outcome.error.is_none();
    {
        let record = &mut summary.commands[index];
        record.finished_at_unix_ms = Some(now_unix_ms());
        record.exit_code = outcome.exit_code;
        record.error = outcome.error;
        record.status = if passed {
            CommandStatus::Passed
        } else {
            CommandStatus::Failed
        };
    }
    sync_file(&stdout_path)?;
    sync_file(&stderr_path)?;
    write_command_result(&result_path, &summary.commands[index])?;
    checkpoint_summary(summary_path, summary, false)?;
    if progress {
        eprintln!(
            "validation [{}] {}: {}",
            spec.id,
            if passed { "passed" } else { "failed" },
            spec.label
        );
    }
    let _ = root;
    Ok(())
}

fn append_dynamic_spec(
    root: &Path,
    run_directory: &Path,
    specs: &mut Vec<CommandSpec>,
    summary: &mut ValidationRunSummary,
    mut spec: CommandSpec,
) -> Result<usize> {
    let index = specs.len();
    assign_dynamic_id(&mut spec, index + 1);
    resolve_run_directory(std::slice::from_mut(&mut spec), run_directory);
    summary
        .commands
        .push(result_from_spec(root, run_directory, &spec)?);
    specs.push(spec);
    Ok(index)
}

fn result_from_spec(
    root: &Path,
    run_directory: &Path,
    spec: &CommandSpec,
) -> Result<ValidationCommandResult> {
    let commands = run_directory.join("commands");
    Ok(ValidationCommandResult {
        id: spec.id.clone(),
        label: spec.label.clone(),
        invocation: spec.invocation.clone(),
        status: CommandStatus::Pending,
        decomposed: spec.decomposed,
        aggregate_attempt: spec.aggregate_attempt,
        started_at_unix_ms: None,
        finished_at_unix_ms: None,
        exit_code: None,
        stdout_path: Some(repository_relative(
            root,
            &commands.join(format!("{}.stdout.log", spec.id)),
        )?),
        stderr_path: Some(repository_relative(
            root,
            &commands.join(format!("{}.stderr.log", spec.id)),
        )?),
        result_path: Some(repository_relative(
            root,
            &commands.join(format!("{}.json", spec.id)),
        )?),
        error: None,
        skipped_reason: None,
    })
}

fn skip_pending(
    summary_path: &Path,
    summary: &mut ValidationRunSummary,
    reason: &str,
) -> Result<()> {
    for command in &mut summary.commands {
        if command.status == CommandStatus::Pending && command.started_at_unix_ms.is_none() {
            command.status = CommandStatus::Skipped;
            command.skipped_reason = Some(reason.to_owned());
            command.stdout_path = None;
            command.stderr_path = None;
            command.result_path = None;
        }
    }
    checkpoint_summary(summary_path, summary, false)
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct RunLocator {
    run_id: String,
    summary_path: String,
    profile: ValidationProfile,
    started_at_unix_ms: u64,
}

#[derive(Debug)]
struct RunLifecycle {
    active_path: PathBuf,
}

fn initialize_run_lifecycle(
    run_directory: &Path,
    summary: &ValidationRunSummary,
) -> Result<RunLifecycle> {
    let validation_directory = run_directory
        .parent()
        .context("validation run directory has no parent")?;
    let active_directory = validation_directory.join("active");
    fs::create_dir_all(&active_directory)?;
    let locator = RunLocator {
        run_id: summary.run_id.clone(),
        summary_path: summary.summary_path.clone(),
        profile: summary.profile,
        started_at_unix_ms: summary.started_at_unix_ms,
    };
    let active_path = active_directory.join(format!("{}.json", summary.run_id));
    write_json_atomically(&active_path, &locator)?;
    write_json_atomically(&validation_directory.join("latest-run.json"), &locator)?;
    Ok(RunLifecycle { active_path })
}

fn run_discovery_message(summary: &ValidationRunSummary) -> String {
    format!(
        "validation run id: {}\nvalidation summary: {}\n",
        summary.run_id, summary.summary_path
    )
}

fn finish_summary(
    summary_path: &Path,
    lifecycle: &RunLifecycle,
    mut summary: ValidationRunSummary,
) -> Result<ValidationRunSummary> {
    summary.finished_at_unix_ms = Some(now_unix_ms());
    checkpoint_summary(summary_path, &mut summary, true)?;
    if lifecycle.active_path.exists() {
        fs::remove_file(&lifecycle.active_path)?;
        sync_parent_directory(&lifecycle.active_path)?;
    }
    Ok(summary)
}

fn checkpoint_summary(
    summary_path: &Path,
    summary: &mut ValidationRunSummary,
    finished: bool,
) -> Result<()> {
    summary.refresh_categories_and_status(finished);
    write_json_atomically(summary_path, summary)
}

fn write_command_result(path: &Path, command: &ValidationCommandResult) -> Result<()> {
    write_json_atomically(path, command)
}

fn write_json_atomically(path: &Path, value: &impl serde::Serialize) -> Result<()> {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .context("atomic JSON path has no UTF-8 file name")?;
    let temporary = path.with_file_name(format!(
        ".{file_name}.{}.{}.next",
        std::process::id(),
        ATOMIC_WRITE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    serde_json::to_writer_pretty(&mut file, value)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    #[cfg(windows)]
    if path.exists() {
        fs::remove_file(path)?;
    }
    fs::rename(&temporary, path)?;
    sync_parent_directory(path)?;
    Ok(())
}

fn sync_parent_directory(path: &Path) -> Result<()> {
    #[cfg(unix)]
    if let Some(parent) = path.parent() {
        File::open(parent)?.sync_all()?;
    }
    let _ = path;
    Ok(())
}

fn resolve_run_directory(specs: &mut [CommandSpec], run_directory: &Path) {
    let run_directory = run_directory.display().to_string();
    for spec in specs {
        for argument in &mut spec.invocation.args {
            if argument.contains(RUN_DIRECTORY_PLACEHOLDER) {
                *argument = argument.replace(RUN_DIRECTORY_PLACEHOLDER, &run_directory);
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FailedTarget {
    package: String,
    target_args: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum FailedTargetAnalysis {
    Single(FailedTarget),
    Missing,
    Ambiguous { packages: Vec<String> },
}

impl FailedTargetAnalysis {
    fn description(&self) -> String {
        match self {
            Self::Single(target) => format!("identified package {}", target.package),
            Self::Missing => "did not identify exactly one parseable rerun target".to_owned(),
            Self::Ambiguous { packages } if packages.len() > 1 => format!(
                "identified rerun targets from multiple packages: {}",
                packages.join(", ")
            ),
            Self::Ambiguous { packages } => format!(
                "identified multiple rerun targets for package {}",
                packages.first().map(String::as_str).unwrap_or("unknown")
            ),
        }
    }
}

fn failed_target(root: &Path, command: &ValidationCommandResult) -> Result<FailedTargetAnalysis> {
    let mut candidates = BTreeSet::new();
    for path in [&command.stdout_path, &command.stderr_path]
        .into_iter()
        .flatten()
    {
        let contents = fs::read_to_string(root.join(path))?;
        for line in contents
            .lines()
            .filter(|line| line.contains("to rerun pass"))
        {
            if let Some(arguments) = rerun_arguments(line) {
                if let Some(target) = target_from_arguments(&arguments) {
                    candidates.insert((target.package, target.target_args));
                }
            }
        }
    }
    if candidates.is_empty() {
        return Ok(FailedTargetAnalysis::Missing);
    }
    if candidates.len() != 1 {
        let packages = candidates
            .iter()
            .map(|(package, _)| package.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        return Ok(FailedTargetAnalysis::Ambiguous { packages });
    }
    let (package, target_args) = candidates.into_iter().next().expect("one candidate");
    Ok(FailedTargetAnalysis::Single(FailedTarget {
        package,
        target_args,
    }))
}

fn rerun_arguments(line: &str) -> Option<Vec<String>> {
    let marker = line.find("to rerun pass")?;
    let suffix = &line[marker + "to rerun pass".len()..];
    for delimiter in ['`', '\'', '"'] {
        let Some(start) = suffix.find(delimiter) else {
            continue;
        };
        let rest = &suffix[start + delimiter.len_utf8()..];
        let Some(end) = rest.find(delimiter) else {
            continue;
        };
        return shlex::split(&rest[..end]);
    }
    None
}

fn target_from_arguments(arguments: &[String]) -> Option<FailedTarget> {
    let package_index = arguments.iter().position(|argument| argument == "-p")?;
    let package = arguments.get(package_index + 1)?.clone();
    let mut target_args = Vec::new();
    for flag in ["--lib", "--test", "--bin", "--example"] {
        if let Some(index) = arguments.iter().position(|argument| argument == flag) {
            target_args.push(flag.to_owned());
            if flag != "--lib" {
                target_args.push(arguments.get(index + 1)?.clone());
            }
            return Some(FailedTarget {
                package,
                target_args,
            });
        }
    }
    None
}

fn isolated_target_spec(root: &Path, target: &FailedTarget) -> CommandSpec {
    let mut args = vec![
        "test".to_owned(),
        "--locked".to_owned(),
        "-p".to_owned(),
        target.package.clone(),
    ];
    args.extend(target.target_args.clone());
    args.push("--all-features".to_owned());
    let mut spec = process_owned(
        root,
        &format!("isolated failing target for {}", target.package),
        "cargo",
        args,
    );
    spec.decomposed = true;
    spec
}

fn full_package_spec(root: &Path, package: &str, context: &str) -> CommandSpec {
    let mut spec = process_owned(
        root,
        &format!("{context}: {package}"),
        "cargo",
        vec![
            "test".to_owned(),
            "--locked".to_owned(),
            "-p".to_owned(),
            package.to_owned(),
            "--all-targets".to_owned(),
            "--all-features".to_owned(),
        ],
    );
    spec.decomposed = true;
    spec
}

fn workspace_excluding_spec(root: &Path, package: &str) -> CommandSpec {
    let mut spec = process_owned(
        root,
        &format!("workspace excluding {package}"),
        "cargo",
        vec![
            "test".to_owned(),
            "--locked".to_owned(),
            "--workspace".to_owned(),
            "--exclude".to_owned(),
            package.to_owned(),
            "--all-targets".to_owned(),
            "--all-features".to_owned(),
        ],
    );
    spec.decomposed = true;
    spec
}

fn new_run_id() -> String {
    format!(
        "run-{}-{}-{}",
        now_unix_ms(),
        std::process::id(),
        RUN_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    )
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn write_and_sync(path: &Path, contents: &str) -> Result<()> {
    let mut file = File::create(path)?;
    file.write_all(contents.as_bytes())?;
    file.sync_all()?;
    Ok(())
}

fn append_and_sync(path: &Path, contents: &str) -> Result<()> {
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    file.write_all(contents.as_bytes())?;
    file.sync_all()?;
    Ok(())
}

fn sync_file(path: &Path) -> Result<()> {
    OpenOptions::new().read(true).open(path)?.sync_all()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::{Arc, Barrier};

    #[derive(Clone)]
    struct ScriptedResult {
        exit_code: Option<i32>,
        stdout: String,
        stderr: String,
        error: Option<String>,
    }

    struct ScriptedExecutor {
        results: VecDeque<ScriptedResult>,
        invocations: Vec<super::super::CommandInvocation>,
    }

    impl ScriptedExecutor {
        fn new(results: Vec<ScriptedResult>) -> Self {
            Self {
                results: results.into(),
                invocations: Vec::new(),
            }
        }
    }

    impl ProcessExecutor for ScriptedExecutor {
        fn execute(
            &mut self,
            invocation: &super::super::CommandInvocation,
            stdout_path: &Path,
            stderr_path: &Path,
        ) -> ExecutionOutcome {
            let run_directory = stdout_path
                .parent()
                .and_then(Path::parent)
                .expect("scripted run directory");
            let validation_directory = run_directory.parent().expect("validation directory");
            assert!(
                run_directory.join("summary.json").is_file(),
                "pending summary must exist before command execution"
            );
            assert!(
                validation_directory.join("latest-run.json").is_file(),
                "latest-run locator must exist before command execution"
            );
            assert!(
                validation_directory.join("active/test-run.json").is_file(),
                "active-run locator must exist before command execution"
            );
            self.invocations.push(invocation.clone());
            let result = self.results.pop_front().expect("scripted command result");
            write_and_sync(stdout_path, &result.stdout).expect("scripted stdout");
            write_and_sync(stderr_path, &result.stderr).expect("scripted stderr");
            ExecutionOutcome {
                exit_code: result.exit_code,
                error: result.error,
            }
        }
    }

    fn scripted(exit_code: i32, stdout: &str, stderr: &str) -> ScriptedResult {
        ScriptedResult {
            exit_code: Some(exit_code),
            stdout: stdout.to_owned(),
            stderr: stderr.to_owned(),
            error: None,
        }
    }

    fn plan(_root: &Path, commands: Vec<CommandSpec>, changed: &[&str]) -> ValidationPlan {
        let mut commands = commands;
        for (index, command) in commands.iter_mut().enumerate() {
            assign_dynamic_id(command, index + 1);
        }
        ValidationPlan {
            base_revision: "base".to_owned(),
            head_revision: "head".to_owned(),
            changed_paths: vec!["changed.rs".to_owned()],
            changed_packages: changed.iter().map(|value| (*value).to_owned()).collect(),
            validation_classes: vec!["rust".to_owned()],
            commands,
        }
    }

    fn run_scripted(
        commands: Vec<CommandSpec>,
        changed: &[&str],
        results: Vec<ScriptedResult>,
    ) -> (tempfile::TempDir, ValidationRunSummary, ScriptedExecutor) {
        let directory = tempfile::tempdir().expect("validation test directory");
        let root = directory.path();
        let run_directory = root.join("target/volicord-validation/test-run");
        fs::create_dir_all(run_directory.join("commands")).expect("command directory");
        let plan = plan(root, commands, changed);
        let mut executor = ScriptedExecutor::new(results);
        let summary = run_plan_with_executor(
            root,
            &run_directory,
            "test-run".to_owned(),
            ValidationProfile::Focused,
            plan,
            &mut executor,
            false,
        )
        .expect("scripted validation");
        (directory, summary, executor)
    }

    fn locator_summary(run_id: &str) -> ValidationRunSummary {
        ValidationRunSummary {
            run_id: run_id.to_owned(),
            summary_path: format!("target/volicord-validation/{run_id}/summary.json"),
            profile: ValidationProfile::Focused,
            base_revision: "base".to_owned(),
            head_revision: "head".to_owned(),
            changed_paths: Vec::new(),
            changed_packages: Vec::new(),
            validation_classes: Vec::new(),
            started_at_unix_ms: 1,
            finished_at_unix_ms: None,
            status: ValidationStatus::Pending,
            exact_aggregate_attempts: 0,
            exact_aggregate_failed: false,
            aggregate_diagnostic: None,
            commands: Vec::new(),
            categories: ValidationCategories::default(),
        }
    }

    fn aggregate_spec(root: &Path) -> CommandSpec {
        let mut aggregate = process_owned(
            root,
            "exact workspace aggregate",
            "cargo",
            vec!["test".to_owned(), "--workspace".to_owned()],
        );
        aggregate.kind = CommandKind::ExactAggregate;
        aggregate.aggregate_attempt = Some(1);
        aggregate
    }

    fn run_second_aggregate_failure(
        second_failure: &str,
        changed: &[&str],
    ) -> (ValidationRunSummary, ScriptedExecutor) {
        let first_failure = "error: test failed, to rerun pass `-p first-package --test lease`\n";
        let (_, summary, executor) = run_scripted(
            vec![aggregate_spec(Path::new("/repository"))],
            changed,
            vec![
                scripted(1, "", first_failure),
                scripted(0, "isolated ok\n", ""),
                scripted(0, "package ok\n", ""),
                scripted(1, "", second_failure),
            ],
        );
        (summary, executor)
    }

    #[test]
    fn concurrent_runs_keep_distinct_active_records_and_valid_latest_locator() {
        let directory = tempfile::tempdir().expect("validation test directory");
        let root = directory.path().to_path_buf();
        let barrier = Arc::new(Barrier::new(2));
        let handles = ["run-a", "run-b"].map(|run_id| {
            let root = root.clone();
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                let run_directory = root.join("target/volicord-validation").join(run_id);
                fs::create_dir_all(run_directory.join("commands"))
                    .expect("concurrent run directory");
                barrier.wait();
                initialize_run_lifecycle(&run_directory, &locator_summary(run_id))
                    .expect("initialize concurrent run")
            })
        });
        let lifecycles = handles.map(|handle| handle.join().expect("concurrent run thread"));
        for lifecycle in &lifecycles {
            assert!(lifecycle.active_path.is_file());
        }
        let latest: RunLocator = serde_json::from_slice(
            &fs::read(root.join("target/volicord-validation/latest-run.json"))
                .expect("concurrent latest-run locator"),
        )
        .expect("decode concurrent latest-run locator");
        assert!(matches!(latest.run_id.as_str(), "run-a" | "run-b"));
    }

    #[test]
    fn durable_logs_and_summary_preserve_exit_code_after_handle_loss() {
        let root = Path::new("/repository");
        let commands = vec![
            process_owned(root, "first", "fake", vec!["one".to_owned()]),
            process_owned(root, "second", "fake", vec!["two".to_owned()]),
        ];
        let (directory, summary, _) = run_scripted(
            commands,
            &[],
            vec![scripted(23, "complete stdout\n", "complete stderr\n")],
        );
        assert_eq!(summary.status, ValidationStatus::Failed);
        assert_eq!(summary.commands[0].exit_code, Some(23));
        assert_eq!(summary.commands[1].status, CommandStatus::Skipped);
        let summary_path = directory.path().join(&summary.summary_path);
        drop(summary);

        let recovered: ValidationRunSummary = serde_json::from_slice(
            &fs::read(&summary_path).expect("durable summary after lost handle"),
        )
        .expect("decode recovered summary");
        let locator: RunLocator = serde_json::from_slice(
            &fs::read(
                directory
                    .path()
                    .join("target/volicord-validation/latest-run.json"),
            )
            .expect("latest-run locator"),
        )
        .expect("decode latest-run locator");
        assert_eq!(locator.run_id, "test-run");
        assert_eq!(locator.summary_path, recovered.summary_path);
        assert!(!directory
            .path()
            .join("target/volicord-validation/active/test-run.json")
            .exists());
        assert_eq!(recovered.commands[0].exit_code, Some(23));
        assert_eq!(
            fs::read_to_string(
                directory
                    .path()
                    .join(recovered.commands[0].stdout_path.as_ref().unwrap())
            )
            .unwrap(),
            "complete stdout\n"
        );
        assert_eq!(
            fs::read_to_string(
                directory
                    .path()
                    .join(recovered.commands[0].stderr_path.as_ref().unwrap())
            )
            .unwrap(),
            "complete stderr\n"
        );
    }

    #[test]
    fn unchanged_aggregate_failure_retries_once_then_reports_decomposition_truthfully() {
        let root = Path::new("/repository");
        let mut aggregate = process_owned(
            root,
            "exact workspace aggregate",
            "cargo",
            vec![
                "test".to_owned(),
                "--workspace".to_owned(),
                "--all-targets".to_owned(),
                "--all-features".to_owned(),
            ],
        );
        aggregate.kind = CommandKind::ExactAggregate;
        aggregate.aggregate_attempt = Some(1);
        let failure = "error: test failed, to rerun pass `-p unchanged --test lease`\n";
        let (_, summary, executor) = run_scripted(
            vec![aggregate],
            &[],
            vec![
                scripted(1, "", failure),
                scripted(0, "isolated ok\n", ""),
                scripted(0, "package ok\n", ""),
                scripted(1, "", failure),
                scripted(0, "excluded ok\n", ""),
                scripted(0, "package ok again\n", ""),
            ],
        );
        assert_eq!(summary.exact_aggregate_attempts, 2);
        assert!(summary.exact_aggregate_failed);
        assert_eq!(summary.status, ValidationStatus::Failed);
        assert_eq!(summary.categories.decomposed.len(), 4);
        assert_eq!(
            summary
                .commands
                .iter()
                .filter(|command| command.aggregate_attempt.is_some())
                .count(),
            2
        );
        assert!(executor.invocations.iter().any(|invocation| {
            invocation
                .args
                .windows(2)
                .any(|pair| pair == ["--exclude", "unchanged"])
        }));
        assert!(summary
            .aggregate_diagnostic
            .as_deref()
            .is_some_and(|reason| reason.contains("matched unchanged package unchanged")));
        assert!(!summary.render_human().contains("profile result: passed"));
    }

    #[test]
    fn second_failure_in_a_different_package_stops_without_stale_decomposition() {
        let second = "error: test failed, to rerun pass `-p second-package --test another`\n";
        let (summary, executor) = run_second_aggregate_failure(second, &[]);
        assert_eq!(executor.invocations.len(), 4);
        assert!(!executor.invocations.iter().any(|invocation| {
            invocation
                .args
                .iter()
                .any(|argument| argument == "--exclude")
        }));
        assert!(summary
            .aggregate_diagnostic
            .as_deref()
            .is_some_and(|reason| reason.contains("different package second-package")
                && reason.contains("first failure identified first-package")));
    }

    #[test]
    fn second_failure_in_a_changed_package_stops_without_decomposition() {
        let second = "error: test failed, to rerun pass `-p changed-package --test another`\n";
        let (summary, executor) = run_second_aggregate_failure(second, &["changed-package"]);
        assert_eq!(executor.invocations.len(), 4);
        assert!(summary
            .aggregate_diagnostic
            .as_deref()
            .is_some_and(|reason| reason.contains("identified changed package changed-package")));
    }

    #[test]
    fn ambiguous_second_failure_records_packages_and_stops() {
        let second = concat!(
            "error: test failed, to rerun pass `-p first-package --test one`\n",
            "error: test failed, to rerun pass `-p other-package --test two`\n"
        );
        let (summary, executor) = run_second_aggregate_failure(second, &[]);
        assert_eq!(executor.invocations.len(), 4);
        assert!(summary
            .aggregate_diagnostic
            .as_deref()
            .is_some_and(|reason| reason
                .contains("multiple packages: first-package, other-package")
                && reason.contains("without reusing the first failure target")));
    }

    #[test]
    fn multiple_targets_in_the_same_second_failure_package_are_ambiguous() {
        let second = concat!(
            "error: test failed, to rerun pass `-p first-package --test one`\n",
            "error: test failed, to rerun pass `-p first-package --test two`\n"
        );
        let (summary, executor) = run_second_aggregate_failure(second, &[]);
        assert_eq!(executor.invocations.len(), 4);
        assert!(summary
            .aggregate_diagnostic
            .as_deref()
            .is_some_and(|reason| reason
                .contains("identified multiple rerun targets for package first-package")));
    }

    #[test]
    fn unparseable_second_failure_records_reason_and_stops() {
        let (summary, executor) =
            run_second_aggregate_failure("aggregate failed without rerun guidance\n", &[]);
        assert_eq!(executor.invocations.len(), 4);
        assert!(summary
            .aggregate_diagnostic
            .as_deref()
            .is_some_and(
                |reason| reason.contains("did not identify exactly one parseable")
                    && reason.contains("without reusing the first failure target")
            ));
    }

    #[test]
    fn changed_package_aggregate_failure_is_not_decomposed() {
        let root = Path::new("/repository");
        let mut aggregate = process_owned(
            root,
            "exact workspace aggregate",
            "cargo",
            vec!["test".to_owned(), "--workspace".to_owned()],
        );
        aggregate.kind = CommandKind::ExactAggregate;
        aggregate.aggregate_attempt = Some(1);
        let failure = "error: test failed, to rerun pass `-p changed --lib`\n";
        let (_, summary, executor) = run_scripted(
            vec![aggregate],
            &["changed"],
            vec![scripted(1, "", failure)],
        );
        assert_eq!(summary.status, ValidationStatus::Failed);
        assert!(summary.categories.decomposed.is_empty());
        assert_eq!(executor.invocations.len(), 1);
    }

    #[test]
    fn human_and_json_summaries_share_command_categories() {
        let root = Path::new("/repository");
        let command = process_owned(root, "focused command", "fake", vec!["ok".to_owned()]);
        let (_, summary, _) = run_scripted(vec![command], &[], vec![scripted(0, "ok\n", "")]);
        let json = serde_json::to_string_pretty(&summary).unwrap();
        let human = summary.render_human();
        assert_eq!(summary.status, ValidationStatus::Passed);
        for id in summary
            .categories
            .passed
            .iter()
            .chain(summary.categories.failed.iter())
            .chain(summary.categories.decomposed.iter())
            .chain(summary.categories.skipped.iter())
        {
            assert!(json.contains(id));
            assert!(human.contains(id));
        }
    }

    #[test]
    fn run_discovery_message_is_stderr_ready_and_identifies_the_pending_summary() {
        let summary = locator_summary("run-visible");
        assert_eq!(
            run_discovery_message(&summary),
            concat!(
                "validation run id: run-visible\n",
                "validation summary: target/volicord-validation/run-visible/summary.json\n"
            )
        );
    }

    #[test]
    fn rerun_parser_preserves_package_and_target() {
        let arguments = rerun_arguments(
            "error: test failed, to rerun pass `-p volicord-platform-fs --test mutation_lease_process`",
        )
        .expect("rerun arguments");
        let target = target_from_arguments(&arguments).expect("failing target");
        assert_eq!(target.package, "volicord-platform-fs");
        assert_eq!(target.target_args, ["--test", "mutation_lease_process"]);
    }
}
