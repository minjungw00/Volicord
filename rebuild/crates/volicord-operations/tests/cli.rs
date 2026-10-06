use rusqlite::Connection;
use serde_json::{json, Value};
use std::{collections::BTreeSet, fs, path::Path, process::Command, sync::Barrier, thread};
use volicord_operations::{run_cli, CliExit, RuntimeLayout};

#[test]
fn help_is_hierarchical_and_obsolete_public_forms_are_rejected() {
    let (exit, output, error) = cli(["--help"]);
    assert_eq!(exit, CliExit::SUCCESS);
    assert!(error.is_empty());
    for command in [
        "status",
        "analyze",
        "recall",
        "questions",
        "decisions",
        "document",
        "viewer",
        "context",
        "privacy",
        "doctor",
        "codex",
        "advanced",
    ] {
        assert!(output.contains(command), "missing {command} in {output}");
    }
    assert!(output.contains("volicord status"));
    assert!(output.contains("--json"));

    let (exit, output, error) = cli(["document", "--help"]);
    assert_eq!(exit, CliExit::SUCCESS);
    assert!(error.is_empty());
    assert!(output.contains("preview"));
    assert!(output.contains("export"));

    for obsolete in [
        vec!["project", "init", "Old"],
        vec!["canonical", "inspect"],
        vec!["portable", "export"],
        vec!["guarded", "show"],
        vec!["documents", "preview"],
        vec!["viewer", "open", "--level", "deep"],
        vec!["viewer", "open", "--view", "invalid"],
        vec![
            "viewer", "export", "--output", "out.html", "--view", "overview",
        ],
        vec![
            "viewer", "export", "--output", "out.html", "--level", "working",
        ],
    ] {
        let (exit, _, error) = cli(obsolete);
        assert_eq!(exit, CliExit::USAGE);
        assert!(
            error.contains("Usage:") || error.contains("possible values:"),
            "{error}"
        );
        assert!(
            error.contains("tip:")
                || error.contains("unrecognized subcommand")
                || error.contains("invalid value")
                || error.contains("unexpected argument")
        );
    }
}

#[test]
fn bound_repository_journey_needs_no_project_id_and_defaults_to_human_output(
) -> Result<(), Box<dyn std::error::Error>> {
    let temporary = tempfile::tempdir()?;
    let runtime = temporary.path().join("runtime");
    let repository = temporary.path().join("repository");
    fs::create_dir_all(repository.join("src"))?;
    fs::write(
        repository.join("src/main.py"),
        "def main():\n    return 0\n",
    )?;

    let init = Command::new(env!("CARGO_BIN_EXE_volicord"))
        .current_dir(&repository)
        .arg("--runtime")
        .arg(&runtime)
        .args(["init", "CLI Fixture"])
        .output()?;
    assert!(
        init.status.success(),
        "{}",
        String::from_utf8_lossy(&init.stderr)
    );
    let init_text = String::from_utf8(init.stdout)?;
    assert!(init_text.starts_with("Project initialized\n"));
    assert!(!init_text.trim_start().starts_with('{'));

    let status = binary(&runtime, &repository, ["status"])?;
    assert!(
        status.status.success(),
        "{}",
        String::from_utf8_lossy(&status.stderr)
    );
    let status_text = String::from_utf8(status.stdout)?;
    assert!(status_text.starts_with("Project Understanding\n"));
    assert!(status_text.contains("project name: CLI Fixture"));
    assert!(status_text.contains("current work:"));
    assert!(status_text.contains("architecture:"));
    assert!(!status_text.trim_start().starts_with('{'));

    let status_json = binary(&runtime, &repository, ["--json", "status"])?;
    assert!(status_json.status.success());
    let status_json: Value = serde_json::from_slice(&status_json.stdout)?;
    assert_eq!(status_json["operation"], "project_status");
    assert_eq!(status_json["project_name"], "CLI Fixture");
    assert!(status_json["project_id"].is_string());
    assert!(status_json["architecture"].is_object());

    let analyzed = binary(&runtime, &repository, ["analyze"])?;
    assert!(
        analyzed.status.success(),
        "{}",
        String::from_utf8_lossy(&analyzed.stderr)
    );
    let analyzed = String::from_utf8(analyzed.stdout)?;
    assert!(analyzed.starts_with("Repository analysis\n"));
    assert!(analyzed.contains("Scope:"));
    assert!(analyzed.contains("Coverage (static evidence)"));
    assert!(!analyzed.contains("analysis snapshot:"));
    assert!(!analyzed.contains("diagnostic ids:"));

    for command in ["recall", "questions", "decisions"] {
        let result = binary(&runtime, &repository, [command])?;
        assert!(
            result.status.success(),
            "{command}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(!String::from_utf8(result.stdout)?
            .trim_start()
            .starts_with('{'));
    }
    Ok(())
}

#[test]
fn task_groups_keep_portable_privacy_doctor_document_viewer_and_guarded_paths_reachable(
) -> Result<(), Box<dyn std::error::Error>> {
    let temporary = tempfile::tempdir()?;
    let runtime = temporary.path().join("runtime");
    let repository = temporary.path().join("repository");
    fs::create_dir(&repository)?;
    initialize(&runtime, &repository)?;

    for args in [
        vec!["privacy", "status"],
        vec!["doctor", "check"],
        vec!["viewer", "locate"],
        vec!["advanced", "candidates"],
        vec!["advanced", "records", "list"],
    ] {
        let (exit, output, error) = project_cli(&runtime, &repository, args);
        assert_eq!(exit, CliExit::SUCCESS, "{error}");
        assert!(!output.trim_start().starts_with('{'));
    }

    let bundle = temporary.path().join("portable.json");
    let (exit, output, error) = project_cli(
        &runtime,
        &repository,
        vec!["--json", "context", "export", "--output", text(&bundle)?],
    );
    assert_eq!(exit, CliExit::SUCCESS, "{error}");
    let exported: Value = serde_json::from_str(&output)?;
    assert_eq!(exported["operation"], "portable_export");
    assert!(bundle.is_file());

    let (exit, output, error) = project_cli(
        &runtime,
        &repository,
        vec![
            "--json",
            "document",
            "preview",
            "handoff-resume",
            "--language",
            "es",
        ],
    );
    assert_eq!(exit, CliExit::SUCCESS, "{error}");
    let preview: Value = serde_json::from_str(&output)?;
    assert_eq!(preview["outcome"], "unavailable");
    assert_eq!(preview["requested_language"], "es");
    assert!(preview.get("content").is_none());

    let korean_handoff = temporary.path().join("handoff-ko.md");
    let (exit, output, error) = project_cli(
        &runtime,
        &repository,
        vec![
            "--json",
            "--locale",
            "ko",
            "document",
            "export",
            "handoff-resume",
            "--format",
            "markdown",
            "--output",
            text(&korean_handoff)?,
            "--language",
            "ko",
        ],
    );
    assert_eq!(exit, CliExit::SUCCESS, "{error}");
    let exported: Value = serde_json::from_str(&output)?;
    assert_eq!(exported["operation"], "document_export");
    assert_eq!(exported["kind"], "handoff-resume");
    assert_eq!(exported["format"], "markdown");
    assert_eq!(exported["destination"], text(&korean_handoff)?);
    assert!(std::fs::read_to_string(&korean_handoff)?.contains("인계 / 재개 문서"));

    let expiration = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_micros()
        + 60_000_000)
        .to_string();
    let (exit, output, error) = project_cli(
        &runtime,
        &repository,
        vec![
            "--json",
            "advanced",
            "guarded",
            "request",
            "external-publication",
            "--action",
            "publish",
            "--target",
            "registry.example/release",
            "--effect",
            "publish release",
            "--risk",
            "external users can observe it",
            "--expires",
            &expiration,
            "--scope",
            "artifact:release",
        ],
    );
    assert_eq!(exit, CliExit::SUCCESS, "{error}");
    let request: Value = serde_json::from_str(&output)?;
    assert_eq!(request["operation"], "guarded_request");
    assert!(request["effect_fingerprint"].is_string());
    Ok(())
}

#[test]
fn korean_fixed_output_and_actionable_unbound_error_are_available(
) -> Result<(), Box<dyn std::error::Error>> {
    let temporary = tempfile::tempdir()?;
    let runtime = temporary.path().join("runtime");
    let repository = temporary.path().join("repository");
    fs::create_dir(&repository)?;
    initialize(&runtime, &repository)?;

    let (exit, output, error) =
        project_cli(&runtime, &repository, vec!["--locale", "ko", "status"]);
    assert_eq!(exit, CliExit::SUCCESS, "{error}");
    assert!(output.starts_with("프로젝트 이해\n"));
    assert!(output.contains("프로젝트 이름:"));

    let unbound = temporary.path().join("unbound");
    fs::create_dir(&unbound)?;
    let (exit, _, error) = project_cli(&runtime, &unbound, vec!["status"]);
    assert_eq!(exit, CliExit::FAILURE);
    assert!(error.contains("no Project is bound"));
    assert!(error.contains("volicord init"));
    assert!(error.contains("--project PROJECT_ID"));
    Ok(())
}

#[test]
fn repository_selected_candidate_cli_distinguishes_healthy_empty_from_dependency_failures(
) -> Result<(), Box<dyn std::error::Error>> {
    for state in [
        "healthy",
        "older_schema",
        "newer_schema",
        "corrupt",
        "unavailable",
    ] {
        let temporary = tempfile::tempdir()?;
        let runtime = temporary.path().join("runtime");
        let repository = temporary.path().join("repository");
        fs::create_dir(&repository)?;
        let project = initialize(&runtime, &repository)?;
        let candidate_path = RuntimeLayout::new(runtime.clone())?.candidate_store();
        match state {
            "older_schema" | "newer_schema" => {
                Connection::open(&candidate_path)?.execute(
                    "UPDATE metadata SET value = ?1 WHERE key = 'schema_version'",
                    [if state == "older_schema" { "0" } else { "999" }],
                )?;
            }
            "corrupt" => {
                Connection::open(&candidate_path)?.execute("DROP TABLE candidates", [])?;
            }
            "unavailable" => {
                fs::remove_file(&candidate_path)?;
                fs::create_dir(&candidate_path)?;
            }
            _ => {}
        }

        // The current CLI exposes projection degradation, including when the
        // Candidate array is empty, through both structured and human output.
        let result = binary(&runtime, &repository, ["--json", "advanced", "candidates"])?;
        assert!(result.status.success(), "{state}: {result:?}");
        let inspected: Value = serde_json::from_slice(&result.stdout)?;
        assert_eq!(inspected["operation"], "candidate_inspection");
        assert_eq!(inspected["project_id"], project["project_id"]);
        assert_eq!(inspected["candidates"], json!([]));
        let health = if state == "healthy" {
            "complete"
        } else {
            "degraded"
        };
        assert_eq!(inspected["health"], health, "{state}: {inspected}");

        let human = binary(&runtime, &repository, ["advanced", "candidates"])?;
        assert!(human.status.success(), "{state}: {human:?}");
        let human = String::from_utf8(human.stdout)?;
        assert!(
            human.contains(&format!("health: {health}")),
            "{state}: {human}"
        );
    }
    Ok(())
}

#[test]
fn concurrent_repository_selected_cli_writers_preserve_committed_sources(
) -> Result<(), Box<dyn std::error::Error>> {
    let temporary = tempfile::tempdir()?;
    let runtime = temporary.path().join("runtime");
    let repository = temporary.path().join("repository");
    fs::create_dir(&repository)?;
    let project = initialize(&runtime, &repository)?;
    const WRITERS: usize = 12;
    let barrier = Barrier::new(WRITERS);
    let results = thread::scope(|scope| {
        let handles = (0..WRITERS)
            .map(|index| {
                let (runtime, repository, barrier) = (&runtime, &repository, &barrier);
                scope.spawn(move || {
                    let session = format!("concurrent-session-{index}");
                    let text = format!("Concurrent CLI Source {index}");
                    barrier.wait();
                    let result = binary(
                        runtime,
                        repository,
                        [
                            "--json",
                            "advanced",
                            "records",
                            "source",
                            "--host",
                            "cli-test",
                            "--session",
                            &session,
                            "--text",
                            &text,
                        ],
                    );
                    (text, result)
                })
            })
            .collect::<Vec<_>>();
        handles
            .into_iter()
            .map(|handle| handle.join())
            .collect::<Vec<_>>()
    });
    let mut committed = Vec::new();
    for result in results {
        let (text, result) = result.map_err(|_| "CLI writer thread panicked")?;
        let result = result?;
        assert!(result.status.success(), "{text}: {result:?}");
        let recorded: Value = serde_json::from_slice(&result.stdout)?;
        assert_eq!(recorded["operation"], "canonical_user_source");
        assert_eq!(recorded["record_kind"], "source");
        assert_eq!(recorded["revision"], 1);
        assert_eq!(recorded["replayed"], false);
        let identity = recorded["identity"]
            .as_str()
            .ok_or("missing Source identity")?;
        committed.push((identity.to_owned(), text));
    }
    assert_eq!(
        committed
            .iter()
            .map(|(id, _)| id)
            .collect::<BTreeSet<_>>()
            .len(),
        WRITERS
    );

    // A fresh product process reads canonical records after all writers exit.
    let result = binary(
        &runtime,
        &repository,
        ["--json", "advanced", "records", "list"],
    )?;
    assert!(result.status.success(), "{result:?}");
    let inspected: Value = serde_json::from_slice(&result.stdout)?;
    assert_eq!(inspected["operation"], "canonical_inspect");
    assert_eq!(inspected["project_id"], project["project_id"]);
    assert_eq!(inspected["health"], "complete");
    let records = inspected["records"]
        .as_array()
        .ok_or("missing canonical records")?;
    for (identity, text) in committed {
        let matches = records
            .iter()
            .filter(|record| record["identity"] == identity)
            .collect::<Vec<_>>();
        assert_eq!(
            matches.len(),
            1,
            "committed Source {identity} missing or duplicated"
        );
        assert_eq!(matches[0]["kind"], "source");
        assert_eq!(matches[0]["revision"], 1);
        assert!(matches[0]["summary"]
            .as_str()
            .is_some_and(|summary| summary.contains(&text)));
    }
    Ok(())
}

fn initialize(runtime: &Path, repository: &Path) -> Result<Value, Box<dyn std::error::Error>> {
    let (exit, output, error) =
        project_cli(runtime, repository, vec!["--json", "init", "CLI Fixture"]);
    if exit != CliExit::SUCCESS {
        return Err(error.into());
    }
    Ok(serde_json::from_str(&output)?)
}

fn binary<const N: usize>(
    runtime: &Path,
    repository: &Path,
    args: [&str; N],
) -> Result<std::process::Output, std::io::Error> {
    Command::new(env!("CARGO_BIN_EXE_volicord"))
        .current_dir(repository)
        .arg("--runtime")
        .arg(runtime)
        .args(args)
        .output()
}

fn project_cli<'a>(
    runtime: &'a Path,
    repository: &'a Path,
    args: Vec<&'a str>,
) -> (CliExit, String, String) {
    let mut command = vec![
        "--runtime",
        runtime.to_str().expect("runtime UTF-8"),
        "--repository",
        repository.to_str().expect("repository UTF-8"),
    ];
    command.extend(args);
    cli(command)
}

fn cli<I, S>(args: I) -> (CliExit, String, String)
where
    I: IntoIterator<Item = S>,
    S: Into<std::ffi::OsString>,
{
    let mut output = Vec::new();
    let mut error = Vec::new();
    let exit = run_cli(args, &mut output, &mut error);
    (
        exit,
        String::from_utf8(output).expect("stdout UTF-8"),
        String::from_utf8(error).expect("stderr UTF-8"),
    )
}

fn text(path: &Path) -> Result<&str, Box<dyn std::error::Error>> {
    path.to_str().ok_or_else(|| "path is not UTF-8".into())
}

#[test]
fn analyze_human_summary_and_complete_json_keep_operation_and_failure_semantics(
) -> Result<(), Box<dyn std::error::Error>> {
    for (language, files) in [
        (
            "rust",
            vec![("src/lib.rs", "pub fn answer() -> i32 { 42 }")],
        ),
        (
            "python",
            vec![("main.py", "def answer():\n    return 42\n")],
        ),
        (
            "type_script",
            vec![
                ("src/client.ts", "export function answer() { return 42; }"),
                ("worker.py", "def worker():\n    return 42\n"),
                ("native/query.c", "int query(void) { return 42; }"),
            ],
        ),
    ] {
        let temporary = tempfile::tempdir()?;
        let root = temporary.path().join("repository");
        let runtime = temporary.path().join("runtime");
        for (path, contents) in files {
            let file = root.join(path);
            fs::create_dir_all(file.parent().ok_or("file parent")?)?;
            fs::write(file, contents)?;
        }
        initialize(&runtime, &root)?;
        let human = binary(&runtime, &root, ["analyze"])?;
        assert_eq!(human.status.code(), Some(0));
        assert!(human.stderr.is_empty());
        let text = String::from_utf8(human.stdout)?;
        for expected in [
            "Scope:",
            "Operation: partial",
            "Results:",
            "Coverage (static evidence)",
            "Affected capability scopes:",
            "Next:",
            "--json",
        ] {
            assert!(text.contains(expected), "missing {expected}: {text}");
        }
        let display_language = match language {
            "rust" => "Rust",
            "python" => "Python",
            _ => "TypeScript",
        };
        assert!(text.contains(display_language));
        for audit in [
            "operation id:",
            "analysis snapshot:",
            "diagnostic ids:",
            "adapter:",
            "agent_assisted",
        ] {
            assert!(!text.contains(audit), "ordinary audit clutter {audit}");
        }
        let machine = binary(&runtime, &root, ["--json", "analyze"])?;
        assert_eq!(machine.status.code(), Some(0));
        let value: Value = serde_json::from_slice(&machine.stdout)?;
        assert_eq!(value["state"], "partial");
        assert_eq!(value["diagnostics_omitted_count"], 0);
        assert!(value["analysis_snapshot"].is_string());
        assert!(value["operation_id"].is_string());
        assert!(value["capability_reports"]
            .as_array()
            .ok_or("reports")?
            .iter()
            .all(|r| r["coverage_scopes"].is_object()
                && r["diagnostic_ids"].as_array().is_some_and(
                    |ids| ids.len() == r["diagnostic_count"].as_u64().unwrap_or(0) as usize
                )));
        assert!(value["human_summary"]["coverage"]
            .as_array()
            .ok_or("coverage")?
            .iter()
            .any(|r| r["language"]["kind"] == language));
        let failed = binary(
            &runtime,
            &root,
            ["--json", "analyze", "--exclude", "../outside"],
        )?;
        assert_eq!(failed.status.code(), Some(1));
        assert!(!failed.stderr.is_empty());
    }
    Ok(())
}

#[test]
fn analyze_json_keeps_all_parse_diagnostics_above_host_bounds(
) -> Result<(), Box<dyn std::error::Error>> {
    let temporary = tempfile::tempdir()?;
    let root = temporary.path().join("repository");
    let runtime = temporary.path().join("runtime");
    fs::create_dir_all(root.join("src"))?;
    for i in 0..80 {
        fs::write(
            root.join(format!("src/broken_{i:02}.rs")),
            "pub fn broken( {\n",
        )?;
    }
    initialize(&runtime, &root)?;
    let machine = binary(&runtime, &root, ["--json", "analyze"])?;
    assert_eq!(machine.status.code(), Some(0)); // Existing partial-result exit.
    let value: Value = serde_json::from_slice(&machine.stdout)?;
    let diagnostics = value["diagnostics"].as_array().ok_or("diagnostics")?;
    assert!(
        diagnostics.len() > 64,
        "insufficient adversarial diagnostics: {}",
        diagnostics.len()
    );
    assert_eq!(value["diagnostics_omitted_count"], 0);
    let identities = diagnostics
        .iter()
        .map(|d| d["identity"].as_str().ok_or("diagnostic identity"))
        .collect::<Result<BTreeSet<_>, _>>()?;
    for report in value["capability_reports"].as_array().ok_or("reports")? {
        let ids = report["diagnostic_ids"].as_array().ok_or("ids")?;
        assert_eq!(
            ids.len() as u64,
            report["diagnostic_count"].as_u64().ok_or("count")?
        );
        assert!(ids
            .iter()
            .all(|id| id.as_str().is_some_and(|id| identities.contains(id))));
    }
    for i in 0..80 {
        let path = format!("src/broken_{i:02}.rs");
        assert!(
            diagnostics
                .iter()
                .any(|d| d["affected_area"]["path"] == path),
            "lost diagnostic for {path}"
        );
    }
    let human = binary(&runtime, &root, ["analyze"])?;
    assert_eq!(human.status.code(), Some(0));
    let text = String::from_utf8(human.stdout)?;
    assert!(text.contains("structural partial"));
    assert!(text.contains("Affected areas:"));
    assert!(text.contains("src/broken_"));
    Ok(())
}
