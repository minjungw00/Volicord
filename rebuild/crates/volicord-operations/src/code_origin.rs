//! Explicit local verification of a retained command range against a pinned Git
//! blob. The annotation adds reading context, never a new Source or currentness.
use crate::{
    operations::{now_micros, parse_identity},
    Error, LocalOperations,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{ffi::OsString, fs, io::Read, path::Path, time::Duration};
use volicord_context::{CanonicalReadBasis, ContextItemId, ProjectId, SourceId, TimestampMicros};
use volicord_local_platform::CancellationFlag;
use volicord_privacy::{
    ManagedCanonicalLink, ManagedDerivedDraft, ManagedDerivedKind, ManagedDerivedState,
    PrivacyStore, ProjectPrivacyInspection,
};
use volicord_projections::{explanation_text_hash, ExplanationCitation, VerifiedCodeOrigin};
use volicord_repository_intelligence::{
    AnalysisSnapshotId, CanonicalGrounding, InventoryClassification,
};

const KIND: &str = "volicord_code_origin";
const VERSION: u32 = 1;
const FILE_LIMIT: usize = 16 * 1024 * 1024;

/// A path/revision is a verification request, never an authored display assertion.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CodeOriginRequest {
    pub field: String,
    pub start_byte: usize,
    pub end_byte: usize,
    pub path: String,
    pub revision: String,
    pub origin_start_byte: usize,
    pub origin_end_byte: usize,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Annotation {
    format_kind: String,
    format_version: u32,
    project: ProjectId,
    work: ContextItemId,
    source: SourceId,
    observation_fingerprint: String,
    field: String,
    start_byte: usize,
    end_byte: usize,
    range_sha256: String,
    origin: VerifiedCodeOrigin,
}

fn purpose(work: ContextItemId, source: SourceId, field: &str, start: usize, end: usize) -> String {
    format!("code-origin:{work}:{source}:{field}:{start}:{end}")
}

impl LocalOperations {
    /// Local CLI only: bounded, non-network Git object reads. No command is
    /// accepted from the Host, and no Source/Work/publication is modified.
    pub fn verify_work_code_origin(
        &self,
        project: ProjectId,
        work: ContextItemId,
        source: SourceId,
        request: CodeOriginRequest,
    ) -> Result<Value, Error> {
        let inspection_guard = self.layout().acquire_health_lock()?;
        validate_request(&request)?;
        let read = self.read_explanation_source(project, work, source)?;
        let observation = &read["observation"];
        if observation["detail"]["kind"] != "command" {
            return Err(Error::new(
                "code origin requires an exact retained command range",
            ));
        }
        let text = observation["detail"][&request.field]["body"]
            .as_str()
            .and_then(|s| s.get(request.start_byte..request.end_byte))
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::new("invalid retained command UTF-8 range"))?;
        let analysis = AnalysisSnapshotId::from_hex(
            observation["analysis_snapshot_id"]
                .as_str()
                .ok_or_else(|| Error::new("Source analysis unavailable"))?,
        )
        .map_err(Error::new)?;
        let snapshot = self.load_analysis_snapshot(project, analysis)?;
        if !snapshot.inventory.entries.iter().any(|e| {
            e.area.path == request.path
                && e.classifications
                    .contains(&InventoryClassification::Included)
        }) {
            return Err(Error::new(
                "origin path is outside the Source's included repository inventory",
            ));
        }
        let binding = self
            .open_canonical()?
            .get_local_binding(project)
            .map_err(|e| Error::with_source("origin repository binding unavailable", e))?;
        // No revision expressions, replacement objects, filters, shell, remote
        // fetch, or symlink/tree/submodule contents participate in verification.
        let root = volicord_local_platform::RepositoryRoot::open(&binding.absolute_path)
            .map_err(|e| Error::with_source("origin repository unavailable", e))?;
        if root.canonical_path() != binding.absolute_path {
            return Err(Error::new("origin repository binding was redirected"));
        }
        // Child supervision initializes the Runtime under its own mutation lock.
        // Revalidate the exact Source and binding after these transient reads.
        drop(inspection_guard);
        let git = |args: &[&str], limit| self.origin_git(root.canonical_path(), args, limit);
        if git(&["cat-file", "-t", &request.revision], 32)? != b"commit\n" {
            return Err(Error::new(
                "origin revision must name an existing commit object",
            ));
        }
        let tree = git(
            &["ls-tree", "-z", &request.revision, "--", &request.path],
            8192,
        )?;
        let entry =
            std::str::from_utf8(&tree).map_err(|_| Error::new("origin tree entry is not UTF-8"))?;
        if !(entry.starts_with("100644 blob ") || entry.starts_with("100755 blob "))
            || entry.split_once('\t').map(|(_, p)| p)
                != Some(format!("{}\0", request.path).as_str())
        {
            return Err(Error::new("origin must be one exact regular Git file"));
        }
        let object = format!("{}:{}", request.revision, request.path);
        let size = git(&["cat-file", "-s", &object], 32)?;
        let size: usize = std::str::from_utf8(&size)
            .ok()
            .and_then(|s| s.trim().parse().ok())
            .filter(|n| *n <= FILE_LIMIT)
            .ok_or_else(|| Error::new("origin file exceeds bounded verification size"))?;
        let bytes = git(&["cat-file", "blob", &object], size)?;
        let file =
            std::str::from_utf8(&bytes).map_err(|_| Error::new("origin file is not UTF-8"))?;
        if bytes.len() != size
            || file.get(request.origin_start_byte..request.origin_end_byte) != Some(text)
        {
            return Err(Error::new(
                "pinned origin range does not equal the exact retained command bytes",
            ));
        }
        let _inspection = self.layout().acquire_health_lock()?;
        if read != self.read_explanation_source(project, work, source)?
            || self
                .open_canonical()?
                .get_local_binding(project)
                .map_err(|e| Error::with_source("origin repository binding changed", e))?
                .absolute_path
                != binding.absolute_path
        {
            return Err(Error::new(
                "Source or repository binding changed during origin verification",
            ));
        }
        let basis = self.canonical_basis(project)?;
        let now = now_micros()?;
        let until = read["metadata"]["retained_until_unix_micros"]
            .as_i64()
            .filter(|t| *t > now.as_unix_micros())
            .ok_or_else(|| Error::new("Source expired during origin verification"))?;
        let line = |at| {
            1 + file.as_bytes()[..at]
                .iter()
                .filter(|b| **b == b'\n')
                .count()
        };
        let origin = VerifiedCodeOrigin {
            path: request.path,
            revision: request.revision,
            start_byte: request.origin_start_byte,
            end_byte: request.origin_end_byte,
            start_line: line(request.origin_start_byte),
            end_line: line(request.origin_end_byte - 1),
            file_sha256: explanation_text_hash(file),
            verified_at_unix_micros: now.as_unix_micros(),
        };
        let annotation = Annotation {
            format_kind: KIND.into(),
            format_version: VERSION,
            project,
            work,
            source,
            observation_fingerprint: read["observation_fingerprint"]
                .as_str()
                .ok_or_else(|| Error::new("Source fingerprint unavailable"))?
                .into(),
            field: request.field,
            start_byte: request.start_byte,
            end_byte: request.end_byte,
            range_sha256: explanation_text_hash(text),
            origin,
        };
        let grounding = CanonicalGrounding::from_read_basis(&basis)
            .map_err(|e| Error::with_source("origin grounding unavailable", e))?;
        let repository_source = SourceId::from_bytes(parse_identity(
            read["citation_basis"]["repository_source_id"]
                .as_str()
                .ok_or_else(|| Error::new("Source repository unavailable"))?,
        )?);
        let links = vec![
            ManagedCanonicalLink::ContextItem(work),
            ManagedCanonicalLink::Source(source),
            ManagedCanonicalLink::Source(repository_source),
        ];
        let included_sources = [source, repository_source]
            .into_iter()
            .map(|id| {
                grounding
                    .source_reference(id)
                    .map_err(|e| Error::with_source("origin Source unavailable", e))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let record = PrivacyStore::open(self.layout().privacy_store())
            .map_err(|e| Error::with_source("origin annotation store unavailable", e))?
            .record_managed_derived(ManagedDerivedDraft { project_id: project,
                kind: ManagedDerivedKind::CodeOrigin, provider: None, model: None,
                purpose: purpose(work, source, &annotation.field, annotation.start_byte, annotation.end_byte),
                analysis_snapshot: Some(analysis), included_sources, canonical_links: links,
                content: serde_json::to_string(&annotation).map_err(|e| Error::with_source("origin encoding failed", e))?,
                uncertainty: None, retained_until: Some(TimestampMicros::from_unix_micros(until)),
                retention_basis: "Explicit local pinned code origin verification; metadata only; capped by original Source deadline".into() })
            .map_err(|e| Error::with_source("origin annotation retention failed", e))?;
        Ok(
            json!({"annotation_id":record.id.to_string(),"source_id":source.to_string(),
            "field":annotation.field,"start_byte":annotation.start_byte,"end_byte":annotation.end_byte,
            "range_sha256":annotation.range_sha256,"origin":annotation.origin,
            "retained_until_unix_micros":until,"authority":"byte_equality_only_no_currentness_or_execution_assertion"}),
        )
    }

    fn origin_git(&self, root: &Path, args: &[&str], limit: usize) -> Result<Vec<u8>, Error> {
        let layout = volicord_local_platform::GitWorktreeLayout::resolve(root)
            .map_err(|e| Error::with_source("origin Git layout unavailable", e))?
            .ok_or_else(|| Error::new("origin requires a local Git repository"))?;
        let mut arguments: Vec<OsString> = [
            "-u",
            "GIT_DIR",
            "-u",
            "GIT_WORK_TREE",
            "-u",
            "GIT_COMMON_DIR",
            "-u",
            "GIT_OBJECT_DIRECTORY",
            "-u",
            "GIT_ALTERNATE_OBJECT_DIRECTORIES",
            "-u",
            "GIT_CONFIG_PARAMETERS",
            "GIT_CONFIG_COUNT=0",
            "GIT_ALLOW_PROTOCOL=",
            "GIT_NO_LAZY_FETCH=1",
            "GIT_NO_REPLACE_OBJECTS=1",
            "GIT_OPTIONAL_LOCKS=0",
            "GIT_TERMINAL_PROMPT=0",
            "git",
            "--no-pager",
            "--literal-pathspecs",
        ]
        .into_iter()
        .map(OsString::from)
        .collect();
        arguments.push(OsString::from("--git-dir"));
        arguments.push(layout.git_dir().as_os_str().to_owned());
        arguments.extend(args.iter().map(OsString::from));
        let result = self.run_child(
            "env",
            arguments,
            Some(root),
            vec!["explicit_pinned_code_origin".into()],
            Duration::from_secs(10),
            CancellationFlag::default(),
        )?;
        let dir = self
            .layout()
            .artifacts_dir()
            .join(result.operation_id.to_string());
        let bytes = (|| {
            let outcome = result
                .value
                .as_ref()
                .filter(|_| result.state == crate::OperationState::Succeeded)
                .ok_or_else(|| Error::new("local pinned Git object verification failed"))?;
            if outcome.stdout.bytes() > limit as u64 {
                return Err(Error::new("origin Git result exceeds verification bound"));
            }
            let mut bytes = Vec::new();
            fs::File::open(outcome.stdout.path())
                .and_then(|f| f.take(limit as u64 + 1).read_to_end(&mut bytes))
                .map_err(|e| Error::with_source("origin Git result unavailable", e))?;
            if bytes.len() > limit {
                return Err(Error::new(
                    "origin Git result grew beyond verification bound",
                ));
            }
            Ok(bytes)
        })();
        // The full historical file is transient verification input, never retained.
        fs::remove_dir_all(dir)
            .map_err(|e| Error::with_source("origin verification artifact cleanup failed", e))?;
        bytes
    }

    pub(crate) fn verified_code_origin(
        &self,
        basis: &CanonicalReadBasis,
        work: ContextItemId,
        citation: &ExplanationCitation,
        inspection: &ProjectPrivacyInspection,
    ) -> Option<VerifiedCodeOrigin> {
        let source = SourceId::from_bytes(parse_identity(&citation.source_id).ok()?);
        let target = purpose(
            work,
            source,
            &citation.field,
            citation.start_byte,
            citation.end_byte,
        );
        let record = inspection
            .managed_derived
            .iter()
            .rfind(|r| r.kind == ManagedDerivedKind::CodeOrigin && r.purpose == target)?;
        let now = now_micros().ok()?;
        if !matches!(
            record.state,
            ManagedDerivedState::Current | ManagedDerivedState::Stale
        ) || record.retained_until.is_none_or(|t| t <= now)
            || inspection
                .withheld_for_canonical_forgetting
                .contains(&record.id)
        {
            return None;
        }
        let stored = PrivacyStore::open(self.layout().privacy_store())
            .ok()?
            .get_derived(basis.project.id, record.id)
            .ok()?;
        let content = stored.content.as_deref().filter(|s| s.len() <= 8192)?;
        let annotation: Annotation = serde_json::from_str(content).ok()?;
        (annotation.format_kind == KIND
            && annotation.format_version == VERSION
            && annotation.project == basis.project.id
            && annotation.work == work
            && annotation.source == source
            && annotation.field == citation.field
            && annotation.start_byte == citation.start_byte
            && annotation.end_byte == citation.end_byte
            && annotation.range_sha256 == citation.range_sha256
            && annotation.observation_fingerprint == citation.observation_fingerprint)
            .then_some(annotation.origin)
    }
}

fn validate_request(r: &CodeOriginRequest) -> Result<(), Error> {
    if !matches!(r.field.as_str(), "stdout" | "stderr")
        || r.start_byte >= r.end_byte
        || r.end_byte > 8192
        || r.origin_start_byte >= r.origin_end_byte
        || r.origin_end_byte > FILE_LIMIT
        || !matches!(r.revision.len(), 40 | 64)
        || !r.revision.bytes().all(|b| b.is_ascii_hexdigit())
        || r.path.is_empty()
        || r.path.len() > 4096
        || r.path.starts_with('/')
        || r.path.contains(['\\', '\0', '\n', '\r'])
        || r.path
            .split('/')
            .any(|s| s.is_empty() || matches!(s, "." | ".." | ".git"))
    {
        return Err(Error::new("invalid bounded pinned code origin request"));
    }
    Ok(())
}
