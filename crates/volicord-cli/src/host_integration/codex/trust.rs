use std::{
    fs,
    path::{Path, PathBuf},
};

use toml_edit::{Item, Table};

use crate::host_integration::verification::{ProjectTrustDiagnostic, ProjectTrustStatus};
use crate::host_integration::{HostPlan, HostTarget};

use super::{adapter::CodexEnvironment, config::parse_document};

pub(crate) fn project_trust_diagnostic(
    env: &CodexEnvironment,
    repo_root: &Path,
) -> ProjectTrustDiagnostic {
    let Some(config_path) = codex_user_config_path(env) else {
        return ProjectTrustDiagnostic {
            status: ProjectTrustStatus::Unknown,
            code: "project_trust_config_unavailable".to_owned(),
            config_path: String::new(),
            repo_root: repo_root.display().to_string(),
            details: "CODEX_HOME was not set and HOME was unavailable, so Codex user configuration could not be located".to_owned(),
        };
    };
    let config_path_text = config_path.display().to_string();
    let repo_root_text = repo_root.display().to_string();
    let text = match fs::read_to_string(&config_path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return ProjectTrustDiagnostic {
                status: ProjectTrustStatus::Missing,
                code: "project_trust_config_missing".to_owned(),
                config_path: config_path_text,
                repo_root: repo_root_text,
                details: "Codex user configuration file was not found".to_owned(),
            };
        }
        Err(_) => {
            return ProjectTrustDiagnostic {
                status: ProjectTrustStatus::Unreadable,
                code: "project_trust_config_unreadable".to_owned(),
                config_path: config_path_text,
                repo_root: repo_root_text,
                details: "Codex user configuration could not be read".to_owned(),
            };
        }
    };
    let document = match parse_document(Some(&text), &config_path) {
        Ok(document) => document,
        Err(_) => {
            return ProjectTrustDiagnostic {
                status: ProjectTrustStatus::Malformed,
                code: "project_trust_config_malformed".to_owned(),
                config_path: config_path_text,
                repo_root: repo_root_text,
                details: "Codex user configuration is malformed TOML".to_owned(),
            };
        }
    };
    let Some(projects) = document.get("projects").and_then(Item::as_table) else {
        return ProjectTrustDiagnostic {
            status: ProjectTrustStatus::Missing,
            code: "project_trust_entry_missing".to_owned(),
            config_path: config_path_text,
            repo_root: repo_root_text,
            details: "Codex user configuration has no matching projects table entry".to_owned(),
        };
    };
    let Some((_, project_item)) = matching_project_entry(projects, repo_root) else {
        return ProjectTrustDiagnostic {
            status: ProjectTrustStatus::Missing,
            code: "project_trust_entry_missing".to_owned(),
            config_path: config_path_text,
            repo_root: repo_root_text,
            details: "Codex user configuration has no matching project trust entry".to_owned(),
        };
    };
    let Some(table) = project_item.as_table() else {
        return ProjectTrustDiagnostic {
            status: ProjectTrustStatus::Malformed,
            code: "project_trust_entry_malformed".to_owned(),
            config_path: config_path_text,
            repo_root: repo_root_text,
            details: "Codex project trust entry is not a table".to_owned(),
        };
    };
    let trust_level = table.get("trust_level").and_then(Item::as_str);
    let (status, code) = match trust_level {
        Some("trusted") => (ProjectTrustStatus::Trusted, "project_trust_satisfied"),
        Some("untrusted") => (ProjectTrustStatus::Untrusted, "project_trust_required"),
        Some(_) | None => (ProjectTrustStatus::Malformed, "project_trust_value_invalid"),
    };
    let details = match status {
        ProjectTrustStatus::Trusted => "Codex user configuration marks the project trusted",
        ProjectTrustStatus::Untrusted => "Codex user configuration marks the project untrusted",
        ProjectTrustStatus::Unknown => {
            "Codex user configuration project entry does not contain a recognized trust_level"
        }
        ProjectTrustStatus::Missing
        | ProjectTrustStatus::Unreadable
        | ProjectTrustStatus::Malformed => {
            "Codex project trust could not be confirmed from user configuration"
        }
    };
    ProjectTrustDiagnostic {
        status,
        code: code.to_owned(),
        config_path: config_path_text,
        repo_root: repo_root_text,
        details: details.to_owned(),
    }
}

pub(super) fn project_trust_for_plan(
    env: &CodexEnvironment,
    plan: &HostPlan,
) -> ProjectTrustDiagnostic {
    let HostTarget::File(target) = &plan.target else {
        return ProjectTrustDiagnostic {
            status: ProjectTrustStatus::Unknown,
            code: "project_trust_target_invalid".to_owned(),
            config_path: String::new(),
            repo_root: String::new(),
            details: "Codex project trust could not be checked for a non-file target".to_owned(),
        };
    };
    let Some(repo_root) = target.parent().and_then(Path::parent) else {
        return ProjectTrustDiagnostic {
            status: ProjectTrustStatus::Unknown,
            code: "project_trust_repo_root_unavailable".to_owned(),
            config_path: String::new(),
            repo_root: String::new(),
            details: "Codex project trust could not be checked because the repository root was unavailable".to_owned(),
        };
    };
    project_trust_diagnostic(env, repo_root)
}

fn codex_user_config_path(env: &CodexEnvironment) -> Option<PathBuf> {
    env.codex_home
        .as_ref()
        .map(|path| path.join("config.toml"))
        .or_else(|| {
            env.home
                .as_ref()
                .map(|path| path.join(".codex/config.toml"))
        })
}

fn matching_project_entry<'a>(
    projects: &'a Table,
    repo_root: &Path,
) -> Option<(&'a str, &'a Item)> {
    projects
        .iter()
        .find(|(project_path, _)| project_path_matches(project_path, repo_root))
}

fn project_path_matches(project_path: &str, repo_root: &Path) -> bool {
    if !Path::new(project_path).is_absolute() || !repo_root.is_absolute() {
        return false;
    }
    let normalized_project_path = normalize_trailing_slashes(project_path);
    let repo_root_text = repo_root.display().to_string();
    let normalized_repo_root = normalize_trailing_slashes(&repo_root_text);
    if normalized_project_path == normalized_repo_root {
        return true;
    }
    let project_canonical = fs::canonicalize(project_path);
    let repo_canonical = fs::canonicalize(repo_root);
    matches!(
        (project_canonical, repo_canonical),
        (Ok(project), Ok(repo)) if project == repo
    )
}

fn normalize_trailing_slashes(path: &str) -> String {
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() {
        "/".to_owned()
    } else {
        trimmed.to_owned()
    }
}
