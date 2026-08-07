use crate::repository::normalize_existing_root;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::process::Command;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CiBaseResolution {
    pub event_name: String,
    pub base_revision: String,
    pub head_revision: String,
    pub changed_paths: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct EventPayload {
    #[serde(default)]
    before: Option<String>,
    #[serde(default)]
    pull_request: Option<PullRequestPayload>,
    #[serde(default)]
    inputs: Option<BTreeMap<String, String>>,
}

#[derive(Debug, Deserialize)]
struct PullRequestPayload {
    base: PullRequestBase,
}

#[derive(Debug, Deserialize)]
struct PullRequestBase {
    sha: String,
}

pub fn resolve_ci_base(
    root: &Path,
    event_name: &str,
    event_path: &Path,
    head: &str,
) -> Result<CiBaseResolution> {
    let root = normalize_existing_root(root)?;
    let event_contents = fs::read_to_string(event_path)
        .with_context(|| format!("failed to read CI event payload {}", event_path.display()))?;
    let event: EventPayload = serde_json::from_str(&event_contents)
        .with_context(|| format!("failed to parse CI event payload {}", event_path.display()))?;
    let selected = select_event_base(event_name, &event)?;
    let head_revision = resolve_commit(&root, head)
        .with_context(|| format!("CI head revision {head:?} is missing or unreachable"))?;
    let base_revision = resolve_commit(&root, selected)
        .with_context(|| format!("CI base revision {selected:?} is missing or unreachable"))?;

    if base_revision == head_revision {
        bail!(
            "CI base revision resolves to HEAD {}; refusing an empty HEAD..HEAD change range",
            head_revision
        );
    }
    ensure_ancestor(&root, &base_revision, &head_revision)?;
    let changed_paths = changed_paths(&root, &base_revision, &head_revision)?;
    if changed_paths.is_empty() {
        bail!(
            "CI base {} and head {} do not describe a nonempty changed-path series",
            base_revision,
            head_revision
        );
    }

    Ok(CiBaseResolution {
        event_name: event_name.to_owned(),
        base_revision,
        head_revision,
        changed_paths,
    })
}

pub fn append_github_output(path: &Path, resolution: &CiBaseResolution) -> Result<()> {
    let mut output = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("failed to open GitHub output file {}", path.display()))?;
    writeln!(output, "base={}", resolution.base_revision)
        .with_context(|| format!("failed to write GitHub output file {}", path.display()))
}

fn select_event_base<'a>(event_name: &str, event: &'a EventPayload) -> Result<&'a str> {
    let selected = match event_name {
        "pull_request" => event
            .pull_request
            .as_ref()
            .map(|pull_request| pull_request.base.sha.as_str())
            .context("pull_request event is missing pull_request.base.sha")?,
        "push" => {
            let before = event
                .before
                .as_deref()
                .context("push event is missing before")?;
            if !before.is_empty() && before.bytes().all(|byte| byte == b'0') {
                bail!("push event before is the zero object ID and cannot identify a change-series base");
            }
            before
        }
        "workflow_dispatch" => event
            .inputs
            .as_ref()
            .and_then(|inputs| inputs.get("base"))
            .map(String::as_str)
            .context("workflow_dispatch event is missing required inputs.base")?,
        other => bail!("unsupported CI event {other:?}"),
    };
    if selected.trim().is_empty() {
        bail!("CI event {event_name} selected an empty base revision");
    }
    Ok(selected.trim())
}

fn resolve_commit(root: &Path, revision: &str) -> Result<String> {
    let expression = format!("{revision}^{{commit}}");
    git_text(
        root,
        &["rev-parse", "--verify", "--end-of-options", &expression],
    )
}

fn ensure_ancestor(root: &Path, base: &str, head: &str) -> Result<()> {
    let output = Command::new("git")
        .current_dir(root)
        .args(["merge-base", "--is-ancestor", base, head])
        .output()
        .context("failed to execute git merge-base --is-ancestor")?;
    if output.status.success() {
        return Ok(());
    }
    if output.status.code() == Some(1) {
        bail!("CI base {base} is not an ancestor of head {head}");
    }
    bail!(
        "git merge-base --is-ancestor failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    )
}

fn changed_paths(root: &Path, base: &str, head: &str) -> Result<Vec<String>> {
    let output = git_output(root, &["diff", "--name-only", "-z", base, head, "--"])?;
    let mut paths = BTreeSet::new();
    for raw in output.split(|byte| *byte == 0) {
        if raw.is_empty() {
            continue;
        }
        let path = std::str::from_utf8(raw).context("Git returned a non-UTF-8 changed path")?;
        paths.insert(path.to_owned());
    }
    Ok(paths.into_iter().collect())
}

fn git_text(root: &Path, args: &[&str]) -> Result<String> {
    let output = git_output(root, args)?;
    Ok(std::str::from_utf8(&output)?.trim().to_owned())
}

fn git_output(root: &Path, args: &[&str]) -> Result<Vec<u8>> {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .with_context(|| format!("failed to execute git {}", args.join(" ")))?;
    if !output.status.success() {
        bail!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(output.stdout)
}
