//! Explicit, bounded local Source copies. Canonical Sources identify observations;
//! their disposable bodies use the existing privacy lifecycle, never canonical truth.
use crate::operations::{new_operation_id, now_micros};
use crate::{Error, LocalOperations};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{ffi::OsString, fs::File, io::Read, path::Path, time::Duration};
use volicord_context::{
    Availability, CanonicalReadBasis, ContextItemId, ContextItemRole, Principal, PrincipalKind,
    ProjectId, Source, SourceDraft, SourceId, SourcePayload, TimestampMicros,
};
use volicord_local_platform::{CancellationFlag, ProcessTermination, RepositoryRoot};
use volicord_privacy::{
    ManagedCanonicalLink, ManagedDerivedDraft, ManagedDerivedKind, ManagedDerivedRecord,
    ManagedDerivedState, PrivacyStore,
};
use volicord_repository_intelligence::{
    AnalysisSnapshotId, CanonicalGrounding, InventoryClassification,
};

const KIND: &str = "volicord_source_observation";
const VERSION: u32 = 1;
const FILE_BYTES: usize = 16_384;
const STREAM_BYTES: usize = 8_192;
const MAX_RETENTION_SECONDS: u64 = 7 * 24 * 60 * 60;

#[derive(Clone, Debug)]
pub struct FileObservationRequest {
    pub project_id: ProjectId,
    pub work_item_id: ContextItemId,
    pub analysis_snapshot_id: AnalysisSnapshotId,
    pub path: String,
    /// Explicit earlier observation, never inferred from path or chronology.
    pub before_source_id: Option<SourceId>,
    pub retain_seconds: u64,
}

/// Local caller explicitly authorizes this execution. No Host/background tool
/// accepts this request or arbitrary command execution on its behalf.
pub struct WorkCommandRequest {
    pub project_id: ProjectId,
    pub work_item_id: ContextItemId,
    pub analysis_snapshot_id: AnalysisSnapshotId,
    pub command_label: String,
    pub program: String,
    pub arguments: Vec<String>,
    pub timeout: Duration,
    pub retain_seconds: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct SourceObservationReceipt {
    pub project_id: ProjectId,
    pub work_item_id: ContextItemId,
    pub source_id: SourceId,
    pub source_revision: u64,
    pub observed_at_unix_micros: i64,
    pub retained_until_unix_micros: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SourceObservation {
    format_kind: String,
    format_version: u32,
    pub(crate) project_id: ProjectId,
    pub(crate) work_item_id: ContextItemId,
    work_revision: u64,
    pub(crate) source_id: SourceId,
    source_revision: u64,
    analysis_snapshot_id: AnalysisSnapshotId,
    repository_snapshot_id: String,
    repository_source_id: SourceId,
    observed_at_unix_micros: i64,
    pub(crate) detail: Value,
}

fn purpose(work: ContextItemId, source: SourceId) -> String {
    format!("work-source:{work}:{source}")
}

impl LocalOperations {
    pub fn observe_work_file(
        &self,
        request: FileObservationRequest,
    ) -> Result<SourceObservationReceipt, Error> {
        let _mutation = self.layout().acquire_mutation_lock()?;
        let now = now_micros()?;
        retention_until(now, request.retain_seconds)?;
        let basis = self.canonical_basis(request.project_id)?;
        let work_revision = work_revision(&basis, request.work_item_id)?;
        let analysis =
            self.load_analysis_snapshot(request.project_id, request.analysis_snapshot_id)?;
        // Only an included, hashed inventory file is admissible. Unsupported
        // language analysis does not remove inventory/body capability.
        let entry = analysis.inventory.entries.iter().find(|e| e.area.path == request.path)
            .filter(|e| e.classifications.contains(&InventoryClassification::Included))
            .and_then(|e| e.content_sha256.as_ref())
            .ok_or_else(|| Error::new("source path is excluded, unavailable or absent from the exact Analysis Snapshot"))?;
        let canonical = self.open_canonical()?;
        let binding = canonical
            .get_local_binding(request.project_id)
            .map_err(|e| Error::with_source("repository binding unavailable", e))?;
        drop(canonical);
        let root = RepositoryRoot::open(&binding.absolute_path)
            .map_err(|e| Error::with_source("repository unavailable", e))?;
        let bytes = root
            .read_bounded_regular_file(Path::new(&request.path), FILE_BYTES)
            .map_err(|e| Error::with_source("scoped source observation unavailable", e))?;
        let hash = format!("{:x}", Sha256::digest(&bytes));
        if &hash != entry {
            return Err(Error::new("source hash changed since the selected Analysis Snapshot; analyze and observe afresh"));
        }
        let body = String::from_utf8(bytes)
            .map_err(|_| Error::new("binary/non-UTF-8 source body is unsupported"))?;
        let now = now_micros()?;
        let until = retention_until(now, request.retain_seconds)?;
        let mut detail = json!({"kind":"file","path":request.path,"sha256":hash,"representation":"whole_file",
            "body":body,"observed_utf8_bytes":body.len(),"omitted_utf8_bytes":0,
            "comparison":"missing_before_state","before_source_id":null,"current_repository_state":"not_rechecked",
            "repository_worktree":analysis.repository_worktree,"reconstruction":"not_attempted"});
        if let Some(before) = request.before_source_id {
            let earlier = self
                .retained_source_observation(&basis, request.work_item_id, before)?
                .ok_or_else(|| Error::new("before-state Source body is unavailable"))?;
            if earlier.detail["kind"] != "file"
                || earlier.detail["path"] != request.path
                || earlier.observed_at_unix_micros >= now.as_unix_micros()
            {
                return Err(Error::new(
                    "before-state path or observation chronology is incompatible",
                ));
            }
            detail["comparison"] = json!(if earlier.detail["sha256"] == hash {
                "no_change"
            } else {
                "changed"
            });
            detail["before_source_id"] = json!(before.to_string());
        }
        self.retain_source_observation(
            &basis,
            request.work_item_id,
            work_revision,
            &analysis,
            SourcePayload::File {
                locator: request.path,
                snapshot: format!("sha256:{hash}"),
            },
            PrincipalKind::Repository,
            now,
            until,
            detail,
        )
    }

    pub fn run_work_command(
        &self,
        request: WorkCommandRequest,
    ) -> Result<SourceObservationReceipt, Error> {
        let now = now_micros()?;
        retention_until(now, request.retain_seconds)?;
        if request.command_label.trim().is_empty()
            || request.command_label.len() > 4096
            || request.program.is_empty()
            || request.timeout.is_zero()
            || request.timeout > Duration::from_secs(300)
            || request.arguments.len() > 128
        {
            return Err(Error::new("invalid bounded Work command request"));
        }
        let basis = self.canonical_basis(request.project_id)?;
        let revision = work_revision(&basis, request.work_item_id)?;
        let analysis =
            self.load_analysis_snapshot(request.project_id, request.analysis_snapshot_id)?;
        let canonical = self.open_canonical()?;
        let binding = canonical
            .get_local_binding(request.project_id)
            .map_err(|e| Error::with_source("repository binding unavailable", e))?;
        drop(canonical);
        // JSON array framing preserves argv boundaries; raw argv is transient.
        let invocation = serde_json::to_vec(&(&request.program, &request.arguments))
            .map_err(|e| Error::with_source("cannot fingerprint command", e))?;
        if invocation.len() > 16_384 {
            return Err(Error::new("command invocation exceeds bounded input"));
        }
        let fingerprint = format!("sha256:{:x}", Sha256::digest(&invocation));
        let result = self.run_child(
            &request.program,
            request.arguments.iter().map(OsString::from),
            Some(&binding.absolute_path),
            vec![format!(
                "project:{}:work:{}",
                request.project_id, request.work_item_id
            )],
            request.timeout,
            CancellationFlag::default(),
        )?;
        let artifact_dir = self
            .layout()
            .artifacts_dir()
            .join(result.operation_id.to_string());
        let observed = (|| {
            let outcome = result.value.as_ref().ok_or_else(|| {
                Error::new("command failed to start; no execution Source was recorded")
            })?;
            let stream = |artifact: &volicord_local_platform::ProcessStreamArtifact| -> Result<Value, Error> {
                let mut bytes = Vec::new();
                File::open(artifact.path()).map_err(|e| Error::with_source("command stream unavailable",e))?
                    .take(STREAM_BYTES as u64).read_to_end(&mut bytes).map_err(|e| Error::with_source("command stream read failed",e))?;
                // Never invent replacement bytes for non-text output.
                let mut end = bytes.len();
                let body = loop {
                    match std::str::from_utf8(&bytes[..end]) {
                        Ok(text) => break text.to_owned(),
                        Err(e) if e.error_len().is_none() => end = e.valid_up_to(),
                        Err(_) => return Ok(json!({"representation":"unavailable","reason":"non_utf8_output","observed_bytes":artifact.bytes(),"body":null})),
                    }
                };
                Ok(json!({"body":body,"representation":if artifact.bytes()==end as u64 {"whole_stream"} else {"bounded_excerpt"},
                    "observed_bytes":artifact.bytes(),"omitted_bytes":artifact.bytes().saturating_sub(end as u64),
                    "capture_completeness":format!("{:?}",artifact.completeness()).to_lowercase()}))
            };
            let (exit_code, termination) = match outcome.termination {
                ProcessTermination::ExitCode(code) => {
                    (Some(code), volicord_context::CommandTermination::Exited)
                }
                ProcessTermination::Signal(_) => {
                    (None, volicord_context::CommandTermination::Signaled)
                }
                ProcessTermination::Unknown => {
                    (None, volicord_context::CommandTermination::Indeterminate)
                }
            };
            let detail = json!({"kind":"command","operation_id":result.operation_id.to_string(),
                "started_at_unix_micros":result.started_at_unix_micros,"ended_at_unix_micros":result.ended_at_unix_micros,
                "exit_code":exit_code,"termination":format!("{:?}",termination).to_lowercase(),
                "operation_state":format!("{:?}",result.state).to_lowercase(),"stdout":stream(&outcome.stdout)?,"stderr":stream(&outcome.stderr)?,
                "repository_state":"selected_analysis_only_not_rechecked","comparison":"not_applicable"});
            Ok((
                SourcePayload::CommandExecution {
                    command_label: request.command_label,
                    invocation_fingerprint: fingerprint,
                    outcome: volicord_context::CommandOutcome {
                        exit_code,
                        termination,
                    },
                },
                detail,
            ))
        })();
        // Operational raw streams must not bypass managed expiry/deletion.
        std::fs::remove_dir_all(&artifact_dir).map_err(|e| {
            Error::with_source(
                "transient command artifact cleanup failed; no retention receipt issued",
                e,
            )
        })?;
        let (payload, detail) = observed?;
        let _mutation = self.layout().acquire_mutation_lock()?;
        let current = self.canonical_basis(request.project_id)?;
        if work_revision(&current, request.work_item_id)? != revision {
            return Err(Error::new(
                "Work changed during command observation; no body retained",
            ));
        }
        let current_binding = self
            .open_canonical()?
            .get_local_binding(request.project_id)
            .map_err(|e| Error::with_source("repository binding changed", e))?;
        if current_binding != binding {
            return Err(Error::new("repository binding changed during execution"));
        }
        self.retain_source_observation(
            &current,
            request.work_item_id,
            revision,
            &analysis,
            payload,
            PrincipalKind::Command,
            TimestampMicros::from_unix_micros(result.ended_at_unix_micros),
            retention_until(
                TimestampMicros::from_unix_micros(result.ended_at_unix_micros),
                request.retain_seconds,
            )?,
            detail,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn retain_source_observation(
        &self,
        basis: &CanonicalReadBasis,
        work: ContextItemId,
        work_revision: u64,
        analysis: &volicord_repository_intelligence::AnalysisSnapshot,
        payload: SourcePayload,
        actor_kind: PrincipalKind,
        observed_at: TimestampMicros,
        retained_until: TimestampMicros,
        detail: Value,
    ) -> Result<SourceObservationReceipt, Error> {
        // Open/check both stores before publishing the canonical identity.
        let mut privacy = PrivacyStore::open(self.layout().privacy_store())
            .map_err(|e| Error::with_source("source body store unavailable", e))?;
        let mut canonical = self.open_canonical()?;
        let source = canonical
            .record_source(
                new_operation_id()?,
                basis.project.id,
                SourceDraft {
                    expected_project_revision: basis.project.revision,
                    payload,
                    actor: Principal {
                        kind: actor_kind,
                        identity: "volicord-local-source-observer".into(),
                    },
                    observer: Some(Principal {
                        kind: PrincipalKind::Agent,
                        identity: "volicord-local-operations".into(),
                    }),
                    availability: Availability::Available,
                },
            )
            .map_err(|e| Error::with_source("cannot record observed Source", e))?
            .value;
        let current = self.canonical_basis(basis.project.id)?;
        let grounding = CanonicalGrounding::from_read_basis(&current)
            .map_err(|e| Error::with_source("Source grounding failed", e))?;
        let observation = SourceObservation {
            format_kind: KIND.into(),
            format_version: VERSION,
            project_id: basis.project.id,
            work_item_id: work,
            work_revision,
            source_id: source.id,
            source_revision: 1,
            analysis_snapshot_id: analysis.identity,
            repository_snapshot_id: analysis.repository_snapshot.to_string(),
            repository_source_id: analysis.repository_source.identity(),
            observed_at_unix_micros: observed_at.as_unix_micros(),
            detail,
        };
        let mut links = vec![
            ManagedCanonicalLink::Source(source.id),
            ManagedCanonicalLink::Source(analysis.repository_source.identity()),
            ManagedCanonicalLink::ContextItem(work),
        ];
        let mut sources = vec![source.id, analysis.repository_source.identity()];
        if let Some(before) = observation.detail["before_source_id"].as_str() {
            let id = SourceId::from_bytes(crate::operations::parse_identity(before)?);
            links.push(ManagedCanonicalLink::Source(id));
            sources.push(id);
        }
        if let Some(goal) = current.context_items.iter().find(|i| i.id == work) {
            for id in &goal.source_basis {
                links.push(ManagedCanonicalLink::Source(*id));
                sources.push(*id);
            }
        }
        sources.sort_unstable();
        sources.dedup();
        let included_sources = sources
            .iter()
            .map(|id| {
                grounding
                    .source_reference(*id)
                    .map_err(|e| Error::with_source("Source grounding failed", e))
            })
            .collect::<Result<Vec<_>, _>>()?;
        privacy.record_managed_derived(ManagedDerivedDraft {project_id:basis.project.id,kind:ManagedDerivedKind::SourceObservation,
            provider:None,model:None,purpose:purpose(work,source.id),analysis_snapshot:Some(analysis.identity),included_sources,canonical_links:links,
            content:serde_json::to_string(&observation).map_err(|e|Error::with_source("cannot encode Source copy",e))?,uncertainty:None,
            retained_until:Some(retained_until),retention_basis:"Explicit local bounded Work Source observation; disposable copy; no background transmission".into(),
        }).map_err(|e|Error::new(format!("Source {} recorded but body retention failed: {e}; body unavailable; do not infer bytes",source.id)))?;
        Ok(SourceObservationReceipt {
            project_id: basis.project.id,
            work_item_id: work,
            source_id: source.id,
            source_revision: 1,
            observed_at_unix_micros: observed_at.as_unix_micros(),
            retained_until_unix_micros: retained_until.as_unix_micros(),
        })
    }

    pub(crate) fn retained_source_observation(
        &self,
        basis: &CanonicalReadBasis,
        work: ContextItemId,
        source: SourceId,
    ) -> Result<Option<SourceObservation>, Error> {
        work_revision(basis, work)?;
        let records = self.privacy_status(basis.project.id)?;
        let record = records.managed_derived.iter().find(|r| {
            r.kind == ManagedDerivedKind::SourceObservation && r.purpose == purpose(work, source)
        });
        record
            .map(|r| {
                if !matches!(
                    r.state,
                    ManagedDerivedState::Current | ManagedDerivedState::Stale
                ) {
                    return Ok(None);
                }
                let stored = PrivacyStore::open(self.layout().privacy_store())
                    .and_then(|store| store.get_derived(basis.project.id, r.id))
                    .map_err(|e| Error::with_source("exact Source copy unavailable", e))?;
                decode_observation(basis, work, source, &stored)
            })
            .transpose()
            .map(Option::flatten)
    }
}

fn retention_until(now: TimestampMicros, seconds: u64) -> Result<TimestampMicros, Error> {
    if seconds == 0 || seconds > MAX_RETENTION_SECONDS {
        return Err(Error::new("Source retention must be 1..604800 seconds"));
    }
    now.as_unix_micros()
        .checked_add((seconds * 1_000_000) as i64)
        .map(TimestampMicros::from_unix_micros)
        .ok_or_else(|| Error::new("Source retention clock overflow"))
}

pub(crate) fn work_revision(basis: &CanonicalReadBasis, work: ContextItemId) -> Result<u64, Error> {
    basis
        .context_items
        .iter()
        .find(|i| i.id == work && i.role == ContextItemRole::Goal)
        .map(|i| i.revision)
        .ok_or_else(|| Error::new("Work is not a retained Goal in the requested Project"))
}

fn decode_observation(
    basis: &CanonicalReadBasis,
    work: ContextItemId,
    source: SourceId,
    record: &ManagedDerivedRecord,
) -> Result<Option<SourceObservation>, Error> {
    let now = now_micros()?;
    if !matches!(
        record.state,
        ManagedDerivedState::Current | ManagedDerivedState::Stale
    ) || record.retained_until.is_none_or(|t| t <= now)
    {
        return Ok(None);
    }
    let Some(content) = record.content.as_deref() else {
        return Ok(None);
    };
    if content.len() > volicord_privacy::MANAGED_DERIVED_CONTENT_BYTE_LIMIT {
        return Err(Error::new("Source copy exceeds managed content bound"));
    }
    let header: Value =
        serde_json::from_str(content).map_err(|_| Error::new("corrupt Source observation copy"))?;
    if header["format_kind"] != KIND || header["format_version"] != VERSION {
        return Err(Error::new("unsupported Source observation format"));
    }
    let observation: SourceObservation = serde_json::from_value(header)
        .map_err(|_| Error::new("corrupt Source observation shape"))?;
    if observation.project_id != basis.project.id
        || observation.work_item_id != work
        || observation.source_id != source
        || observation.work_revision != work_revision(basis, work)?
        || observation.source_revision != 1
        || record.analysis_snapshot != Some(observation.analysis_snapshot_id)
    {
        return Err(Error::new(
            "Source copy identity/revision binding is incompatible",
        ));
    }
    let Some(canonical) = basis.sources.iter().find(|s| s.source.id == source) else {
        return Ok(None);
    };
    if canonical.availability != Availability::Available {
        return Ok(None);
    };
    if record.included_sources.iter().any(|reference| {
        !basis.sources.iter().any(|s| {
            s.source.id == reference.identity() && s.availability == Availability::Available
        })
    }) {
        return Ok(None);
    }
    verify_body(&canonical.source, &observation)?;
    Ok(Some(observation))
}

fn verify_body(source: &Source, observation: &SourceObservation) -> Result<(), Error> {
    match &source.payload {
        SourcePayload::File { locator, snapshot } => {
            let body = observation.detail["body"]
                .as_str()
                .ok_or_else(|| Error::new("file body unavailable"))?;
            let hash = format!("{:x}", Sha256::digest(body.as_bytes()));
            if observation.detail["kind"] != "file"
                || observation.detail["representation"] != "whole_file"
                || observation.detail["path"] != *locator
                || observation.detail["sha256"] != hash
                || snapshot != &format!("sha256:{hash}")
            {
                return Err(Error::new("Source copy hash or locator mismatch"));
            }
        }
        SourcePayload::CommandExecution { outcome, .. } => {
            if observation.detail["kind"] != "command"
                || observation.detail["exit_code"] != json!(outcome.exit_code)
                || observation.detail["termination"]
                    != format!("{:?}", outcome.termination).to_lowercase()
            {
                return Err(Error::new("command Source copy outcome mismatch"));
            }
        }
        _ => return Err(Error::new("Source kind has no authorized retained body")),
    }
    Ok(())
}
