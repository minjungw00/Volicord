#[path = "support/scoped_check_fixture.rs"]
#[allow(dead_code)]
mod fixture;
use std::error::Error;
use volicord_context::*;
#[test]
fn direct_canonical_attachment_rejects_another_same_argv_execution_or_changed_capture_basis(
) -> Result<(), Box<dyn Error>> {
    let f = fixture::fixture()?;
    let basis = f.ops.canonical_basis(f.project)?;
    let original = &basis.checkpoint_history[0];
    for variant in ["same_argv_source", "operation", "artifact", "other_work"] {
        let mut draft = CheckpointDraft {
            expected_project_revision: basis.project.revision,
            work_item_id: original.work_item_id,
            kind: original.kind,
            goal: original.goal.clone(),
            work_state: original.work_state,
            state_change: original.state_change.clone(),
            source_basis: original.source_basis.clone(),
            changed_source_basis: original.changed_source_basis.clone(),
            changed_paths: original.changed_paths.clone(),
            applied_decisions: original.applied_decisions.clone(),
            verification: original.verification.clone(),
            user_review: original.user_review.clone(),
            user_acceptance: original.user_acceptance.clone(),
            known_limits: original.known_limits.clone(),
            non_goals: original.non_goals.clone(),
            open_questions: original.open_questions.clone(),
            next_step: original.next_step.clone(),
            handoff_to: original.handoff_to.clone(),
        };
        let run = draft.verification[0].scoped_run.as_mut().ok_or("run")?;
        match variant {
            "same_argv_source" => {
                draft.verification[0].source_id = Some(f.passed.source_id);
            }
            "operation" => run.operation_id = "ab".repeat(16),
            "artifact" => run.artifacts[0].sha256 = "0".repeat(64),
            "other_work" => {
                run.work_item_id = f.other;
                draft.work_item_id = Some(f.other);
            }
            _ => {}
        }
        let mut store = Store::open(f.ops.layout().canonical_store())?;
        assert!(
            store
                .record_checkpoint(
                    OperationId::from_bytes([variant.len() as u8; 16]),
                    f.project,
                    draft
                )
                .is_err(),
            "{variant}"
        );
        assert_eq!(basis, f.ops.canonical_basis(f.project)?);
    }
    Ok(())
}

#[test]
fn portable_capture_tampering_is_rejected_even_after_rehashing_the_envelope(
) -> Result<(), Box<dyn Error>> {
    use serde_json::{json, Value};
    use sha2::{Digest, Sha256};
    use std::fs;
    let f = fixture::fixture()?;
    let original = f.temp.path().join("original.json");
    f.ops.export_bundle(f.project, &original)?;
    let source: Value = serde_json::from_slice(&fs::read(&original)?)?;
    for variant in ["artifact", "operation", "source"] {
        let mut bundle = source.clone();
        let tables = bundle["payload"]["tables"].as_array_mut().ok_or("tables")?;
        let table = tables
            .iter_mut()
            .find(|t| t["name"] == "checkpoint_verifications")
            .ok_or("verification table")?;
        let column = table["columns"]
            .as_array()
            .ok_or("columns")?
            .iter()
            .position(|c| c == "scoped_run")
            .ok_or("scoped column")?;
        let row = table["rows"]
            .as_array()
            .ok_or("rows")?
            .iter()
            .position(|r| r[1]["value"] == f.failed.checkpoint_id.to_string())
            .ok_or("original failed row")?;
        let mut run: Value =
            serde_json::from_str(table["rows"][row][column]["value"].as_str().ok_or("run")?)?;
        match variant {
            "artifact" => run["artifacts"][0]["sha256"] = json!("0".repeat(64)),
            "operation" => run["operation_id"] = json!("ab".repeat(16)),
            _ => {
                table["rows"][row][4] =
                    json!({"type":"bytes","value":f.passed.source_id.to_string()});
            }
        }
        table["rows"][row][column]["value"] = json!(run.to_string());
        let state = json!({"project_id":bundle["payload"]["project_id"],"tables":bundle["payload"]["tables"]});
        let history = format!("{:x}", Sha256::digest(serde_json::to_vec(&state)?));
        bundle["payload"]["lineage"]["history_basis"] = json!(history);
        bundle["payload"]["lineage"]["common_base_basis"] = json!(history);
        bundle["checksum"] = json!(format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&bundle["payload"])?)
        ));
        let path = f.temp.path().join(format!("tampered-{variant}.json"));
        let mut encoded = serde_json::to_vec(&bundle)?;
        encoded.push(b'\n');
        fs::write(&path, encoded)?;
        let imported =
            volicord_operations::LocalOperations::new(volicord_operations::RuntimeLayout::new(
                f.temp.path().join(format!("import-{variant}")),
            )?);
        fs::create_dir_all(imported.layout().root())?;
        let mut kernel = Store::open(imported.layout().canonical_store())?;
        let error = kernel
            .import_bundle(OperationId::from_bytes([23; 16]), &path)
            .err()
            .ok_or("forged capture imported")?;
        assert!(
            error.to_string().contains("scoped") || error.to_string().contains("Checkpoint"),
            "{error}"
        );
        assert!(imported.canonical_basis(f.project).is_err());
    }
    Ok(())
}
