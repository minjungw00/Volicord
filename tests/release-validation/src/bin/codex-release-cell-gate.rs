use std::{env, fs, path::Path, process::ExitCode};

use volicord_release_validation_tests::catalog::{
    generate_support_entry, parse_declared_capabilities, serialize_support_entry,
};
use volicord_release_validation_tests::gate::{
    capture_candidate_cell, checked_in_cell_statuses, run_checked_in_cell_gate,
};
use volicord_release_validation_tests::io::ValidationContext;
use volicord_release_validation_tests::pipeline::{
    verify_build_artifact, verify_cell_evidence, verify_publish_inputs,
    write_verified_release_index,
};
use volicord_types::{
    CodexReleaseCellStatus, IntegrationProfile, PlatformEnvironment, ReleaseTargetTriple,
};

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
            "Usage:\n  codex-release-cell-gate --status\n  codex-release-cell-gate --generate-support-entry --codex-path <path> --target <target-triple> --platform <linux|macos|native_windows|wsl2> --profile record --capabilities <comma-delimited-capabilities>\n  codex-release-cell-gate --capture-candidate <target-triple> --platform <linux|macos|native_windows|wsl2>\n  codex-release-cell-gate --target <target-triple> --platform <linux|macos|native_windows|wsl2>\n  codex-release-cell-gate --verify-build-artifact --build-artifact-dir <path> --source-revision <git-object-id> --target <target-triple>\n  codex-release-cell-gate --verify-cell-evidence --build-artifact-dir <path> --evidence-artifact-dir <path> --source-revision <git-object-id> --target <target-triple> --platform <linux|macos|native_windows|wsl2>\n  codex-release-cell-gate --verify-publish-evidence --build-root <path> --evidence-root <path> --source-revision <git-object-id> --run-id <id> --run-attempt <attempt> --verified-index-output <path>"
        );
        return Ok(());
    }
    match arguments.as_slice() {
        [generate, codex_flag, codex_path, target_flag, target, platform_flag, platform, profile_flag, profile, capabilities_flag, capabilities]
            if generate == "--generate-support-entry"
                && codex_flag == "--codex-path"
                && target_flag == "--target"
                && platform_flag == "--platform"
                && profile_flag == "--profile"
                && capabilities_flag == "--capabilities" =>
        {
            let current_directory = env::current_dir().map_err(|error| error.to_string())?;
            let context = ValidationContext::from_process(&current_directory)
                .map_err(|error| error.to_string())?;
            let canonical_codex_path = fs::canonicalize(codex_path)
                .map_err(|error| format!("cannot canonicalize Codex artifact path: {error}"))?;
            let entry = generate_support_entry(
                &context,
                &canonical_codex_path,
                parse_target(target)?,
                parse_platform(platform)?,
                parse_profile(profile)?,
                &parse_declared_capabilities(capabilities).map_err(|error| error.to_string())?,
            )
            .map_err(|error| error.to_string())?;
            let bytes = serialize_support_entry(&entry).map_err(|error| error.to_string())?;
            println!(
                "{}",
                String::from_utf8(bytes)
                    .map_err(|_| "canonical support entry is not UTF-8".to_owned())?
            );
            return Ok(());
        }
        [verify, build_flag, build_directory, revision_flag, source_revision, target_flag, target]
            if verify == "--verify-build-artifact"
                && build_flag == "--build-artifact-dir"
                && revision_flag == "--source-revision"
                && target_flag == "--target" =>
        {
            let report = verify_build_artifact(
                Path::new(build_directory),
                parse_target(target)?,
                source_revision,
            )
            .map_err(|error| error.to_string())?;
            println!(
                "target={} source_revision={} volicord_sha256={} build_artifact=verified",
                report.target_triple, source_revision, report.binary_sha256
            );
            return Ok(());
        }
        [verify, build_flag, build_directory, evidence_flag, evidence_directory, revision_flag, source_revision, target_flag, target, platform_flag, platform]
            if verify == "--verify-cell-evidence"
                && build_flag == "--build-artifact-dir"
                && evidence_flag == "--evidence-artifact-dir"
                && revision_flag == "--source-revision"
                && target_flag == "--target"
                && platform_flag == "--platform" =>
        {
            let report = verify_cell_evidence(
                Path::new(build_directory),
                Path::new(evidence_directory),
                source_revision,
                parse_target(target)?,
                parse_platform(platform)?,
            )
            .map_err(|error| error.to_string())?;
            println!(
                "target={} platform={} status=passed volicord_sha256={} evidence_sha256={} retained_evidence=verified",
                report.target_triple,
                report.platform_environment.as_str(),
                report.binary_sha256,
                report.evidence_sha256
            );
            return Ok(());
        }
        [verify, build_flag, build_root, evidence_flag, evidence_root, revision_flag, source_revision, run_id_flag, run_id, run_attempt_flag, run_attempt, index_flag, index_output]
            if verify == "--verify-publish-evidence"
                && build_flag == "--build-root"
                && evidence_flag == "--evidence-root"
                && revision_flag == "--source-revision"
                && run_id_flag == "--run-id"
                && run_attempt_flag == "--run-attempt"
                && index_flag == "--verified-index-output" =>
        {
            let index = verify_publish_inputs(
                Path::new(build_root),
                Path::new(evidence_root),
                source_revision,
                run_id,
                run_attempt,
            )
            .map_err(|error| error.to_string())?;
            write_verified_release_index(Path::new(index_output), &index)
                .map_err(|error| error.to_string())?;
            for build in &index.published_artifacts {
                println!(
                    "target={} volicord_sha256={} build_artifact=verified",
                    build.target_triple, build.binary_sha256
                );
            }
            for cell in &index.release_evidence {
                println!(
                    "target={} platform={} status=passed volicord_sha256={} evidence_sha256={}",
                    cell.target_triple,
                    cell.platform_environment.as_str(),
                    cell.volicord_artifact_digest,
                    cell.evidence_digest
                );
            }
            println!(
                "source_revision={} verified_release_index={}",
                index.source_revision, index_output
            );
            return Ok(());
        }
        _ => {}
    }
    let (mode, target_triple, platform) = match arguments.as_slice() {
        [capture, target, platform_flag, platform]
            if capture == "--capture-candidate" && platform_flag == "--platform" =>
        {
            (
                Mode::Capture,
                parse_target(target)?,
                parse_platform(platform)?,
            )
        }
        [target_flag, target, platform_flag, platform]
            if target_flag == "--target" && platform_flag == "--platform" =>
        {
            (Mode::Gate, parse_target(target)?, parse_platform(platform)?)
        }
        _ => {
            return Err(
                "invalid arguments; use --help for the exact release gate commands".to_owned(),
            )
        }
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

fn parse_profile(value: &str) -> Result<IntegrationProfile, String> {
    match value {
        "record" => Ok(IntegrationProfile::Record),
        _ => Err(format!("unknown integration profile {value}")),
    }
}

fn status_name(status: CodexReleaseCellStatus) -> &'static str {
    match status {
        CodexReleaseCellStatus::Passed => "passed",
        CodexReleaseCellStatus::Failed => "failed",
        CodexReleaseCellStatus::Unavailable => "unavailable",
        CodexReleaseCellStatus::NotRun => "not_run",
    }
}
