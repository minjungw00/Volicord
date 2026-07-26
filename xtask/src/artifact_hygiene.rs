use crate::diagnostics::ValidationIssue;
use std::path::Path;
use std::process::Command;

const IGNORE_OWNER_PATH: &str = ".gitignore";

pub(crate) fn validate_tracked_artifacts(root: &Path, issues: &mut Vec<ValidationIssue>) {
    if !root.join(".git").exists() || !root.join(IGNORE_OWNER_PATH).exists() {
        return;
    }

    let output = match Command::new("git")
        .current_dir(root)
        .args([
            "ls-files",
            "--cached",
            "--ignored",
            "--exclude-from=.gitignore",
            "-z",
        ])
        .output()
    {
        Ok(output) => output,
        Err(error) => {
            issues.push(ValidationIssue::new(
                IGNORE_OWNER_PATH,
                "artifact_hygiene.git_index",
                format!("failed to inspect tracked files against repository ignore rules: {error}"),
            ));
            return;
        }
    };

    if !output.status.success() {
        issues.push(ValidationIssue::new(
            IGNORE_OWNER_PATH,
            "artifact_hygiene.git_index",
            format!(
                "git could not inspect tracked files against repository ignore rules: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ),
        ));
        return;
    }

    for path in output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
    {
        let path = String::from_utf8_lossy(path).into_owned();
        issues.push(ValidationIssue::new(
            &path,
            "artifact_hygiene.tracked_ignored",
            format!(
                "tracked file matches repository artifact-exclusion rules owned by {IGNORE_OWNER_PATH}"
            ),
        ));
    }
}
