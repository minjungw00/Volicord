//! Read-only render context; no observation or human verdict is inferred here.
use crate::{ViewerError, ViewerRequest};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{fs, io::Read, path::Path, sync::OnceLock};
use volicord_projections::{ExplanationReading, ProjectProjection};

fn digest(data: &[u8]) -> String {
    format!("{:x}", Sha256::digest(data))
}
static PROCESS: OnceLock<Result<Value, String>> = OnceLock::new();
pub(crate) fn prime_process_binding() {
    let _ = PROCESS.get_or_init(process_binding);
}
fn process_binding() -> Result<Value, String> {
    let executable = std::env::current_exe().map_err(|_| "executable identity unavailable")?;
    let mut file = fs::File::open("/proc/self/exe").map_err(|_| "executable bytes unavailable")?;
    let mut hash = Sha256::new();
    let mut chunk = [0_u8; 65536];
    loop {
        let count = file
            .read(&mut chunk)
            .map_err(|_| "executable read failed")?;
        if count == 0 {
            break;
        }
        hash.update(&chunk[..count]);
    }
    let stat = fs::read_to_string("/proc/self/stat").map_err(|_| "process identity unavailable")?;
    let start_ticks = stat
        .rsplit_once(") ")
        .and_then(|(_, s)| s.split_whitespace().nth(19))
        .and_then(|s| s.parse::<u64>().ok())
        .ok_or("process start identity unavailable")?;
    let boot = fs::read_to_string("/proc/sys/kernel/random/boot_id")
        .map_err(|_| "boot identity unavailable")?;
    Ok(
        json!({"pid":std::process::id(),"start_ticks":start_ticks,"boot_id":boot.trim(),
        "executable_path":executable,"executable_sha256":format!("{:x}",hash.finalize())}),
    )
}
fn explanation(
    kind: &str,
    identity: String,
    readings: &[ExplanationReading],
    language: &str,
) -> Value {
    let reading = readings.iter().find(|r| r.language == language);
    let content = reading.and_then(|r| r.content.as_ref());
    json!({"kind":kind,"identity":identity,
        "state":reading.map(|r| json!(r.state)).unwrap_or(json!("unavailable")),
        "plan_fingerprint":content.map(|c| &c.realization.plan_fingerprint),
        "generated_at_unix_micros":content.map(|c| c.generated_at_unix_micros),
        "realization_sha256":content.and_then(|c| serde_json::to_vec(&c.realization).ok()).map(|b| digest(&b))})
}
pub(crate) fn context(
    runtime: &Path,
    request: &ViewerRequest,
    projection: &ProjectProjection,
) -> Result<Value, ViewerError> {
    let process = PROCESS
        .get_or_init(process_binding)
        .as_ref()
        .map_err(|s| ViewerError::new(s.clone()))?;
    let mut random = [0_u8; 16];
    getrandom::fill(&mut random).map_err(|_| ViewerError::new("render identity unavailable"))?;
    let render_id = random
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let mut subjects = std::collections::BTreeMap::new();
    for w in projection
        .work_history
        .iter()
        .chain(projection.selected_work.iter())
        .chain(&projection.work_overview.current.items)
        .chain(&projection.work_overview.completed.items)
        .chain(&projection.work_overview.remaining.items)
        .chain(&projection.work_overview.next_steps.items)
    {
        subjects.insert(
            format!("work:{}", w.work_item_id),
            explanation(
                "work",
                w.work_item_id.to_string(),
                &w.reading.explanations,
                &request.requested_language,
            ),
        );
    }
    for d in projection
        .decision_catalog
        .iter()
        .chain(projection.selected_decision.iter())
    {
        subjects.insert(
            format!("decision:{}", d.decision.decision_id),
            explanation(
                "decision",
                d.decision.decision_id.to_string(),
                &d.decision.explanations,
                &request.requested_language,
            ),
        );
    }
    let sources=projection.source_catalog.iter().map(|s| json!({"identity":s.source.id.to_string(),
        "availability":format!("{:?}",s.availability).to_lowercase(),"freshness":format!("{:?}",s.freshness).to_lowercase(),
        "snapshot_basis":s.snapshot_basis})).collect::<Vec<_>>();
    let source_bytes = serde_json::to_vec(&sources)
        .map_err(|_| ViewerError::new("source observation encoding failed"))?;
    let runtime = fs::canonicalize(runtime)
        .map_err(|_| ViewerError::new("Runtime observation identity unavailable"))?;
    Ok(
        json!({"kind":"volicord_viewer_observation_context","schema_version":1,
        "render_id":render_id,"mode":"live","process":process,
        "runtime_binding":digest(runtime.as_os_str().as_encoded_bytes()),"project_id":request.project_id.to_string(),
        "locale":super::render::locale_key(request.locale),"language":request.requested_language,
        "view":request.view.fields().into_iter().collect::<std::collections::BTreeMap<_,_>>(),
        "selected_work":projection.selected_work.as_ref().map(|w|w.work_item_id.to_string()),
        "selected_decision":projection.selected_decision.as_ref().map(|d|d.decision.decision_id.to_string()),
        "canonical_read_fingerprint":projection.canonical_read_fingerprint,
        "source_state_sha256":digest(&source_bytes),"source_count":sources.len(),
        "analysis":projection.resume.snapshots.iter().map(|s|json!({"analysis_snapshot":s.analysis_snapshot.to_string(),
            "repository_snapshot":s.repository_snapshot.to_string(),"freshness":format!("{:?}",s.freshness.state).to_lowercase()})).collect::<Vec<_>>(),
        "explanations":subjects.into_values().collect::<Vec<_>>()}),
    )
}
