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
    compute_codex_release_evidence_digest, CodexReleaseCellStatus, CodexReleaseEvidenceEntry,
    CodexReleaseEvidenceManifest, CodexReleaseScenarioStatus, CodexReleaseValidationEvidence,
    CodexReleaseValidationResult, CodexSupportCatalog, IntegrationProfile, PlatformEnvironment,
    PlatformReleaseCoordinate, ReleaseTargetTriple, FIRST_RELEASE_CODEX_CAPABILITIES,
    PINNED_WSL2_DISTRIBUTION_NAME,
};

use crate::{
    contracts::{
        embedded_codex_support_catalog, load_codex_release_evidence_manifest,
        load_codex_support_catalog, load_release_target_contract, ReleaseCell,
        ReleaseTargetContract, CODEX_RELEASE_EVIDENCE_MANIFEST_PATH, CODEX_SUPPORT_CATALOG_PATH,
        RELEASE_TARGETS_PATH,
    },
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
    pub target_triple: ReleaseTargetTriple,
    pub platform: PlatformEnvironment,
    pub codex_artifact_digest: String,
    pub volicord_artifact_digest: String,
    pub evidence_digest: String,
    pub scenario_count: usize,
}

/// Summary for a newly written, external one-cell review candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureReport {
    pub target_triple: ReleaseTargetTriple,
    pub platform: PlatformEnvironment,
    pub validation_result: CodexReleaseValidationResult,
    pub candidate_path: PathBuf,
    pub codex_artifact_digest: String,
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

/// Runs the blocking gate for one exact target/environment cell against the embedded support catalog
/// and canonical checked-in external evidence manifest.
pub fn run_checked_in_cell_gate(
    target_triple: ReleaseTargetTriple,
    platform: PlatformEnvironment,
) -> ValidationResult<GateReport> {
    validate_process_boundary(platform)?;
    let context = validation_context(platform)?;
    let (targets, _, evidence_manifest) = load_checked_in_contracts(&context)?;
    let cell = targets
        .require_cell(target_triple, platform, IntegrationProfile::Record)
        .map_err(ValidationError::new)?;

    let mut matching_entries = evidence_manifest
        .entries()
        .iter()
        .filter(|entry| entry_matches_cell(entry, cell));
    let Some(entry) = matching_entries.next() else {
        return Err(ValidationError::new(format!(
            "Codex release cell {target_triple}/{} has checked-in status not_run; exact passing evidence is required",
            platform.as_str(),
        )));
    };
    if matching_entries.next().is_some() {
        return Err(ValidationError::new(format!(
            "Codex release cell {target_triple}/{} has multiple exact artifact identities",
            platform.as_str()
        )));
    }
    if entry.validation_evidence.validation_result != CodexReleaseValidationResult::Passed {
        return Err(ValidationError::new(format!(
            "Codex release cell {target_triple}/{} has checked-in status {}; exact passing evidence is required",
            platform.as_str(),
            release_status_name(evidence_manifest.cell_status(
                target_triple,
                platform,
                IntegrationProfile::Record
            ))
        )));
    }

    let configuration = GateConfiguration::from_process(platform)?;
    validate_gate_paths(&context, platform, &configuration)?;
    validate_actual_runner_coordinate(entry, target_triple, platform, &configuration)?;

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
    if codex_before != entry.codex_artifact_digest {
        return Err(ValidationError::new(format!(
            "actual Codex executable digest does not match the exact checked-in {} cell",
            platform.as_str()
        )));
    }
    if volicord_before != entry.validation_evidence.volicord_artifact_digest {
        return Err(ValidationError::new(format!(
            "actual Volicord executable digest does not match the exact checked-in {} cell",
            platform.as_str()
        )));
    }

    probe_executable(platform, &configuration.codex_path, &configuration)?;
    probe_executable(platform, &configuration.volicord_path, &configuration)?;
    run_checked_scenario_catalog(&context, platform, entry, &configuration)?;

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
        target_triple,
        platform,
        codex_artifact_digest: codex_after,
        volicord_artifact_digest: volicord_after,
        evidence_digest: entry.validation_evidence.evidence_digest.clone(),
        scenario_count: entry.validation_evidence.scenario_results.len(),
    })
}

/// Executes one qualifying platform attempt and writes a new external review candidate.
///
/// This producer never edits or promotes either canonical contract. Release
/// publication continues to require `run_checked_in_cell_gate` against reviewed
/// external evidence that matches the embedded support catalog.
pub fn capture_candidate_cell(
    target_triple: ReleaseTargetTriple,
    platform: PlatformEnvironment,
) -> ValidationResult<CaptureReport> {
    validate_process_boundary(platform)?;
    let context = validation_context(platform)?;
    // Refuse to produce from a build whose embedded support policy and checkout
    // disagree, or whose checked-in external evidence is not policy-bound.
    let (targets, support_catalog, _) = load_checked_in_contracts(&context)?;
    targets
        .require_cell(target_triple, platform, IntegrationProfile::Record)
        .map_err(ValidationError::new)?;

    let configuration = GateConfiguration::from_process(platform)?;
    validate_gate_paths(&context, platform, &configuration)?;
    let runner = collect_runner_coordinate(target_triple, platform, &configuration)?;
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
    let platform_release_coordinate = if platform == PlatformEnvironment::Wsl2 {
        PlatformReleaseCoordinate::first_release_wsl2()
    } else {
        PlatformReleaseCoordinate::native()
    };
    support_catalog
        .lookup_supported_entry(
            &codex_before,
            target_triple,
            platform,
            &platform_release_coordinate,
            &FIRST_RELEASE_CODEX_CAPABILITIES,
            IntegrationProfile::Record,
        )
        .map_err(|error| {
            ValidationError::new(format!(
                "candidate Codex artifact is absent from the embedded support catalog: {error}"
            ))
        })?;
    probe_executable(platform, &configuration.codex_path, &configuration)?;
    probe_executable(platform, &configuration.volicord_path, &configuration)?;
    let scenario_results = capture_scenario_catalog(
        &context,
        target_triple,
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

    let validation_result = aggregate_scenario_status(&scenario_results)?;
    let observed_at =
        DateTime::<Utc>::from(SystemTime::now()).to_rfc3339_opts(SecondsFormat::AutoSi, true);
    let capabilities = FIRST_RELEASE_CODEX_CAPABILITIES.to_vec();
    let mut evidence = CodexReleaseValidationEvidence {
        validation_result,
        codex_artifact_digest: codex_after.clone(),
        target_triple,
        platform_environment: platform,
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
    let entry = CodexReleaseEvidenceEntry {
        codex_artifact_digest: codex_after.clone(),
        target_triple,
        platform_environment: platform,
        observed_capabilities: capabilities,
        integration_profile: IntegrationProfile::Record,
        validation_evidence: evidence,
    };
    let candidate_manifest = CodexReleaseEvidenceManifest::from_entries(vec![entry.clone()])
        .map_err(|error| ValidationError::new(format!("invalid candidate evidence: {error}")))?;
    candidate_manifest
        .validate_against_support_catalog(&support_catalog)
        .map_err(|error| {
            ValidationError::new(format!("unsupported candidate evidence: {error}"))
        })?;
    write_json_create_new(
        &context,
        &candidate_path,
        &candidate_manifest,
        MAX_MANIFEST_JSON_BYTES,
    )?;
    let verified = load_codex_release_evidence_manifest(&candidate_path).map_err(|error| {
        ValidationError::new(format!(
            "new candidate manifest failed strict verification at {}: {error}",
            candidate_path.display()
        ))
    })?;
    if verified != candidate_manifest {
        return Err(ValidationError::new(
            "new candidate evidence manifest does not round-trip to its captured entry",
        ));
    }

    Ok(CaptureReport {
        target_triple,
        platform,
        validation_result,
        candidate_path,
        codex_artifact_digest: codex_after,
        volicord_artifact_digest: volicord_after,
        evidence_digest: entry.validation_evidence.evidence_digest,
        scenario_count: entry.validation_evidence.scenario_results.len(),
    })
}

fn aggregate_scenario_status(
    results: &[volicord_types::CodexReleaseScenarioResult],
) -> ValidationResult<CodexReleaseValidationResult> {
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
        return Ok(CodexReleaseValidationResult::Failed);
    }
    if results
        .iter()
        .any(|result| result.status == CodexReleaseScenarioStatus::Unavailable)
    {
        return Ok(CodexReleaseValidationResult::Unavailable);
    }
    if results
        .iter()
        .all(|result| result.status == CodexReleaseScenarioStatus::Passed)
    {
        return Ok(CodexReleaseValidationResult::Passed);
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

fn load_checked_in_contracts(
    context: &ValidationContext,
) -> ValidationResult<(
    ReleaseTargetContract,
    CodexSupportCatalog,
    CodexReleaseEvidenceManifest,
)> {
    let release_targets_path = context.source_checkout().join(RELEASE_TARGETS_PATH);
    let support_catalog_path = context.source_checkout().join(CODEX_SUPPORT_CATALOG_PATH);
    let evidence_manifest_path = context
        .source_checkout()
        .join(CODEX_RELEASE_EVIDENCE_MANIFEST_PATH);
    let release_targets = load_release_target_contract(&release_targets_path).map_err(|error| {
        ValidationError::new(format!(
            "release target contract is invalid at {}: {error}",
            release_targets_path.display()
        ))
    })?;
    let embedded_support_catalog = embedded_codex_support_catalog().map_err(|error| {
        ValidationError::new(format!(
            "embedded Codex support catalog is invalid: {error}"
        ))
    })?;
    let disk_support_catalog =
        load_codex_support_catalog(&support_catalog_path).map_err(|error| {
            ValidationError::new(format!(
                "on-disk Codex support catalog is invalid at {}: {error}",
                support_catalog_path.display()
            ))
        })?;
    if embedded_support_catalog != disk_support_catalog {
        return Err(ValidationError::new(
            "embedded and on-disk Codex support catalogs differ",
        ));
    }
    let evidence_manifest =
        load_codex_release_evidence_manifest(&evidence_manifest_path).map_err(|error| {
            ValidationError::new(format!(
                "external Codex release-evidence manifest is invalid at {}: {error}",
                evidence_manifest_path.display()
            ))
        })?;
    evidence_manifest
        .validate_against_support_catalog(&embedded_support_catalog)
        .map_err(|error| {
            ValidationError::new(format!(
                "external Codex release evidence is not supported by the embedded catalog: {error}"
            ))
        })?;
    validate_target_contract_bindings(
        &release_targets,
        &embedded_support_catalog,
        &evidence_manifest,
    )?;
    Ok((release_targets, embedded_support_catalog, evidence_manifest))
}

fn validate_target_contract_bindings(
    targets: &ReleaseTargetContract,
    support_catalog: &CodexSupportCatalog,
    evidence_manifest: &CodexReleaseEvidenceManifest,
) -> ValidationResult<()> {
    for entry in support_catalog.entries() {
        targets
            .require_cell(
                entry.target_triple,
                entry.platform_environment,
                entry.integration_profile,
            )
            .map_err(|_| {
                ValidationError::new(format!(
                    "support-catalog entry {}/{} cannot map to an actual required release target cell",
                    entry.target_triple,
                    entry.platform_environment.as_str()
                ))
            })?;
    }
    for entry in evidence_manifest.entries() {
        targets
            .require_cell(
                entry.target_triple,
                entry.platform_environment,
                entry.integration_profile,
            )
            .map_err(|_| {
                ValidationError::new(format!(
                    "release-evidence entry {}/{} is not a required release target cell",
                    entry.target_triple,
                    entry.platform_environment.as_str()
                ))
            })?;
    }
    Ok(())
}

fn entry_matches_cell(entry: &CodexReleaseEvidenceEntry, cell: ReleaseCell) -> bool {
    entry.target_triple == cell.target_triple
        && entry.platform_environment == cell.platform_environment
        && entry.integration_profile == cell.integration_profile
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

fn release_status_name(status: CodexReleaseCellStatus) -> &'static str {
    match status {
        CodexReleaseCellStatus::Passed => "passed",
        CodexReleaseCellStatus::Failed => "failed",
        CodexReleaseCellStatus::Unavailable => "unavailable",
        CodexReleaseCellStatus::NotRun => "not_run",
    }
}

/// Returns all exact required cell statuses for honest preflight reporting.
pub fn checked_in_cell_statuses() -> ValidationResult<Vec<(ReleaseCell, CodexReleaseCellStatus)>> {
    let context = validation_context(PlatformEnvironment::Linux)?;
    let (targets, _, manifest) = load_checked_in_contracts(&context)?;
    Ok(targets
        .required_cells()
        .iter()
        .copied()
        .map(|cell| {
            (
                cell,
                manifest.cell_status(
                    cell.target_triple,
                    cell.platform_environment,
                    cell.integration_profile,
                ),
            )
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_in_gate_statuses_report_all_six_absent_cells_as_not_run() {
        let statuses = checked_in_cell_statuses().expect("checked-in statuses");
        assert_eq!(statuses.len(), 6);
        assert!(statuses
            .iter()
            .all(|(_, status)| *status == CodexReleaseCellStatus::NotRun));
    }

    #[test]
    fn support_entry_without_an_actual_required_cell_is_rejected() {
        let mut document: serde_json::Value =
            serde_json::from_slice(include_bytes!("../contracts/release-targets.json"))
                .expect("release target JSON");
        document["required_cells"]
            .as_array_mut()
            .expect("required cells")
            .retain(|cell| {
                cell["target_triple"] != "x86_64-unknown-linux-gnu"
                    || cell["platform_environment"] != "linux"
            });
        let targets = crate::contracts::parse_release_target_contract(
            &serde_json::to_vec(&document).expect("serialize contract"),
        )
        .expect("x86-64 target still has the WSL2 required cell");
        let support = CodexSupportCatalog::from_entries(vec![volicord_types::CodexSupportEntry {
            codex_artifact_digest: "1".repeat(64),
            target_triple: ReleaseTargetTriple::X86_64UnknownLinuxGnu,
            platform_environment: PlatformEnvironment::Linux,
            platform_release_coordinate: PlatformReleaseCoordinate::native(),
            integration_profile: IntegrationProfile::Record,
            verified_capabilities: FIRST_RELEASE_CODEX_CAPABILITIES.to_vec(),
        }])
        .expect("valid standalone support entry");
        let evidence = CodexReleaseEvidenceManifest::from_entries(Vec::new())
            .expect("empty evidence manifest");
        assert!(
            validate_target_contract_bindings(&targets, &support, &evidence)
                .expect_err("unmapped support entry")
                .to_string()
                .contains("cannot map")
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
