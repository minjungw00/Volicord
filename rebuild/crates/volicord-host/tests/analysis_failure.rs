use serde_json::{json, Value};
use std::{error::Error, fs};
use volicord_host::HostAdapter;
use volicord_operations::{LocalOperations, RuntimeLayout};

#[test]
fn mcp_distinguishes_primary_failure_from_published_analysis_with_missing_receipt(
) -> Result<(), Box<dyn Error>> {
    for primary_failed in [true, false] {
        let temp = tempfile::tempdir()?;
        let repository = temp.path().join("private-repository");
        fs::create_dir(&repository)?;
        fs::write(repository.join("main.py"), "def value():\n    return 7\n")?;
        let ops = LocalOperations::new(RuntimeLayout::new(temp.path().join("private-runtime"))?);
        let project = ops
            .initialize_project("Controlled analysis", Some(&repository))?
            .project
            .id;
        let before = ops.canonical_basis(project)?;
        if primary_failed {
            fs::rename(&repository, temp.path().join("offline"))?;
        }
        let attempts = ops.layout().artifacts_dir().join("analysis-attempts");
        fs::create_dir_all(&attempts)?;
        fs::write(
            attempts.join(project.to_string()),
            b"PRIVATE receipt blocker",
        )?;
        let mut host = HostAdapter::new(ops);
        let response = host
            .handle(json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
            "params":{"name":"repository_analyze","arguments":{"project_id":project.to_string()}}}))
            .ok_or("MCP response")?;
        assert_eq!(response["result"]["isError"], true, "{response}");
        let result = &response["result"]["structuredContent"];
        let diagnostic = &result["details"];
        assert_eq!(
            diagnostic["diagnostic"],
            "analysis_attempt_receipt_unavailable"
        );
        assert_eq!(diagnostic["primary_failed"], primary_failed);
        assert_eq!(diagnostic["project_id"], project.to_string());
        assert!(diagnostic["operation_id"]
            .as_str()
            .is_some_and(|id| id.len() == 32));
        assert_eq!(
            diagnostic["next_supported_action"]["tool"],
            "repository_understanding"
        );
        if primary_failed {
            assert_eq!(result["error"], "bound repository is unavailable");
            assert_eq!(diagnostic["published_analysis_snapshot"], Value::Null);
            assert_eq!(diagnostic["published_operation_state"], Value::Null);
            assert_eq!(before, host.operations().canonical_basis(project)?);
        } else {
            assert!(result["error"]
                .as_str()
                .is_some_and(|s| s.contains("analysis was published")));
            assert_eq!(diagnostic["published_operation_state"], "partial");
            let snapshot = diagnostic["published_analysis_snapshot"]
                .as_str()
                .ok_or("snapshot")?;
            let manifest = host
                .operations()
                .layout()
                .analysis_project_dir(project)
                .join(format!("{snapshot}.json"));
            assert!(!fs::read(manifest)?.is_empty());
        }
        let encoded = serde_json::to_string(&response)?;
        assert!(!encoded.contains("PRIVATE"));
        assert!(!encoded.contains("private-repository"));
        assert!(!encoded.contains("private-runtime"));
        assert!(!encoded.contains("cannot prepare analysis attempt receipts"));
    }
    Ok(())
}
