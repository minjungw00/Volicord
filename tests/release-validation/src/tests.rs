use std::{fs, path::PathBuf};

use serde_json::Value;

use crate::{
    contracts::{
        checked_in_manifest, compute_evidence_digest, load_manifest, parse_manifest,
        parse_test_only_descriptor, PlatformReleaseStatus, UNSUPPORTED_HOST_ARTIFACT_REASON,
    },
    hosts::codex::FIRST_RELEASE_CODEX_CAPABILITIES,
    platforms::{self, PlatformRunnerBoundary},
    scenarios::{definition, ScenarioExpectation, BASE_SCENARIOS, WSL2_ADDITIONAL_SCENARIOS},
};
use volicord_types::{
    lookup_checked_in_supported_codex_release_cell, CodexReleaseCell, CodexReleaseManifest,
    CodexReleaseManifestError, CodexReleaseRunnerArchitecture as RunnerArchitecture,
    CodexReleaseRunnerCoordinate, CodexReleaseScenarioId, CodexReleaseScenarioResult,
    CodexReleaseScenarioStatus as ScenarioStatus, CodexReleaseValidationEvidence,
    CodexReleaseValidationStatus as ValidationEvidenceStatus, ErrorCode, FailureCategory,
    IntegrationProfile, PlatformEnvironment, PlatformReleaseCoordinate, RequiredNullable,
    CODEX_RELEASE_PLATFORMS,
};

const CODEX_DIGEST: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const VOLICORD_DIGEST: &str = "2222222222222222222222222222222222222222222222222222222222222222";
const SCENARIO_DIGEST: &str = "3333333333333333333333333333333333333333333333333333333333333333";
const OBSERVED_AT: &str = "2026-07-17T00:00:00Z";

#[test]
fn checked_in_manifest_is_honest_and_absent_cells_are_not_run() {
    let manifest = checked_in_manifest().expect("checked-in manifest");
    assert!(manifest.cells().is_empty());
    assert!(!manifest.has_four_passing_platforms());
    for platform in CODEX_RELEASE_PLATFORMS {
        assert_eq!(
            manifest.platform_status(platform),
            PlatformReleaseStatus::NotRun
        );
    }

    let error = lookup_checked_in_supported_codex_release_cell(
        CODEX_DIGEST,
        PlatformEnvironment::Linux,
        &PlatformReleaseCoordinate::Native,
        &FIRST_RELEASE_CODEX_CAPABILITIES,
        IntegrationProfile::Record,
    )
    .expect_err("an absent checked-in cell must not register support");
    assert_eq!(error.error_code(), ErrorCode::UnsupportedContract);
    assert_eq!(
        error.failure_category(),
        FailureCategory::UnsupportedContract
    );
    assert_eq!(error.reason(), UNSUPPORTED_HOST_ARTIFACT_REASON);
}

#[test]
fn checked_in_manifest_path_uses_the_same_strict_loader() {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("contracts/codex-release-manifest.json");
    assert_eq!(
        checked_in_manifest().expect("embedded manifest"),
        load_manifest(&path).expect("path manifest")
    );
}

#[test]
fn platform_contract_linux() {
    let definition = platforms::linux::definition();
    assert_eq!(definition.platform, PlatformEnvironment::Linux);
    assert_eq!(
        definition.runner_boundary,
        PlatformRunnerBoundary::NativeLinux
    );
    assert_eq!(definition.scenarios, BASE_SCENARIOS);
}

#[test]
fn platform_contract_macos() {
    let definition = platforms::macos::definition();
    assert_eq!(definition.platform, PlatformEnvironment::Macos);
    assert_eq!(
        definition.runner_boundary,
        PlatformRunnerBoundary::NativeMacos
    );
    assert_eq!(definition.scenarios, BASE_SCENARIOS);
}

#[test]
fn platform_contract_native_windows() {
    let definition = platforms::windows::definition();
    assert_eq!(definition.platform, PlatformEnvironment::NativeWindows);
    assert_eq!(
        definition.runner_boundary,
        PlatformRunnerBoundary::NativeWindows
    );
    assert_eq!(definition.scenarios, BASE_SCENARIOS);
}

#[test]
fn platform_contract_wsl2() {
    let definition = platforms::wsl2::definition();
    assert_eq!(definition.platform, PlatformEnvironment::Wsl2);
    assert_eq!(
        definition.runner_boundary,
        PlatformRunnerBoundary::PinnedUbuntuLtsWsl2
    );
    assert_eq!(
        definition.scenarios,
        BASE_SCENARIOS
            .into_iter()
            .chain(WSL2_ADDITIONAL_SCENARIOS)
            .collect::<Vec<_>>()
    );
    assert_eq!(platforms::wsl2::TOPOLOGY_SCENARIOS.len(), 5);
    assert_eq!(
        platforms::wsl2::TOPOLOGY_SCENARIOS.map(|scenario| scenario.expectation),
        [
            ScenarioExpectation::AcceptWsl2Ext4,
            ScenarioExpectation::RejectWsl2Drvfs,
            ScenarioExpectation::RejectWsl2CrossTopology,
            ScenarioExpectation::RejectWsl1,
            ScenarioExpectation::RejectNativeWindowsReceiptReuse,
        ]
    );
}

#[test]
fn platform_definitions_are_independent_and_canonical() {
    let definitions = platforms::all();
    assert_eq!(
        definitions.each_ref().map(|definition| definition.platform),
        CODEX_RELEASE_PLATFORMS
    );
    assert!(definitions[..3]
        .iter()
        .all(|definition| definition.scenarios == BASE_SCENARIOS));
    assert_eq!(definitions[3].scenarios.len(), 21);
}

#[test]
fn wsl2_static_scenarios_keep_ext4_and_rejection_boundaries_explicit() {
    let expectations = WSL2_ADDITIONAL_SCENARIOS.map(|scenario| definition(scenario).expectation);
    assert_eq!(
        expectations,
        [
            ScenarioExpectation::RejectStaleWsl2ProcessAndReceipt,
            ScenarioExpectation::AcceptWsl2Ext4,
            ScenarioExpectation::RejectWsl2Drvfs,
            ScenarioExpectation::RejectWsl2CrossTopology,
            ScenarioExpectation::RejectWsl1,
            ScenarioExpectation::RejectNativeWindowsReceiptReuse,
        ]
    );
}

#[test]
fn strict_manifest_accepts_current_cell_shapes_and_canonical_platform_order() {
    let cells = CODEX_RELEASE_PLATFORMS
        .into_iter()
        .enumerate()
        .map(|(index, platform)| passed_cell(platform, digit_digest(index + 1)))
        .collect::<Vec<_>>();
    let manifest = parse_cells(&cells).expect("four current cells");
    assert!(manifest.has_four_passing_platforms());
    assert_eq!(manifest.cells(), cells);
}

#[test]
fn strict_manifest_rejects_duplicate_unknown_missing_and_reordered_fields() {
    let bytes = serialize_cells(&[passed_cell(
        PlatformEnvironment::Linux,
        CODEX_DIGEST.to_owned(),
    )]);
    let text = String::from_utf8(bytes).expect("manifest UTF-8");

    let duplicate = text.replacen(
        &format!("\"artifact_digest\":\"{CODEX_DIGEST}\""),
        &format!("\"artifact_digest\":\"{CODEX_DIGEST}\",\"artifact_digest\":\"{CODEX_DIGEST}\""),
        1,
    );
    assert!(parse_manifest(duplicate.as_bytes())
        .expect_err("duplicate field")
        .detail()
        .contains("duplicate JSON field"));

    let unknown = text.replacen(
        &format!("\"artifact_digest\":\"{CODEX_DIGEST}\""),
        &format!("\"unknown\":true,\"artifact_digest\":\"{CODEX_DIGEST}\""),
        1,
    );
    assert!(parse_manifest(unknown.as_bytes()).is_err());

    let missing = text.replacen(&format!("\"artifact_digest\":\"{CODEX_DIGEST}\","), "", 1);
    assert!(parse_manifest(missing.as_bytes()).is_err());

    let value: Value = serde_json::from_str(&text).expect("manifest JSON");
    let reordered = serde_json::to_vec(&value).expect("reordered JSON");
    assert!(parse_manifest(&reordered).is_err());
}

#[test]
fn strict_manifest_requires_nullable_scenario_members_to_be_present() {
    let bytes = serialize_cells(&[passed_cell(
        PlatformEnvironment::Linux,
        CODEX_DIGEST.to_owned(),
    )]);
    let text = String::from_utf8(bytes).expect("manifest UTF-8");
    let missing_reason = text.replacen("\"reason\":null,", "", 1);
    assert!(parse_manifest(missing_reason.as_bytes()).is_err());
    let missing_observed_at = text.replacen(&format!("\"observed_at\":\"{OBSERVED_AT}\"}}"), "", 1);
    assert!(parse_manifest(missing_observed_at.as_bytes()).is_err());
}

#[test]
fn strict_manifest_rejects_noncanonical_platform_order_and_duplicate_platforms() {
    let linux = passed_cell(PlatformEnvironment::Linux, digit_digest(1));
    let macos = passed_cell(PlatformEnvironment::Macos, digit_digest(2));
    assert!(parse_cells(&[macos.clone(), linux.clone()]).is_err());
    assert!(parse_cells(&[linux.clone(), linux]).is_err());
    assert!(parse_cells(&[
        passed_cell(PlatformEnvironment::Linux, digit_digest(1)),
        passed_cell(PlatformEnvironment::Macos, digit_digest(2)),
        passed_cell(PlatformEnvironment::NativeWindows, digit_digest(3)),
        passed_cell(PlatformEnvironment::Wsl2, digit_digest(4)),
        passed_cell(PlatformEnvironment::Wsl2, digit_digest(5)),
    ])
    .is_err());
}

#[test]
fn strict_manifest_rejects_noncanonical_capabilities_and_scenario_catalogs() {
    let mut capabilities = passed_cell(PlatformEnvironment::Linux, CODEX_DIGEST.to_owned());
    capabilities.observed_capabilities.swap(0, 1);
    capabilities.validation_evidence.observed_capabilities =
        capabilities.observed_capabilities.clone();
    refresh_evidence_digest(&mut capabilities);
    assert!(parse_cells(&[capabilities]).is_err());

    let mut missing_scenario = passed_cell(PlatformEnvironment::Linux, CODEX_DIGEST.to_owned());
    missing_scenario.validation_evidence.scenario_results.pop();
    refresh_evidence_digest(&mut missing_scenario);
    assert!(parse_cells(&[missing_scenario]).is_err());

    let mut wsl_without_wsl_scenarios =
        passed_cell(PlatformEnvironment::Wsl2, CODEX_DIGEST.to_owned());
    wsl_without_wsl_scenarios
        .validation_evidence
        .scenario_results
        .truncate(BASE_SCENARIOS.len());
    refresh_evidence_digest(&mut wsl_without_wsl_scenarios);
    assert!(parse_cells(&[wsl_without_wsl_scenarios]).is_err());
}

#[test]
fn strict_manifest_rejects_coordinate_and_evidence_digest_mismatches() {
    let mut coordinate = passed_cell(PlatformEnvironment::Linux, CODEX_DIGEST.to_owned());
    coordinate.validation_evidence.platform = PlatformEnvironment::Macos;
    refresh_evidence_digest(&mut coordinate);
    assert!(parse_cells(&[coordinate]).is_err());

    let mut digest = passed_cell(PlatformEnvironment::Linux, CODEX_DIGEST.to_owned());
    digest.validation_evidence.evidence_digest = digit_digest(9);
    assert!(parse_cells(&[digest]).is_err());
}

#[test]
fn scenario_cross_field_rules_and_top_level_status_are_strict() {
    let mut failed = failed_cell(PlatformEnvironment::Linux, CODEX_DIGEST.to_owned());
    assert_eq!(
        parse_cells(&[failed.clone()])
            .expect("failed attempt")
            .platform_status(PlatformEnvironment::Linux),
        PlatformReleaseStatus::Failed
    );
    failed.validation_evidence.status = ValidationEvidenceStatus::Passed;
    refresh_evidence_digest(&mut failed);
    assert!(parse_cells(&[failed]).is_err());

    let unavailable = unavailable_cell(PlatformEnvironment::Wsl2, CODEX_DIGEST.to_owned());
    assert_eq!(
        parse_cells(&[unavailable])
            .expect("unavailable attempt")
            .platform_status(PlatformEnvironment::Wsl2),
        PlatformReleaseStatus::Unavailable
    );

    let mut invalid_not_run = unavailable_cell(PlatformEnvironment::Wsl2, CODEX_DIGEST.to_owned());
    let result = &mut invalid_not_run.validation_evidence.scenario_results[1];
    result.evidence_digest = RequiredNullable::some(SCENARIO_DIGEST.to_owned());
    refresh_evidence_digest(&mut invalid_not_run);
    assert!(parse_cells(&[invalid_not_run]).is_err());
}

#[test]
fn digests_timestamps_reasons_and_runner_strings_are_validated() {
    let mut uppercase = passed_cell(PlatformEnvironment::Linux, CODEX_DIGEST.to_owned());
    uppercase.artifact_digest = "A".repeat(64);
    uppercase.validation_evidence.artifact_digest = uppercase.artifact_digest.clone();
    refresh_evidence_digest(&mut uppercase);
    assert!(parse_cells(&[uppercase]).is_err());

    let mut timestamp = passed_cell(PlatformEnvironment::Linux, CODEX_DIGEST.to_owned());
    timestamp.validation_evidence.observed_at = "2026-07-17T00:00:00+00:00".to_owned();
    refresh_evidence_digest(&mut timestamp);
    assert!(parse_cells(&[timestamp]).is_err());

    let mut reason = failed_cell(PlatformEnvironment::Linux, CODEX_DIGEST.to_owned());
    reason.validation_evidence.scenario_results[0].reason =
        RequiredNullable::some("Invalid-Reason".to_owned());
    refresh_evidence_digest(&mut reason);
    assert!(parse_cells(&[reason]).is_err());

    let mut runner = passed_cell(PlatformEnvironment::Linux, CODEX_DIGEST.to_owned());
    runner.validation_evidence.runner.runner_id = "\n".to_owned();
    refresh_evidence_digest(&mut runner);
    assert!(parse_cells(&[runner]).is_err());

    let mut wsl2_image = passed_cell(PlatformEnvironment::Wsl2, digit_digest(4));
    wsl2_image.validation_evidence.runner.environment_image = "Ubuntu-22.04-LTS-WSL2".to_owned();
    refresh_evidence_digest(&mut wsl2_image);
    assert!(parse_cells(&[wsl2_image]).is_err());
}

#[test]
fn exact_support_lookup_never_widens_artifact_platform_or_capabilities() {
    let passed = passed_cell(PlatformEnvironment::Linux, CODEX_DIGEST.to_owned());
    let unavailable = unavailable_cell(PlatformEnvironment::Wsl2, digit_digest(4));
    let manifest = parse_cells(&[passed, unavailable]).expect("mixed manifest");

    assert!(manifest
        .lookup_supported_cell(
            CODEX_DIGEST,
            PlatformEnvironment::Linux,
            &PlatformReleaseCoordinate::Native,
            &FIRST_RELEASE_CODEX_CAPABILITIES,
            IntegrationProfile::Record,
        )
        .is_ok());

    let mut reversed = FIRST_RELEASE_CODEX_CAPABILITIES;
    reversed.reverse();
    let partial = &FIRST_RELEASE_CODEX_CAPABILITIES[..3];
    for result in [
        manifest.lookup_supported_cell(
            &digit_digest(8),
            PlatformEnvironment::Linux,
            &PlatformReleaseCoordinate::Native,
            &FIRST_RELEASE_CODEX_CAPABILITIES,
            IntegrationProfile::Record,
        ),
        manifest.lookup_supported_cell(
            CODEX_DIGEST,
            PlatformEnvironment::Macos,
            &PlatformReleaseCoordinate::Native,
            &FIRST_RELEASE_CODEX_CAPABILITIES,
            IntegrationProfile::Record,
        ),
        manifest.lookup_supported_cell(
            CODEX_DIGEST,
            PlatformEnvironment::Linux,
            &PlatformReleaseCoordinate::Native,
            &reversed,
            IntegrationProfile::Record,
        ),
        manifest.lookup_supported_cell(
            CODEX_DIGEST,
            PlatformEnvironment::Linux,
            &PlatformReleaseCoordinate::Native,
            partial,
            IntegrationProfile::Record,
        ),
        manifest.lookup_supported_cell(
            &digit_digest(4),
            PlatformEnvironment::Wsl2,
            &PlatformReleaseCoordinate::first_release_wsl2(),
            &FIRST_RELEASE_CODEX_CAPABILITIES,
            IntegrationProfile::Record,
        ),
    ] {
        assert_eq!(
            result.expect_err("unsupported coordinates").reason(),
            UNSUPPORTED_HOST_ARTIFACT_REASON
        );
    }
}

#[test]
fn explicit_test_only_descriptor_is_valid_only_on_the_fixture_boundary() {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/test-only-codex-descriptor.json");
    let bytes = fs::read(path).expect("test-only descriptor fixture");
    let descriptor = parse_test_only_descriptor(&bytes).expect("explicit test-only descriptor");
    assert!(descriptor.test_only);
    assert_eq!(descriptor.platform, PlatformEnvironment::Linux);
    assert!(
        parse_manifest(&format!("[{}]", String::from_utf8(bytes).unwrap()).into_bytes()).is_err()
    );

    let subset = br#"{
      "test_only": true,
      "fixture_id": "negative-capability-fixture",
      "artifact_digest": "0000000000000000000000000000000000000000000000000000000000000000",
      "platform": "linux",
      "observed_capabilities": ["managed_stdio_mcp"]
    }"#;
    assert!(parse_test_only_descriptor(subset).is_ok());

    let false_marker = br#"{
      "test_only": false,
      "fixture_id": "fixture",
      "artifact_digest": "0000000000000000000000000000000000000000000000000000000000000000",
      "platform": "linux",
      "observed_capabilities": ["managed_stdio_mcp", "personal_managed_binding", "record_workflow", "shared_managed_binding"]
    }"#;
    assert!(parse_test_only_descriptor(false_marker).is_err());
}

#[test]
fn evidence_digest_encoding_is_deterministic_and_field_sensitive() {
    let cell = passed_cell(PlatformEnvironment::Linux, CODEX_DIGEST.to_owned());
    let digest = compute_evidence_digest(&cell.validation_evidence).expect("evidence digest");
    assert_eq!(digest, cell.validation_evidence.evidence_digest);
    assert_eq!(
        digest,
        "ede149aaeac1daf57301df4848d28a873d8d2c30f244bf6c719c3efcbb6f7118"
    );

    let mut changed = cell.validation_evidence;
    changed.runner.environment_image.push_str("-changed");
    assert_ne!(
        compute_evidence_digest(&changed).expect("changed evidence digest"),
        digest
    );
}

fn parse_cells(
    cells: &[CodexReleaseCell],
) -> Result<CodexReleaseManifest, CodexReleaseManifestError> {
    parse_manifest(&serialize_cells(cells))
}

fn serialize_cells(cells: &[CodexReleaseCell]) -> Vec<u8> {
    serde_json::to_vec(cells).expect("serialize release cells")
}

fn passed_cell(platform: PlatformEnvironment, artifact_digest: String) -> CodexReleaseCell {
    cell_with_status(platform, artifact_digest, ValidationEvidenceStatus::Passed)
}

fn failed_cell(platform: PlatformEnvironment, artifact_digest: String) -> CodexReleaseCell {
    cell_with_status(platform, artifact_digest, ValidationEvidenceStatus::Failed)
}

fn unavailable_cell(platform: PlatformEnvironment, artifact_digest: String) -> CodexReleaseCell {
    cell_with_status(
        platform,
        artifact_digest,
        ValidationEvidenceStatus::Unavailable,
    )
}

fn cell_with_status(
    platform: PlatformEnvironment,
    artifact_digest: String,
    status: ValidationEvidenceStatus,
) -> CodexReleaseCell {
    let scenario_ids = if platform == PlatformEnvironment::Wsl2 {
        BASE_SCENARIOS
            .into_iter()
            .chain(WSL2_ADDITIONAL_SCENARIOS)
            .collect::<Vec<_>>()
    } else {
        BASE_SCENARIOS.to_vec()
    };
    let scenario_results = scenario_ids
        .into_iter()
        .enumerate()
        .map(|(index, scenario_id)| match status {
            ValidationEvidenceStatus::Passed => passed_scenario(scenario_id),
            ValidationEvidenceStatus::Failed if index == 0 => failed_scenario(scenario_id),
            ValidationEvidenceStatus::Unavailable if index == 0 => {
                unavailable_scenario(scenario_id)
            }
            ValidationEvidenceStatus::Failed | ValidationEvidenceStatus::Unavailable => {
                not_run_scenario(scenario_id)
            }
        })
        .collect();
    let mut validation_evidence = CodexReleaseValidationEvidence {
        status,
        artifact_digest: artifact_digest.clone(),
        platform,
        observed_capabilities: FIRST_RELEASE_CODEX_CAPABILITIES.to_vec(),
        integration_profile: IntegrationProfile::Record,
        volicord_artifact_digest: VOLICORD_DIGEST.to_owned(),
        runner: CodexReleaseRunnerCoordinate {
            runner_id: format!("runner-{}", platform.as_str()),
            target_triple: target_triple(platform).to_owned(),
            architecture: RunnerArchitecture::X86_64,
            os_release: format!("{}-release", platform.as_str()),
            environment_image: environment_image(platform).to_owned(),
        },
        scenario_results,
        evidence_digest: String::new(),
        observed_at: OBSERVED_AT.to_owned(),
    };
    validation_evidence.evidence_digest =
        compute_evidence_digest(&validation_evidence).expect("compute fixture evidence digest");
    CodexReleaseCell {
        artifact_digest,
        platform,
        observed_capabilities: FIRST_RELEASE_CODEX_CAPABILITIES.to_vec(),
        integration_profile: IntegrationProfile::Record,
        validation_evidence,
    }
}

fn passed_scenario(scenario_id: CodexReleaseScenarioId) -> CodexReleaseScenarioResult {
    CodexReleaseScenarioResult {
        scenario_id,
        status: ScenarioStatus::Passed,
        reason: RequiredNullable::null(),
        evidence_digest: RequiredNullable::some(SCENARIO_DIGEST.to_owned()),
        observed_at: RequiredNullable::some(OBSERVED_AT.to_owned()),
    }
}

fn failed_scenario(scenario_id: CodexReleaseScenarioId) -> CodexReleaseScenarioResult {
    CodexReleaseScenarioResult {
        scenario_id,
        status: ScenarioStatus::Failed,
        reason: RequiredNullable::some("assertion_failed".to_owned()),
        evidence_digest: RequiredNullable::some(SCENARIO_DIGEST.to_owned()),
        observed_at: RequiredNullable::some(OBSERVED_AT.to_owned()),
    }
}

fn unavailable_scenario(scenario_id: CodexReleaseScenarioId) -> CodexReleaseScenarioResult {
    CodexReleaseScenarioResult {
        scenario_id,
        status: ScenarioStatus::Unavailable,
        reason: RequiredNullable::some("runner_unavailable".to_owned()),
        evidence_digest: RequiredNullable::null(),
        observed_at: RequiredNullable::some(OBSERVED_AT.to_owned()),
    }
}

fn not_run_scenario(scenario_id: CodexReleaseScenarioId) -> CodexReleaseScenarioResult {
    CodexReleaseScenarioResult {
        scenario_id,
        status: ScenarioStatus::NotRun,
        reason: RequiredNullable::some("prerequisite_unavailable".to_owned()),
        evidence_digest: RequiredNullable::null(),
        observed_at: RequiredNullable::null(),
    }
}

fn refresh_evidence_digest(cell: &mut CodexReleaseCell) {
    cell.validation_evidence.evidence_digest =
        compute_evidence_digest(&cell.validation_evidence).expect("refresh evidence digest");
}

fn digit_digest(digit: usize) -> String {
    char::from_digit(u32::try_from(digit).expect("one decimal digit"), 16)
        .expect("hex digit")
        .to_string()
        .repeat(64)
}

fn target_triple(platform: PlatformEnvironment) -> &'static str {
    match platform {
        PlatformEnvironment::Linux | PlatformEnvironment::Wsl2 => "x86_64-unknown-linux-gnu",
        PlatformEnvironment::Macos => "x86_64-apple-darwin",
        PlatformEnvironment::NativeWindows => "x86_64-pc-windows-msvc",
    }
}

fn environment_image(platform: PlatformEnvironment) -> &'static str {
    match platform {
        PlatformEnvironment::Linux => "ubuntu-24.04",
        PlatformEnvironment::Macos => "macos-15",
        PlatformEnvironment::NativeWindows => "windows-2022",
        PlatformEnvironment::Wsl2 => "Ubuntu-24.04-LTS-WSL2",
    }
}
