//! Actual supported non-Viewer callers for the operations retired from HTTP.
#[path = "../../volicord-operations/tests/support/reading_fixture.rs"]
#[allow(dead_code)]
mod reading_fixture;
use serde_json::{json, Value};
use volicord_host::HostAdapter;
use volicord_operations::{run_cli, CliExit, LocalOperations};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

fn cli(f: &reading_fixture::Fixture, args: &[&str]) -> (CliExit, String, String) {
    let base = [
        "--runtime".to_owned(),
        f.operations.layout().root().display().to_string(),
        "--project".into(),
        f.project.to_string(),
        "--json".into(),
    ];
    let mut out = Vec::new();
    let mut err = Vec::new();
    let exit = run_cli(
        base.iter().map(String::as_str).chain(args.iter().copied()),
        &mut out,
        &mut err,
    );
    (
        exit,
        String::from_utf8_lossy(&out).into_owned(),
        String::from_utf8_lossy(&err).into_owned(),
    )
}
fn successful_cli(f: &reading_fixture::Fixture, args: &[&str]) -> Result<Value> {
    let (exit, out, err) = cli(f, args);
    assert_eq!(exit, CliExit::SUCCESS, "{err}");
    Ok(serde_json::from_str(&out)?)
}
fn mcp(host: &mut HostAdapter, args: Value) -> Value {
    host.handle(json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"canonical_mutate","arguments":args}})).expect("MCP response")["result"].clone()
}

#[test]
fn cli_correction_supersession_and_forgetting_remain_callable_with_exact_authority() -> Result {
    let f = reading_fixture::fixture()?;
    let auth = successful_cli(
        &f,
        &[
            "advanced",
            "records",
            "source",
            "--host",
            "test",
            "--session",
            "reader-scope",
            "--text",
            "Correct the wording, choose Remote, then forget the named Goal.",
        ],
    )?;
    let auth = auth["identity"].as_str().ok_or("authorization Source")?;
    let work = f.goals["older"].to_string();
    let original = f.operations.canonical_basis(f.project)?;
    let text = format!(
        "{}.",
        original
            .context_items
            .iter()
            .find(|c| c.id == f.goals["older"])
            .ok_or("Goal")?
            .statement
    );
    let wrong = cli(
        &f,
        &[
            "advanced",
            "records",
            "correct-context",
            &work,
            "--revision",
            "999",
            "--source",
            auth,
            "--text",
            &text,
        ],
    );
    assert_ne!(wrong.0, CliExit::SUCCESS);
    assert_eq!(original, f.operations.canonical_basis(f.project)?);
    successful_cli(
        &f,
        &[
            "advanced",
            "records",
            "correct-context",
            &work,
            "--revision",
            "1",
            "--source",
            auth,
            "--text",
            &text,
        ],
    )?;
    let decision = f.decisions["explicit"].to_string();
    let project_decision = f.decisions["project"].to_string();
    successful_cli(
        &f,
        &[
            "advanced",
            "records",
            "correct-decision",
            &project_decision,
            "--revision",
            "1",
            "--source",
            auth,
            "--text",
            "Apply across this Project.",
        ],
    )?;
    successful_cli(
        &f,
        &[
            "advanced",
            "records",
            "supersede-decision",
            &decision,
            "--source",
            auth,
            "--alternative",
            "remote",
            "--rationale",
            "The user now chooses the remote boundary.",
        ],
    )?;
    let current = f.operations.canonical_basis(f.project)?;
    assert!(current
        .superseded_decisions
        .iter()
        .any(|d| d.decision.id == f.decisions["explicit"]));
    assert!(current.active_decisions.iter().any(|d| Some(d.decision.id)
        == current
            .superseded_decisions
            .iter()
            .find(|d| d.decision.id == f.decisions["explicit"])
            .and_then(|d| d.superseded_by)));
    // Existing references are protected by the canonical domain, regardless of caller.
    let denied = cli(
        &f,
        &[
            "advanced",
            "records",
            "forget",
            "context_item",
            &work,
            "--source",
            auth,
        ],
    );
    assert_ne!(denied.0, CliExit::SUCCESS);
    assert!(f
        .operations
        .canonical_basis(f.project)?
        .context_items
        .iter()
        .any(|c| c.id == f.goals["older"]));
    let forgotten = f.goals["goal_only"].to_string();
    successful_cli(
        &f,
        &[
            "advanced",
            "records",
            "forget",
            "context_item",
            &forgotten,
            "--source",
            auth,
        ],
    )?;
    assert!(!f
        .operations
        .canonical_basis(f.project)?
        .context_items
        .iter()
        .any(|c| c.id == f.goals["goal_only"]));
    assert!(f
        .operations
        .canonical_basis(f.project)?
        .context_items
        .iter()
        .any(|c| c.id == f.goals["older"]));
    assert!(successful_cli(&f, &["decisions"])?["decisions"]
        .as_array()
        .is_some_and(|ds| !ds.is_empty()));
    Ok(())
}

#[test]
fn mcp_correction_supersession_and_forgetting_keep_the_recorded_user_boundary() -> Result {
    let f = reading_fixture::fixture()?;
    let mut host = HostAdapter::new(LocalOperations::new(f.operations.layout().clone()));
    let project = f.project.to_string();
    let work = f.goals["older"].to_string();
    let goal = f
        .operations
        .canonical_basis(f.project)?
        .context_items
        .into_iter()
        .find(|c| c.id == f.goals["older"])
        .ok_or("Goal")?;
    let result = mcp(
        &mut host,
        json!({"action":"correct_context","project_id":project,"record_id":work,"expected_revision":1,"corrected_text":format!("{}.",goal.statement),"user_turn":"Correct this exact Goal wording."}),
    );
    assert_eq!(result["isError"], false, "{result}");
    let decision = f.decisions["explicit"].to_string();
    let result = mcp(
        &mut host,
        json!({"action":"correct_decision","project_id":project,"record_id":f.decisions["project"].to_string(),"expected_revision":1,"corrected_text":"Apply across this Project.","user_turn":"Correct the rationale of this exact Decision."}),
    );
    assert_eq!(result["isError"], false, "{result}");
    let result = mcp(
        &mut host,
        json!({"action":"supersede_decision","project_id":project,"record_id":decision,"alternative_key":"remote","rationale":"The user now chooses Remote.","user_turn":"Supersede this exact choice with Remote."}),
    );
    assert_eq!(result["isError"], false, "{result}");
    let canonical = f.operations.canonical_basis(f.project)?;
    let replacement = canonical
        .active_decisions
        .iter()
        .find(|d| {
            Some(d.decision.id)
                == canonical
                    .superseded_decisions
                    .iter()
                    .find(|d| d.decision.id == f.decisions["explicit"])
                    .and_then(|d| d.superseded_by)
        })
        .ok_or("superseding Decision")?;
    assert!(canonical
        .sources
        .iter()
        .any(|s| s.source.id == replacement.decision.user_turn_source_id));
    let result = mcp(
        &mut host,
        json!({"action":"forget","project_id":project,"record_kind":"context_item","record_id":f.goals["goal_only"].to_string(),"user_turn":"Forget this exact unreferenced Goal and dependent content."}),
    );
    assert_eq!(result["isError"], false, "{result}");
    assert!(!f
        .operations
        .canonical_basis(f.project)?
        .context_items
        .iter()
        .any(|c| c.id == f.goals["goal_only"]));
    assert!(f
        .operations
        .canonical_basis(f.project)?
        .context_items
        .iter()
        .any(|c| c.id == f.goals["same_title"]));
    Ok(())
}
