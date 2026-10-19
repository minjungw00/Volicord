//! Task-owned authored control. Executes Python assertions and retains actual
//! independent process receipts; supplies no ordinary user or trusted-host claim.
use sha2::{Digest, Sha256};
use std::{error::Error, fs, time::Duration};
use volicord_context::*;
use volicord_operations::*;

pub struct ScopedFixture {
    pub temp: tempfile::TempDir,
    pub ops: LocalOperations,
    pub project: ProjectId,
    pub work: ContextItemId,
    pub other: ContextItemId,
    pub failed: WorkCheckReceipt,
    pub passed: WorkCheckReceipt,
}
pub fn declaration() -> CheckDeclaration {
    CheckDeclaration {
        key: "value.equals.seven".into(),
        purpose: "Verify the value returned to the caller".into(),
        assertion: "main.value() == 7".into(),
        coverage: Some("main.value() return value through check.py; no other behavior".into()),
        environment: Some(
            "explicit local Python 3; -B disables bytecode; no external services".into(),
        ),
        target_paths: vec!["main.py".into(), "check.py".into()],
    }
}
pub fn recording(work: ContextItemId, source: SourceId, passed: bool) -> WorkCheckRecording {
    let line = if passed {
        "actual=7; expected=7\n"
    } else {
        "actual=3; expected=7\n"
    };
    WorkCheckRecording {
        work_item_id: work,
        source_id: source,
        assertion_state: if passed {
            AssertionState::Passed
        } else {
            AssertionState::Failed
        },
        assertion_evidence: Some(AssertionEvidence {
            stream: AssertionStream::Stdout,
            start_byte: 0,
            end_byte: line.len(),
            sha256: format!("{:x}", Sha256::digest(line.as_bytes())),
        }),
        retest: None,
        outcome: if passed {
            "Recorded actual=7; expected=7; assertion completed".into()
        } else {
            "Recorded actual=3; expected=7; assertion failed".into()
        },
        next_step: "Inspect the bounded comparison and its limits".into(),
    }
}
pub fn command(
    ops: &LocalOperations,
    project: ProjectId,
    work: ContextItemId,
) -> Result<WorkCommandRequest, Box<dyn Error>> {
    let analysis = ops
        .analyze(project, vec![])?
        .value
        .ok_or("analysis")?
        .analysis
        .identity;
    Ok(WorkCommandRequest {
        project_id: project,
        work_item_id: work,
        analysis_snapshot_id: analysis,
        command_label: "value assertion".into(),
        program: "python3".into(),
        arguments: vec!["-B".into(), "check.py".into()],
        timeout: Duration::from_secs(5),
        retain_seconds: 3600,
    })
}
pub fn fixture() -> Result<ScopedFixture, Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path().join("repo");
    fs::create_dir(&root)?;
    fs::write(root.join("main.py"), "def value():\n    return 3\n")?;
    fs::write(root.join("check.py"),"from main import value\nactual = value()\nprint(f'actual={actual}; expected=7', flush=True)\nassert actual == 7, 'value assertion'\n")?;
    let ops = LocalOperations::new(RuntimeLayout::new(temp.path().join("runtime"))?);
    let project = ops
        .initialize_project("Controlled task verification", Some(&root))?
        .project
        .id;
    let work = ops
        .record_current_host_user_context(
            project,
            "explicit-local-control".into(),
            "task-owned".into(),
            "Controlled task: make value equal seven".into(),
            ContextItemRole::Goal,
            "Controlled task: make value equal seven".into(),
        )?
        .context_item_id;
    let other = ops
        .record_current_host_user_context(
            project,
            "explicit-local-control".into(),
            "task-owned".into(),
            "Other Work".into(),
            ContextItemRole::Goal,
            "Other Work".into(),
        )?
        .context_item_id;
    let original = ops.run_work_check(command(&ops, project, work)?, declaration())?;
    let failed = ops.record_work_check(project, recording(work, original.source_id, false))?;
    fs::write(root.join("main.py"), "def value():\n    return 7\n")?;
    let repeat = ops.run_work_check(command(&ops, project, work)?, declaration())?;
    let mut report = recording(work, repeat.source_id, true);
    report.retest=Some(CheckRetestBasis {checkpoint_id:failed.checkpoint_id,checkpoint_revision:failed.checkpoint_revision,verification_index:0,source_id:failed.source_id,target_relation:CheckTargetRelation::ChangedArtifacts,rationale:"Same assertion and bounded coverage after changing main.value() from 3 to 7; both artifact sets sampled at execution boundaries".into()});
    let passed = ops.record_work_check(project, report)?;
    Ok(ScopedFixture {
        temp,
        ops,
        project,
        work,
        other,
        failed,
        passed,
    })
}
