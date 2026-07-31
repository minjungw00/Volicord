use std::{
    ffi::OsString,
    fmt, fs,
    path::{Path, PathBuf},
};

use serde_json::{json, Value};
use volicord_command_model::{
    JsonArgs, ProjectArgs, ProjectCommand, ProjectForgetArgs, ProjectRenameArgs, ProjectUseArgs,
};
use volicord_store::{
    bootstrap::{
        ensure_project_for_repo, forget_project, list_projects, project_record_by_repo_root,
        project_record_by_repo_root_admitted, rename_project, ProjectRecord,
        RepoProjectRegistration, ACTIVE_PROJECT_STATUS,
    },
    runtime_home::{resolve_runtime_home, RuntimeHomeResolutionError},
    RuntimeHomeMutationContext, StoreError,
};

use crate::{
    mutation_admission::{with_cli_runtime_home_mutation, CliMutationAdmissionError},
    presentation::{ActionHint, CollectionItem, Document, Field, HumanValue},
};

const PROJECT_METADATA_CREATED_BY: &str = "volicord_cli_project_command";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectCommandError {
    Usage(String),
    Runtime(String),
    MutationAdmission(CliMutationAdmissionError),
}

impl ProjectCommandError {
    fn runtime(message: impl Into<String>) -> Self {
        Self::Runtime(message.into())
    }
}

impl fmt::Display for ProjectCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(message) | Self::Runtime(message) => formatter.write_str(message),
            Self::MutationAdmission(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for ProjectCommandError {}

impl From<CliMutationAdmissionError> for ProjectCommandError {
    fn from(error: CliMutationAdmissionError) -> Self {
        Self::MutationAdmission(error)
    }
}

impl From<StoreError> for ProjectCommandError {
    fn from(error: StoreError) -> Self {
        Self::runtime(error.to_string())
    }
}

impl From<RuntimeHomeResolutionError> for ProjectCommandError {
    fn from(error: RuntimeHomeResolutionError) -> Self {
        Self::runtime(error.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputFormat {
    Text,
    Json,
}

pub fn run_project_command<F>(
    args: ProjectArgs,
    env_var: F,
    current_dir: &Path,
) -> Result<String, ProjectCommandError>
where
    F: Fn(&str) -> Option<OsString>,
{
    match args.command {
        ProjectCommand::Use(options) => {
            let runtime_home = resolve_runtime_home(&env_var, current_dir)?;
            with_cli_runtime_home_mutation(&runtime_home, "cli.project.use", |context| {
                command_use(context, options, current_dir)
                    .map_err(|error| CliMutationAdmissionError::Operation(error.to_string()))
            })
            .map_err(Into::into)
        }
        ProjectCommand::Current(options) => {
            let runtime_home = resolve_runtime_home(&env_var, current_dir)?;
            command_current(options, &runtime_home, current_dir)
        }
        ProjectCommand::List(options) => {
            let runtime_home = resolve_runtime_home(&env_var, current_dir)?;
            command_list(options, &runtime_home)
        }
        ProjectCommand::Rename(options) => {
            let runtime_home = resolve_runtime_home(&env_var, current_dir)?;
            with_cli_runtime_home_mutation(&runtime_home, "cli.project.rename", |context| {
                command_rename(context, options, current_dir)
                    .map_err(|error| CliMutationAdmissionError::Operation(error.to_string()))
            })
            .map_err(Into::into)
        }
        ProjectCommand::Forget(options) => {
            let runtime_home = resolve_runtime_home(&env_var, current_dir)?;
            with_cli_runtime_home_mutation(&runtime_home, "cli.project.forget", |context| {
                command_forget(context, options, current_dir)
                    .map_err(|error| CliMutationAdmissionError::Operation(error.to_string()))
            })
            .map_err(Into::into)
        }
    }
}

fn command_use(
    context: &RuntimeHomeMutationContext<'_>,
    options: ProjectUseArgs,
    current_dir: &Path,
) -> Result<String, ProjectCommandError> {
    let runtime_home = context.runtime_home().as_path();
    let repo_root = resolve_repository_root(current_dir, options.path.as_deref())?;
    let existing = project_record_by_repo_root(runtime_home, &repo_root)?;
    let created = existing.is_none();
    let project = match existing {
        Some(project) => project,
        None => ensure_project_for_repo(
            context,
            RepoProjectRegistration {
                project_name: None,
                project_alias: None,
                repo_root,
                project_home: None,
                status: ACTIVE_PROJECT_STATUS.to_owned(),
                metadata_json: project_metadata_json()?,
            },
        )?,
    };

    render_use_output(output_format(options.json), &project, created)
}

fn command_current(
    options: JsonArgs,
    runtime_home: &Path,
    current_dir: &Path,
) -> Result<String, ProjectCommandError> {
    let repo_root = resolve_repository_root(current_dir, None)?;
    let project = project_record_by_repo_root(runtime_home, &repo_root)?;
    render_current_output(output_format(options.json), project.as_ref(), &repo_root)
}

fn command_list(options: JsonArgs, runtime_home: &Path) -> Result<String, ProjectCommandError> {
    let projects = list_projects(runtime_home)?;
    render_list_output(output_format(options.json), &projects)
}

fn command_rename(
    context: &RuntimeHomeMutationContext<'_>,
    options: ProjectRenameArgs,
    current_dir: &Path,
) -> Result<String, ProjectCommandError> {
    let repo_root = resolve_repository_root(current_dir, options.repo.as_deref())?;
    let project = registered_project_for_repo_admitted(context, &repo_root)?;
    let project = rename_project(context, &project.project_internal_id, &options.name, None)?;
    render_project_action_output(
        output_format(options.json),
        "renamed",
        "project renamed",
        &project,
    )
}

fn command_forget(
    context: &RuntimeHomeMutationContext<'_>,
    options: ProjectForgetArgs,
    current_dir: &Path,
) -> Result<String, ProjectCommandError> {
    let runtime_home = context.runtime_home().as_path();
    let project = match options.selector.as_ref() {
        Some(selector) if selector_is_path(selector, current_dir)? => {
            let repo_root = resolve_repository_root(current_dir, Some(Path::new(selector)))?;
            registered_project_for_repo_admitted(context, &repo_root)?
        }
        Some(name) => project_by_name(runtime_home, name)?,
        None => {
            let repo_root = resolve_repository_root(current_dir, None)?;
            registered_project_for_repo_admitted(context, &repo_root)?
        }
    };
    if !forget_project(context, &project.project_internal_id)? {
        return Err(ProjectCommandError::runtime(format!(
            "project is not registered for repository {}",
            project.repo_root.display()
        )));
    }
    render_forget_output(output_format(options.json), &project)
}

fn output_format(json: bool) -> OutputFormat {
    if json {
        OutputFormat::Json
    } else {
        OutputFormat::Text
    }
}

pub(crate) fn resolve_repository_root(
    current_dir: &Path,
    selected_path: Option<&Path>,
) -> Result<PathBuf, ProjectCommandError> {
    let selected = selected_path.unwrap_or(current_dir);
    let absolute = absolute_path(current_dir, selected.to_path_buf());
    let canonical = fs::canonicalize(&absolute).map_err(|error| {
        ProjectCommandError::runtime(format!(
            "repository path is not accessible: {} ({error})",
            absolute.display()
        ))
    })?;
    let metadata = fs::metadata(&canonical).map_err(|error| {
        ProjectCommandError::runtime(format!(
            "repository path is not accessible: {} ({error})",
            canonical.display()
        ))
    })?;
    let mut cursor = if metadata.is_file() {
        canonical
            .parent()
            .ok_or_else(|| {
                ProjectCommandError::runtime(format!(
                    "repository path has no parent directory: {}",
                    canonical.display()
                ))
            })?
            .to_path_buf()
    } else {
        canonical
    };

    loop {
        let git_path = cursor.join(".git");
        match git_path.try_exists() {
            Ok(true) => return Ok(cursor),
            Ok(false) => {}
            Err(error) => {
                return Err(ProjectCommandError::runtime(format!(
                    "failed to inspect Git repository marker {}: {error}",
                    git_path.display()
                )));
            }
        }
        if !cursor.pop() {
            break;
        }
    }

    Err(ProjectCommandError::runtime(format!(
        "no Git repository root found from {}; run `volicord project use PATH` from inside a Git repository or pass a repository path",
        absolute.display()
    )))
}

fn selector_is_path(selector: &str, current_dir: &Path) -> Result<bool, ProjectCommandError> {
    let path = Path::new(selector);
    if path.is_absolute()
        || selector == "."
        || selector == ".."
        || selector.contains('/')
        || selector.contains('\\')
    {
        return Ok(true);
    }
    current_dir.join(path).try_exists().map_err(|error| {
        ProjectCommandError::runtime(format!("failed to inspect selector {}: {error}", selector))
    })
}

pub(crate) fn registered_project_for_repo(
    runtime_home: &Path,
    repo_root: &Path,
) -> Result<ProjectRecord, ProjectCommandError> {
    project_record_by_repo_root(runtime_home, repo_root)?.ok_or_else(|| {
        ProjectCommandError::runtime(format!(
            "project is not registered for repository {}; run `volicord project use`",
            repo_root.display()
        ))
    })
}

pub(crate) fn registered_project_for_repo_admitted(
    context: &RuntimeHomeMutationContext<'_>,
    repo_root: &Path,
) -> Result<ProjectRecord, ProjectCommandError> {
    project_record_by_repo_root_admitted(context, repo_root)?.ok_or_else(|| {
        ProjectCommandError::runtime(format!(
            "project is not registered for repository {}; run `volicord project use`",
            repo_root.display()
        ))
    })
}

fn project_by_name(runtime_home: &Path, name: &str) -> Result<ProjectRecord, ProjectCommandError> {
    let matches = list_projects(runtime_home)?
        .into_iter()
        .filter(|project| project.project_name == name)
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [project] => Ok(project.clone()),
        [] => Err(ProjectCommandError::runtime(format!(
            "project not found by name: {name}"
        ))),
        projects => {
            let mut message =
                format!("project name is ambiguous: {name}; use a repository path instead\n");
            for project in projects {
                message.push_str(&format!("- {}\n", project.repo_root.display()));
            }
            Err(ProjectCommandError::runtime(message))
        }
    }
}

fn render_use_output(
    output: OutputFormat,
    project: &ProjectRecord,
    created: bool,
) -> Result<String, ProjectCommandError> {
    let status = if created { "registered" } else { "selected" };
    let text_label = if created {
        "project registered"
    } else {
        "project selected"
    };
    render_project_action_output(output, status, text_label, project)
}

fn render_project_action_output(
    output: OutputFormat,
    status: &str,
    text_label: &str,
    project: &ProjectRecord,
) -> Result<String, ProjectCommandError> {
    match output {
        OutputFormat::Json => serde_json::to_string_pretty(&json!({
            "status": status,
            "project": project_json(project),
        }))
        .map(|text| format!("{text}\n"))
        .map_err(|error| ProjectCommandError::runtime(error.to_string())),
        OutputFormat::Text => Ok(format!(
            "{text_label}\nname: {}\nrepo_root: {}\nstatus: {}\n",
            project.project_name,
            project.repo_root.display(),
            project.status
        )),
    }
}

fn render_current_output(
    output: OutputFormat,
    project: Option<&ProjectRecord>,
    repo_root: &Path,
) -> Result<String, ProjectCommandError> {
    match (output, project) {
        (OutputFormat::Json, Some(project)) => serde_json::to_string_pretty(&json!({
            "status": "registered",
            "project": project_json(project),
        }))
        .map(|text| format!("{text}\n"))
        .map_err(|error| ProjectCommandError::runtime(error.to_string())),
        (OutputFormat::Json, None) => serde_json::to_string_pretty(&json!({
            "status": "not_registered",
            "repo_root": path_text(repo_root),
            "action": "volicord project use",
        }))
        .map(|text| format!("{text}\n"))
        .map_err(|error| ProjectCommandError::runtime(error.to_string())),
        (OutputFormat::Text, Some(project)) => Ok(Document::new(
            "Current project",
            vec![
                project_field("Name", &project.project_name),
                project_field("Repository", path_text(&project.repo_root)),
                project_field("Status", &project.status),
            ],
        )
        .render()),
        (OutputFormat::Text, None) => Ok(Document::new(
            "Repository is not registered.",
            vec![
                project_field("Repository", path_text(repo_root)),
                ActionHint::new("Run `volicord project use`.").into(),
            ],
        )
        .render()),
    }
}

fn render_list_output(
    output: OutputFormat,
    projects: &[ProjectRecord],
) -> Result<String, ProjectCommandError> {
    match output {
        OutputFormat::Json => {
            let values = projects.iter().map(project_json).collect::<Vec<_>>();
            serde_json::to_string_pretty(&json!({ "projects": values }))
                .map(|text| format!("{text}\n"))
                .map_err(|error| ProjectCommandError::runtime(error.to_string()))
        }
        OutputFormat::Text if projects.is_empty() => {
            Ok(Document::new("No projects are registered.", Vec::new()).render())
        }
        OutputFormat::Text => {
            let count = HumanValue::Count(projects.len());
            let items = projects
                .iter()
                .map(|project| {
                    CollectionItem::new(
                        &project.project_name,
                        vec![
                            Field::new("Status", HumanValue::text(&project.status)),
                            Field::new(
                                "Repository",
                                HumanValue::text(path_text(&project.repo_root)),
                            ),
                        ],
                    )
                    .into()
                })
                .collect();
            Ok(Document::new(format!("Projects ({count})"), items).render())
        }
    }
}

fn project_field(label: &str, value: impl Into<String>) -> crate::presentation::Element {
    Field::new(label, HumanValue::text(value)).into()
}

fn render_forget_output(
    output: OutputFormat,
    project: &ProjectRecord,
) -> Result<String, ProjectCommandError> {
    match output {
        OutputFormat::Json => serde_json::to_string_pretty(&json!({
            "status": "forgotten",
            "project": project_json(project),
            "project_state_deleted": false,
        }))
        .map(|text| format!("{text}\n"))
        .map_err(|error| ProjectCommandError::runtime(error.to_string())),
        OutputFormat::Text => Ok(format!(
            "project forgotten\nname: {}\nrepo_root: {}\nproject_state_deleted: false\n",
            project.project_name,
            project.repo_root.display()
        )),
    }
}

fn project_json(project: &ProjectRecord) -> Value {
    json!({
        "project_internal_id": &project.project_internal_id,
        "project_name": &project.project_name,
        "project_alias": &project.project_alias,
        "repo_root": path_text(&project.repo_root),
        "project_home": path_text(&project.project_home),
        "state_db_path": path_text(&project.state_db_path),
        "status": &project.status,
    })
}

fn project_metadata_json() -> Result<String, ProjectCommandError> {
    serde_json::to_string(&json!({ "created_by": PROJECT_METADATA_CREATED_BY }))
        .map_err(|error| ProjectCommandError::runtime(error.to_string()))
}

fn absolute_path(current_dir: &Path, path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        current_dir.join(path)
    }
}

fn path_text(path: &Path) -> String {
    path.display().to_string()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use volicord_platform_fs::{
        RuntimeHomeMutationLease, RuntimeHomeMutationLeaseMode, RuntimeHomeMutationLeaseOutcome,
        RuntimeHomeMutationWaitPolicy,
    };
    use volicord_store::bootstrap::{
        initialize_runtime_home, project_record_by_repo_root, register_project,
        ProjectRegistration, ACTIVE_PROJECT_STATUS,
    };
    use volicord_test_support::{with_test_runtime_home_setup, TempRuntimeHome};

    use super::*;

    fn record(name: &str, repo_root: impl Into<PathBuf>) -> ProjectRecord {
        let repo_root = repo_root.into();
        ProjectRecord {
            project_internal_id: format!("internal-{name}"),
            project_id: format!("project-{name}"),
            project_name: name.to_owned(),
            project_alias: format!("alias-{name}"),
            runtime_home_id: "runtime-home".to_owned(),
            project_home: repo_root.join(".volicord"),
            state_db_path: repo_root.join(".volicord/state.sqlite"),
            repo_root,
            status: ACTIVE_PROJECT_STATUS.to_owned(),
            metadata_json: "{}".to_owned(),
        }
    }

    fn assert_one_trailing_newline(output: &str) {
        assert!(output.ends_with('\n'));
        assert!(!output.ends_with("\n\n"));
    }

    #[test]
    fn current_human_and_json_project_the_same_registered_record() {
        let project = record("Sandbox_Project", "/path/to/Sandbox_Project");
        let human = render_current_output(OutputFormat::Text, Some(&project), &project.repo_root)
            .expect("human rendering");
        let machine = render_current_output(OutputFormat::Json, Some(&project), &project.repo_root)
            .expect("JSON rendering");
        let machine: Value = serde_json::from_str(&machine).expect("valid JSON");

        assert_eq!(
            human,
            concat!(
                "Current project\n",
                "\n",
                "Name: Sandbox_Project\n",
                "Repository: /path/to/Sandbox_Project\n",
                "Status: active\n",
            )
        );
        assert_eq!(machine["project"]["project_name"], project.project_name);
        assert_eq!(
            machine["project"]["repo_root"],
            path_text(&project.repo_root)
        );
        assert_eq!(machine["project"]["status"], project.status);
        assert_eq!(
            machine["project"]["project_internal_id"],
            project.project_internal_id
        );
        assert!(!human.contains("internal-Sandbox_Project"));
        assert!(!human.contains('\t'));
        assert_one_trailing_newline(&human);
    }

    #[test]
    fn current_unregistered_has_one_conclusion_path_and_action() {
        let repo_root = Path::new("/path/to/unregistered-repository");
        let human =
            render_current_output(OutputFormat::Text, None, repo_root).expect("human rendering");
        let machine =
            render_current_output(OutputFormat::Json, None, repo_root).expect("JSON rendering");
        let machine: Value = serde_json::from_str(&machine).expect("valid JSON");

        assert_eq!(
            human,
            concat!(
                "Repository is not registered.\n",
                "\n",
                "Repository: /path/to/unregistered-repository\n",
                "Next action: Run `volicord project use`.\n",
            )
        );
        assert_eq!(human.matches("Next action:").count(), 1);
        assert_eq!(machine["status"], "not_registered");
        assert_eq!(machine["repo_root"], path_text(repo_root));
        assert_eq!(machine["action"], "volicord project use");
        assert!(!human.contains('\t'));
        assert_one_trailing_newline(&human);
    }

    #[test]
    fn list_human_handles_one_and_several_different_length_records_in_input_order() {
        let long_root = format!(
            "/workspace/{}/Long_Project",
            "complete-long-directory-segment/".repeat(12)
        );
        let one = record("Long_Project", &long_root);
        let single = render_list_output(OutputFormat::Text, std::slice::from_ref(&one))
            .expect("single project rendering");
        assert_eq!(
            single,
            format!("Projects (1)\n\nLong_Project\n  Status: active\n  Repository: {long_root}\n")
        );
        assert!(single.contains(&long_root));

        let projects = vec![
            record("A", "/repos/a"),
            record("Medium_Project", "/repos/medium"),
            record("Very_Long_Project_Name", "/repos/very-long"),
        ];
        let human =
            render_list_output(OutputFormat::Text, &projects).expect("project list rendering");
        let machine =
            render_list_output(OutputFormat::Json, &projects).expect("JSON list rendering");
        let machine: Value = serde_json::from_str(&machine).expect("valid JSON");

        assert_eq!(
            human,
            concat!(
                "Projects (3)\n",
                "\n",
                "A\n",
                "  Status: active\n",
                "  Repository: /repos/a\n",
                "\n",
                "Medium_Project\n",
                "  Status: active\n",
                "  Repository: /repos/medium\n",
                "\n",
                "Very_Long_Project_Name\n",
                "  Status: active\n",
                "  Repository: /repos/very-long\n",
            )
        );
        let positions = projects
            .iter()
            .map(|project| human.find(&project.project_name).expect("project name"))
            .collect::<Vec<_>>();
        assert!(positions.windows(2).all(|window| window[0] < window[1]));
        for (index, project) in projects.iter().enumerate() {
            assert_eq!(
                machine["projects"][index]["project_internal_id"],
                project.project_internal_id
            );
            assert_eq!(
                machine["projects"][index]["project_name"],
                project.project_name
            );
            assert_eq!(
                machine["projects"][index]["repo_root"],
                path_text(&project.repo_root)
            );
            assert!(!human.contains(&project.project_internal_id));
        }
        assert!(!human.contains('\t'));
        assert_one_trailing_newline(&human);
        assert_one_trailing_newline(&single);
    }

    #[test]
    fn empty_list_is_a_clear_sentence_and_lossless_empty_json_collection() {
        let human = render_list_output(OutputFormat::Text, &[]).expect("empty list rendering");
        let machine = render_list_output(OutputFormat::Json, &[]).expect("empty JSON list");
        let machine: Value = serde_json::from_str(&machine).expect("valid JSON");

        assert_eq!(human, "No projects are registered.\n");
        assert_eq!(
            machine["projects"].as_array().map(Vec::len),
            Some(0),
            "JSON must keep the real empty collection"
        );
        assert!(!human.contains('\t'));
        assert_one_trailing_newline(&human);
    }

    #[test]
    fn project_writers_are_typed_no_effect_while_setup_is_exclusive_and_resume_after_release(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("cli-project-setup-busy")?;
        let existing_repo = fixture.create_product_repo("existing-repo")?;
        let candidate_repo = fixture.create_product_repo("candidate-repo")?;
        fs::create_dir(candidate_repo.join(".git"))?;
        with_test_runtime_home_setup(fixture.path(), |context| {
            initialize_runtime_home(context, "runtime_home_cli_project_busy", "{}")?;
            register_project(
                context,
                ProjectRegistration {
                    project_id: "project_existing".to_owned(),
                    repo_root: existing_repo.clone(),
                    project_home: None,
                    status: ACTIVE_PROJECT_STATUS.to_owned(),
                    metadata_json: "{}".to_owned(),
                },
            )?;
            Ok(())
        })?;
        let registry_before = fs::read(fixture.registry_db_path())?;
        let outcome = RuntimeHomeMutationLease::acquire(
            fixture.path(),
            RuntimeHomeMutationLeaseMode::ExclusiveSetup,
            RuntimeHomeMutationWaitPolicy::Immediate,
        )?;
        let RuntimeHomeMutationLeaseOutcome::Acquired(exclusive) = outcome else {
            panic!("test setup must acquire ExclusiveSetup");
        };
        let env = |name: &str| (name == "VOLICORD_HOME").then(|| OsString::from(fixture.path()));

        let cases = [
            (
                ProjectCommand::Use(ProjectUseArgs {
                    path: Some(candidate_repo.clone()),
                    json: true,
                }),
                "cli.project.use",
            ),
            (
                ProjectCommand::Rename(ProjectRenameArgs {
                    name: "Renamed While Busy".to_owned(),
                    repo: Some(existing_repo.clone()),
                    json: true,
                }),
                "cli.project.rename",
            ),
            (
                ProjectCommand::Forget(ProjectForgetArgs {
                    selector: Some("project_existing".to_owned()),
                    json: true,
                }),
                "cli.project.forget",
            ),
        ];
        for (command, expected_domain) in cases {
            let error = run_project_command(ProjectArgs { command }, env, &existing_repo)
                .expect_err("project mutation must be rejected while setup is exclusive");
            let ProjectCommandError::MutationAdmission(CliMutationAdmissionError::SetupInProgress(
                condition,
            )) = error
            else {
                panic!("project mutation must return the typed setup condition: {error}");
            };
            assert_eq!(condition.code(), "runtime_home.mutation.setup_in_progress");
            assert_eq!(condition.mutation_domain(), expected_domain);
            assert!(condition.retryable());
            assert_eq!(fs::read(fixture.registry_db_path())?, registry_before);
        }
        assert!(project_record_by_repo_root(fixture.path(), &candidate_repo)?.is_none());
        let existing = project_record_by_repo_root(fixture.path(), &existing_repo)?
            .expect("busy project rename/forget must preserve the project");
        assert_eq!(existing.project_name, "project_existing");
        drop(exclusive);

        run_project_command(
            ProjectArgs {
                command: ProjectCommand::Use(ProjectUseArgs {
                    path: Some(candidate_repo.clone()),
                    json: true,
                }),
            },
            env,
            &candidate_repo,
        )?;
        assert!(project_record_by_repo_root(fixture.path(), &candidate_repo)?.is_some());
        Ok(())
    }
}
