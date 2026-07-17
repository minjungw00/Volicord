//! Executable producer and blocking gate for exact Codex release cells.

mod platform;
mod scenarios;

use std::{
    env, fs,
    path::PathBuf,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant, SystemTime},
};

use chrono::{DateTime, SecondsFormat, Utc};
use volicord_types::{
    compute_codex_release_evidence_digest, CodexReleaseCell, CodexReleaseScenarioStatus,
    CodexReleaseValidationEvidence, CodexReleaseValidationStatus, IntegrationProfile,
    PlatformEnvironment, PlatformReleaseStatus, CODEX_RELEASE_PLATFORMS,
    FIRST_RELEASE_CODEX_CAPABILITIES, PINNED_WSL2_DISTRIBUTION_NAME,
};

use crate::{
    contracts::{checked_in_manifest, load_manifest, CHECKED_IN_CODEX_RELEASE_MANIFEST_PATH},
    error::{ValidationError, ValidationResult},
    io::{write_json_create_new, ValidationContext, MAX_MANIFEST_JSON_BYTES},
};

use self::{
    platform::{
        collect_runner_coordinate, hash_artifact, probe_executable,
        validate_actual_runner_coordinate, validate_gate_paths, validate_process_boundary,
    },
    scenarios::{capture_scenario_catalog, run_checked_scenario_catalog},
};

/// Process-environment variables consumed by the live cell producer and gate.
pub const CODEX_PATH_ENV: &str = "VOLICORD_CODEX_RELEASE_CODEX_PATH";
pub const VOLICORD_PATH_ENV: &str = "VOLICORD_CODEX_RELEASE_VOLICORD_PATH";
pub const SCENARIO_DRIVER_ENV: &str = "VOLICORD_CODEX_RELEASE_SCENARIO_DRIVER";
pub const EVIDENCE_DIRECTORY_ENV: &str = "VOLICORD_CODEX_RELEASE_EVIDENCE_DIR";
pub const WORK_ROOT_ENV: &str = "VOLICORD_CODEX_RELEASE_WORK_ROOT";
pub const RUNTIME_HOME_ENV: &str = "VOLICORD_HOME";
pub const ENVIRONMENT_IMAGE_ENV: &str = "VOLICORD_CODEX_RELEASE_ENVIRONMENT_IMAGE";
pub const WSL2_DISTRIBUTION_ENV: &str = "VOLICORD_CODEX_RELEASE_WSL2_DISTRIBUTION";
pub const CANDIDATE_CELL_PATH_ENV: &str = "VOLICORD_CODEX_RELEASE_CANDIDATE_CELL_PATH";

/// Successful live-gate summary for one independently executed cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateReport {
    pub platform: PlatformEnvironment,
    pub artifact_digest: String,
    pub volicord_artifact_digest: String,
    pub evidence_digest: String,
    pub scenario_count: usize,
}

/// Summary for a newly written, external one-cell review candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureReport {
    pub platform: PlatformEnvironment,
    pub status: CodexReleaseValidationStatus,
    pub candidate_path: PathBuf,
    pub artifact_digest: String,
    pub volicord_artifact_digest: String,
    pub evidence_digest: String,
    pub scenario_count: usize,
}

#[derive(Debug)]
pub(super) struct GateConfiguration {
    pub(super) codex_path: String,
    pub(super) volicord_path: String,
    pub(super) scenario_driver: PathBuf,
    pub(super) evidence_directory: PathBuf,
    pub(super) work_root: String,
    pub(super) runtime_home: String,
    pub(super) environment_image: String,
    pub(super) runner_id: String,
    pub(super) wsl2_distribution: Option<String>,
}

/// Runs the blocking gate for one platform against only the canonical checked-in manifest.
pub fn run_checked_in_cell_gate(platform: PlatformEnvironment) -> ValidationResult<GateReport> {
    validate_process_boundary(platform)?;
    let context = validation_context(platform)?;
    let embedded_manifest = load_matching_checked_in_manifest(&context)?;

    let Some(cell) = embedded_manifest
        .cells()
        .iter()
        .find(|cell| cell.platform == platform)
    else {
        return Err(ValidationError::new(format!(
            "Codex release platform {} has checked-in status not_run; an exact passing cell is required",
            platform.as_str()
        )));
    };
    if cell.validation_evidence.status != CodexReleaseValidationStatus::Passed {
        return Err(ValidationError::new(format!(
            "Codex release platform {} has checked-in status {}; an exact passing cell is required",
            platform.as_str(),
            release_status_name(embedded_manifest.platform_status(platform))
        )));
    }

    let configuration = GateConfiguration::from_process(platform)?;
    validate_gate_paths(&context, platform, &configuration)?;
    validate_actual_runner_coordinate(cell, platform, &configuration)?;

    let codex_before = hash_artifact(
        &context,
        platform,
        &configuration.codex_path,
        &configuration,
    )?;
    let volicord_before = hash_artifact(
        &context,
        platform,
        &configuration.volicord_path,
        &configuration,
    )?;
    if codex_before != cell.artifact_digest {
        return Err(ValidationError::new(format!(
            "actual Codex executable digest does not match the exact checked-in {} cell",
            platform.as_str()
        )));
    }
    if volicord_before != cell.validation_evidence.volicord_artifact_digest {
        return Err(ValidationError::new(format!(
            "actual Volicord executable digest does not match the exact checked-in {} cell",
            platform.as_str()
        )));
    }

    probe_executable(platform, &configuration.codex_path, &configuration)?;
    probe_executable(platform, &configuration.volicord_path, &configuration)?;
    run_checked_scenario_catalog(&context, platform, cell, &configuration)?;

    let codex_after = hash_artifact(
        &context,
        platform,
        &configuration.codex_path,
        &configuration,
    )?;
    let volicord_after = hash_artifact(
        &context,
        platform,
        &configuration.volicord_path,
        &configuration,
    )?;
    if codex_after != codex_before {
        return Err(ValidationError::new(
            "actual Codex executable bytes changed during the release cell",
        ));
    }
    if volicord_after != volicord_before {
        return Err(ValidationError::new(
            "actual Volicord executable bytes changed during the release cell",
        ));
    }

    Ok(GateReport {
        platform,
        artifact_digest: codex_after,
        volicord_artifact_digest: volicord_after,
        evidence_digest: cell.validation_evidence.evidence_digest.clone(),
        scenario_count: cell.validation_evidence.scenario_results.len(),
    })
}

/// Executes one qualifying platform attempt and writes a new external review candidate.
///
/// This producer never edits or promotes the canonical checked-in manifest. Release
/// publication continues to require `run_checked_in_cell_gate` against a reviewed cell.
pub fn capture_candidate_cell(platform: PlatformEnvironment) -> ValidationResult<CaptureReport> {
    validate_process_boundary(platform)?;
    let context = validation_context(platform)?;
    // Refuse to produce from a build whose embedded manifest and checkout disagree,
    // even though capture itself does not require a pre-existing cell.
    load_matching_checked_in_manifest(&context)?;

    let configuration = GateConfiguration::from_process(platform)?;
    validate_gate_paths(&context, platform, &configuration)?;
    let runner = collect_runner_coordinate(platform, &configuration)?;
    let candidate_path = PathBuf::from(required_environment(CANDIDATE_CELL_PATH_ENV)?);
    context.validate_new_output(&candidate_path)?;
    if candidate_path.starts_with(&configuration.evidence_directory)
        || (platform != PlatformEnvironment::Wsl2
            && (candidate_path.starts_with(PathBuf::from(&configuration.work_root))
                || candidate_path.starts_with(PathBuf::from(&configuration.runtime_home))))
    {
        return Err(ValidationError::new(
            "candidate cell path must be separate from evidence, work, and runtime roots",
        ));
    }

    let codex_before = hash_artifact(
        &context,
        platform,
        &configuration.codex_path,
        &configuration,
    )?;
    let volicord_before = hash_artifact(
        &context,
        platform,
        &configuration.volicord_path,
        &configuration,
    )?;
    probe_executable(platform, &configuration.codex_path, &configuration)?;
    probe_executable(platform, &configuration.volicord_path, &configuration)?;
    let scenario_results = capture_scenario_catalog(
        &context,
        platform,
        &codex_before,
        &volicord_before,
        &configuration,
    )?;

    let codex_after = hash_artifact(
        &context,
        platform,
        &configuration.codex_path,
        &configuration,
    )?;
    let volicord_after = hash_artifact(
        &context,
        platform,
        &configuration.volicord_path,
        &configuration,
    )?;
    if codex_after != codex_before {
        return Err(ValidationError::new(
            "actual Codex executable bytes changed during candidate capture",
        ));
    }
    if volicord_after != volicord_before {
        return Err(ValidationError::new(
            "actual Volicord executable bytes changed during candidate capture",
        ));
    }

    let status = aggregate_scenario_status(&scenario_results)?;
    let observed_at =
        DateTime::<Utc>::from(SystemTime::now()).to_rfc3339_opts(SecondsFormat::AutoSi, true);
    let capabilities = FIRST_RELEASE_CODEX_CAPABILITIES.to_vec();
    let mut evidence = CodexReleaseValidationEvidence {
        status,
        artifact_digest: codex_after.clone(),
        platform,
        observed_capabilities: capabilities.clone(),
        integration_profile: IntegrationProfile::Record,
        volicord_artifact_digest: volicord_after.clone(),
        runner,
        scenario_results,
        evidence_digest: String::new(),
        observed_at,
    };
    evidence.evidence_digest =
        compute_codex_release_evidence_digest(&evidence).map_err(|error| {
            ValidationError::new(format!("cannot digest candidate evidence: {error}"))
        })?;
    let cell = CodexReleaseCell {
        artifact_digest: codex_after.clone(),
        platform,
        observed_capabilities: capabilities,
        integration_profile: IntegrationProfile::Record,
        validation_evidence: evidence,
    };
    write_json_create_new(
        &context,
        &candidate_path,
        &vec![cell.clone()],
        MAX_MANIFEST_JSON_BYTES,
    )?;
    let verified = load_manifest(&candidate_path).map_err(|error| {
        ValidationError::new(format!(
            "new candidate manifest failed strict verification at {}: {error}",
            candidate_path.display()
        ))
    })?;
    if verified.cells() != std::slice::from_ref(&cell) {
        return Err(ValidationError::new(
            "new candidate manifest does not round-trip to its captured cell",
        ));
    }

    Ok(CaptureReport {
        platform,
        status,
        candidate_path,
        artifact_digest: codex_after,
        volicord_artifact_digest: volicord_after,
        evidence_digest: cell.validation_evidence.evidence_digest,
        scenario_count: cell.validation_evidence.scenario_results.len(),
    })
}

fn aggregate_scenario_status(
    results: &[volicord_types::CodexReleaseScenarioResult],
) -> ValidationResult<CodexReleaseValidationStatus> {
    let mut earlier_terminal_outcome = false;
    for result in results {
        if result.status == CodexReleaseScenarioStatus::NotRun && !earlier_terminal_outcome {
            return Err(ValidationError::new(
                "scenario catalog contains not_run before any failed or unavailable result",
            ));
        }
        earlier_terminal_outcome |= matches!(
            result.status,
            CodexReleaseScenarioStatus::Failed | CodexReleaseScenarioStatus::Unavailable
        );
    }
    if results
        .iter()
        .any(|result| result.status == CodexReleaseScenarioStatus::Failed)
    {
        return Ok(CodexReleaseValidationStatus::Failed);
    }
    if results
        .iter()
        .any(|result| result.status == CodexReleaseScenarioStatus::Unavailable)
    {
        return Ok(CodexReleaseValidationStatus::Unavailable);
    }
    if results
        .iter()
        .all(|result| result.status == CodexReleaseScenarioStatus::Passed)
    {
        return Ok(CodexReleaseValidationStatus::Passed);
    }
    Err(ValidationError::new(
        "scenario catalog has no failed or unavailable result explaining not_run",
    ))
}

fn validation_context(platform: PlatformEnvironment) -> ValidationResult<ValidationContext> {
    let current_directory = env::current_dir()?;
    if platform == PlatformEnvironment::Wsl2 {
        // In this cell `VOLICORD_HOME` is a Linux path inside the selected
        // distribution, not a native-Windows exclusion path.
        ValidationContext::from_process_environment(
            &current_directory,
            None,
            env::var_os("HOME"),
            env::var_os("USERPROFILE"),
        )
    } else {
        ValidationContext::from_process(&current_directory)
    }
}

fn load_matching_checked_in_manifest(
    context: &ValidationContext,
) -> ValidationResult<volicord_types::CodexReleaseManifest> {
    let manifest_path = context
        .source_checkout()
        .join(CHECKED_IN_CODEX_RELEASE_MANIFEST_PATH);
    let embedded_manifest = checked_in_manifest().map_err(|error| {
        ValidationError::new(format!(
            "embedded Codex release manifest is invalid: {error}"
        ))
    })?;
    let disk_manifest = load_manifest(&manifest_path).map_err(|error| {
        ValidationError::new(format!(
            "checked-in Codex release manifest is invalid at {}: {error}",
            manifest_path.display()
        ))
    })?;
    if embedded_manifest != disk_manifest {
        return Err(ValidationError::new(
            "compiled and on-disk checked-in Codex release manifests differ",
        ));
    }
    Ok(embedded_manifest)
}

impl GateConfiguration {
    fn from_process(platform: PlatformEnvironment) -> ValidationResult<Self> {
        let wsl2_distribution = if platform == PlatformEnvironment::Wsl2 {
            let value = required_environment(WSL2_DISTRIBUTION_ENV)?;
            if value != PINNED_WSL2_DISTRIBUTION_NAME {
                return Err(ValidationError::new(format!(
                    "{WSL2_DISTRIBUTION_ENV} must be {PINNED_WSL2_DISTRIBUTION_NAME} for the first-release WSL2 cell"
                )));
            }
            Some(value)
        } else {
            if env::var_os(WSL2_DISTRIBUTION_ENV).is_some() {
                return Err(ValidationError::new(format!(
                    "{WSL2_DISTRIBUTION_ENV} is valid only for the WSL2 cell"
                )));
            }
            None
        };
        let mut codex_path = required_environment(CODEX_PATH_ENV)?;
        let mut volicord_path = required_environment(VOLICORD_PATH_ENV)?;
        let scenario_driver = canonical_process_path(
            &required_environment(SCENARIO_DRIVER_ENV)?,
            "scenario driver",
        )?;
        let evidence_directory = canonical_process_path(
            &required_environment(EVIDENCE_DIRECTORY_ENV)?,
            "release evidence directory",
        )?;
        let mut work_root = required_environment(WORK_ROOT_ENV)?;
        let mut runtime_home = required_environment(RUNTIME_HOME_ENV)?;
        if platform != PlatformEnvironment::Wsl2 {
            codex_path = canonical_process_path(&codex_path, "Codex executable")?
                .to_string_lossy()
                .into_owned();
            volicord_path = canonical_process_path(&volicord_path, "Volicord executable")?
                .to_string_lossy()
                .into_owned();
            let canonical_work_root = canonical_process_path(&work_root, "cell work root")?;
            let configured_runtime_home = PathBuf::from(&runtime_home);
            let runtime_name = configured_runtime_home
                .file_name()
                .ok_or_else(|| ValidationError::new("VOLICORD_HOME must have one child name"))?;
            let runtime_parent = configured_runtime_home.parent().ok_or_else(|| {
                ValidationError::new("VOLICORD_HOME must have an existing parent")
            })?;
            let canonical_runtime_parent = fs::canonicalize(runtime_parent).map_err(|error| {
                ValidationError::new(format!(
                    "cannot canonicalize the VOLICORD_HOME parent {}: {error}",
                    runtime_parent.display()
                ))
            })?;
            if canonical_runtime_parent != canonical_work_root {
                return Err(ValidationError::new(
                    "VOLICORD_HOME must be a direct absent child of the cell work root",
                ));
            }
            work_root = canonical_work_root.to_string_lossy().into_owned();
            runtime_home = canonical_work_root
                .join(runtime_name)
                .to_string_lossy()
                .into_owned();
        }
        Ok(Self {
            codex_path,
            volicord_path,
            scenario_driver,
            evidence_directory,
            work_root,
            runtime_home,
            environment_image: required_environment(ENVIRONMENT_IMAGE_ENV)?,
            runner_id: required_environment("RUNNER_NAME")?,
            wsl2_distribution,
        })
    }
}

fn canonical_process_path(value: &str, label: &str) -> ValidationResult<PathBuf> {
    let path = PathBuf::from(value);
    if !path.is_absolute() {
        return Err(ValidationError::new(format!("{label} must be absolute")));
    }
    fs::canonicalize(&path).map_err(|error| {
        ValidationError::new(format!(
            "cannot canonicalize {label} {}: {error}",
            path.display()
        ))
    })
}

fn required_environment(name: &str) -> ValidationResult<String> {
    let value = env::var(name).map_err(|_| {
        ValidationError::new(format!(
            "required environment variable {name} is missing or not UTF-8"
        ))
    })?;
    if value.is_empty() || value.chars().any(char::is_control) {
        return Err(ValidationError::new(format!(
            "required environment variable {name} must be nonempty and control-free"
        )));
    }
    Ok(value)
}

pub(super) fn run_bounded_status(
    command: &mut Command,
    timeout: Duration,
    label: &str,
) -> ValidationResult<()> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let mut child = command
        .spawn()
        .map_err(|error| ValidationError::new(format!("cannot start {label}: {error}")))?;
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(status) = child.try_wait()? {
            return if status.success() {
                Ok(())
            } else {
                Err(ValidationError::new(format!(
                    "{label} failed with status {status}"
                )))
            };
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(ValidationError::new(format!(
                "{label} exceeded its {} second bound",
                timeout.as_secs()
            )));
        }
        thread::sleep(Duration::from_millis(25));
    }
}

fn release_status_name(status: PlatformReleaseStatus) -> &'static str {
    match status {
        PlatformReleaseStatus::Passed => "passed",
        PlatformReleaseStatus::Failed => "failed",
        PlatformReleaseStatus::Unavailable => "unavailable",
        PlatformReleaseStatus::NotRun => "not_run",
    }
}

/// Returns all independent platform statuses for honest preflight reporting.
pub fn checked_in_platform_statuses(
) -> ValidationResult<[(PlatformEnvironment, PlatformReleaseStatus); 4]> {
    let manifest = checked_in_manifest().map_err(|error| {
        ValidationError::new(format!(
            "embedded Codex release manifest is invalid: {error}"
        ))
    })?;
    Ok(CODEX_RELEASE_PLATFORMS.map(|platform| (platform, manifest.platform_status(platform))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_in_gate_statuses_report_every_absent_cell_as_not_run() {
        assert_eq!(
            checked_in_platform_statuses().expect("checked-in statuses"),
            CODEX_RELEASE_PLATFORMS.map(|platform| (platform, PlatformReleaseStatus::NotRun))
        );
    }

    #[test]
    fn aggregate_status_requires_a_real_terminal_outcome() {
        use volicord_types::{
            CodexReleaseScenarioId, CodexReleaseScenarioResult, RequiredNullable,
        };

        let result = CodexReleaseScenarioResult {
            scenario_id: CodexReleaseScenarioId::FreshInstall,
            status: CodexReleaseScenarioStatus::NotRun,
            reason: RequiredNullable::some("earlier_prerequisite".to_owned()),
            evidence_digest: RequiredNullable::null(),
            observed_at: RequiredNullable::null(),
        };
        assert!(aggregate_scenario_status(&[result]).is_err());
    }
}
