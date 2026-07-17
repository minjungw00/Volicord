use std::{
    ffi::OsString,
    fmt, fs,
    path::{Path, PathBuf},
};

use serde_json::{json, Value};
use volicord_store::{
    bootstrap::{
        ensure_project_for_repo, forget_project, list_projects, project_record_by_repo_root,
        rename_project, ProjectRecord, RepoProjectRegistration, ACTIVE_PROJECT_STATUS,
    },
    runtime_home::{resolve_runtime_home, RuntimeHomeResolutionError},
    StoreError,
};

use crate::cli::{
    JsonArgs, ProjectArgs, ProjectCommand, ProjectForgetArgs, ProjectRenameArgs, ProjectUseArgs,
};

const PROJECT_METADATA_CREATED_BY: &str = "volicord_cli_project_command";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectCommandError {
    Usage(String),
    Runtime(String),
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
        }
    }
}

impl std::error::Error for ProjectCommandError {}

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
            command_use(options, &runtime_home, current_dir)
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
            command_rename(options, &runtime_home, current_dir)
        }
        ProjectCommand::Forget(options) => {
            let runtime_home = resolve_runtime_home(&env_var, current_dir)?;
            command_forget(options, &runtime_home, current_dir)
        }
    }
}

fn command_use(
    options: ProjectUseArgs,
    runtime_home: &Path,
    current_dir: &Path,
) -> Result<String, ProjectCommandError> {
    let repo_root = resolve_repository_root(current_dir, options.path.as_deref())?;
    let existing = project_record_by_repo_root(runtime_home, &repo_root)?;
    let created = existing.is_none();
    let project = match existing {
        Some(project) => project,
        None => ensure_project_for_repo(
            runtime_home,
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
    options: ProjectRenameArgs,
    runtime_home: &Path,
    current_dir: &Path,
) -> Result<String, ProjectCommandError> {
    let repo_root = resolve_repository_root(current_dir, options.repo.as_deref())?;
    let project = registered_project_for_repo(runtime_home, &repo_root)?;
    let project = rename_project(
        runtime_home,
        &project.project_internal_id,
        &options.name,
        None,
    )?;
    render_project_action_output(
        output_format(options.json),
        "renamed",
        "project renamed",
        &project,
    )
}

fn command_forget(
    options: ProjectForgetArgs,
    runtime_home: &Path,
    current_dir: &Path,
) -> Result<String, ProjectCommandError> {
    let project = match options.selector.as_ref() {
        Some(selector) if selector_is_path(selector, current_dir)? => {
            let repo_root = resolve_repository_root(current_dir, Some(Path::new(selector)))?;
            registered_project_for_repo(runtime_home, &repo_root)?
        }
        Some(name) => project_by_name(runtime_home, name)?,
        None => {
            let repo_root = resolve_repository_root(current_dir, None)?;
            registered_project_for_repo(runtime_home, &repo_root)?
        }
    };
    if !forget_project(runtime_home, &project.project_internal_id)? {
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
        (OutputFormat::Text, Some(project)) => Ok(format!(
            "project current\nname: {}\nrepo_root: {}\nstatus: {}\n",
            project.project_name,
            project.repo_root.display(),
            project.status
        )),
        (OutputFormat::Text, None) => Ok(format!(
            "project not registered\nrepo_root: {}\naction: run `volicord project use`\n",
            repo_root.display()
        )),
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
        OutputFormat::Text => {
            let mut text = String::from("name\trepo_root\tstatus\n");
            for project in projects {
                text.push_str(&format!(
                    "{}\t{}\t{}\n",
                    project.project_name,
                    project.repo_root.display(),
                    project.status
                ));
            }
            Ok(text)
        }
    }
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
