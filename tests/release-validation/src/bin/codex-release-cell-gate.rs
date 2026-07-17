use std::{env, process::ExitCode};

use volicord_release_validation_tests::gate::{
    capture_candidate_cell, checked_in_cell_statuses, run_checked_in_cell_gate,
};
use volicord_types::{CodexReleaseCellStatus, PlatformEnvironment, ReleaseTargetTriple};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Codex release-cell gate failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if arguments == ["--status"] {
        for (cell, status) in checked_in_cell_statuses().map_err(|error| error.to_string())? {
            println!(
                "target={} platform={} profile={} status={}",
                cell.target_triple,
                cell.platform_environment.as_str(),
                cell.integration_profile.as_str(),
                status_name(status)
            );
        }
        return Ok(());
    }
    if arguments == ["--help"] || arguments == ["-h"] {
        println!(
            "Usage:\n  codex-release-cell-gate --status\n  codex-release-cell-gate --capture-candidate <target-triple> --platform <linux|macos|native_windows|wsl2>\n  codex-release-cell-gate --target <target-triple> --platform <linux|macos|native_windows|wsl2>"
        );
        return Ok(());
    }
    let (mode, target_triple, platform) = match arguments.as_slice() {
        [capture, target, platform_flag, platform]
            if capture == "--capture-candidate" && platform_flag == "--platform" =>
        {
            (Mode::Capture, parse_target(target)?, parse_platform(platform)?)
        }
        [target_flag, target, platform_flag, platform]
            if target_flag == "--target" && platform_flag == "--platform" =>
        {
            (Mode::Gate, parse_target(target)?, parse_platform(platform)?)
        }
        _ => return Err(
            "expected exactly --status, --capture-candidate <target> --platform <platform>, or --target <target> --platform <platform>".to_owned(),
        ),
    };
    match mode {
        Mode::Gate => {
            let report = run_checked_in_cell_gate(target_triple, platform)
                .map_err(|error| error.to_string())?;
            println!(
                "target={} platform={} status=passed codex_sha256={} volicord_sha256={} evidence_sha256={} scenarios={}",
                report.target_triple,
                report.platform.as_str(),
                report.codex_artifact_digest,
                report.volicord_artifact_digest,
                report.evidence_digest,
                report.scenario_count
            );
        }
        Mode::Capture => {
            let report = capture_candidate_cell(target_triple, platform)
                .map_err(|error| error.to_string())?;
            println!(
                "target={} platform={} status={} candidate={} codex_sha256={} volicord_sha256={} evidence_sha256={} scenarios={}",
                report.target_triple,
                report.platform.as_str(),
                report.validation_result.as_str(),
                report.candidate_path.display(),
                report.codex_artifact_digest,
                report.volicord_artifact_digest,
                report.evidence_digest,
                report.scenario_count
            );
            if report.validation_result != volicord_types::CodexReleaseValidationResult::Passed {
                return Err(format!(
                    "captured candidate has non-passing status {}",
                    report.validation_result.as_str()
                ));
            }
        }
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum Mode {
    Capture,
    Gate,
}

fn parse_platform(value: &str) -> Result<PlatformEnvironment, String> {
    match value {
        "linux" => Ok(PlatformEnvironment::Linux),
        "macos" => Ok(PlatformEnvironment::Macos),
        "native_windows" => Ok(PlatformEnvironment::NativeWindows),
        "wsl2" => Ok(PlatformEnvironment::Wsl2),
        _ => Err(format!("unknown release platform {value}")),
    }
}

fn parse_target(value: &str) -> Result<ReleaseTargetTriple, String> {
    value
        .parse()
        .map_err(|_| format!("unknown release target {value}"))
}

fn status_name(status: CodexReleaseCellStatus) -> &'static str {
    match status {
        CodexReleaseCellStatus::Passed => "passed",
        CodexReleaseCellStatus::Failed => "failed",
        CodexReleaseCellStatus::Unavailable => "unavailable",
        CodexReleaseCellStatus::NotRun => "not_run",
    }
}
