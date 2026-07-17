use std::{fs, path::Path};

use crate::{
    contracts::{
        embedded_codex_support_catalog, load_codex_release_evidence_manifest,
        load_codex_support_catalog, parse_codex_release_evidence_manifest,
        parse_codex_support_catalog, parse_test_only_codex_descriptor,
        serialize_codex_release_evidence_manifest, serialize_codex_support_catalog,
        CODEX_RELEASE_EVIDENCE_MANIFEST_PATH, CODEX_SUPPORT_CATALOG_PATH,
        UNSUPPORTED_HOST_ARTIFACT_REASON,
    },
    hosts::codex::FIRST_RELEASE_CODEX_CAPABILITIES,
    platforms::{self, PlatformRunnerBoundary},
    scenarios::{definition, ScenarioExpectation, BASE_SCENARIOS, WSL2_ADDITIONAL_SCENARIOS},
};
use volicord_types::{
    compute_codex_release_evidence_digest, lookup_embedded_codex_support_entry,
    CodexReleaseEvidenceEntry, CodexReleaseEvidenceManifest, CodexReleaseEvidenceRunner,
    CodexReleasePlatformStatus, CodexReleaseRunnerArchitecture as RunnerArchitecture,
    CodexReleaseScenarioId, CodexReleaseScenarioResult,
    CodexReleaseScenarioStatus as ScenarioStatus, CodexReleaseValidationEvidence,
    CodexReleaseValidationResult, CodexSupportCatalog, CodexSupportEntry, ErrorCode,
    FailureCategory, IntegrationProfile, PlatformEnvironment, PlatformReleaseCoordinate,
    RequiredNullable, CODEX_RELEASE_PLATFORMS,
};

const CODEX_DIGEST: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const VOLICORD_DIGEST: &str = "2222222222222222222222222222222222222222222222222222222222222222";
const OTHER_VOLICORD_DIGEST: &str =
    "4444444444444444444444444444444444444444444444444444444444444444";
const SCENARIO_DIGEST: &str = "3333333333333333333333333333333333333333333333333333333333333333";
const OBSERVED_AT: &str = "2026-07-17T00:00:00Z";

#[test]
fn checked_in_contracts_are_empty_honest_and_separate() {
    let catalog = embedded_codex_support_catalog().expect("embedded support catalog");
    assert!(catalog.entries().is_empty());

    let root = repository_root();
    assert_eq!(
        catalog,
        load_codex_support_catalog(&root.join(CODEX_SUPPORT_CATALOG_PATH))
            .expect("on-disk support catalog")
    );
    let evidence =
        load_codex_release_evidence_manifest(&root.join(CODEX_RELEASE_EVIDENCE_MANIFEST_PATH))
            .expect("external evidence manifest");
    assert!(evidence.entries().is_empty());
    assert!(!evidence.has_four_passing_platforms());
    evidence
        .validate_against_support_catalog(&catalog)
        .expect("empty evidence is supported by empty policy");
    for platform in CODEX_RELEASE_PLATFORMS {
        assert_eq!(
            evidence.platform_status(platform),
            CodexReleasePlatformStatus::NotRun
        );
    }
}

#[test]
fn runtime_support_lookup_does_not_consume_release_evidence() {
    let evidence = evidence_manifest(vec![passed_evidence_entry(
        PlatformEnvironment::Linux,
        CODEX_DIGEST.to_owned(),
        VOLICORD_DIGEST.to_owned(),
    )]);
    assert_eq!(evidence.entries().len(), 1);

    let error = lookup_embedded_codex_support_entry(
        CODEX_DIGEST,
        PlatformEnvironment::Linux,
        &PlatformReleaseCoordinate::native(),
        &FIRST_RELEASE_CODEX_CAPABILITIES,
        IntegrationProfile::Record,
    )
    .expect_err("external passing evidence must not register runtime support");
    assert_eq!(error.error_code(), ErrorCode::UnsupportedContract);
    assert_eq!(
        error.failure_category(),
        FailureCategory::UnsupportedContract
    );
    assert_eq!(error.reason(), UNSUPPORTED_HOST_ARTIFACT_REASON);
}

#[test]
fn release_evidence_volicord_digest_does_not_affect_catalog_identity() {
    let catalog = support_catalog(vec![support_entry(
        PlatformEnvironment::Linux,
        CODEX_DIGEST.to_owned(),
    )]);
    let identity = catalog.identity_digest().expect("catalog identity");

    let first = passed_evidence_entry(
        PlatformEnvironment::Linux,
        CODEX_DIGEST.to_owned(),
        VOLICORD_DIGEST.to_owned(),
    );
    let second = passed_evidence_entry(
        PlatformEnvironment::Linux,
        CODEX_DIGEST.to_owned(),
        OTHER_VOLICORD_DIGEST.to_owned(),
    );
    assert_ne!(
        first.validation_evidence.evidence_digest,
        second.validation_evidence.evidence_digest
    );
    assert_eq!(
        identity,
        catalog.identity_digest().expect("catalog identity")
    );
}

#[test]
fn support_entry_matches_without_a_volicord_binary_digest() {
    let catalog = support_catalog(vec![support_entry(
        PlatformEnvironment::Linux,
        CODEX_DIGEST.to_owned(),
    )]);
    let matched = catalog
        .lookup_supported_entry(
            CODEX_DIGEST,
            PlatformEnvironment::Linux,
            &PlatformReleaseCoordinate::native(),
            &FIRST_RELEASE_CODEX_CAPABILITIES,
            IntegrationProfile::Record,
        )
        .expect("exact support entry");
    assert_eq!(matched.codex_artifact_digest, CODEX_DIGEST);

    let serialized = String::from_utf8(
        serialize_codex_support_catalog(&catalog).expect("canonical support serialization"),
    )
    .expect("catalog UTF-8");
    assert!(!serialized.contains("volicord_artifact_digest"));
    assert!(!serialized.contains("validation_result"));
    assert!(!serialized.contains("validation_evidence"));
}

#[test]
fn release_evidence_without_catalog_artifact_is_rejected() {
    let catalog = CodexSupportCatalog::from_entries(Vec::new()).expect("empty catalog");
    let evidence = evidence_manifest(vec![passed_evidence_entry(
        PlatformEnvironment::Linux,
        CODEX_DIGEST.to_owned(),
        VOLICORD_DIGEST.to_owned(),
    )]);
    let error = evidence
        .validate_against_support_catalog(&catalog)
        .expect_err("unregistered Codex evidence");
    assert!(error
        .detail()
        .contains("no exact Codex support-catalog entry"));
}

#[test]
fn empty_support_catalog_fails_closed_for_every_artifact() {
    let catalog = support_catalog(Vec::new());
    for platform in CODEX_RELEASE_PLATFORMS {
        let coordinate = if platform == PlatformEnvironment::Wsl2 {
            PlatformReleaseCoordinate::first_release_wsl2()
        } else {
            PlatformReleaseCoordinate::native()
        };
        assert!(catalog
            .lookup_supported_entry(
                CODEX_DIGEST,
                platform,
                &coordinate,
                &FIRST_RELEASE_CODEX_CAPABILITIES,
                IntegrationProfile::Record,
            )
            .is_err());
    }
}

#[test]
fn production_runtime_sources_do_not_embed_external_release_evidence() {
    let crates = repository_root().join("crates");
    visit_files(&crates, &mut |path| {
        if matches!(
            path.extension().and_then(|value| value.to_str()),
            Some("rs" | "toml")
        ) {
            let source = fs::read_to_string(path).expect("production source is UTF-8");
            assert!(
                !source.contains("codex-release-evidence-manifest.json"),
                "production source references external release evidence: {}",
                path.display()
            );
        }
    });
}

#[test]
fn malformed_and_duplicate_support_entries_are_rejected_deterministically() {
    let entry = support_entry(PlatformEnvironment::Linux, CODEX_DIGEST.to_owned());
    let first = CodexSupportCatalog::from_entries(vec![entry.clone(), entry.clone()])
        .expect_err("duplicate support entries");
    let second = CodexSupportCatalog::from_entries(vec![entry.clone(), entry])
        .expect_err("duplicate support entries");
    assert_eq!(first.detail(), second.detail());

    let malformed =
        br#"{"contract_id":"volicord.codex-support-catalog","entries":[],"unknown":true}"#;
    let first = parse_codex_support_catalog(malformed).expect_err("unknown support field");
    let second = parse_codex_support_catalog(malformed).expect_err("unknown support field");
    assert_eq!(first.detail(), second.detail());
}

#[test]
fn malformed_and_duplicate_evidence_entries_are_rejected_deterministically() {
    let entry = passed_evidence_entry(
        PlatformEnvironment::Linux,
        CODEX_DIGEST.to_owned(),
        VOLICORD_DIGEST.to_owned(),
    );
    let first = CodexReleaseEvidenceManifest::from_entries(vec![entry.clone(), entry.clone()])
        .expect_err("duplicate evidence entries");
    let second = CodexReleaseEvidenceManifest::from_entries(vec![entry.clone(), entry])
        .expect_err("duplicate evidence entries");
    assert_eq!(first.detail(), second.detail());

    let malformed = br#"{"contract_id":"volicord.codex-release-evidence-manifest","entries":[],"unknown":true}"#;
    let first =
        parse_codex_release_evidence_manifest(malformed).expect_err("unknown evidence field");
    let second =
        parse_codex_release_evidence_manifest(malformed).expect_err("unknown evidence field");
    assert_eq!(first.detail(), second.detail());
}

#[test]
fn canonical_serialization_round_trips_and_evidence_digest_is_field_sensitive() {
    let catalog = support_catalog(vec![support_entry(
        PlatformEnvironment::Linux,
        CODEX_DIGEST.to_owned(),
    )]);
    let catalog_bytes = serialize_codex_support_catalog(&catalog).expect("serialize catalog");
    assert_eq!(
        parse_codex_support_catalog(&catalog_bytes).expect("parse catalog"),
        catalog
    );

    let entry = passed_evidence_entry(
        PlatformEnvironment::Linux,
        CODEX_DIGEST.to_owned(),
        VOLICORD_DIGEST.to_owned(),
    );
    let digest = entry.validation_evidence.evidence_digest.clone();
    assert_eq!(
        compute_codex_release_evidence_digest(&entry.validation_evidence)
            .expect("recompute evidence digest"),
        digest
    );
    let manifest = evidence_manifest(vec![entry.clone()]);
    let evidence_bytes =
        serialize_codex_release_evidence_manifest(&manifest).expect("serialize evidence");
    assert_eq!(
        parse_codex_release_evidence_manifest(&evidence_bytes).expect("parse evidence"),
        manifest
    );

    let changed = passed_evidence_entry(
        PlatformEnvironment::Linux,
        CODEX_DIGEST.to_owned(),
        OTHER_VOLICORD_DIGEST.to_owned(),
    );
    assert_ne!(changed.validation_evidence.evidence_digest, digest);
}

#[test]
fn exact_support_lookup_never_widens_artifact_platform_or_capabilities() {
    let catalog = support_catalog(vec![support_entry(
        PlatformEnvironment::Linux,
        CODEX_DIGEST.to_owned(),
    )]);
    let mut reversed = FIRST_RELEASE_CODEX_CAPABILITIES;
    reversed.reverse();
    for result in [
        catalog.lookup_supported_entry(
            &"8".repeat(64),
            PlatformEnvironment::Linux,
            &PlatformReleaseCoordinate::native(),
            &FIRST_RELEASE_CODEX_CAPABILITIES,
            IntegrationProfile::Record,
        ),
        catalog.lookup_supported_entry(
            CODEX_DIGEST,
            PlatformEnvironment::Macos,
            &PlatformReleaseCoordinate::native(),
            &FIRST_RELEASE_CODEX_CAPABILITIES,
            IntegrationProfile::Record,
        ),
        catalog.lookup_supported_entry(
            CODEX_DIGEST,
            PlatformEnvironment::Linux,
            &PlatformReleaseCoordinate::native(),
            &reversed,
            IntegrationProfile::Record,
        ),
        catalog.lookup_supported_entry(
            CODEX_DIGEST,
            PlatformEnvironment::Linux,
            &PlatformReleaseCoordinate::native(),
            &FIRST_RELEASE_CODEX_CAPABILITIES[..3],
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
fn strict_evidence_status_and_scenario_catalog_rules_are_enforced() {
    let mut failed = evidence_entry_with_result(
        PlatformEnvironment::Linux,
        CODEX_DIGEST.to_owned(),
        VOLICORD_DIGEST.to_owned(),
        CodexReleaseValidationResult::Failed,
    );
    assert_eq!(
        evidence_manifest(vec![failed.clone()]).platform_status(PlatformEnvironment::Linux),
        CodexReleasePlatformStatus::Failed
    );
    failed.validation_evidence.validation_result = CodexReleaseValidationResult::Passed;
    refresh_evidence_digest(&mut failed);
    assert!(CodexReleaseEvidenceManifest::from_entries(vec![failed]).is_err());

    let mut missing = passed_evidence_entry(
        PlatformEnvironment::Linux,
        CODEX_DIGEST.to_owned(),
        VOLICORD_DIGEST.to_owned(),
    );
    missing.validation_evidence.scenario_results.pop();
    refresh_evidence_digest(&mut missing);
    assert!(CodexReleaseEvidenceManifest::from_entries(vec![missing]).is_err());
}

#[test]
fn explicit_test_only_descriptor_stays_outside_both_contracts() {
    let path =
        repository_root().join("tests/release-validation/fixtures/test-only-codex-descriptor.json");
    let bytes = fs::read(path).expect("test-only descriptor fixture");
    let descriptor = parse_test_only_codex_descriptor(&bytes).expect("test-only descriptor");
    assert!(descriptor.test_only);
    assert_eq!(descriptor.platform_environment, PlatformEnvironment::Linux);
    assert!(parse_codex_support_catalog(&bytes).is_err());
    assert!(parse_codex_release_evidence_manifest(&bytes).is_err());
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

fn support_entry(platform: PlatformEnvironment, digest: String) -> CodexSupportEntry {
    CodexSupportEntry {
        codex_artifact_digest: digest,
        platform_environment: platform,
        platform_release_coordinate: if platform == PlatformEnvironment::Wsl2 {
            PlatformReleaseCoordinate::first_release_wsl2()
        } else {
            PlatformReleaseCoordinate::native()
        },
        integration_profile: IntegrationProfile::Record,
        verified_capabilities: FIRST_RELEASE_CODEX_CAPABILITIES.to_vec(),
    }
}

fn support_catalog(entries: Vec<CodexSupportEntry>) -> CodexSupportCatalog {
    CodexSupportCatalog::from_entries(entries).expect("valid support catalog")
}

fn evidence_manifest(entries: Vec<CodexReleaseEvidenceEntry>) -> CodexReleaseEvidenceManifest {
    CodexReleaseEvidenceManifest::from_entries(entries).expect("valid release evidence")
}

fn passed_evidence_entry(
    platform: PlatformEnvironment,
    codex_digest: String,
    volicord_digest: String,
) -> CodexReleaseEvidenceEntry {
    evidence_entry_with_result(
        platform,
        codex_digest,
        volicord_digest,
        CodexReleaseValidationResult::Passed,
    )
}

fn evidence_entry_with_result(
    platform: PlatformEnvironment,
    codex_digest: String,
    volicord_digest: String,
    validation_result: CodexReleaseValidationResult,
) -> CodexReleaseEvidenceEntry {
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
        .map(|(index, scenario_id)| match validation_result {
            CodexReleaseValidationResult::Passed => passed_scenario(scenario_id),
            CodexReleaseValidationResult::Failed if index == 0 => failed_scenario(scenario_id),
            CodexReleaseValidationResult::Unavailable if index == 0 => {
                unavailable_scenario(scenario_id)
            }
            CodexReleaseValidationResult::Failed | CodexReleaseValidationResult::Unavailable => {
                not_run_scenario(scenario_id)
            }
        })
        .collect();
    let mut validation_evidence = CodexReleaseValidationEvidence {
        validation_result,
        codex_artifact_digest: codex_digest.clone(),
        platform_environment: platform,
        observed_capabilities: FIRST_RELEASE_CODEX_CAPABILITIES.to_vec(),
        integration_profile: IntegrationProfile::Record,
        volicord_artifact_digest: volicord_digest,
        runner: CodexReleaseEvidenceRunner {
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
        compute_codex_release_evidence_digest(&validation_evidence)
            .expect("compute fixture evidence digest");
    CodexReleaseEvidenceEntry {
        codex_artifact_digest: codex_digest,
        platform_environment: platform,
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

fn refresh_evidence_digest(entry: &mut CodexReleaseEvidenceEntry) {
    entry.validation_evidence.evidence_digest =
        compute_codex_release_evidence_digest(&entry.validation_evidence)
            .expect("refresh evidence digest");
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

fn repository_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("release-validation package is below repository root")
        .to_path_buf()
}

fn visit_files(directory: &Path, visitor: &mut impl FnMut(&Path)) {
    for entry in fs::read_dir(directory).expect("read production source directory") {
        let entry = entry.expect("read production source entry");
        let path = entry.path();
        if path.is_dir() {
            visit_files(&path, visitor);
        } else {
            visitor(&path);
        }
    }
}
