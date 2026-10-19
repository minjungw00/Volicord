//! Real Host/Operations producers and consumers; no generated answer or campaign input.
use serde_json::{json, Value};
use std::{error::Error, fs, time::Duration};
use volicord_context::{
    CanonicalRecordId, ContextItemCorrectionDraft, ContextItemRole, CorrectionKind, SourceId,
};
use volicord_host::HostAdapter;
use volicord_operations::{LocalOperations, RuntimeLayout, WorkCommandRequest};

fn rpc(adapter: &mut HostAdapter, name: &str, args: Value) -> Result<Value, Box<dyn Error>> {
    let response=adapter.handle(json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args}})).ok_or("missing RPC response")?;
    if response["result"]["isError"] == true || response.get("error").is_some() {
        return Err(response.to_string().into());
    }
    Ok(response["result"]["structuredContent"].clone())
}
fn denied(adapter: &mut HostAdapter, name: &str, args: Value) -> Result<(), Box<dyn Error>> {
    let response=adapter.handle(json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args}})).ok_or("missing response")?;
    assert!(
        response["result"]["isError"] == true || response.get("error").is_some(),
        "{response}"
    );
    Ok(())
}
fn complete(adapter: &mut HostAdapter, mut selector: Value) -> Result<Value, Box<dyn Error>> {
    let mut bytes = String::new();
    let mut fingerprint = None;
    loop {
        let page = rpc(adapter, "canonical_inspect", selector.clone())?;
        assert!(
            matches!(page["state"].as_str(), Some("partial" | "complete")),
            "{page}"
        );
        assert_eq!(page["read_only"], true);
        let chunk = page["chunk"].as_str().ok_or("missing chunk")?;
        assert!(chunk.len() <= 2048);
        bytes.push_str(chunk);
        if let Some(previous) = &fingerprint {
            assert_eq!(previous, &page["fingerprint"]);
        } else {
            fingerprint = Some(page["fingerprint"].clone());
        }
        if page["next_offset"].is_null() {
            return Ok(serde_json::from_str(&bytes)?);
        }
        selector["offset"] = page["next_offset"].clone();
        selector["expected_fingerprint"] = page["fingerprint"].clone();
    }
}
fn identity(value: &str) -> Result<[u8; 16], Box<dyn Error>> {
    let mut bytes = [0; 16];
    for (i, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[i * 2..i * 2 + 2], 16)?;
    }
    Ok(bytes)
}

#[test]
fn scoped_source_metadata_keeps_explicit_work_binding_without_reading_copies(
) -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path().join("repo");
    fs::create_dir(&root)?;
    fs::write(root.join("main.custom"), "private file bytes\n")?;
    let ops = LocalOperations::new(RuntimeLayout::new(temp.path().join("runtime"))?);
    let project = ops
        .initialize_project("source metadata", Some(&root))?
        .project
        .id;
    let foreign = ops.initialize_project("foreign", None)?.project.id;
    let mut host = HostAdapter::new(ops);
    let mut works = Vec::new();
    for turn in ["Inspect command metadata", "Inspect another Work"] {
        works.push(rpc(
            &mut host,
            "context_record",
            json!({"project_id":project.to_string(),"role":"goal","work_transition":"start_new","user_turn":turn,"statement":turn}),
        )?);
    }
    let work = works[0]["context_item_id"].as_str().ok_or("Work")?;
    let analysis = host
        .operations()
        .analyze(project, vec![])?
        .value
        .ok_or("analysis")?;
    let source = host.operations().run_work_command(WorkCommandRequest {
        project_id: project,
        work_item_id: volicord_context::ContextItemId::from_bytes(identity(work)?),
        analysis_snapshot_id: analysis.analysis.identity,
        command_label: "Observed verification command".into(),
        program: "python3".into(),
        arguments: vec!["-c".into(), "print('private command output')".into()],
        timeout: Duration::from_secs(5),
        retain_seconds: 3600,
    })?;
    let selector = json!({"project_id":project.to_string(),"work_item_id":work,
        "record_kind":"source","record_id":source.source_id.to_string(),"revision":1,"field":"observation"});
    let before = host.operations().privacy_status(project)?.managed_derived;
    let metadata = complete(&mut host, selector.clone())?;
    assert_eq!(
        metadata["payload"]["command_label"],
        "Observed verification command"
    );
    assert_eq!(metadata["payload"]["exit_code"], 0);
    assert!(!metadata.to_string().contains("private command output"));
    let mut unscoped = selector.clone();
    unscoped
        .as_object_mut()
        .ok_or("selector")?
        .remove("work_item_id");
    assert_eq!(metadata, complete(&mut host, unscoped)?);
    let mut status = selector.clone();
    status["field"] = json!("source_status");
    assert_eq!(
        complete(&mut host, status)?[0]["source_id"],
        source.source_id.to_string()
    );
    assert_eq!(
        before,
        host.operations().privacy_status(project)?.managed_derived
    );

    let mut wrong = selector.clone();
    wrong["work_item_id"] = works[1]["context_item_id"].clone();
    denied(&mut host, "canonical_inspect", wrong)?;
    let mut foreign_source = selector.clone();
    foreign_source["project_id"] = json!(foreign.to_string());
    assert_eq!(
        rpc(&mut host, "canonical_inspect", foreign_source)?["reason"],
        "record_unavailable"
    );
    let mut changed_revision = selector.clone();
    changed_revision["revision"] = json!(2);
    assert_eq!(
        rpc(&mut host, "canonical_inspect", changed_revision)?["reason"],
        "revision_unavailable"
    );
    let unrelated = host.operations().record_user_source(
        project,
        "codex".into(),
        "fixture".into(),
        "Unassociated Source".into(),
    )?;
    let mut unrelated_selector = selector.clone();
    unrelated_selector["record_id"] = json!(unrelated.identity);
    denied(&mut host, "canonical_inspect", unrelated_selector)?;

    // Inventory eligibility and metadata scope do not require a supported analyzer.
    let file = rpc(
        &mut host,
        "work_source",
        json!({"action":"observe_file","project_id":project.to_string(),"work_item_id":work,
            "analysis_snapshot_id":analysis.analysis.identity.to_string(),"path":"main.custom","retain_seconds":1}),
    )?;
    let mut file_metadata = selector.clone();
    file_metadata["record_id"] = file["source_id"].clone();
    let file_observation = complete(&mut host, file_metadata.clone())?;
    assert_eq!(file_observation["payload"]["kind"], "source_reference");
    assert!(!file_observation.to_string().contains("private file bytes"));
    std::thread::sleep(Duration::from_millis(1100));
    let expired = host.operations().privacy_status(project)?.managed_derived;
    assert_eq!(
        file_observation,
        complete(&mut host, file_metadata.clone())?
    );
    file_metadata["field"] = json!("body");
    assert_eq!(
        rpc(&mut host, "canonical_inspect", file_metadata)?["reason"],
        "source_body_expired"
    );
    assert_eq!(
        expired,
        host.operations().privacy_status(project)?.managed_derived
    );

    rpc(
        &mut host,
        "work_source",
        json!({"action":"delete_source_copy","project_id":project.to_string(),"work_item_id":work,"source_id":source.source_id.to_string()}),
    )?;
    let deleted = host.operations().privacy_status(project)?.managed_derived;
    assert_eq!(metadata, complete(&mut host, selector.clone())?);
    let mut body = selector;
    body["field"] = json!("body");
    assert_eq!(
        rpc(&mut host, "canonical_inspect", body)?["reason"],
        "source_body_deleted"
    );
    assert_eq!(
        deleted,
        host.operations().privacy_status(project)?.managed_derived
    );
    Ok(())
}

#[test]
fn normal_host_observation_is_exact_scoped_bounded_historical_and_deletable(
) -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path().join("repo");
    fs::create_dir(&root)?;
    // Independent source expectations, including multibyte continuation boundaries.
    let original = format!("# {}\ndef value():\n    return 7\n", "한".repeat(1500));
    fs::write(root.join("main.py"), &original)?;
    fs::write(root.join("helper.py"), "def helper():\n    return 2\n")?;
    fs::write(
        root.join("test.py"),
        "from main import value\nassert value() == 7\n",
    )?;
    let runtime = RuntimeLayout::new(temp.path().join("runtime"))?;
    let ops = LocalOperations::new(runtime.clone());
    let p = ops
        .initialize_project("ordinary work", Some(&root))?
        .project
        .id;
    let foreign = ops.initialize_project("foreign", None)?.project.id;
    let mut host = HostAdapter::new(ops);
    let turn="Inspect value. If there is a consequential comparison, investigate the helper and test first.";
    let work = rpc(
        &mut host,
        "context_record",
        json!({"project_id":p.to_string(),"role":"goal","work_transition":"start_new","user_turn":turn,"statement":turn}),
    )?;
    let w = work["context_item_id"]
        .as_str()
        .ok_or("missing Work")?
        .to_owned();
    let analysis = rpc(
        &mut host,
        "repository_analyze",
        json!({"project_id":p.to_string()}),
    )?;
    let a = analysis["analysis_snapshot_id"].clone();
    let observe = json!({"action":"observe_file","project_id":p.to_string(),"work_item_id":w,"analysis_snapshot_id":a,"path":"main.py","retain_seconds":3600});
    let first = rpc(&mut host, "work_source", observe.clone())?;
    let source = first["source_id"]
        .as_str()
        .ok_or("missing source")?
        .to_owned();
    let mut selector = first["inspect"].clone();
    selector.as_object_mut().ok_or("selector")?.remove("tool");
    let page = rpc(&mut host, "canonical_inspect", selector.clone())?;
    assert_eq!(page["state"], "partial");
    let mut continuation = selector.clone();
    continuation["offset"] = page["next_offset"].clone();
    continuation["expected_fingerprint"] = page["fingerprint"].clone();
    let body = complete(&mut host, selector.clone())?;
    assert_eq!(body["detail"]["body"], original);
    assert_eq!(body["detail"]["representation"], "whole_file");
    assert_eq!(body["detail"]["comparison"], "missing_before_state");
    assert_eq!(
        body["source_id"],
        json!(SourceId::from_bytes(identity(&source)?))
    );
    assert_eq!(
        body["repository_snapshot_id"],
        analysis["repository_snapshot_id"]
    );
    println!("Scoped Product read: project={p} work={w} source={source} revision=1 representation=whole_file body_utf8_bytes={} selected_repository={} observed_at={}", original.len(), analysis["repository_snapshot_id"], first["observed_at_unix_micros"]);
    // Scoped caller/helper/test inspection is explicit, never an unbounded expansion.
    for path in ["helper.py", "test.py"] {
        let mut request = observe.clone();
        request["path"] = json!(path);
        let receipt = rpc(&mut host, "work_source", request)?;
        let mut detail = receipt["inspect"].clone();
        detail.as_object_mut().ok_or("selector")?.remove("tool");
        assert_eq!(
            complete(&mut host, detail)?["detail"]["body"],
            fs::read_to_string(root.join(path))?
        );
    }
    let mut equal = observe.clone();
    equal["before_source_id"] = json!(source);
    let no_change = rpc(&mut host, "work_source", equal)?;
    let mut no_change_read = no_change["inspect"].clone();
    no_change_read
        .as_object_mut()
        .ok_or("selector")?
        .remove("tool");
    assert_eq!(
        complete(&mut host, no_change_read)?["detail"]["comparison"],
        "no_change"
    );
    let manifest = complete(
        &mut host,
        json!({"project_id":p.to_string(),"record_kind":"context_item","record_id":w,"revision":1,"field":"source_observations"}),
    )?;
    assert_eq!(
        manifest["observations"]
            .as_array()
            .ok_or("observations")?
            .len(),
        4
    );
    assert!(!manifest.to_string().contains(&original));
    let direction = complete(
        &mut host,
        json!({"project_id":p.to_string(),"record_kind":"context_item","record_id":w,"revision":1,"field":"statement"}),
    )?;
    assert_eq!(direction, turn);
    let mut wrong = selector.clone();
    wrong["project_id"] = json!(foreign.to_string());
    assert_eq!(
        rpc(&mut host, "canonical_inspect", wrong)?["reason"],
        "record_unavailable"
    );
    let mut no_work = selector.clone();
    no_work
        .as_object_mut()
        .ok_or("selector")?
        .remove("work_item_id");
    assert_eq!(
        rpc(&mut host, "canonical_inspect", no_work)?["reason"],
        "source_body_requires_work_selector"
    );
    let another = rpc(
        &mut host,
        "context_record",
        json!({"project_id":p.to_string(),"role":"goal","work_transition":"start_new","user_turn":turn,"statement":turn}),
    )?;
    let mut wrong_work = selector.clone();
    wrong_work["work_item_id"] = another["context_item_id"].clone();
    denied(&mut host, "canonical_inspect", wrong_work)?;
    let mut wrong_revision = selector.clone();
    wrong_revision["revision"] = json!(2);
    assert_eq!(
        rpc(&mut host, "canonical_inspect", wrong_revision)?["reason"],
        "revision_unavailable"
    );
    let mut wrong_hash = continuation.clone();
    wrong_hash["expected_fingerprint"] = json!("0".repeat(64));
    denied(&mut host, "canonical_inspect", wrong_hash)?;
    let mut bad_path = observe.clone();
    bad_path["path"] = json!("../outside.py");
    denied(&mut host, "work_source", bad_path)?;
    let mut execution = observe.clone();
    execution["action"] = json!("run_command");
    execution["program"] = json!("touch");
    execution["arguments"] = json!(["forbidden-mcp-execution"]);
    denied(&mut host, "work_source", execution)?;
    assert!(!root.join("forbidden-mcp-execution").exists());
    let basis_before = host.operations().canonical_basis(p)?;
    assert!(basis_before.active_decisions.is_empty());
    assert!(basis_before.latest_checkpoint.is_none());
    assert!(!basis_before
        .context_items
        .iter()
        .any(|i| i.role == ContextItemRole::Learning));
    // This current-host instruction remained a Goal through record, Recall and exact read.
    let recalled = rpc(&mut host, "recall", json!({"project_id":p.to_string()}))?;
    assert_eq!(recalled["behaviorally_relevant_context"], json!([]));
    // Current files never replace observed historical bytes, even after restart/removal.
    fs::write(root.join("main.py"), "def value():\n    return 9\n")?;
    denied(&mut host, "work_source", observe.clone())?;
    drop(host);
    let mut host = HostAdapter::new(LocalOperations::new(runtime));
    assert_eq!(
        complete(&mut host, selector.clone())?["detail"]["body"],
        original
    );
    fs::remove_file(root.join("main.py"))?;
    assert_eq!(
        complete(&mut host, selector.clone())?["detail"]["body"],
        original
    );
    let deleted = rpc(
        &mut host,
        "work_source",
        json!({"action":"delete_source_copy","project_id":p.to_string(),"work_item_id":w,"source_id":source}),
    )?;
    assert_eq!(deleted["canonical_source_deleted"], false);
    assert_eq!(
        rpc(&mut host, "canonical_inspect", selector.clone())?["reason"],
        "source_body_deleted"
    );
    // A continuation cannot return a previously prepared suffix after deletion.
    let stale = rpc(&mut host, "canonical_inspect", continuation)?;
    assert_eq!(stale["state"], "unavailable");
    assert!(stale.get("chunk").is_none());
    assert_eq!(basis_before, host.operations().canonical_basis(p)?);
    // Deleting a before-state prevents the dependent comparison from providing bytes.
    let unavailable = rpc(
        &mut host,
        "canonical_inspect",
        json!({"project_id":p.to_string(),"work_item_id":w,"record_kind":"source",
        "record_id":no_change["source_id"],"revision":1,"field":"body"}),
    )?;
    assert!(matches!(
        unavailable["reason"].as_str(),
        Some(
            "source_body_unavailable"
                | "source_body_invalidated"
                | "before_source_body_unavailable"
        )
    ));
    Ok(())
}

#[test]
fn actual_command_copy_and_source_lifecycle_have_no_choice_or_learning_side_effect(
) -> Result<(), Box<dyn Error>> {
    for excerpt in [false, true] {
        source_lifecycle(excerpt)?;
    }
    Ok(())
}
fn source_lifecycle(excerpt: bool) -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path().join("repo");
    fs::create_dir(&root)?;
    fs::write(
        root.join("main.py"),
        if excerpt {
            format!("{}value = 1\n", "# prefix\n".repeat(2000))
        } else {
            "value = 1\n".into()
        },
    )?;
    let ops = LocalOperations::new(RuntimeLayout::new(temp.path().join("runtime"))?);
    let p = ops.initialize_project("lifecycle", Some(&root))?.project.id;
    let work = ops.record_current_host_user_context(
        p,
        "codex".into(),
        "fixture".into(),
        "Inspect value".into(),
        ContextItemRole::Goal,
        "Inspect value".into(),
    )?;
    let a = ops.analyze(p, vec![])?.value.ok_or("analysis")?;
    let result = ops.run_work_command(WorkCommandRequest {
        project_id: p,
        work_item_id: work.context_item_id,
        analysis_snapshot_id: a.analysis.identity,
        command_label: "failed test observation".into(),
        program: "python3".into(),
        arguments: vec![
            "-c".into(),
            "import sys; sys.stdout.write('x' * 9000); sys.stderr.write('failure\\n'); sys.exit(3)"
                .into(),
        ],
        timeout: Duration::from_secs(5),
        retain_seconds: 3600,
    })?;
    let mut host = HostAdapter::new(ops);
    let selector = json!({"project_id":p.to_string(),"work_item_id":work.context_item_id.to_string(),"record_kind":"source",
        "record_id":result.source_id.to_string(),"revision":1,"field":"body"});
    let body = complete(&mut host, selector.clone())?;
    assert_eq!(body["detail"]["exit_code"], 3);
    assert_eq!(body["detail"]["termination"], "exited");
    assert_eq!(body["detail"]["stdout"]["body"], "x".repeat(8192));
    assert_eq!(
        body["detail"]["stdout"]["representation"],
        "bounded_excerpt"
    );
    assert_eq!(body["detail"]["stdout"]["omitted_bytes"], 808);
    assert_eq!(body["detail"]["stderr"]["body"], "failure\n");
    let ptext = p.to_string();
    let w = work.context_item_id.to_string();
    let mut observe = json!({"project_id":ptext,"work_item_id":w,"action":"observe_file","path":"main.py","analysis_snapshot_id":a.analysis.identity.to_string(),"retain_seconds":1});
    if excerpt {
        observe["byte_range"] = json!({"start_byte":18000,"end_byte":18010});
    }
    let expired = rpc(&mut host, "work_source", observe.clone())?;
    std::thread::sleep(Duration::from_millis(1100));
    let mut expired_selector = expired["inspect"].clone();
    expired_selector
        .as_object_mut()
        .ok_or("selector")?
        .remove("tool");
    assert_eq!(
        rpc(&mut host, "canonical_inspect", expired_selector)?["reason"],
        "source_body_expired"
    );
    let mut retained_request = observe;
    retained_request["retain_seconds"] = json!(3600);
    let retained = rpc(&mut host, "work_source", retained_request)?;
    let mut retained_selector = retained["inspect"].clone();
    retained_selector
        .as_object_mut()
        .ok_or("selector")?
        .remove("tool");
    let auth = host.operations().record_user_source(
        p,
        "codex".into(),
        "fixture".into(),
        "Correct the Goal formatting".into(),
    )?;
    host.operations().correct_context_item(
        p,
        work.context_item_id,
        ContextItemCorrectionDraft {
            expected_revision: 1,
            corrected_statement: "Inspect  value".into(),
            kind: CorrectionKind::Formatting,
            user_authorization_source_id: SourceId::from_bytes(identity(&auth.identity)?),
        },
    )?;
    assert_eq!(
        rpc(&mut host, "canonical_inspect", retained_selector.clone())?["reason"],
        "work_revision_changed"
    );
    // Generic privacy inspection cannot leak any unscoped observation body.
    assert!(host
        .operations()
        .privacy_status(p)?
        .managed_derived
        .iter()
        .all(|r| r.content.is_none()));
    let canonical = host.operations().canonical_basis(p)?;
    assert!(canonical.active_decisions.is_empty());
    assert!(canonical.latest_checkpoint.is_none());
    // Genuine user-authored Learning is separate from a conditional task, and stays bounded.
    let learning = rpc(
        &mut host,
        "context_record",
        json!({"project_id":ptext,"role":"learning",
        "user_turn":"I want to learn the consequential trade-offs.","statement":"I want to learn the consequential trade-offs"}),
    )?;
    assert_eq!(learning["role"], "learning");
    let recall = rpc(&mut host, "recall", json!({"project_id":ptext}))?;
    assert!(recall["behaviorally_relevant_context"]
        .as_array()
        .ok_or("contexts")?
        .iter()
        .any(|i| i["role"] == "learning"));
    let forget_auth = host.operations().record_user_source(
        p,
        "codex".into(),
        "fixture".into(),
        "Forget observed source".into(),
    )?;
    let observed_id =
        SourceId::from_bytes(identity(retained["source_id"].as_str().ok_or("source")?)?);
    host.operations().forget_record(
        p,
        CanonicalRecordId::Source(observed_id),
        SourceId::from_bytes(identity(&forget_auth.identity)?),
    )?;
    assert_eq!(
        rpc(&mut host, "canonical_inspect", retained_selector)?["reason"],
        "record_unavailable"
    );
    assert!(host
        .operations()
        .canonical_basis(p)?
        .context_items
        .iter()
        .any(|i| i.id == work.context_item_id));
    Ok(())
}

#[test]
fn corrupt_unsupported_or_ambiguous_observation_cannot_supply_bytes() -> Result<(), Box<dyn Error>>
{
    for excerpt in [false, true] {
        corrupt_observation(excerpt)?;
    }
    Ok(())
}
fn corrupt_observation(excerpt: bool) -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path().join("repo");
    fs::create_dir(&root)?;
    let file = if excerpt {
        format!("{}value = 11\n", "# prefix\n".repeat(2000))
    } else {
        "value = 11\n".into()
    };
    fs::write(root.join("main.py"), &file)?;
    let ops = LocalOperations::new(RuntimeLayout::new(temp.path().join("runtime"))?);
    let p = ops.initialize_project("integrity", Some(&root))?.project.id;
    let work = ops.record_current_host_user_context(
        p,
        "codex".into(),
        "fixture".into(),
        "Inspect value".into(),
        ContextItemRole::Goal,
        "Inspect value".into(),
    )?;
    let analysis = ops.analyze(p, vec![])?.value.ok_or("analysis")?;
    let receipt = ops.observe_work_file(volicord_operations::FileObservationRequest {
        byte_range: excerpt.then_some(volicord_operations::FileByteRange {
            start_byte: 18000,
            end_byte: 18011,
        }),
        project_id: p,
        work_item_id: work.context_item_id,
        analysis_snapshot_id: analysis.analysis.identity,
        path: "main.py".into(),
        before_source_id: None,
        retain_seconds: 3600,
    })?;
    let record = ops
        .privacy_status(p)?
        .managed_derived
        .into_iter()
        .find(|r| r.purpose.ends_with(&receipt.source_id.to_string()))
        .ok_or("record")?;
    let db = rusqlite::Connection::open(ops.layout().privacy_store())?;
    let original: String = db.query_row(
        "SELECT record_json FROM managed_derived WHERE id=?1",
        [record.id.as_bytes().as_slice()],
        |row| row.get(0),
    )?;
    let original_json: Value = serde_json::from_str(&original)?;
    let original_body: Value =
        serde_json::from_str(original_json["content"].as_str().ok_or("body")?)?;
    let mut host = HostAdapter::new(ops);
    let selector = json!({"project_id":p.to_string(),"work_item_id":work.context_item_id.to_string(),"record_kind":"source",
        "record_id":receipt.source_id.to_string(),"revision":1,"field":"body"});
    for variant in [
        "hash",
        "chronology",
        "unsupported",
        "old_version",
        "missing_bytes",
        "coordinates",
        "rehashed_body",
        "repository_identity",
        "observation_time",
        "overflow_line",
    ] {
        let mut record = original_json.clone();
        let mut body = original_body.clone();
        match variant {
            "hash" => body["detail"]["body"] = json!("forbidden replacement bytes"),
            "chronology" => body["observed_at_unix_micros"] = json!(i64::MAX),
            "unsupported" => body["format_version"] = json!(99),
            "old_version" => body["format_version"] = json!(1),
            "repository_identity" if excerpt => {
                body["repository_snapshot_id"] = json!("0".repeat(64))
            }
            "observation_time" if excerpt => body["observed_at_unix_micros"] = json!(1),
            "overflow_line" if excerpt => {
                body["detail"]["region"]["start_line"] = json!(u64::MAX);
                body["detail"]["region"]["end_line"] = json!(u64::MAX);
            }
            "coordinates" if excerpt => {
                body["detail"]["region"]["start_byte"] = json!(17999);
                body["detail"]["region"]["end_byte"] = json!(18010);
                body["detail"]["region"]["start_line"] = json!(2000);
                body["detail"]["region"]["end_line"] = json!(2000);
            }
            "rehashed_body" if excerpt => {
                body["detail"]["body"] = json!("value = 99\n");
                body["detail"]["region"]["excerpt_sha256"] =
                    json!(volicord_projections::explanation_text_hash("value = 99\n")
                        .trim_start_matches("sha256:"));
            }
            _ => {
                body["detail"]
                    .as_object_mut()
                    .ok_or("detail")?
                    .remove("body");
            }
        }
        record["content"] = json!(body.to_string());
        db.execute(
            "UPDATE managed_derived SET record_json=?1 WHERE id=?2",
            rusqlite::params![record.to_string(), record_id_bytes(&original_json)?],
        )?;
        denied(&mut host, "canonical_inspect", selector.clone())?;
        // No fallback to still-present matching current file or latest Source.
        assert_eq!(fs::read_to_string(root.join("main.py"))?, file);
    }
    db.execute(
        "UPDATE managed_derived SET record_json=?1 WHERE id=?2",
        rusqlite::params![original, record_id_bytes(&original_json)?],
    )?;
    assert_eq!(
        complete(&mut host, selector)?["detail"]["body"],
        "value = 11\n"
    );
    // Independent canonical availability fixtures exercise the retained reader's stale barrier.
    let canonical_db = rusqlite::Connection::open(host.operations().layout().canonical_store())?;
    for availability in ["stale", "unavailable", "unknown"] {
        canonical_db.execute(
            "UPDATE sources SET availability=?1 WHERE id=?2",
            rusqlite::params![availability, receipt.source_id.as_bytes().as_slice()],
        )?;
        let read = rpc(
            &mut host,
            "canonical_inspect",
            json!({"project_id":p.to_string(),"work_item_id":work.context_item_id.to_string(),
            "record_kind":"source","record_id":receipt.source_id.to_string(),"revision":1,"field":"body"}),
        )?;
        assert_eq!(read["state"], "unavailable");
        assert_eq!(
            read["metadata"]["source_status"][0]["availability"],
            availability
        );
        assert!(read.get("chunk").is_none());
    }
    canonical_db.execute(
        "UPDATE sources SET availability='available' WHERE id=?1",
        [receipt.source_id.as_bytes().as_slice()],
    )?;
    let export = temp.path().join("context.bundle");
    host.operations().export_bundle(p, &export)?;
    let portable = fs::read_to_string(export)?;
    assert!(!portable.contains("value = 11"));
    assert!(!portable.contains("volicord_source_observation"));
    Ok(())
}
fn record_id_bytes(record: &Value) -> Result<Vec<u8>, Box<dyn Error>> {
    // The owning ManagedDerived identity's maintained serialization is a byte array.
    Ok(serde_json::from_value(record["id"].clone())?)
}
