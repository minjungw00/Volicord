//! Shared actual-store faults for independent Operations/CLI and MCP consumers.
use serde_json::{json, Value};
use volicord_context::{ProjectId, TimestampMicros};
use volicord_operations::{LocalOperations, RuntimeLayout};
use volicord_privacy::*;

pub struct ExpiryControl {
    _root: tempfile::TempDir,
    pub ops: LocalOperations,
    pub project: ProjectId,
    pub record: ManagedDerivedId,
}
impl ExpiryControl {
    pub fn new(
        cleanup_fails: bool,
        inspection_fails: bool,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let root = tempfile::tempdir()?;
        let ops = LocalOperations::new(RuntimeLayout::new(root.path().join("runtime"))?);
        let project = ops
            .initialize_project("Expiry diagnostics", None)?
            .project
            .id;
        let record = PrivacyStore::open(ops.layout().privacy_store())?
            .record_managed_derived(ManagedDerivedDraft {
                project_id: project,
                kind: ManagedDerivedKind::CachedSummary,
                provider: None,
                model: None,
                purpose: "expiry control".into(),
                analysis_snapshot: None,
                included_sources: vec![],
                canonical_links: vec![],
                content: "private expired control body".into(),
                retained_until: Some(TimestampMicros::from_unix_micros(1)),
                retention_basis: "explicit expiry control".into(),
                uncertainty: None,
            })?
            .id;
        let conn = rusqlite::Connection::open(ops.layout().privacy_store())?;
        if cleanup_fails {
            conn.execute_batch("CREATE TRIGGER fail_expiry BEFORE UPDATE ON managed_derived BEGIN SELECT RAISE(FAIL,'private cleanup blocker'); END;")?;
        }
        if inspection_fails {
            // Cleanup reads only managed records. Full remaining-count inspection
            // independently reads authority observations and must reject corruption.
            conn.execute("INSERT INTO authority_observations(project_id, observed_at, observation_json) VALUES (?1, 1, 'private invalid inspection record')", [project.as_bytes().as_slice()])?;
        }
        Ok(Self {
            _root: root,
            ops,
            project,
            record,
        })
    }
    pub fn repair(&self) -> Result<(), Box<dyn std::error::Error>> {
        let conn = rusqlite::Connection::open(self.ops.layout().privacy_store())?;
        conn.execute_batch(
            "DROP TRIGGER IF EXISTS fail_expiry; DELETE FROM authority_observations;",
        )?;
        Ok(())
    }
    pub fn assert_outcome(&self, result: &Value, cleanup_fails: bool, inspection_fails: bool) {
        let managed = &result["managed"];
        assert_eq!(
            managed["state"],
            if cleanup_fails || inspection_fails {
                "incomplete"
            } else {
                "completed"
            }
        );
        assert_eq!(
            managed["cleanup_diagnostic"],
            if cleanup_fails {
                json!("managed expiry batch incomplete; inspect and retry cleanup")
            } else {
                Value::Null
            }
        );
        assert_eq!(
            managed["inspection_diagnostic"],
            if inspection_fails {
                json!("managed cleanup inspection failed")
            } else {
                Value::Null
            }
        );
        assert_eq!(
            managed["removed_ids"],
            if cleanup_fails {
                Value::Null
            } else {
                json!([self.record.to_string()])
            }
        );
        assert_eq!(
            managed["remaining_eligible"],
            if inspection_fails {
                Value::Null
            } else {
                json!(usize::from(cleanup_fails))
            }
        );
        assert_eq!(result["candidates"]["state"], "completed");
        assert_eq!(result["candidates"]["remaining_eligible"], 0);
        assert_eq!(result["candidates"]["cleanup_diagnostic"], Value::Null);
        assert_eq!(result["candidates"]["inspection_diagnostic"], Value::Null);
        assert_eq!(result["local_guarantee"], "logical_content_removal");
        assert_eq!(result["provider_outcome"], "not_requested");
        assert!(!result.to_string().contains("private"));
        let record = PrivacyStore::open(self.ops.layout().privacy_store())
            .unwrap()
            .get_derived(self.project, self.record)
            .unwrap();
        assert_eq!(record.content.is_some(), cleanup_fails);
    }
    pub fn assert_retry(&self, result: &Value, cleanup_failed: bool) {
        assert_eq!(result["managed"]["state"], "completed");
        assert_eq!(result["managed"]["remaining_eligible"], 0);
        assert_eq!(result["managed"]["cleanup_diagnostic"], Value::Null);
        assert_eq!(result["managed"]["inspection_diagnostic"], Value::Null);
        assert_eq!(
            result["managed"]["removed_ids"],
            if cleanup_failed {
                json!([self.record.to_string()])
            } else {
                json!([])
            }
        );
    }
}
