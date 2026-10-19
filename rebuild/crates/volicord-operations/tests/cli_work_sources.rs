//! Authored controls for the public local caller, never naturalistic task evidence.
use serde_json::{json, Value};
use std::{
    error::Error,
    fs,
    path::PathBuf,
    process::{Command, Output},
    time::Duration,
};
use volicord_context::ContextItemRole;
use volicord_operations::{LocalOperations, RuntimeLayout};

struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
    runtime: PathBuf,
    ops: LocalOperations,
    project: String,
    work: String,
    analysis: String,
}
impl Fixture {
    fn new() -> Result<Self, Box<dyn Error>> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("repository");
        let runtime = temp.path().join("runtime");
        fs::create_dir(&root)?;
        fs::write(root.join("main.py"), "value = 4\n")?;
        let ops = LocalOperations::new(RuntimeLayout::new(runtime.clone())?);
        let p = ops
            .initialize_project("Authored caller control", Some(&root))?
            .project
            .id;
        let w = ops
            .record_current_host_user_context(
                p,
                "control".into(),
                "fixture".into(),
                "Inspect value without inventing an earlier state.".into(),
                ContextItemRole::Goal,
                "Inspect value without inventing an earlier state.".into(),
            )?
            .context_item_id;
        let analysis = ops
            .analyze(p, vec![])?
            .value
            .ok_or("analysis")?
            .analysis
            .identity
            .to_string();
        Ok(Self {
            _temp: temp,
            root,
            runtime,
            ops,
            project: p.to_string(),
            work: w.to_string(),
            analysis,
        })
    }
    fn call(&self, args: &[&str]) -> Result<Output, Box<dyn Error>> {
        Ok(Command::new(env!("CARGO_BIN_EXE_volicord"))
            .current_dir(&self.root)
            .arg("--runtime")
            .arg(&self.runtime)
            .args(["--project", &self.project, "--json"])
            .args(args)
            .output()?)
    }
    fn ok(&self, args: &[&str]) -> Result<Value, Box<dyn Error>> {
        let output = self.call(args)?;
        if !output.status.success() {
            return Err(format!(
                "CLI failed {:?}: {}",
                output.status.code(),
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }
        assert!(output.stderr.is_empty());
        Ok(serde_json::from_slice(&output.stdout)?)
    }
    fn observe(&self, before: Option<&str>, retention: &str) -> Result<Value, Box<dyn Error>> {
        let mut args = vec![
            "work",
            "source",
            "observe",
            "--work",
            &self.work,
            "--analysis",
            &self.analysis,
            "--path",
            "main.py",
            "--retain-seconds",
            retention,
        ];
        if let Some(source) = before {
            args.extend(["--before-source", source]);
        }
        self.ok(&args)
    }
    fn read(&self, source: &str) -> Result<Value, Box<dyn Error>> {
        self.ok(&[
            "work", "source", "read", "--work", &self.work, "--source", source,
        ])
    }
    fn run(&self, label: &str, program: &str, script: &str) -> Result<Value, Box<dyn Error>> {
        self.ok(&[
            "work",
            "source",
            "run",
            "--work",
            &self.work,
            "--analysis",
            &self.analysis,
            "--retain-seconds",
            "3600",
            "--timeout-seconds",
            "1",
            "--label",
            label,
            "--",
            program,
            "-B",
            "-c",
            script,
        ])
    }
    fn id(value: &Value) -> Result<&str, Box<dyn Error>> {
        value["source_id"]
            .as_str()
            .ok_or_else(|| "Source identity".into())
    }
}

#[test]
fn public_capture_read_compose_record_journey_uses_two_observed_states(
) -> Result<(), Box<dyn Error>> {
    let mut f = Fixture::new()?;
    let before = f.observe(None, "3600")?;
    let before_id = Fixture::id(&before)?;
    let first = f.read(before_id)?;
    assert_eq!(first["observation"]["detail"]["body"], "value = 4\n");
    assert_eq!(
        first["observation"]["detail"]["comparison"],
        "missing_before_state"
    );
    let equal = f.observe(Some(before_id), "3600")?;
    assert_eq!(
        f.read(Fixture::id(&equal)?)?["observation"]["detail"]["comparison"],
        "no_change"
    );
    fs::write(f.root.join("main.py"), "value = 5\n")?;
    let p = f.ops.initialize_project("Other control", None)?.project.id;
    assert!(!f
        .call(&[
            "work",
            "source",
            "observe",
            "--work",
            &f.work,
            "--analysis",
            &f.analysis,
            "--path",
            "main.py",
            "--retain-seconds",
            "3600"
        ])?
        .status
        .success());
    // A new scan observes the actual new bytes; no reconstruction from current code.
    let project = f.ops.resolve_project(&f.root)?;
    let volicord_operations::ProjectResolution::Found { project, .. } = project else {
        return Err("Project".into());
    };
    let project_id = project.id;
    f.analysis = f
        .ops
        .analyze(project_id, vec![])?
        .value
        .ok_or("analysis")?
        .analysis
        .identity
        .to_string();
    let after = f.observe(Some(before_id), "3600")?;
    let after_id = Fixture::id(&after)?;
    let read = f.read(after_id)?;
    assert_eq!(read["observation"]["detail"]["comparison"], "changed");
    assert_eq!(read["observation"]["detail"]["before_source_id"], before_id);
    assert_eq!(read["observation"]["detail"]["body"], "value = 5\n");
    assert!(
        before["observation"]["observed_at_unix_micros"].as_i64()
            < after["observation"]["observed_at_unix_micros"].as_i64()
    );
    assert_eq!(read["observation"]["analysis_snapshot_id"], f.analysis);
    let check = f.run(
        "Check observed value",
        "python3",
        "from main import value; assert value == 5; print('checked value 5')",
    )?;
    let check_id = Fixture::id(&check)?;
    let command = f.read(check_id)?;
    assert_eq!(
        command["command_source"]["command_label"],
        "Check observed value"
    );
    assert!(command["command_source"]["invocation_fingerprint"]
        .as_str()
        .ok_or("fingerprint")?
        .starts_with("sha256:"));
    assert_eq!(command["command_source"]["exit_code"], 0);
    assert_eq!(command["observation"]["detail"]["exit_code"], 0);
    assert_eq!(
        command["observation"]["detail"]["stdout"]["body"],
        "checked value 5\n"
    );
    assert_eq!(
        command["observation"]["detail"]["repository_state"],
        "selected_analysis_only_not_rechecked"
    );
    assert!(
        command["observation"]["detail"]["started_at_unix_micros"].as_i64()
            <= command["observation"]["detail"]["ended_at_unix_micros"].as_i64()
    );
    let manifest = f.ok(&["work", "source", "list", "--work", &f.work])?;
    assert_eq!(
        manifest["observations"].as_array().ok_or("manifest")?.len(),
        4
    );
    let plan = f.ok(&["work", "explain", "prepare", "--work", &f.work])?;
    let draft = json!({"plan_fingerprint":plan["plan"]["fingerprint"], "language":"en",
        "generator":{"host":"authored-control", "session":"cli-capture", "agent":null,"model":null},
        "paragraphs":[
            {"question":"heading","text":"Inspect the stored value", "evidence_keys":["goal"],"sources":[]},
            {"question":"synopsis","text":"An authored structural control inspects an observed value.","evidence_keys":["goal"],"sources":[]},
            {"question":"observation","text":"The observed file sets value to 5.","evidence_keys":["goal"],
                "sources":[{"source_id":after_id,"field":"body","use_kind":"current_code","start_byte":0,"end_byte":10,"claim":null}]},
            {"question":"observation","text":"The command printed its value check marker; coverage is limited to that assertion.","evidence_keys":["goal"],
                "sources":[{"source_id":check_id,"field":"stdout","use_kind":"observed_execution","start_byte":0,"end_byte":16,"claim":null}]}]});
    let draft_path = f._temp.path().join("authored-draft.json");
    fs::write(&draft_path, serde_json::to_vec(&draft)?)?;
    let composed = f.ok(&[
        "work",
        "explain",
        "compose",
        "--work",
        &f.work,
        "--input",
        draft_path.to_str().ok_or("path")?,
    ])?;
    let realization_path = f._temp.path().join("authored-realization.json");
    fs::write(
        &realization_path,
        serde_json::to_vec(&composed["realization"])?,
    )?;
    f.ok(&[
        "work",
        "explain",
        "record",
        "--work",
        &f.work,
        "--input",
        realization_path.to_str().ok_or("path")?,
    ])?;
    let privacy_before = f.ops.privacy_status(project_id)?;
    let source_count = f.ops.canonical_basis(project_id)?.sources.len();
    let status = f.ok(&["status"])?;
    let answers = &status["selected_work"]["answers"];
    assert_eq!(answers["explanation_state"], "current");
    assert_eq!(answers["prose"].as_array().ok_or("prose")?.len(), 4);
    assert_eq!(
        answers["source_excerpts"]
            .as_array()
            .ok_or("excerpts")?
            .len(),
        2
    );
    f.read(check_id)?;
    assert_eq!(f.ops.privacy_status(project_id)?, privacy_before);
    assert_eq!(
        f.ops.canonical_basis(project_id)?.sources.len(),
        source_count
    );
    assert!(f
        .ops
        .canonical_basis(project_id)?
        .latest_checkpoint
        .is_none());
    assert!(f
        .ops
        .canonical_basis(project_id)?
        .active_decisions
        .is_empty());
    // Wrong Project is denied by the real public caller, not just a helper.
    let denied = Command::new(env!("CARGO_BIN_EXE_volicord"))
        .arg("--runtime")
        .arg(&f.runtime)
        .args([
            "--project",
            &p.to_string(),
            "work",
            "source",
            "read",
            "--work",
            &f.work,
            "--source",
            after_id,
        ])
        .output()?;
    assert!(!denied.status.success());
    assert!(!String::from_utf8_lossy(&denied.stdout).contains("value = 5"));
    println!("AUTHORED CONTROL: caller capture -> scoped Source read -> Work prepare/compose/record -> status; two observed states, missing-before, actual named assertion; no naturalistic or semantic approval");
    Ok(())
}

#[test]
fn public_command_preserves_failure_termination_and_rejects_invalid_execution(
) -> Result<(), Box<dyn Error>> {
    let f = Fixture::new()?;
    let failed = f.run(
        "Named failing control",
        "python3",
        "import sys; print('failed control'); sys.exit(13)",
    )?;
    let failed_read = f.read(Fixture::id(&failed)?)?;
    assert_eq!(failed_read["observation"]["detail"]["exit_code"], 13);
    assert_eq!(
        failed_read["observation"]["detail"]["termination"],
        "exited"
    );
    let passed = f.run(
        "Unrelated pass control",
        "python3",
        "print('unrelated pass')",
    )?;
    assert_ne!(Fixture::id(&failed)?, Fixture::id(&passed)?);
    assert_eq!(f.read(Fixture::id(&failed)?)?, failed_read);
    let timeout = f.run(
        "Timed out control",
        "python3",
        "import time; time.sleep(20)",
    )?;
    let timed = f.read(Fixture::id(&timeout)?)?;
    assert_eq!(timed["observation"]["detail"]["timeout_detected"], true);
    assert_ne!(timed["observation"]["detail"]["termination"], "exited");
    assert!(timed["observation"]["detail"]["exit_code"].is_null());
    for seconds in ["0", "301"] {
        let invalid = f.call(&[
            "work",
            "source",
            "run",
            "--work",
            &f.work,
            "--analysis",
            &f.analysis,
            "--retain-seconds",
            "3600",
            "--timeout-seconds",
            seconds,
            "--label",
            "Never run",
            "--",
            "touch",
            "forbidden-marker",
        ])?;
        assert_eq!(invalid.status.code(), Some(2));
        assert!(!f.root.join("forbidden-marker").exists());
    }
    assert!(f
        .run(
            "Spawn failure",
            "volicord-nonexistent-control-program",
            "unused"
        )
        .is_err());
    let manifest = f.ok(&["work", "source", "list", "--work", &f.work])?;
    assert_eq!(
        manifest["observations"].as_array().ok_or("manifest")?.len(),
        3
    );
    assert!(!failed_read.to_string().contains("sys.exit"));
    assert!(!failed_read.to_string().contains("environment"));
    Ok(())
}

#[test]
fn public_source_scope_expiry_deletion_and_failed_publication_remain_bounded(
) -> Result<(), Box<dyn Error>> {
    let f = Fixture::new()?;
    let short = f.observe(None, "1")?;
    std::thread::sleep(Duration::from_millis(1100));
    let expired = f.call(&[
        "work",
        "source",
        "read",
        "--work",
        &f.work,
        "--source",
        Fixture::id(&short)?,
    ])?;
    assert!(!expired.status.success());
    assert!(String::from_utf8_lossy(&expired.stderr).contains("source_body_expired"));
    assert!(f.observe(Some(Fixture::id(&short)?), "3600").is_err());
    let retained = f.observe(None, "3600")?;
    let source = Fixture::id(&retained)?;
    let foreign_work = "00000000000000000000000000000001";
    assert!(!f
        .call(&[
            "work",
            "source",
            "read",
            "--work",
            foreign_work,
            "--source",
            source
        ])?
        .status
        .success());
    assert!(!f
        .call(&[
            "work",
            "source",
            "observe",
            "--work",
            &f.work,
            "--analysis",
            &f.analysis,
            "--retain-seconds",
            "3600",
            "--path",
            "../outside.py"
        ])?
        .status
        .success());
    f.ok(&[
        "work", "source", "delete", "--work", &f.work, "--source", source,
    ])?;
    let deleted = f.call(&[
        "work", "source", "read", "--work", &f.work, "--source", source,
    ])?;
    assert!(!deleted.status.success());
    assert!(String::from_utf8_lossy(&deleted.stderr).contains("source_body_deleted"));
    assert!(f.observe(Some(source), "3600").is_err());
    let project = f.ops.resolve_project(&f.root)?;
    let volicord_operations::ProjectResolution::Found { project, .. } = project else {
        return Err("Project".into());
    };
    let project_id = project.id;
    let count = f.ops.canonical_basis(project_id)?.sources.len();
    // A non-current body store fails rather than issuing a capture receipt.
    let connection = rusqlite::Connection::open(f.ops.layout().privacy_store())?;
    connection.execute("UPDATE privacy_metadata SET schema_version=1", [])?;
    assert!(f.observe(None, "3600").is_err());
    assert_eq!(f.ops.canonical_basis(project_id)?.sources.len(), count);
    Ok(())
}
