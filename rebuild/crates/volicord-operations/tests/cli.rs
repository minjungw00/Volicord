use serde_json::Value;
use std::{fs, path::Path, process::Command};
use volicord_operations::{run_cli, CliExit};

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
    ] {
        let (exit, _, error) = cli(obsolete);
        assert_eq!(exit, CliExit::USAGE);
        assert!(error.contains("Usage:"), "{error}");
        assert!(error.contains("tip:") || error.contains("unrecognized subcommand"));
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
    assert!(analyzed.contains("analysis snapshot:"));

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
