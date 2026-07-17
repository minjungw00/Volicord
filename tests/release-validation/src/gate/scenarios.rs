use std::{collections::BTreeSet, fs, path::Path, process::Command, time::Duration};

use serde::{Deserialize, Serialize};
use volicord_types::{
    canonical_json_bare_sha256, CodexReleaseEvidenceEntry, CodexReleaseScenarioId,
    CodexReleaseScenarioResult, CodexReleaseScenarioStatus, PlatformEnvironment,
    ReleaseTargetTriple, RequiredNullable, UtcTimestamp, FIRST_RELEASE_CODEX_CAPABILITIES,
};

use crate::{
    error::{ValidationError, ValidationResult},
    io::{
        read_strict_json, sha256_external_file, write_json_create_new, ValidationContext,
        MAX_CELL_JSON_BYTES, MAX_EVIDENCE_BYTES,
    },
    platforms,
    scenarios::{
        definition as scenario_definition, ScenarioBoundary, ScenarioDefinition,
        ScenarioDomainDisposition, ScenarioExpectation, ScenarioFixture, ScenarioOutcomeCode,
        ScenarioProjection,
    },
};

use super::{run_bounded_status, GateConfiguration, WSL2_DISTRIBUTION_ENV};

const SCENARIO_TIMEOUT: Duration = Duration::from_secs(30 * 60);
const SCENARIO_EVIDENCE_CONTRACT: &str = "volicord.release_scenario_evidence";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DriverScenarioOutcome {
    scenario_id: CodexReleaseScenarioId,
    status: CodexReleaseScenarioStatus,
    reason: RequiredNullable<String>,
    observed_at: RequiredNullable<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DriverScenarioEvidence {
    contract: String,
    scenario_id: CodexReleaseScenarioId,
    platform: PlatformEnvironment,
    state_setup: ScenarioStateSetup,
    boundary_execution: ScenarioBoundaryExecution,
    domain_outcome: ScenarioDomainOutcome,
    adapter_projection: ScenarioAdapterProjection,
    cleanup_complete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScenarioStateSetup {
    canonical_project_state: CanonicalScenarioProjectState,
    canonical_project_state_digest: String,
    validated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CanonicalScenarioProjectState {
    fixture_id: CodexReleaseScenarioId,
    fixture: ScenarioFixture,
    platform: PlatformEnvironment,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScenarioBoundaryExecution {
    canonical_invocation: CanonicalScenarioInvocation,
    invocation_digest: String,
    completed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CanonicalScenarioInvocation {
    scenario_id: CodexReleaseScenarioId,
    platform: PlatformEnvironment,
    boundary: ScenarioBoundary,
    canonical_project_state_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScenarioDomainOutcome {
    canonical_outcome: CanonicalScenarioDomainOutcome,
    canonical_outcome_digest: String,
    validated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CanonicalScenarioDomainOutcome {
    scenario_id: CodexReleaseScenarioId,
    expectation: ScenarioExpectation,
    disposition: ScenarioDomainDisposition,
    outcome_code: ScenarioOutcomeCode,
    invocation_digest: String,
    observed_paths_preserved: RequiredNullable<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScenarioAdapterProjection {
    canonical_projection: CanonicalScenarioAdapterProjection,
    canonical_projection_digest: String,
    validated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CanonicalScenarioAdapterProjection {
    scenario_id: CodexReleaseScenarioId,
    projection: ScenarioProjection,
    outcome_code: ScenarioOutcomeCode,
    canonical_outcome_digest: String,
}

#[derive(Serialize)]
struct ScenarioEvidenceEnvelope<'a> {
    scenario_id: CodexReleaseScenarioId,
    target_triple: ReleaseTargetTriple,
    platform: PlatformEnvironment,
    codex_artifact_digest: &'a str,
    volicord_artifact_digest: &'a str,
    scenario_driver_digest: &'a str,
    integration_profile: &'static str,
    observed_capabilities: Vec<&'static str>,
    scenario_definition: ScenarioDefinition,
    outcome_status: CodexReleaseScenarioStatus,
    outcome_reason: &'a RequiredNullable<String>,
    driver_payload_digest: RequiredNullable<String>,
}

struct ScenarioDriverInvocation<'a> {
    target_triple: ReleaseTargetTriple,
    platform: PlatformEnvironment,
    scenario: ScenarioDefinition,
    codex_artifact_digest: &'a str,
    volicord_artifact_digest: &'a str,
    scenario_driver_digest: &'a str,
    configuration: &'a GateConfiguration,
    payload_path: &'a Path,
    outcome_path: &'a Path,
}

pub(super) fn run_checked_scenario_catalog(
    context: &ValidationContext,
    platform: PlatformEnvironment,
    entry: &CodexReleaseEvidenceEntry,
    configuration: &GateConfiguration,
) -> ValidationResult<()> {
    let actual = execute_scenario_catalog(
        context,
        entry.target_triple,
        platform,
        &entry.codex_artifact_digest,
        &entry.validation_evidence.volicord_artifact_digest,
        configuration,
    )?;
    if actual.len() != entry.validation_evidence.scenario_results.len() {
        return Err(ValidationError::new(
            "executed scenario catalog length differs from the checked-in cell",
        ));
    }
    for (actual, expected) in actual
        .iter()
        .zip(&entry.validation_evidence.scenario_results)
    {
        if actual.scenario_id != expected.scenario_id
            || actual.status != CodexReleaseScenarioStatus::Passed
            || expected.status != CodexReleaseScenarioStatus::Passed
            || actual.reason.is_some()
        {
            return Err(ValidationError::new(format!(
                "scenario {} is not an exact current and checked-in pass",
                expected.scenario_id.as_str()
            )));
        }
        if actual.evidence_digest.as_ref() != expected.evidence_digest.as_ref() {
            return Err(ValidationError::new(format!(
                "scenario {} evidence digest does not match the exact checked-in cell",
                expected.scenario_id.as_str()
            )));
        }
    }
    Ok(())
}

pub(super) fn capture_scenario_catalog(
    context: &ValidationContext,
    target_triple: ReleaseTargetTriple,
    platform: PlatformEnvironment,
    codex_artifact_digest: &str,
    volicord_artifact_digest: &str,
    configuration: &GateConfiguration,
) -> ValidationResult<Vec<CodexReleaseScenarioResult>> {
    execute_scenario_catalog(
        context,
        target_triple,
        platform,
        codex_artifact_digest,
        volicord_artifact_digest,
        configuration,
    )
}

fn execute_scenario_catalog(
    context: &ValidationContext,
    target_triple: ReleaseTargetTriple,
    platform: PlatformEnvironment,
    codex_artifact_digest: &str,
    volicord_artifact_digest: &str,
    configuration: &GateConfiguration,
) -> ValidationResult<Vec<CodexReleaseScenarioResult>> {
    let definition = platforms::all()
        .into_iter()
        .find(|definition| {
            definition.target_triple == target_triple && definition.platform == platform
        })
        .expect("every required target/environment cell has one definition");
    let scenario_driver_digest =
        sha256_external_file(context, &configuration.scenario_driver, None)?;

    let driver_directory = configuration.evidence_directory.join(".driver");
    context.validate_new_directory(&driver_directory)?;
    fs::create_dir(&driver_directory)?;

    let mut results = Vec::with_capacity(definition.scenarios.len());
    let mut expected_evidence_names = BTreeSet::new();
    for (index, scenario_id) in definition.scenarios.into_iter().enumerate() {
        let scenario = scenario_definition(scenario_id);
        let stem = format!("{:02}-{}", index + 1, scenario_id.as_str());
        let payload_path = driver_directory.join(format!("{stem}.payload"));
        let outcome_path = driver_directory.join(format!("{stem}.outcome.json"));
        let evidence_path = configuration
            .evidence_directory
            .join(format!("{stem}.evidence"));
        let retained_payload_path = configuration
            .evidence_directory
            .join(format!("{stem}.driver-evidence"));
        context.validate_new_output(&payload_path)?;
        context.validate_new_output(&outcome_path)?;
        context.validate_new_output(&evidence_path)?;
        context.validate_new_output(&retained_payload_path)?;

        let mut command = ScenarioDriverInvocation {
            target_triple,
            platform,
            scenario,
            codex_artifact_digest,
            volicord_artifact_digest,
            scenario_driver_digest: &scenario_driver_digest,
            configuration,
            payload_path: &payload_path,
            outcome_path: &outcome_path,
        }
        .command();
        run_bounded_status(
            &mut command,
            SCENARIO_TIMEOUT,
            &format!("scenario driver for {}", scenario_id.as_str()),
        )?;

        let outcome: DriverScenarioOutcome =
            read_strict_json(context, &outcome_path, MAX_CELL_JSON_BYTES)?;
        validate_driver_outcome(scenario_id, &outcome)?;
        let payload_digest = driver_payload_digest(
            context,
            &payload_path,
            outcome.status,
            platform,
            scenario_id,
        )?;
        let has_payload = payload_digest.as_ref().is_some();
        let evidence_digest = if outcome.status == CodexReleaseScenarioStatus::NotRun {
            RequiredNullable::null()
        } else {
            expected_evidence_names.insert(format!("{stem}.evidence"));
            let envelope = ScenarioEvidenceEnvelope {
                scenario_id,
                target_triple,
                platform,
                codex_artifact_digest,
                volicord_artifact_digest,
                scenario_driver_digest: &scenario_driver_digest,
                integration_profile: "record",
                observed_capabilities: FIRST_RELEASE_CODEX_CAPABILITIES
                    .map(|capability| capability.as_str())
                    .to_vec(),
                scenario_definition: scenario,
                outcome_status: outcome.status,
                outcome_reason: &outcome.reason,
                driver_payload_digest: payload_digest,
            };
            write_json_create_new(context, &evidence_path, &envelope, MAX_EVIDENCE_BYTES)?;
            RequiredNullable::some(sha256_external_file(
                context,
                &evidence_path,
                Some(MAX_EVIDENCE_BYTES),
            )?)
        };
        if has_payload {
            expected_evidence_names.insert(format!("{stem}.driver-evidence"));
            fs::rename(&payload_path, &retained_payload_path)?;
        }
        results.push(CodexReleaseScenarioResult {
            scenario_id,
            status: outcome.status,
            reason: outcome.reason,
            evidence_digest,
            observed_at: outcome.observed_at,
        });

        remove_if_present(&payload_path)?;
        fs::remove_file(&outcome_path)?;
    }
    fs::remove_dir(&driver_directory)?;
    validate_evidence_directory(&configuration.evidence_directory, &expected_evidence_names)?;

    let driver_after = sha256_external_file(context, &configuration.scenario_driver, None)?;
    if driver_after != scenario_driver_digest {
        return Err(ValidationError::new(
            "scenario driver bytes changed during the release cell",
        ));
    }
    Ok(results)
}

fn validate_evidence_directory(
    directory: &Path,
    expected_names: &BTreeSet<String>,
) -> ValidationResult<()> {
    let mut actual_names = BTreeSet::new();
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if !file_type.is_file() {
            return Err(ValidationError::new(
                "release evidence directory contains a non-file result",
            ));
        }
        let name = entry.file_name().into_string().map_err(|_| {
            ValidationError::new("release evidence directory contains a non-UTF-8 name")
        })?;
        actual_names.insert(name);
    }
    if &actual_names != expected_names {
        return Err(ValidationError::new(
            "release evidence directory does not contain the exact scenario evidence set",
        ));
    }
    Ok(())
}

impl ScenarioDriverInvocation<'_> {
    fn command(&self) -> Command {
        let mut command = Command::new(&self.configuration.scenario_driver);
        command
            .arg("--scenario")
            .arg(self.scenario.id.as_str())
            .arg("--fixture")
            .arg(self.scenario.fixture.as_str())
            .arg("--boundary")
            .arg(self.scenario.boundary.as_str())
            .arg("--projection")
            .arg(self.scenario.projection.as_str())
            .arg("--expected-outcome")
            .arg(self.scenario.outcome_code.as_str())
            .arg("--platform")
            .arg(self.platform.as_str())
            .arg("--codex")
            .arg(&self.configuration.codex_path)
            .arg("--volicord")
            .arg(&self.configuration.volicord_path)
            .arg("--work-root")
            .arg(&self.configuration.work_root)
            .arg("--runtime-home")
            .arg(&self.configuration.runtime_home)
            .arg("--evidence-output")
            .arg(self.payload_path)
            .arg("--outcome-output")
            .arg(self.outcome_path)
            .env(
                "VOLICORD_CODEX_RELEASE_TARGET_TRIPLE",
                self.target_triple.as_str(),
            )
            .env(
                "VOLICORD_CODEX_RELEASE_CODEX_ARTIFACT_DIGEST",
                self.codex_artifact_digest,
            )
            .env(
                "VOLICORD_CODEX_RELEASE_VOLICORD_DIGEST",
                self.volicord_artifact_digest,
            )
            .env(
                "VOLICORD_CODEX_RELEASE_SCENARIO_DRIVER_DIGEST",
                self.scenario_driver_digest,
            )
            .env(
                "VOLICORD_CODEX_RELEASE_CAPABILITIES",
                FIRST_RELEASE_CODEX_CAPABILITIES
                    .map(|capability| capability.as_str())
                    .join(","),
            )
            .env("VOLICORD_CODEX_RELEASE_INTEGRATION_PROFILE", "record");
        if let Some(distribution) = &self.configuration.wsl2_distribution {
            command
                .arg("--wsl2-distribution")
                .arg(distribution)
                .env(WSL2_DISTRIBUTION_ENV, distribution);
        }
        command
    }
}

fn validate_driver_outcome(
    expected_id: CodexReleaseScenarioId,
    outcome: &DriverScenarioOutcome,
) -> ValidationResult<()> {
    if outcome.scenario_id != expected_id {
        return Err(ValidationError::new(
            "scenario driver reported a conflicting scenario ID",
        ));
    }
    let valid_reason = outcome.reason.as_ref().is_some_and(|reason| {
        !reason.is_empty()
            && reason.len() <= 128
            && reason.bytes().enumerate().all(|(index, byte)| {
                if index == 0 {
                    byte.is_ascii_lowercase()
                } else {
                    byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_'
                }
            })
    });
    match outcome.status {
        CodexReleaseScenarioStatus::Passed if outcome.reason.is_none() => {}
        CodexReleaseScenarioStatus::Failed
        | CodexReleaseScenarioStatus::Unavailable
        | CodexReleaseScenarioStatus::NotRun
            if valid_reason => {}
        _ => {
            return Err(ValidationError::new(
                "scenario driver returned an invalid status/reason combination",
            ))
        }
    }
    match (outcome.status, outcome.observed_at.as_ref()) {
        (CodexReleaseScenarioStatus::NotRun, None) => Ok(()),
        (CodexReleaseScenarioStatus::NotRun, Some(_)) | (_, None) => Err(ValidationError::new(
            "scenario driver returned an invalid status/observed_at combination",
        )),
        (_, Some(timestamp)) => {
            let parsed = UtcTimestamp::parse(timestamp).map_err(|_| {
                ValidationError::new("scenario driver observed_at is not RFC 3339 UTC")
            })?;
            if parsed.to_canonical_string() != *timestamp {
                return Err(ValidationError::new(
                    "scenario driver observed_at is not canonical RFC 3339 UTC",
                ));
            }
            Ok(())
        }
    }
}

fn driver_payload_digest(
    context: &ValidationContext,
    path: &Path,
    status: CodexReleaseScenarioStatus,
    platform: PlatformEnvironment,
    scenario_id: CodexReleaseScenarioId,
) -> ValidationResult<RequiredNullable<String>> {
    let exists = fs::symlink_metadata(path).is_ok();
    match status {
        CodexReleaseScenarioStatus::Passed | CodexReleaseScenarioStatus::Failed if !exists => {
            Err(ValidationError::new(
                "passed or failed scenario did not produce its bounded driver evidence",
            ))
        }
        CodexReleaseScenarioStatus::NotRun if exists => Err(ValidationError::new(
            "not_run scenario must not produce driver evidence",
        )),
        CodexReleaseScenarioStatus::Passed => {
            let evidence: DriverScenarioEvidence =
                read_strict_json(context, path, MAX_EVIDENCE_BYTES)?;
            validate_driver_evidence(platform, scenario_id, &evidence)?;
            Ok(RequiredNullable::some(canonical_json_bare_sha256(
                &evidence,
            )?))
        }
        _ if exists => Ok(RequiredNullable::some(sha256_external_file(
            context,
            path,
            Some(MAX_EVIDENCE_BYTES),
        )?)),
        _ => Ok(RequiredNullable::null()),
    }
}

fn validate_driver_evidence(
    platform: PlatformEnvironment,
    scenario_id: CodexReleaseScenarioId,
    evidence: &DriverScenarioEvidence,
) -> ValidationResult<()> {
    if evidence.contract != SCENARIO_EVIDENCE_CONTRACT {
        return Err(ValidationError::new(
            "passed scenario evidence has an unsupported contract",
        ));
    }
    if evidence.scenario_id != scenario_id {
        return Err(ValidationError::new(
            "passed scenario evidence does not use the selected scenario ID",
        ));
    }
    if evidence.platform != platform {
        return Err(ValidationError::new(
            "passed scenario evidence reports a conflicting platform",
        ));
    }
    if !evidence.state_setup.validated {
        return Err(ValidationError::new(
            "passed scenario evidence did not validate canonical project state",
        ));
    }
    if !evidence.boundary_execution.completed {
        return Err(ValidationError::new(
            "passed scenario evidence did not complete its selected boundary execution",
        ));
    }
    if !evidence.domain_outcome.validated {
        return Err(ValidationError::new(
            "passed scenario evidence did not validate its canonical domain outcome",
        ));
    }
    if !evidence.adapter_projection.validated {
        return Err(ValidationError::new(
            "passed scenario evidence did not validate its adapter projection",
        ));
    }
    if !evidence.cleanup_complete {
        return Err(ValidationError::new(
            "passed scenario evidence did not complete bounded cleanup",
        ));
    }

    let definition = scenario_definition(scenario_id);
    let project_state_digest = require_canonical_payload_digest(
        "state_setup.canonical_project_state_digest",
        &evidence.state_setup.canonical_project_state,
        &evidence.state_setup.canonical_project_state_digest,
    )?;
    let expected_project_state = CanonicalScenarioProjectState {
        fixture_id: definition.id,
        fixture: definition.fixture,
        platform,
    };
    if evidence.state_setup.canonical_project_state != expected_project_state {
        return Err(ValidationError::new(
            "passed scenario evidence does not match the repository-owned canonical fixture",
        ));
    }

    let invocation_digest = require_canonical_payload_digest(
        "boundary_execution.invocation_digest",
        &evidence.boundary_execution.canonical_invocation,
        &evidence.boundary_execution.invocation_digest,
    )?;
    let expected_invocation = CanonicalScenarioInvocation {
        scenario_id: definition.id,
        platform,
        boundary: definition.boundary,
        canonical_project_state_digest: project_state_digest,
    };
    if evidence.boundary_execution.canonical_invocation != expected_invocation {
        return Err(ValidationError::new(
            "passed scenario evidence does not execute the repository-owned boundary invocation",
        ));
    }

    let outcome_digest = require_canonical_payload_digest(
        "domain_outcome.canonical_outcome_digest",
        &evidence.domain_outcome.canonical_outcome,
        &evidence.domain_outcome.canonical_outcome_digest,
    )?;
    let expected_outcome = CanonicalScenarioDomainOutcome {
        scenario_id: definition.id,
        expectation: definition.expectation,
        disposition: definition.disposition,
        outcome_code: definition.outcome_code,
        invocation_digest,
        observed_paths_preserved: RequiredNullable::new(definition.observed_paths_preserved),
    };
    if evidence.domain_outcome.canonical_outcome != expected_outcome {
        return Err(ValidationError::new(
            "passed scenario evidence does not match the repository-owned domain outcome",
        ));
    }

    require_canonical_payload_digest(
        "adapter_projection.canonical_projection_digest",
        &evidence.adapter_projection.canonical_projection,
        &evidence.adapter_projection.canonical_projection_digest,
    )?;
    let expected_projection = CanonicalScenarioAdapterProjection {
        scenario_id: definition.id,
        projection: definition.projection,
        outcome_code: definition.outcome_code,
        canonical_outcome_digest: outcome_digest,
    };
    if evidence.adapter_projection.canonical_projection != expected_projection {
        return Err(ValidationError::new(
            "passed scenario evidence does not match the repository-owned adapter projection",
        ));
    }
    Ok(())
}

fn require_canonical_payload_digest<T: Serialize>(
    field: &str,
    payload: &T,
    claimed_digest: &str,
) -> ValidationResult<String> {
    let recomputed = canonical_json_bare_sha256(payload)?;
    if claimed_digest != recomputed {
        return Err(ValidationError::new(format!(
            "passed scenario evidence {field} does not match its retained canonical payload"
        )));
    }
    Ok(recomputed)
}

fn remove_if_present(path: &Path) -> ValidationResult<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn driver_outcomes_keep_status_reason_and_time_rules_exact() {
        let passed = DriverScenarioOutcome {
            scenario_id: CodexReleaseScenarioId::FreshInstall,
            status: CodexReleaseScenarioStatus::Passed,
            reason: RequiredNullable::null(),
            observed_at: RequiredNullable::some("2026-07-17T00:00:00Z".to_owned()),
        };
        assert!(validate_driver_outcome(CodexReleaseScenarioId::FreshInstall, &passed).is_ok());

        let mut invalid = passed;
        invalid.reason = RequiredNullable::some("copied pass".to_owned());
        assert!(validate_driver_outcome(CodexReleaseScenarioId::FreshInstall, &invalid).is_err());
    }

    #[test]
    fn passed_evidence_rejects_opaque_driver_claims() {
        assert!(
            crate::io::parse_strict_json::<DriverScenarioEvidence>(br#"{"passed":true}"#).is_err()
        );
    }

    #[test]
    fn documented_fresh_install_payload_digests_are_canonical() {
        let evidence = evidence_for(
            CodexReleaseScenarioId::FreshInstall,
            PlatformEnvironment::Linux,
        );
        assert_eq!(
            evidence.state_setup.canonical_project_state_digest,
            "0412001a986fb601aaec49e5ca491f034735eae9d2b79fc3a1f172ac73268725"
        );
        assert_eq!(
            evidence.boundary_execution.invocation_digest,
            "e123ca5a50d1b8362a8bc8a9a6366692f3901a09184e014da44eaa1e3a1d9fde"
        );
        assert_eq!(
            evidence.domain_outcome.canonical_outcome_digest,
            "9690aa98449e2944b9477d3ffb6496a918556a15432173cdc48b4a432cee19af"
        );
        assert_eq!(
            evidence.adapter_projection.canonical_projection_digest,
            "ca3f422db0290cd0bdd328afd8ca794bca9e7f1eee0a509382e1ddff2dfd0a48"
        );
    }

    #[test]
    fn repository_definitions_drive_every_scenario_evidence_record() {
        let mut boundary_coverage = [false; 5];
        for scenario_id in CodexReleaseScenarioId::BASE
            .into_iter()
            .chain(CodexReleaseScenarioId::WSL2_ADDITIONAL)
        {
            let definition = scenario_definition(scenario_id);
            boundary_coverage[match definition.boundary {
                ScenarioBoundary::Core => 0,
                ScenarioBoundary::McpStdio => 1,
                ScenarioBoundary::Cli => 2,
                ScenarioBoundary::ManagedHost => 3,
                ScenarioBoundary::Platform => 4,
            }] = true;
            assert_eq!(definition.projection, definition.boundary.projection());
            let evidence = evidence_for(scenario_id, PlatformEnvironment::Wsl2);
            assert!(
                validate_driver_evidence(PlatformEnvironment::Wsl2, scenario_id, &evidence).is_ok(),
                "repository-owned evidence should validate for {}",
                scenario_id.as_str()
            );
        }
        assert_eq!(boundary_coverage, [true; 5]);
    }

    #[test]
    fn passed_evidence_rejects_a_self_selected_boundary_and_projection() {
        let scenario_id = CodexReleaseScenarioId::FreshInstall;
        let mut evidence = evidence_for(scenario_id, PlatformEnvironment::Linux);
        evidence.boundary_execution.canonical_invocation.boundary = ScenarioBoundary::Core;
        evidence.adapter_projection.canonical_projection.projection =
            ScenarioProjection::CoreResponse;
        refresh_linked_digests(&mut evidence);

        assert!(
            validate_driver_evidence(PlatformEnvironment::Linux, scenario_id, &evidence).is_err()
        );
    }

    #[test]
    fn passed_evidence_rejects_tampered_payloads_and_self_consistent_alternative_fixtures() {
        let scenario_id = CodexReleaseScenarioId::FreshInstall;
        let mut stale_digest = evidence_for(scenario_id, PlatformEnvironment::Linux);
        stale_digest.state_setup.canonical_project_state.fixture =
            ScenarioFixture::RuntimeHomeAbsent;
        assert!(
            validate_driver_evidence(PlatformEnvironment::Linux, scenario_id, &stale_digest)
                .is_err()
        );

        let mut self_consistent = stale_digest;
        refresh_linked_digests(&mut self_consistent);
        assert!(validate_driver_evidence(
            PlatformEnvironment::Linux,
            scenario_id,
            &self_consistent
        )
        .is_err());
    }

    #[test]
    fn passed_evidence_rejects_broken_digest_links() {
        let scenario_id = CodexReleaseScenarioId::RecordWriteWorkflow;
        let mut evidence = evidence_for(scenario_id, PlatformEnvironment::Macos);
        evidence.domain_outcome.canonical_outcome.invocation_digest =
            "1111111111111111111111111111111111111111111111111111111111111111".to_owned();
        evidence.domain_outcome.canonical_outcome_digest =
            canonical_json_bare_sha256(&evidence.domain_outcome.canonical_outcome)
                .expect("canonical outcome digest");
        evidence
            .adapter_projection
            .canonical_projection
            .canonical_outcome_digest = evidence.domain_outcome.canonical_outcome_digest.clone();
        evidence.adapter_projection.canonical_projection_digest =
            canonical_json_bare_sha256(&evidence.adapter_projection.canonical_projection)
                .expect("canonical projection digest");

        assert!(
            validate_driver_evidence(PlatformEnvironment::Macos, scenario_id, &evidence).is_err()
        );
    }

    fn evidence_for(
        scenario_id: CodexReleaseScenarioId,
        platform: PlatformEnvironment,
    ) -> DriverScenarioEvidence {
        let definition = scenario_definition(scenario_id);
        let mut evidence = DriverScenarioEvidence {
            contract: SCENARIO_EVIDENCE_CONTRACT.to_owned(),
            scenario_id,
            platform,
            state_setup: ScenarioStateSetup {
                canonical_project_state: CanonicalScenarioProjectState {
                    fixture_id: scenario_id,
                    fixture: definition.fixture,
                    platform,
                },
                canonical_project_state_digest: String::new(),
                validated: true,
            },
            boundary_execution: ScenarioBoundaryExecution {
                canonical_invocation: CanonicalScenarioInvocation {
                    scenario_id,
                    platform,
                    boundary: definition.boundary,
                    canonical_project_state_digest: String::new(),
                },
                invocation_digest: String::new(),
                completed: true,
            },
            domain_outcome: ScenarioDomainOutcome {
                canonical_outcome: CanonicalScenarioDomainOutcome {
                    scenario_id,
                    expectation: definition.expectation,
                    disposition: definition.disposition,
                    outcome_code: definition.outcome_code,
                    invocation_digest: String::new(),
                    observed_paths_preserved: RequiredNullable::new(
                        definition.observed_paths_preserved,
                    ),
                },
                canonical_outcome_digest: String::new(),
                validated: true,
            },
            adapter_projection: ScenarioAdapterProjection {
                canonical_projection: CanonicalScenarioAdapterProjection {
                    scenario_id,
                    projection: definition.projection,
                    outcome_code: definition.outcome_code,
                    canonical_outcome_digest: String::new(),
                },
                canonical_projection_digest: String::new(),
                validated: true,
            },
            cleanup_complete: true,
        };
        refresh_linked_digests(&mut evidence);
        evidence
    }

    fn refresh_linked_digests(evidence: &mut DriverScenarioEvidence) {
        evidence.state_setup.canonical_project_state_digest =
            canonical_json_bare_sha256(&evidence.state_setup.canonical_project_state)
                .expect("canonical project-state digest");
        evidence
            .boundary_execution
            .canonical_invocation
            .canonical_project_state_digest =
            evidence.state_setup.canonical_project_state_digest.clone();
        evidence.boundary_execution.invocation_digest =
            canonical_json_bare_sha256(&evidence.boundary_execution.canonical_invocation)
                .expect("canonical invocation digest");
        evidence.domain_outcome.canonical_outcome.invocation_digest =
            evidence.boundary_execution.invocation_digest.clone();
        evidence.domain_outcome.canonical_outcome_digest =
            canonical_json_bare_sha256(&evidence.domain_outcome.canonical_outcome)
                .expect("canonical outcome digest");
        evidence
            .adapter_projection
            .canonical_projection
            .canonical_outcome_digest = evidence.domain_outcome.canonical_outcome_digest.clone();
        evidence.adapter_projection.canonical_projection_digest =
            canonical_json_bare_sha256(&evidence.adapter_projection.canonical_projection)
                .expect("canonical projection digest");
    }
}
