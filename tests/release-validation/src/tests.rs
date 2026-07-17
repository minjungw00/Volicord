use std::{collections::BTreeSet, fs, path::Path};

use crate::{
    contracts::{
        embedded_codex_support_catalog, load_codex_release_evidence_manifest,
        load_codex_support_catalog, load_release_target_contract,
        parse_codex_release_evidence_manifest, parse_codex_support_catalog,
        parse_release_target_contract, parse_test_only_codex_descriptor,
        serialize_codex_release_evidence_manifest, serialize_codex_support_catalog,
        CODEX_RELEASE_EVIDENCE_MANIFEST_PATH, CODEX_SUPPORT_CATALOG_PATH, RELEASE_TARGETS_PATH,
        UNSUPPORTED_HOST_ARTIFACT_REASON,
    },
    hosts::codex::FIRST_RELEASE_CODEX_CAPABILITIES,
    platforms::{self, PlatformRunnerBoundary},
    scenarios::{definition, ScenarioExpectation, BASE_SCENARIOS, WSL2_ADDITIONAL_SCENARIOS},
};
use volicord_types::{
    compute_codex_release_evidence_digest, lookup_embedded_codex_support_entry,
    CodexReleaseCellStatus, CodexReleaseEvidenceEntry, CodexReleaseEvidenceManifest,
    CodexReleaseEvidenceRunner, CodexReleaseRunnerArchitecture as RunnerArchitecture,
    CodexReleaseScenarioId, CodexReleaseScenarioResult,
    CodexReleaseScenarioStatus as ScenarioStatus, CodexReleaseValidationEvidence,
    CodexReleaseValidationResult, CodexSupportCatalog, CodexSupportEntry, ErrorCode,
    FailureCategory, IntegrationProfile, PlatformEnvironment, PlatformReleaseCoordinate,
    ReleaseTargetTriple, RequiredNullable,
};

const CODEX_DIGEST: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const VOLICORD_DIGEST: &str = "2222222222222222222222222222222222222222222222222222222222222222";
const OTHER_VOLICORD_DIGEST: &str =
    "4444444444444444444444444444444444444444444444444444444444444444";
const SCENARIO_DIGEST: &str = "3333333333333333333333333333333333333333333333333333333333333333";
const OBSERVED_AT: &str = "2026-07-17T00:00:00Z";
const LINUX_X86: ReleaseTargetTriple = ReleaseTargetTriple::X86_64UnknownLinuxGnu;

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
    evidence
        .validate_against_support_catalog(&catalog)
        .expect("empty evidence is supported by empty policy");
    let targets = load_release_target_contract(&root.join(RELEASE_TARGETS_PATH))
        .expect("release target contract");
    assert_eq!(targets.published_targets().len(), 5);
    assert_eq!(targets.required_cells().len(), 6);
    for cell in targets.required_cells() {
        assert_eq!(
            evidence.cell_status(
                cell.target_triple,
                cell.platform_environment,
                cell.integration_profile
            ),
            CodexReleaseCellStatus::NotRun
        );
    }
}

#[test]
fn runtime_support_lookup_does_not_consume_release_evidence() {
    let evidence = evidence_manifest(vec![passed_evidence_entry(
        LINUX_X86,
        PlatformEnvironment::Linux,
        CODEX_DIGEST.to_owned(),
        VOLICORD_DIGEST.to_owned(),
    )]);
    assert_eq!(evidence.entries().len(), 1);

    let error = lookup_embedded_codex_support_entry(
        CODEX_DIGEST,
        LINUX_X86,
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
        LINUX_X86,
        PlatformEnvironment::Linux,
        CODEX_DIGEST.to_owned(),
    )]);
    let identity = catalog.identity_digest().expect("catalog identity");

    let first = passed_evidence_entry(
        LINUX_X86,
        PlatformEnvironment::Linux,
        CODEX_DIGEST.to_owned(),
        VOLICORD_DIGEST.to_owned(),
    );
    let second = passed_evidence_entry(
        LINUX_X86,
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
        LINUX_X86,
        PlatformEnvironment::Linux,
        CODEX_DIGEST.to_owned(),
    )]);
    let matched = catalog
        .lookup_supported_entry(
            CODEX_DIGEST,
            LINUX_X86,
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
        LINUX_X86,
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
    let targets = load_release_target_contract(&repository_root().join(RELEASE_TARGETS_PATH))
        .expect("release target contract");
    for cell in targets.required_cells() {
        let coordinate = if cell.platform_environment == PlatformEnvironment::Wsl2 {
            PlatformReleaseCoordinate::first_release_wsl2()
        } else {
            PlatformReleaseCoordinate::native()
        };
        assert!(catalog
            .lookup_supported_entry(
                CODEX_DIGEST,
                cell.target_triple,
                cell.platform_environment,
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
    let entry = support_entry(
        LINUX_X86,
        PlatformEnvironment::Linux,
        CODEX_DIGEST.to_owned(),
    );
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
        LINUX_X86,
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
        LINUX_X86,
        PlatformEnvironment::Linux,
        CODEX_DIGEST.to_owned(),
    )]);
    let catalog_bytes = serialize_codex_support_catalog(&catalog).expect("serialize catalog");
    assert_eq!(
        parse_codex_support_catalog(&catalog_bytes).expect("parse catalog"),
        catalog
    );

    let entry = passed_evidence_entry(
        LINUX_X86,
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
        LINUX_X86,
        PlatformEnvironment::Linux,
        CODEX_DIGEST.to_owned(),
        OTHER_VOLICORD_DIGEST.to_owned(),
    );
    assert_ne!(changed.validation_evidence.evidence_digest, digest);
}

#[test]
fn exact_support_lookup_never_widens_artifact_platform_or_capabilities() {
    let catalog = support_catalog(vec![support_entry(
        LINUX_X86,
        PlatformEnvironment::Linux,
        CODEX_DIGEST.to_owned(),
    )]);
    let mut reversed = FIRST_RELEASE_CODEX_CAPABILITIES;
    reversed.reverse();
    for result in [
        catalog.lookup_supported_entry(
            &"8".repeat(64),
            LINUX_X86,
            PlatformEnvironment::Linux,
            &PlatformReleaseCoordinate::native(),
            &FIRST_RELEASE_CODEX_CAPABILITIES,
            IntegrationProfile::Record,
        ),
        catalog.lookup_supported_entry(
            CODEX_DIGEST,
            ReleaseTargetTriple::X86_64AppleDarwin,
            PlatformEnvironment::Macos,
            &PlatformReleaseCoordinate::native(),
            &FIRST_RELEASE_CODEX_CAPABILITIES,
            IntegrationProfile::Record,
        ),
        catalog.lookup_supported_entry(
            CODEX_DIGEST,
            ReleaseTargetTriple::Aarch64UnknownLinuxGnu,
            PlatformEnvironment::Linux,
            &PlatformReleaseCoordinate::native(),
            &FIRST_RELEASE_CODEX_CAPABILITIES,
            IntegrationProfile::Record,
        ),
        catalog.lookup_supported_entry(
            CODEX_DIGEST,
            LINUX_X86,
            PlatformEnvironment::Linux,
            &PlatformReleaseCoordinate::native(),
            &reversed,
            IntegrationProfile::Record,
        ),
        catalog.lookup_supported_entry(
            CODEX_DIGEST,
            LINUX_X86,
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
        LINUX_X86,
        PlatformEnvironment::Linux,
        CODEX_DIGEST.to_owned(),
        VOLICORD_DIGEST.to_owned(),
        CodexReleaseValidationResult::Failed,
    );
    assert_eq!(
        evidence_manifest(vec![failed.clone()]).cell_status(
            LINUX_X86,
            PlatformEnvironment::Linux,
            IntegrationProfile::Record
        ),
        CodexReleaseCellStatus::Failed
    );
    failed.validation_evidence.validation_result = CodexReleaseValidationResult::Passed;
    refresh_evidence_digest(&mut failed);
    assert!(CodexReleaseEvidenceManifest::from_entries(vec![failed]).is_err());

    let mut missing = passed_evidence_entry(
        LINUX_X86,
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
    for target in [
        ReleaseTargetTriple::X86_64UnknownLinuxGnu,
        ReleaseTargetTriple::Aarch64UnknownLinuxGnu,
    ] {
        let definition = platforms::linux::definition(target);
        assert_eq!(definition.target_triple, target);
        assert_eq!(definition.platform, PlatformEnvironment::Linux);
        assert_eq!(
            definition.runner_boundary,
            PlatformRunnerBoundary::NativeLinux
        );
        assert_eq!(definition.scenarios, BASE_SCENARIOS);
    }
}

#[test]
fn platform_contract_macos() {
    for target in [
        ReleaseTargetTriple::Aarch64AppleDarwin,
        ReleaseTargetTriple::X86_64AppleDarwin,
    ] {
        let definition = platforms::macos::definition(target);
        assert_eq!(definition.target_triple, target);
        assert_eq!(definition.platform, PlatformEnvironment::Macos);
        assert_eq!(
            definition.runner_boundary,
            PlatformRunnerBoundary::NativeMacos
        );
        assert_eq!(definition.scenarios, BASE_SCENARIOS);
    }
}

#[test]
fn platform_contract_native_windows() {
    let definition = platforms::windows::definition(ReleaseTargetTriple::X86_64PcWindowsMsvc);
    assert_eq!(definition.platform, PlatformEnvironment::NativeWindows);
    assert_eq!(
        definition.runner_boundary,
        PlatformRunnerBoundary::NativeWindows
    );
    assert_eq!(definition.scenarios, BASE_SCENARIOS);
}

#[test]
fn platform_contract_wsl2() {
    let definition = platforms::wsl2::definition(LINUX_X86);
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
    assert_eq!(definitions.len(), 6);
    assert!(definitions[..5]
        .iter()
        .all(|definition| definition.scenarios == BASE_SCENARIOS));
    assert_eq!(definitions[5].scenarios.len(), 21);
    assert_eq!(definitions[0].target_triple, LINUX_X86);
    assert_eq!(
        definitions[1].target_triple,
        ReleaseTargetTriple::Aarch64UnknownLinuxGnu
    );
    assert_eq!(
        definitions[2].target_triple,
        ReleaseTargetTriple::Aarch64AppleDarwin
    );
    assert_eq!(
        definitions[3].target_triple,
        ReleaseTargetTriple::X86_64AppleDarwin
    );
    assert_eq!(
        definitions[4].target_triple,
        ReleaseTargetTriple::X86_64PcWindowsMsvc
    );
    assert_eq!(definitions[5].target_triple, LINUX_X86);
    assert_ne!(definitions[0].platform, definitions[5].platform);
}

#[test]
fn all_five_targets_and_six_environment_cells_accept_exact_evidence() {
    let contract = load_release_target_contract(&repository_root().join(RELEASE_TARGETS_PATH))
        .expect("release target contract");
    let mut entries = contract
        .required_cells()
        .iter()
        .map(|cell| {
            passed_evidence_entry(
                cell.target_triple,
                cell.platform_environment,
                CODEX_DIGEST.to_owned(),
                VOLICORD_DIGEST.to_owned(),
            )
        })
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| {
        (
            entry.codex_artifact_digest.clone(),
            entry.target_triple,
            entry.platform_environment,
            entry.integration_profile,
        )
    });
    let manifest = evidence_manifest(entries);
    assert_eq!(manifest.entries().len(), 6);
    for cell in contract.required_cells() {
        assert_eq!(
            manifest.cell_status(
                cell.target_triple,
                cell.platform_environment,
                cell.integration_profile
            ),
            CodexReleaseCellStatus::Passed
        );
    }
}

#[test]
fn evidence_rejects_architecture_and_target_mismatches() {
    let mut architecture = passed_evidence_entry(
        ReleaseTargetTriple::Aarch64AppleDarwin,
        PlatformEnvironment::Macos,
        CODEX_DIGEST.to_owned(),
        VOLICORD_DIGEST.to_owned(),
    );
    architecture.validation_evidence.runner.architecture = RunnerArchitecture::X86_64;
    refresh_evidence_digest(&mut architecture);
    assert!(
        CodexReleaseEvidenceManifest::from_entries(vec![architecture])
            .expect_err("architecture mismatch")
            .detail()
            .contains("architecture")
    );

    let mut target = passed_evidence_entry(
        ReleaseTargetTriple::X86_64AppleDarwin,
        PlatformEnvironment::Macos,
        CODEX_DIGEST.to_owned(),
        VOLICORD_DIGEST.to_owned(),
    );
    target.validation_evidence.runner.target_triple = ReleaseTargetTriple::Aarch64AppleDarwin;
    refresh_evidence_digest(&mut target);
    assert!(CodexReleaseEvidenceManifest::from_entries(vec![target])
        .expect_err("runner target mismatch")
        .detail()
        .contains("target_triple"));
}

#[test]
fn native_linux_and_wsl2_support_are_never_interchangeable() {
    let native = support_catalog(vec![support_entry(
        LINUX_X86,
        PlatformEnvironment::Linux,
        CODEX_DIGEST.to_owned(),
    )]);
    assert!(native
        .lookup_supported_entry(
            CODEX_DIGEST,
            LINUX_X86,
            PlatformEnvironment::Wsl2,
            &PlatformReleaseCoordinate::first_release_wsl2(),
            &FIRST_RELEASE_CODEX_CAPABILITIES,
            IntegrationProfile::Record,
        )
        .is_err());

    let wsl2 = support_catalog(vec![support_entry(
        LINUX_X86,
        PlatformEnvironment::Wsl2,
        CODEX_DIGEST.to_owned(),
    )]);
    assert!(wsl2
        .lookup_supported_entry(
            CODEX_DIGEST,
            LINUX_X86,
            PlatformEnvironment::Linux,
            &PlatformReleaseCoordinate::native(),
            &FIRST_RELEASE_CODEX_CAPABILITIES,
            IntegrationProfile::Record,
        )
        .is_err());
}

#[test]
fn release_target_contract_rejects_missing_duplicate_or_mismatched_cells() {
    let path = repository_root().join(RELEASE_TARGETS_PATH);
    let bytes = fs::read(path).expect("release target contract");
    let canonical: serde_json::Value = serde_json::from_slice(&bytes).expect("contract JSON");

    let mut missing = canonical.clone();
    missing["required_cells"]
        .as_array_mut()
        .expect("cells")
        .retain(|cell| cell["target_triple"] != "aarch64-unknown-linux-gnu");
    assert!(
        parse_release_target_contract(&serde_json::to_vec(&missing).unwrap())
            .expect_err("published target without cell")
            .contains("no corresponding required cell")
    );

    let mut duplicate = canonical.clone();
    let first = duplicate["required_cells"][0].clone();
    duplicate["required_cells"]
        .as_array_mut()
        .expect("cells")
        .push(first);
    assert!(
        parse_release_target_contract(&serde_json::to_vec(&duplicate).unwrap())
            .expect_err("duplicate cell")
            .contains("duplicates")
    );

    let mut mismatch = canonical.clone();
    mismatch["required_cells"][0]["target_triple"] =
        serde_json::Value::String("aarch64-apple-darwin".to_owned());
    assert!(
        parse_release_target_contract(&serde_json::to_vec(&mismatch).unwrap())
            .expect_err("target/environment mismatch")
            .contains("does not match")
    );

    let mut unpublished = canonical.clone();
    unpublished["published_targets"]
        .as_array_mut()
        .expect("targets")
        .retain(|target| target != "x86_64-pc-windows-msvc");
    assert!(
        parse_release_target_contract(&serde_json::to_vec(&unpublished).unwrap())
            .expect_err("required cell target is not published")
            .contains("is not published")
    );

    let mut unknown = canonical;
    unknown["published_targets"][0] =
        serde_json::Value::String("x86_64-unknown-linux-musl".to_owned());
    assert!(parse_release_target_contract(&serde_json::to_vec(&unknown).unwrap()).is_err());

    let mut unknown_environment: serde_json::Value =
        serde_json::from_slice(&bytes).expect("contract JSON");
    unknown_environment["required_cells"][0]["platform_environment"] =
        serde_json::Value::String("ubuntu".to_owned());
    assert!(
        parse_release_target_contract(&serde_json::to_vec(&unknown_environment).unwrap()).is_err()
    );
}

#[test]
fn release_workflow_builds_once_and_publishes_only_validated_raw_artifacts() {
    let root = repository_root();
    let contract = load_release_target_contract(&root.join(RELEASE_TARGETS_PATH))
        .expect("release target contract");
    let release: serde_yaml::Value = serde_yaml::from_str(
        &fs::read_to_string(root.join(".github/workflows/release.yml")).expect("release workflow"),
    )
    .expect("release workflow YAML");
    let build_job = &release["jobs"]["build-binaries"];
    let packaging_entries = build_job["strategy"]["matrix"]["include"]
        .as_sequence()
        .expect("raw binary build matrix");
    let packaging = packaging_entries
        .iter()
        .map(|entry| {
            entry["target"]
                .as_str()
                .expect("packaging target")
                .parse::<ReleaseTargetTriple>()
                .expect("known packaging target")
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(packaging_entries.len(), packaging.len());
    assert_eq!(
        packaging,
        contract
            .published_targets()
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
    );
    let all_release_runs = release["jobs"]
        .as_mapping()
        .expect("release jobs")
        .values()
        .flat_map(workflow_job_runs)
        .collect::<Vec<_>>();
    let volicord_build_commands = all_release_runs
        .iter()
        .filter(|run| {
            run.contains("cargo build")
                && run.contains("-p volicord-cli")
                && run.contains("--bin volicord")
        })
        .collect::<Vec<_>>();
    assert_eq!(volicord_build_commands.len(), 1);
    assert!(workflow_job_runs(build_job).any(|run| {
        run.contains("volicord.release-build-artifact")
            && run.contains("source_revision")
            && run.contains("binary_sha256")
    }));
    assert!(build_job["steps"]
        .as_sequence()
        .expect("build steps")
        .iter()
        .any(|step| {
            step["uses"].as_str() == Some("actions/upload-artifact@v4")
                && step["with"]["name"].as_str()
                    == Some(
                        "volicord-build-${{ matrix.target }}-${{ github.run_id }}-${{ github.run_attempt }}",
                    )
                && step["with"]["if-no-files-found"].as_str() == Some("error")
        }));

    let release_cell_jobs = release["jobs"]
        .as_mapping()
        .expect("release jobs")
        .iter()
        .filter_map(|(job_id, job)| {
            workflow_job_runs(job)
                .find(|run| run.contains("codex-release-cell-gate -- --capture-candidate"))
                .map(|command| {
                    (
                        job_id.as_str().expect("release-cell job ID"),
                        job,
                        release_cell_from_gate_command(command),
                    )
                })
        })
        .collect::<Vec<_>>();
    let release_cells = release_cell_jobs
        .iter()
        .map(|(_, _, cell)| *cell)
        .collect::<BTreeSet<_>>();
    assert_eq!(release_cell_jobs.len(), release_cells.len());
    let required_cells = contract
        .required_cells()
        .iter()
        .map(|cell| {
            (
                cell.target_triple,
                cell.platform_environment,
                cell.integration_profile,
            )
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(release_cells, required_cells);

    let artifact_suffix = "${{ github.run_id }}-${{ github.run_attempt }}";
    for (_, job, (target, platform, _)) in &release_cell_jobs {
        let needs = job["needs"].as_sequence().expect("release-cell needs");
        assert!(needs
            .iter()
            .any(|need| need.as_str() == Some("build-binaries")));
        let expected_build_name = format!("volicord-build-{target}-{artifact_suffix}");
        let expected_evidence_name = format!(
            "volicord-release-evidence-{target}-{}-{artifact_suffix}",
            platform.as_str()
        );
        let steps = job["steps"].as_sequence().expect("release-cell steps");
        assert!(steps.iter().any(|step| {
            step["uses"].as_str() == Some("actions/download-artifact@v4")
                && step["with"]["name"].as_str() == Some(expected_build_name.as_str())
        }));
        assert!(steps.iter().any(|step| {
            step["uses"].as_str() == Some("actions/upload-artifact@v4")
                && step["with"]["name"].as_str() == Some(expected_evidence_name.as_str())
                && step["with"]["if-no-files-found"].as_str() == Some("error")
        }));
        let runs = workflow_job_runs(job).collect::<Vec<_>>();
        assert!(runs
            .iter()
            .any(|run| run.contains("--verify-build-artifact")));
        assert!(runs
            .iter()
            .any(|run| run.contains("--verify-cell-evidence")));
        assert!(runs.iter().any(|run| {
            run.contains("VOLICORD_CODEX_RELEASE_VOLICORD_PATH=")
                && (run.contains("/build/volicord")
                    || run.contains("build/volicord.exe")
                    || *platform == PlatformEnvironment::Wsl2)
        }));
    }

    let linux_x86_job = release_cell_jobs
        .iter()
        .find(|(_, _, cell)| {
            cell.0 == ReleaseTargetTriple::X86_64UnknownLinuxGnu
                && cell.1 == PlatformEnvironment::Linux
        })
        .expect("native Linux x86-64 cell")
        .1;
    let wsl2_job = release_cell_jobs
        .iter()
        .find(|(_, _, cell)| cell.1 == PlatformEnvironment::Wsl2)
        .expect("WSL2 cell")
        .1;
    let linux_download = downloaded_artifact_name(linux_x86_job);
    let wsl2_download = downloaded_artifact_name(wsl2_job);
    assert_eq!(linux_download, wsl2_download);
    assert!(workflow_job_runs(wsl2_job).any(|run| {
        run.contains("wsl.exe")
            && run.contains("cp --")
            && run.contains("sha256sum --")
            && run.contains("VOLICORD_CODEX_RELEASE_VOLICORD_PATH")
    }));

    let gate_job_ids = release_cell_jobs
        .iter()
        .map(|(job_id, _, _)| (*job_id).to_owned())
        .collect::<BTreeSet<_>>();
    let publish_needs = release["jobs"]["publish-release"]["needs"]
        .as_sequence()
        .expect("publish needs")
        .iter()
        .map(|need| need.as_str().expect("need job id").to_owned())
        .collect::<BTreeSet<_>>();
    assert_eq!(gate_job_ids.len(), 6);
    assert!(gate_job_ids.is_subset(&publish_needs));
    assert!(publish_needs.contains("build-binaries"));

    let publish_job = &release["jobs"]["publish-release"];
    let publish_runs = workflow_job_runs(publish_job).collect::<Vec<_>>();
    assert!(publish_runs.iter().all(|run| {
        !run.contains("cargo build")
            && !run.contains("-p volicord-cli")
            && !run.contains("--bin volicord ")
    }));
    assert!(publish_runs
        .iter()
        .any(|run| run.contains("--verify-publish-evidence")));
    assert!(publish_runs
        .iter()
        .any(|run| run.contains("scripts/package-release-artifacts.sh")));
    assert!(publish_runs.iter().all(|run| !run.contains("--clobber")));
    let publish_steps = publish_job["steps"].as_sequence().expect("publish steps");
    assert!(publish_steps.iter().any(|step| {
        step["uses"].as_str() == Some("actions/download-artifact@v4")
            && step["with"]["pattern"]
                .as_str()
                .is_some_and(|pattern| pattern.starts_with("volicord-build-*"))
    }));
    assert!(publish_steps.iter().any(|step| {
        step["uses"].as_str() == Some("actions/download-artifact@v4")
            && step["with"]["pattern"]
                .as_str()
                .is_some_and(|pattern| pattern.starts_with("volicord-release-evidence-*"))
    }));

    let package_script = fs::read_to_string(root.join("scripts/package-release-artifacts.sh"))
        .expect("release packaging script");
    assert!(!package_script.contains("cargo"));
    assert!(package_script.contains("source_binary=\"$artifact/$binary_name\""));
    assert!(package_script.contains("sha256_file \"$verification/$binary_name\""));
    for target in contract.published_targets() {
        assert!(package_script.contains(target.as_str()));
    }

    let ci: serde_yaml::Value = serde_yaml::from_str(
        &fs::read_to_string(root.join(".github/workflows/ci.yml")).expect("CI workflow"),
    )
    .expect("CI workflow YAML");
    let ci_entries = ci["jobs"]["codex-release-cell-contracts"]["strategy"]["matrix"]["include"]
        .as_sequence()
        .expect("CI release-cell matrix");
    let cells = ci_entries
        .iter()
        .map(|entry| {
            (
                entry["target_triple"]
                    .as_str()
                    .expect("CI target")
                    .parse::<ReleaseTargetTriple>()
                    .expect("known CI target"),
                parse_platform_value(entry["platform_environment"].as_str().expect("CI platform")),
                IntegrationProfile::Record,
            )
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(ci_entries.len(), cells.len());
    assert_eq!(
        cells,
        contract
            .required_cells()
            .iter()
            .map(|cell| {
                (
                    cell.target_triple,
                    cell.platform_environment,
                    cell.integration_profile,
                )
            })
            .collect::<BTreeSet<_>>()
    );
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

fn support_entry(
    target_triple: ReleaseTargetTriple,
    platform: PlatformEnvironment,
    digest: String,
) -> CodexSupportEntry {
    CodexSupportEntry {
        codex_artifact_digest: digest,
        target_triple,
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
    target_triple: ReleaseTargetTriple,
    platform: PlatformEnvironment,
    codex_digest: String,
    volicord_digest: String,
) -> CodexReleaseEvidenceEntry {
    evidence_entry_with_result(
        target_triple,
        platform,
        codex_digest,
        volicord_digest,
        CodexReleaseValidationResult::Passed,
    )
}

fn evidence_entry_with_result(
    target_triple: ReleaseTargetTriple,
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
        target_triple,
        platform_environment: platform,
        observed_capabilities: FIRST_RELEASE_CODEX_CAPABILITIES.to_vec(),
        integration_profile: IntegrationProfile::Record,
        volicord_artifact_digest: volicord_digest,
        runner: CodexReleaseEvidenceRunner {
            runner_id: format!("runner-{}", platform.as_str()),
            target_triple,
            architecture: runner_architecture(target_triple),
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
        target_triple,
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

fn runner_architecture(target_triple: ReleaseTargetTriple) -> RunnerArchitecture {
    match target_triple.architecture() {
        "x86_64" => RunnerArchitecture::X86_64,
        "aarch64" => RunnerArchitecture::Aarch64,
        _ => unreachable!("release target architecture is closed"),
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

fn parse_platform_value(value: &str) -> PlatformEnvironment {
    match value {
        "linux" => PlatformEnvironment::Linux,
        "macos" => PlatformEnvironment::Macos,
        "native_windows" => PlatformEnvironment::NativeWindows,
        "wsl2" => PlatformEnvironment::Wsl2,
        _ => panic!("unknown platform environment {value}"),
    }
}

fn workflow_job_runs(job: &serde_yaml::Value) -> impl Iterator<Item = &str> {
    job["steps"]
        .as_sequence()
        .into_iter()
        .flatten()
        .filter_map(|step| step["run"].as_str())
}

fn downloaded_artifact_name(job: &serde_yaml::Value) -> &str {
    job["steps"]
        .as_sequence()
        .expect("workflow job steps")
        .iter()
        .find(|step| step["uses"].as_str() == Some("actions/download-artifact@v4"))
        .and_then(|step| step["with"]["name"].as_str())
        .expect("exact downloaded artifact name")
}

fn release_cell_from_gate_command(
    command: &str,
) -> (ReleaseTargetTriple, PlatformEnvironment, IntegrationProfile) {
    let tokens = command.split_whitespace().collect::<Vec<_>>();
    let target_index = tokens
        .iter()
        .position(|token| *token == "--capture-candidate")
        .expect("release capture target flag");
    let platform_index = tokens
        .iter()
        .position(|token| *token == "--platform")
        .expect("release gate platform flag");
    (
        tokens[target_index + 1]
            .parse()
            .expect("known release gate target"),
        parse_platform_value(tokens[platform_index + 1]),
        IntegrationProfile::Record,
    )
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
