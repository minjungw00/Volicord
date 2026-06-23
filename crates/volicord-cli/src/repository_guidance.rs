use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
};

use serde_json::json;
use sha2::{Digest, Sha256};

use crate::{
    guidance_template::{claude_code_guidance_body, codex_guidance_body},
    host_integration::{
        config_edit::{read_snapshot, remove_file_if_fresh, write_if_fresh, FileSnapshot},
        HostConfigError, HostConflict, HostConflictKind, PlannedChange,
    },
};

const BEGIN_MARKER: &str = "<!-- BEGIN HARNESS MANAGED GUIDANCE v1 -->";
const END_MARKER: &str = "<!-- END HARNESS MANAGED GUIDANCE v1 -->";
const CODEX_TARGET: &str = "codex";
const CLAUDE_CODE_TARGET: &str = "claude_code";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GuidanceTarget {
    Codex,
    ClaudeCode,
}

impl GuidanceTarget {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Codex => CODEX_TARGET,
            Self::ClaudeCode => CLAUDE_CODE_TARGET,
        }
    }

    pub fn path(self, repo_root: &Path) -> PathBuf {
        match self {
            Self::Codex => repo_root.join("AGENTS.md"),
            Self::ClaudeCode => repo_root.join(".claude").join("rules").join("harness.md"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GuidanceStateKind {
    Absent,
    Present,
    Changed,
    Conflicted,
}

impl GuidanceStateKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Absent => "absent",
            Self::Present => "present",
            Self::Changed => "changed",
            Self::Conflicted => "conflicted",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuidanceStatus {
    pub target: GuidanceTarget,
    pub integration_id: String,
    pub project_id: String,
    pub path: PathBuf,
    pub state: GuidanceStateKind,
    pub fingerprint: Option<String>,
    pub detail: String,
    pub planned_content: Option<String>,
}

impl GuidanceStatus {
    fn new(
        target: GuidanceTarget,
        integration_id: &str,
        project_id: &str,
        path: PathBuf,
        state: GuidanceStateKind,
        fingerprint: Option<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            target,
            integration_id: integration_id.to_owned(),
            project_id: project_id.to_owned(),
            path,
            state,
            fingerprint,
            detail: detail.into(),
            planned_content: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GuidancePlan {
    pub target: GuidanceTarget,
    pub integration_id: String,
    pub project_id: String,
    pub path: PathBuf,
    pub change: PlannedChange,
    pub status: GuidanceStatus,
    snapshot: FileSnapshot,
    next_content: Option<Vec<u8>>,
    cleanup_dirs: Vec<PathBuf>,
    new_guidance: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuidanceEffect {
    pub target: GuidanceTarget,
    pub integration_id: String,
    pub project_id: String,
    pub path: PathBuf,
    pub change: PlannedChange,
    pub fingerprint: Option<String>,
    pub residual: Option<String>,
    pub new_guidance: bool,
    pub(crate) prior_snapshot: FileSnapshot,
    pub(crate) applied_snapshot: Option<FileSnapshot>,
}

#[derive(Debug, Clone)]
struct ParsedBlock {
    span: std::ops::Range<usize>,
    block_text: String,
    body: String,
    metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
struct AnalyzedFile {
    status: GuidanceStatus,
    block: Option<ParsedBlock>,
}

pub fn guidance_status(
    repo_root: &Path,
    integration_id: &str,
    project_id: &str,
    target: GuidanceTarget,
) -> Result<GuidanceStatus, HostConfigError> {
    let repo_root = canonical_repo_root(repo_root)?;
    let path = checked_target_path(&repo_root, target)?;
    let snapshot = match read_snapshot(&path) {
        Ok(snapshot) => snapshot,
        Err(HostConfigError::Conflict(conflict)) => {
            return Ok(GuidanceStatus::new(
                target,
                integration_id,
                project_id,
                path,
                GuidanceStateKind::Conflicted,
                None,
                conflict.message,
            ));
        }
        Err(error) => return Err(error),
    };
    let desired_body = desired_body(target, integration_id, project_id);
    analyze_snapshot(
        target,
        integration_id,
        project_id,
        &path,
        &snapshot,
        &desired_body,
    )
    .map(|analysis| analysis.status)
}

pub fn plan_guidance_apply(
    repo_root: &Path,
    integration_id: &str,
    project_id: &str,
    target: GuidanceTarget,
) -> Result<GuidancePlan, HostConfigError> {
    let repo_root = canonical_repo_root(repo_root)?;
    let path = checked_target_path(&repo_root, target)?;
    let snapshot = read_snapshot(&path)?;
    let desired_body = desired_body(target, integration_id, project_id);
    let analysis = analyze_snapshot(
        target,
        integration_id,
        project_id,
        &path,
        &snapshot,
        &desired_body,
    )?;
    reject_unowned_state(&analysis.status)?;

    let cleanup_dirs = match target {
        GuidanceTarget::Codex => Vec::new(),
        GuidanceTarget::ClaudeCode => created_claude_dirs(&repo_root)?,
    };
    let cleanup_metadata = cleanup_dirs
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();
    let desired_block = render_managed_block(
        target,
        integration_id,
        project_id,
        &desired_body,
        &cleanup_metadata,
    );
    let new_guidance = analysis.block.is_none();
    let (next_text, planned_content) = match target {
        GuidanceTarget::Codex => (
            next_codex_text(&snapshot, analysis.block.as_ref(), &desired_block)?,
            desired_block,
        ),
        GuidanceTarget::ClaudeCode => {
            if let Some(block) = analysis.block.as_ref() {
                let existing_created_dirs = block
                    .metadata
                    .get("created_dirs")
                    .map(|value| split_metadata_list(value))
                    .unwrap_or_default();
                let desired_block = render_managed_block(
                    target,
                    integration_id,
                    project_id,
                    &desired_body,
                    &existing_created_dirs,
                );
                (Some(desired_block.clone()), desired_block)
            } else {
                (Some(desired_block.clone()), desired_block)
            }
        }
    };
    let current_text = snapshot_text(&snapshot, &path)?;
    let change = match (&snapshot, &next_text) {
        (FileSnapshot::Missing, Some(_)) => PlannedChange::Create,
        (FileSnapshot::Present { .. }, Some(next))
            if Some(next.as_bytes()) == current_text.as_deref().map(str::as_bytes) =>
        {
            PlannedChange::Noop
        }
        (FileSnapshot::Present { .. }, Some(_)) => PlannedChange::Update,
        _ => PlannedChange::Noop,
    };
    let mut status = analysis.status;
    status.planned_content = Some(planned_content);

    Ok(GuidancePlan {
        target,
        integration_id: integration_id.to_owned(),
        project_id: project_id.to_owned(),
        path,
        change,
        status,
        snapshot,
        next_content: next_text.map(String::into_bytes),
        cleanup_dirs,
        new_guidance,
    })
}

pub fn apply_guidance_plan(plan: &GuidancePlan) -> Result<GuidanceEffect, HostConfigError> {
    if matches!(plan.change, PlannedChange::Noop) {
        return Ok(effect_from_plan(plan, None, Some(plan.snapshot.clone())));
    }
    let content = plan.next_content.as_ref().ok_or_else(|| {
        HostConfigError::StalePlan("guidance apply plan is missing content".to_owned())
    })?;
    write_if_fresh(&plan.path, content, &plan.snapshot)?;
    let applied_snapshot = read_snapshot(&plan.path)?;
    Ok(effect_from_plan(plan, None, Some(applied_snapshot)))
}

pub fn plan_guidance_remove(
    repo_root: &Path,
    integration_id: &str,
    project_id: &str,
    target: GuidanceTarget,
) -> Result<GuidancePlan, HostConfigError> {
    let repo_root = canonical_repo_root(repo_root)?;
    let path = checked_target_path(&repo_root, target)?;
    let snapshot = read_snapshot(&path)?;
    let desired_body = desired_body(target, integration_id, project_id);
    let analysis = analyze_snapshot(
        target,
        integration_id,
        project_id,
        &path,
        &snapshot,
        &desired_body,
    )?;
    reject_unowned_state(&analysis.status)?;

    let (change, next_content, cleanup_dirs) = match (&snapshot, &analysis.block) {
        (FileSnapshot::Missing, _) | (_, None) => (PlannedChange::Noop, None, Vec::new()),
        (FileSnapshot::Present { .. }, Some(block)) => match target {
            GuidanceTarget::Codex => {
                let current = snapshot_text(&snapshot, &path)?.unwrap_or_default();
                let remaining = remove_span(&current, block.span.clone());
                if remaining.trim().is_empty() {
                    (PlannedChange::Remove, None, Vec::new())
                } else {
                    (
                        PlannedChange::Remove,
                        Some(remaining.into_bytes()),
                        Vec::new(),
                    )
                }
            }
            GuidanceTarget::ClaudeCode => {
                let dirs = block
                    .metadata
                    .get("created_dirs")
                    .map(|value| {
                        split_metadata_list(value)
                            .into_iter()
                            .map(PathBuf::from)
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                (PlannedChange::Remove, None, dirs)
            }
        },
    };

    let mut status = analysis.status;
    status.planned_content = analysis
        .block
        .as_ref()
        .map(|block| block.block_text.clone());

    Ok(GuidancePlan {
        target,
        integration_id: integration_id.to_owned(),
        project_id: project_id.to_owned(),
        path,
        change,
        status,
        snapshot,
        next_content,
        cleanup_dirs,
        new_guidance: false,
    })
}

pub fn apply_guidance_remove(plan: &GuidancePlan) -> Result<GuidanceEffect, HostConfigError> {
    if matches!(plan.change, PlannedChange::Noop) {
        return Ok(effect_from_plan(plan, None, Some(plan.snapshot.clone())));
    }
    match &plan.next_content {
        Some(content) => write_if_fresh(&plan.path, content, &plan.snapshot)?,
        None => remove_file_if_fresh(&plan.path, &plan.snapshot)?,
    }
    cleanup_empty_created_dirs(&plan.path, &plan.cleanup_dirs)?;
    let applied_snapshot = read_snapshot(&plan.path)?;
    Ok(effect_from_plan(plan, None, Some(applied_snapshot)))
}

pub fn compensate_new_guidance(effect: &GuidanceEffect) -> Result<GuidanceEffect, HostConfigError> {
    if !effect.new_guidance {
        return Ok(GuidanceEffect {
            residual: Some(
                "pre-existing managed guidance was left in place during compensation".to_owned(),
            ),
            ..effect.clone()
        });
    }
    let repo_root = effect
        .path
        .parent()
        .and_then(|parent| match effect.target {
            GuidanceTarget::Codex => Some(parent.to_path_buf()),
            GuidanceTarget::ClaudeCode => parent.parent()?.parent().map(Path::to_path_buf),
        })
        .ok_or_else(|| {
            HostConfigError::StalePlan("guidance target has no repository root".to_owned())
        })?;
    let remove = plan_guidance_remove(
        &repo_root,
        &effect.integration_id,
        &effect.project_id,
        effect.target,
    )?;
    apply_guidance_remove(&remove)
}

pub fn compensate_guidance_effect(
    effect: &GuidanceEffect,
) -> Result<GuidanceEffect, HostConfigError> {
    if matches!(effect.change, PlannedChange::Noop) {
        return Ok(effect.clone());
    }
    if effect.new_guidance {
        return compensate_new_guidance(effect);
    }
    let Some(applied_snapshot) = &effect.applied_snapshot else {
        return Ok(GuidanceEffect {
            residual: Some(
                "guidance could not be restored because its applied snapshot was unavailable"
                    .to_owned(),
            ),
            ..effect.clone()
        });
    };
    match &effect.prior_snapshot {
        FileSnapshot::Missing => remove_file_if_fresh(&effect.path, applied_snapshot)?,
        FileSnapshot::Present { bytes } => write_if_fresh(&effect.path, bytes, applied_snapshot)?,
    }
    Ok(GuidanceEffect {
        residual: None,
        ..effect.clone()
    })
}

fn analyze_snapshot(
    target: GuidanceTarget,
    integration_id: &str,
    project_id: &str,
    path: &Path,
    snapshot: &FileSnapshot,
    desired_body: &str,
) -> Result<AnalyzedFile, HostConfigError> {
    let Some(text) = snapshot_text(snapshot, path)? else {
        return Ok(AnalyzedFile {
            status: GuidanceStatus::new(
                target,
                integration_id,
                project_id,
                path.to_path_buf(),
                GuidanceStateKind::Absent,
                None,
                "managed guidance is absent",
            ),
            block: None,
        });
    };

    let parsed = match parse_managed_block(&text) {
        Ok(Some(block)) => block,
        Ok(None) if target == GuidanceTarget::Codex => {
            return Ok(AnalyzedFile {
                status: GuidanceStatus::new(
                    target,
                    integration_id,
                    project_id,
                    path.to_path_buf(),
                    GuidanceStateKind::Absent,
                    None,
                    "managed guidance is absent",
                ),
                block: None,
            });
        }
        Ok(None) => {
            return Ok(conflicted_analysis(
                target,
                integration_id,
                project_id,
                path,
                "Claude Code guidance file exists but is not Harness-managed",
            ));
        }
        Err(message) => {
            return Ok(conflicted_analysis(
                target,
                integration_id,
                project_id,
                path,
                message,
            ));
        }
    };

    if target == GuidanceTarget::ClaudeCode {
        let outside = format!("{}{}", &text[..parsed.span.start], &text[parsed.span.end..]);
        if !outside.trim().is_empty() {
            return Ok(conflicted_analysis(
                target,
                integration_id,
                project_id,
                path,
                "Claude Code guidance file contains unmanaged content outside the Harness block",
            ));
        }
    }

    let metadata_target = parsed.metadata.get("target").map(String::as_str);
    let metadata_integration = parsed.metadata.get("integration_id").map(String::as_str);
    let metadata_project = parsed.metadata.get("project_id").map(String::as_str);
    let metadata_fingerprint = parsed.metadata.get("fingerprint").map(String::as_str);
    if metadata_target != Some(target.as_str())
        || metadata_integration != Some(integration_id)
        || metadata_project != Some(project_id)
    {
        return Ok(conflicted_analysis(
            target,
            integration_id,
            project_id,
            path,
            "managed guidance marker belongs to another target, integration, or project",
        ));
    }
    let Some(stored_fingerprint) = metadata_fingerprint else {
        return Ok(conflicted_analysis(
            target,
            integration_id,
            project_id,
            path,
            "managed guidance marker is missing a fingerprint",
        ));
    };
    let current_fingerprint =
        guidance_fingerprint(target, integration_id, project_id, &parsed.body);
    let state = if stored_fingerprint == current_fingerprint {
        GuidanceStateKind::Present
    } else {
        GuidanceStateKind::Changed
    };
    let detail = if state == GuidanceStateKind::Changed {
        "conflict: managed guidance body does not match its fingerprint"
    } else if parsed.body == desired_body {
        "managed guidance is current"
    } else {
        "managed guidance is owned and can be updated"
    };
    Ok(AnalyzedFile {
        status: GuidanceStatus::new(
            target,
            integration_id,
            project_id,
            path.to_path_buf(),
            state,
            Some(current_fingerprint),
            detail,
        ),
        block: Some(parsed),
    })
}

fn parse_managed_block(text: &str) -> Result<Option<ParsedBlock>, String> {
    let begins = text.match_indices(BEGIN_MARKER).collect::<Vec<_>>();
    let ends = text.match_indices(END_MARKER).collect::<Vec<_>>();
    match (begins.len(), ends.len()) {
        (0, 0) => return Ok(None),
        (1, 1) => (),
        (0, _) | (_, 0) => return Err("malformed Harness guidance markers".to_owned()),
        _ => return Err("duplicate or nested Harness guidance markers".to_owned()),
    }
    let begin = begins[0].0;
    let end_marker_start = ends[0].0;
    if begin > end_marker_start {
        return Err("malformed Harness guidance marker order".to_owned());
    }
    let end_marker_end = end_marker_start + END_MARKER.len();
    let span_end = if text[end_marker_end..].starts_with("\r\n") {
        end_marker_end + 2
    } else if text[end_marker_end..].starts_with('\n') {
        end_marker_end + 1
    } else {
        end_marker_end
    };
    let block_text = text[begin..span_end].to_owned();
    let mut inner = &text[begin + BEGIN_MARKER.len()..end_marker_start];
    if let Some(rest) = inner.strip_prefix("\r\n") {
        inner = rest;
    } else if let Some(rest) = inner.strip_prefix('\n') {
        inner = rest;
    }

    let mut metadata = BTreeMap::new();
    let mut body_start = 0usize;
    for line in inner.split_inclusive('\n') {
        let line_without_newline = line.trim_end_matches('\n').trim_end_matches('\r');
        if let Some((key, value)) = parse_metadata_comment(line_without_newline) {
            if metadata.insert(key.to_owned(), value.to_owned()).is_some() {
                return Err(format!("duplicate Harness guidance metadata: {key}"));
            }
            body_start += line.len();
        } else {
            break;
        }
    }
    if metadata.is_empty() {
        return Err("managed guidance marker is missing metadata".to_owned());
    }
    let body = inner[body_start..].to_owned();
    Ok(Some(ParsedBlock {
        span: begin..span_end,
        block_text,
        body,
        metadata,
    }))
}

fn parse_metadata_comment(line: &str) -> Option<(&str, &str)> {
    let inner = line.strip_prefix("<!-- ")?.strip_suffix(" -->")?;
    inner.split_once(": ")
}

fn render_managed_block(
    target: GuidanceTarget,
    integration_id: &str,
    project_id: &str,
    body: &str,
    created_dirs: &[String],
) -> String {
    let body = ensure_final_newline(body);
    let fingerprint = guidance_fingerprint(target, integration_id, project_id, &body);
    let mut block = String::new();
    block.push_str(BEGIN_MARKER);
    block.push('\n');
    block.push_str(&format!("<!-- integration_id: {integration_id} -->\n"));
    block.push_str(&format!("<!-- project_id: {project_id} -->\n"));
    block.push_str(&format!("<!-- target: {} -->\n", target.as_str()));
    if !created_dirs.is_empty() {
        block.push_str(&format!(
            "<!-- created_dirs: {} -->\n",
            created_dirs.join(",")
        ));
    }
    block.push_str(&format!("<!-- fingerprint: {fingerprint} -->\n"));
    block.push_str(&body);
    block.push_str(END_MARKER);
    block.push('\n');
    block
}

fn guidance_fingerprint(
    target: GuidanceTarget,
    integration_id: &str,
    project_id: &str,
    body: &str,
) -> String {
    let payload = json!({
        "format": "harness-repository-guidance-v1",
        "target": target.as_str(),
        "integration_id": integration_id,
        "project_id": project_id,
        "body": body,
    });
    let bytes = serde_json::to_vec(&payload).expect("guidance fingerprint should serialize");
    let digest = Sha256::digest(bytes);
    let mut text = String::with_capacity(64);
    for byte in digest {
        text.push_str(&format!("{byte:02x}"));
    }
    text
}

fn next_codex_text(
    snapshot: &FileSnapshot,
    block: Option<&ParsedBlock>,
    desired_block: &str,
) -> Result<Option<String>, HostConfigError> {
    let current = snapshot_text(snapshot, Path::new("AGENTS.md"))?.unwrap_or_default();
    match block {
        None => Ok(Some(append_block(&current, desired_block))),
        Some(block) if block.block_text == desired_block => Ok(Some(current)),
        Some(block) => {
            let mut next = String::new();
            next.push_str(&current[..block.span.start]);
            next.push_str(desired_block);
            next.push_str(&current[block.span.end..]);
            Ok(Some(next))
        }
    }
}

fn append_block(current: &str, block: &str) -> String {
    if current.is_empty() {
        return block.to_owned();
    }
    let mut next = current.to_owned();
    if next.ends_with("\n\n") {
        // Already separated.
    } else if next.ends_with('\n') {
        next.push('\n');
    } else {
        next.push_str("\n\n");
    }
    next.push_str(block);
    next
}

fn remove_span(text: &str, span: std::ops::Range<usize>) -> String {
    let mut next = String::new();
    next.push_str(&text[..span.start]);
    next.push_str(&text[span.end..]);
    next
}

fn snapshot_text(snapshot: &FileSnapshot, path: &Path) -> Result<Option<String>, HostConfigError> {
    match snapshot {
        FileSnapshot::Missing => Ok(None),
        FileSnapshot::Present { bytes } => {
            String::from_utf8(bytes.clone()).map(Some).map_err(|error| {
                HostConfigError::Malformed(format!(
                    "guidance target is not UTF-8 text: {}: {error}",
                    path.display()
                ))
            })
        }
    }
}

fn desired_body(target: GuidanceTarget, integration_id: &str, project_id: &str) -> String {
    match target {
        GuidanceTarget::Codex => codex_guidance_body(integration_id, project_id),
        GuidanceTarget::ClaudeCode => claude_code_guidance_body(integration_id, project_id),
    }
}

fn reject_unowned_state(status: &GuidanceStatus) -> Result<(), HostConfigError> {
    match status.state {
        GuidanceStateKind::Absent | GuidanceStateKind::Present => Ok(()),
        GuidanceStateKind::Changed => Err(HostConfigError::Conflict(HostConflict::new(
            HostConflictKind::FingerprintMismatch,
            format!(
                "managed repository guidance changed since Harness last managed it: {}",
                status.path.display()
            ),
        ))),
        GuidanceStateKind::Conflicted => Err(HostConfigError::Conflict(HostConflict::new(
            HostConflictKind::UnmanagedNameCollision,
            status.detail.clone(),
        ))),
    }
}

fn conflicted_analysis(
    target: GuidanceTarget,
    integration_id: &str,
    project_id: &str,
    path: &Path,
    detail: impl Into<String>,
) -> AnalyzedFile {
    AnalyzedFile {
        status: GuidanceStatus::new(
            target,
            integration_id,
            project_id,
            path.to_path_buf(),
            GuidanceStateKind::Conflicted,
            None,
            detail,
        ),
        block: None,
    }
}

fn canonical_repo_root(repo_root: &Path) -> Result<PathBuf, HostConfigError> {
    let path = fs::canonicalize(repo_root).map_err(|error| {
        HostConfigError::Io(format!(
            "Product Repository root is not accessible: {}: {error}",
            repo_root.display()
        ))
    })?;
    if path.is_dir() {
        Ok(path)
    } else {
        Err(HostConfigError::Conflict(HostConflict::new(
            HostConflictKind::UnsafeTarget,
            format!(
                "Product Repository root must be a directory: {}",
                path.display()
            ),
        )))
    }
}

fn checked_target_path(
    repo_root: &Path,
    target: GuidanceTarget,
) -> Result<PathBuf, HostConfigError> {
    let path = target.path(repo_root);
    if !path.starts_with(repo_root) {
        return Err(HostConfigError::Conflict(HostConflict::new(
            HostConflictKind::UnsafeTarget,
            format!(
                "guidance target escapes Product Repository: {}",
                path.display()
            ),
        )));
    }
    Ok(path)
}

fn created_claude_dirs(repo_root: &Path) -> Result<Vec<PathBuf>, HostConfigError> {
    let mut created = Vec::new();
    for relative in [PathBuf::from(".claude"), PathBuf::from(".claude/rules")] {
        let path = repo_root.join(&relative);
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_dir() && !metadata.file_type().is_symlink() => {
            }
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(HostConfigError::Conflict(HostConflict::new(
                    HostConflictKind::UnsafeTarget,
                    format!(
                        "Claude Code guidance directory must not be a symlink: {}",
                        path.display()
                    ),
                )));
            }
            Ok(_) => {
                return Err(HostConfigError::Conflict(HostConflict::new(
                    HostConflictKind::UnsafeTarget,
                    format!(
                        "Claude Code guidance parent is not a directory: {}",
                        path.display()
                    ),
                )));
            }
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
                ) =>
            {
                created.push(relative);
            }
            Err(error) => {
                return Err(HostConfigError::Io(format!(
                    "failed to inspect Claude Code guidance directory {}: {error}",
                    path.display()
                )));
            }
        }
    }
    Ok(created)
}

fn cleanup_empty_created_dirs(
    target_path: &Path,
    relative_dirs: &[PathBuf],
) -> Result<(), HostConfigError> {
    let Some(repo_root) = target_path
        .parent()
        .and_then(|parent| parent.parent())
        .and_then(|parent| parent.parent())
    else {
        return Ok(());
    };
    for relative in relative_dirs.iter().rev() {
        let path = repo_root.join(relative);
        match fs::remove_dir(&path) {
            Ok(()) => {}
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::NotFound | io::ErrorKind::DirectoryNotEmpty
                ) => {}
            Err(error) => {
                return Err(HostConfigError::Io(format!(
                    "failed to remove empty guidance directory {}: {error}",
                    path.display()
                )));
            }
        }
    }
    Ok(())
}

fn split_metadata_list(value: &str) -> Vec<String> {
    value
        .split(',')
        .filter(|item| !item.trim().is_empty())
        .map(|item| item.trim().to_owned())
        .collect()
}

fn ensure_final_newline(text: &str) -> String {
    if text.ends_with('\n') {
        text.to_owned()
    } else {
        format!("{text}\n")
    }
}

fn effect_from_plan(
    plan: &GuidancePlan,
    residual: Option<String>,
    applied_snapshot: Option<FileSnapshot>,
) -> GuidanceEffect {
    GuidanceEffect {
        target: plan.target,
        integration_id: plan.integration_id.clone(),
        project_id: plan.project_id.clone(),
        path: plan.path.clone(),
        change: plan.change,
        fingerprint: plan.status.fingerprint.clone(),
        residual,
        new_guidance: plan.new_guidance,
        prior_snapshot: plan.snapshot.clone(),
        applied_snapshot,
    }
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn codex_missing_file_creation_and_safe_removal() -> Result<(), Box<dyn std::error::Error>> {
        let repo = temp_repo("guidance-codex-create")?;
        let plan =
            plan_guidance_apply(&repo, "agent_alpha", "project_alpha", GuidanceTarget::Codex)?;
        assert_eq!(plan.change, PlannedChange::Create);
        apply_guidance_plan(&plan)?;

        let agents = repo.join("AGENTS.md");
        let text = fs::read_to_string(&agents)?;
        assert!(text.contains(BEGIN_MARKER));
        assert!(text.contains("harness.list_projects"));
        let repeated =
            plan_guidance_apply(&repo, "agent_alpha", "project_alpha", GuidanceTarget::Codex)?;
        assert_eq!(repeated.change, PlannedChange::Noop);

        let remove =
            plan_guidance_remove(&repo, "agent_alpha", "project_alpha", GuidanceTarget::Codex)?;
        apply_guidance_remove(&remove)?;
        assert!(!agents.exists());
        Ok(())
    }

    #[test]
    fn codex_preserves_user_content_and_rejects_changed_block(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let repo = temp_repo("guidance-codex-preserve")?;
        let agents = repo.join("AGENTS.md");
        fs::write(&agents, "# Project rules\nKeep this.\n")?;
        let plan =
            plan_guidance_apply(&repo, "agent_alpha", "project_alpha", GuidanceTarget::Codex)?;
        apply_guidance_plan(&plan)?;
        let before = fs::read_to_string(&agents)?;
        assert!(before.starts_with("# Project rules\nKeep this.\n\n"));

        fs::write(
            &agents,
            before.replace("do not guess `project_id`", "guess project_id"),
        )?;
        let status = guidance_status(&repo, "agent_alpha", "project_alpha", GuidanceTarget::Codex)?;
        assert_eq!(status.state, GuidanceStateKind::Changed);
        assert!(matches!(
            plan_guidance_remove(&repo, "agent_alpha", "project_alpha", GuidanceTarget::Codex),
            Err(HostConfigError::Conflict(_))
        ));
        Ok(())
    }

    #[test]
    fn codex_removal_preserves_user_content() -> Result<(), Box<dyn std::error::Error>> {
        let repo = temp_repo("guidance-codex-remove-preserve")?;
        let agents = repo.join("AGENTS.md");
        fs::write(&agents, "# Project rules\nKeep this.\n")?;
        let plan =
            plan_guidance_apply(&repo, "agent_alpha", "project_alpha", GuidanceTarget::Codex)?;
        apply_guidance_plan(&plan)?;

        let remove =
            plan_guidance_remove(&repo, "agent_alpha", "project_alpha", GuidanceTarget::Codex)?;
        apply_guidance_remove(&remove)?;

        assert_eq!(
            fs::read_to_string(&agents)?,
            "# Project rules\nKeep this.\n\n"
        );
        Ok(())
    }

    #[test]
    fn codex_duplicate_markers_are_conflicted() -> Result<(), Box<dyn std::error::Error>> {
        let repo = temp_repo("guidance-codex-duplicate")?;
        let plan =
            plan_guidance_apply(&repo, "agent_alpha", "project_alpha", GuidanceTarget::Codex)?;
        apply_guidance_plan(&plan)?;
        let agents = repo.join("AGENTS.md");
        let text = fs::read_to_string(&agents)?;
        fs::write(&agents, format!("{text}\n{text}"))?;

        let status = guidance_status(&repo, "agent_alpha", "project_alpha", GuidanceTarget::Codex)?;
        assert_eq!(status.state, GuidanceStateKind::Conflicted);
        assert!(status.detail.contains("duplicate"));
        Ok(())
    }

    #[test]
    fn codex_malformed_marker_is_conflicted() -> Result<(), Box<dyn std::error::Error>> {
        let repo = temp_repo("guidance-codex-malformed")?;
        fs::write(
            repo.join("AGENTS.md"),
            format!("{BEGIN_MARKER}\nmissing end\n"),
        )?;

        let status = guidance_status(&repo, "agent_alpha", "project_alpha", GuidanceTarget::Codex)?;
        assert_eq!(status.state, GuidanceStateKind::Conflicted);
        assert!(status.detail.contains("malformed"));
        Ok(())
    }

    #[test]
    fn claude_file_conflicts_with_unmanaged_file_and_cleans_empty_dirs(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let repo = temp_repo("guidance-claude")?;
        let target = repo.join(".claude").join("rules").join("harness.md");
        fs::create_dir_all(target.parent().expect("target parent"))?;
        fs::write(&target, "# unmanaged\n")?;
        let status = guidance_status(
            &repo,
            "agent_alpha",
            "project_alpha",
            GuidanceTarget::ClaudeCode,
        )?;
        assert_eq!(status.state, GuidanceStateKind::Conflicted);
        assert!(matches!(
            plan_guidance_apply(
                &repo,
                "agent_alpha",
                "project_alpha",
                GuidanceTarget::ClaudeCode
            ),
            Err(HostConfigError::Conflict(_))
        ));

        fs::remove_file(&target)?;
        fs::remove_dir(target.parent().expect("target parent"))?;
        fs::remove_dir(repo.join(".claude"))?;
        let plan = plan_guidance_apply(
            &repo,
            "agent_alpha",
            "project_alpha",
            GuidanceTarget::ClaudeCode,
        )?;
        assert_eq!(plan.change, PlannedChange::Create);
        apply_guidance_plan(&plan)?;
        assert!(target.exists());
        let repeated = plan_guidance_apply(
            &repo,
            "agent_alpha",
            "project_alpha",
            GuidanceTarget::ClaudeCode,
        )?;
        assert_eq!(repeated.change, PlannedChange::Noop);

        let remove = plan_guidance_remove(
            &repo,
            "agent_alpha",
            "project_alpha",
            GuidanceTarget::ClaudeCode,
        )?;
        apply_guidance_remove(&remove)?;
        assert!(!target.exists());
        assert!(!repo.join(".claude").exists());
        Ok(())
    }

    #[test]
    fn claude_preserves_unrelated_rule_files() -> Result<(), Box<dyn std::error::Error>> {
        let repo = temp_repo("guidance-claude-unrelated")?;
        let rules = repo.join(".claude").join("rules");
        fs::create_dir_all(&rules)?;
        fs::write(rules.join("other.md"), "# Other\n")?;
        let plan = plan_guidance_apply(
            &repo,
            "agent_alpha",
            "project_alpha",
            GuidanceTarget::ClaudeCode,
        )?;
        apply_guidance_plan(&plan)?;
        let remove = plan_guidance_remove(
            &repo,
            "agent_alpha",
            "project_alpha",
            GuidanceTarget::ClaudeCode,
        )?;
        apply_guidance_remove(&remove)?;
        assert!(rules.join("other.md").exists());
        assert!(rules.exists());
        Ok(())
    }

    #[test]
    fn claude_changed_managed_file_is_conflict() -> Result<(), Box<dyn std::error::Error>> {
        let repo = temp_repo("guidance-claude-changed")?;
        let plan = plan_guidance_apply(
            &repo,
            "agent_alpha",
            "project_alpha",
            GuidanceTarget::ClaudeCode,
        )?;
        apply_guidance_plan(&plan)?;
        let target = repo.join(".claude").join("rules").join("harness.md");
        let before = fs::read_to_string(&target)?;
        fs::write(
            &target,
            before.replace("do not guess `project_id`", "guess project_id"),
        )?;

        let status = guidance_status(
            &repo,
            "agent_alpha",
            "project_alpha",
            GuidanceTarget::ClaudeCode,
        )?;
        assert_eq!(status.state, GuidanceStateKind::Changed);
        assert!(matches!(
            plan_guidance_apply(
                &repo,
                "agent_alpha",
                "project_alpha",
                GuidanceTarget::ClaudeCode
            ),
            Err(HostConfigError::Conflict(_))
        ));
        Ok(())
    }

    fn temp_repo(prefix: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let path = std::env::temp_dir().join(format!("{prefix}-{}-{stamp}", std::process::id()));
        fs::create_dir_all(&path)?;
        Ok(path)
    }
}
