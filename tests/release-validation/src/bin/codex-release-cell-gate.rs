use std::{env, process::ExitCode};

use volicord_release_validation_tests::gate::{
    capture_candidate_cell, checked_in_platform_statuses, run_checked_in_cell_gate,
};
use volicord_types::{CodexReleasePlatformStatus, PlatformEnvironment};

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
        for (platform, status) in
            checked_in_platform_statuses().map_err(|error| error.to_string())?
        {
            println!("{}={}", platform.as_str(), status_name(status));
        }
        return Ok(());
    }
    if arguments == ["--help"] || arguments == ["-h"] {
        println!(
            "Usage:\n  codex-release-cell-gate --status\n  codex-release-cell-gate --capture-candidate <linux|macos|native_windows|wsl2>\n  codex-release-cell-gate --platform <linux|macos|native_windows|wsl2>"
        );
        return Ok(());
    }
    let (mode, platform) = match arguments.as_slice() {
        [flag, value] if flag == "--capture-candidate" => (Mode::Capture, parse_platform(value)?),
        [flag, value] if flag == "--platform" => (Mode::Gate, parse_platform(value)?),
        _ => return Err(
            "expected exactly --status, --capture-candidate <platform>, or --platform <platform>"
                .to_owned(),
        ),
    };
    match mode {
        Mode::Gate => {
            let report = run_checked_in_cell_gate(platform).map_err(|error| error.to_string())?;
            println!(
                "platform={} status=passed codex_sha256={} volicord_sha256={} evidence_sha256={} scenarios={}",
                report.platform.as_str(),
                report.codex_artifact_digest,
                report.volicord_artifact_digest,
                report.evidence_digest,
                report.scenario_count
            );
        }
        Mode::Capture => {
            let report = capture_candidate_cell(platform).map_err(|error| error.to_string())?;
            println!(
                "platform={} status={} candidate={} codex_sha256={} volicord_sha256={} evidence_sha256={} scenarios={}",
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

fn status_name(status: CodexReleasePlatformStatus) -> &'static str {
    match status {
        CodexReleasePlatformStatus::Passed => "passed",
        CodexReleasePlatformStatus::Failed => "failed",
        CodexReleasePlatformStatus::Unavailable => "unavailable",
        CodexReleasePlatformStatus::NotRun => "not_run",
    }
}
