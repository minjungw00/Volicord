use std::{collections::BTreeMap, fs, path::Path};

use serde::Serialize;
use tempfile::TempDir;
use volicord_types::{
    compute_codex_release_evidence_digest, CodexReleaseEvidenceEntry, CodexReleaseEvidenceManifest,
    CodexReleaseEvidenceRunner, CodexReleaseRunnerArchitecture, CodexReleaseScenarioId,
    CodexReleaseScenarioResult, CodexReleaseScenarioStatus, CodexReleaseValidationEvidence,
    CodexReleaseValidationResult, CodexSupportCatalog, IntegrationProfile, PlatformEnvironment,
    ReleaseTargetTriple, RequiredNullable, FIRST_RELEASE_CODEX_CAPABILITIES,
    PINNED_WSL2_ENVIRONMENT_IMAGE,
};

use crate::{
    catalog::{generate_support_entry, parse_declared_capabilities, serialize_support_entry},
    contracts::{
        load_codex_release_evidence_manifest, load_release_target_contract,
        serialize_codex_release_evidence_manifest, ReleaseTargetContract, RELEASE_TARGETS_PATH,
    },
    gate::scenarios::write_synthetic_retained_scenario_evidence,
    io::{git_head, sha256_external_file, ValidationContext},
    pipeline::{
        build_artifact_name, evidence_artifact_name, serialize_verified_release_index,
        verify_publish_inputs_with_contracts, VerifiedReleaseIndex,
    },
};

const RUN_ID: &str = "4242";
const RUN_ATTEMPT: &str = "1";
const OBSERVED_AT: &str = "2026-07-18T00:00:00Z";

#[test]
fn support_entry_generation_is_deterministic_and_hashes_actual_codex_bytes() {
    let temporary = tempfile::tempdir().expect("temporary Codex artifact root");
    let codex_path = temporary.path().join("codex");
    fs::write(&codex_path, b"first exact Codex artifact\n").expect("Codex artifact");
    let context = validation_context();
    let capabilities = parse_declared_capabilities(
        "shared_managed_binding, record_workflow, managed_stdio_mcp, personal_managed_binding",
    )
    .expect("normalized capabilities");

    let first = generate_support_entry(
        &context,
        &codex_path,
        ReleaseTargetTriple::X86_64UnknownLinuxGnu,
        PlatformEnvironment::Linux,
        IntegrationProfile::Record,
        &capabilities,
    )
    .expect("first proposed entry");
    let second = generate_support_entry(
        &context,
        &codex_path,
        ReleaseTargetTriple::X86_64UnknownLinuxGnu,
        PlatformEnvironment::Linux,
        IntegrationProfile::Record,
        &capabilities,
    )
    .expect("second proposed entry");
    assert_eq!(first, second);
    assert_eq!(
        serialize_support_entry(&first).expect("first canonical entry"),
        serialize_support_entry(&second).expect("second canonical entry")
    );

    fs::write(&codex_path, b"changed exact Codex artifact\n").expect("changed Codex artifact");
    let changed = generate_support_entry(
        &context,
        &codex_path,
        ReleaseTargetTriple::X86_64UnknownLinuxGnu,
        PlatformEnvironment::Linux,
        IntegrationProfile::Record,
        &capabilities,
    )
    .expect("changed proposed entry");
    assert_ne!(first.codex_artifact_digest, changed.codex_artifact_digest);
    let output = String::from_utf8(serialize_support_entry(&changed).expect("canonical entry"))
        .expect("entry UTF-8");
    assert!(!output.contains("volicord_artifact_digest"));
    assert!(!output.contains("validation_result"));
    assert!(parse_declared_capabilities("unknown_capability").is_err());
    assert!(generate_support_entry(
        &context,
        &codex_path,
        ReleaseTargetTriple::X86_64UnknownLinuxGnu,
        PlatformEnvironment::Macos,
        IntegrationProfile::Record,
        &capabilities,
    )
    .is_err());
}

#[test]
fn complete_synthetic_bundle_with_calculated_file_digests_is_accepted() {
    let bundle = SyntheticBundle::new();
    let index = bundle
        .verify(&bundle.catalog, true)
        .expect("complete bundle");
    assert_eq!(index.published_artifacts.len(), 5);
    assert_eq!(index.release_evidence.len(), 6);
    assert_eq!(index.source_revision, bundle.source_revision);
}

#[test]
fn verified_release_index_is_deterministic_for_identical_inputs() {
    let bundle = SyntheticBundle::new();
    let first = bundle.verify(&bundle.catalog, true).expect("first index");
    let second = bundle.verify(&bundle.catalog, true).expect("second index");
    assert_eq!(first, second);
    assert_eq!(
        serialize_verified_release_index(&first).expect("first index JSON"),
        serialize_verified_release_index(&second).expect("second index JSON")
    );
}

#[test]
fn empty_production_catalog_is_rejected() {
    let bundle = SyntheticBundle::new();
    let empty = CodexSupportCatalog::from_entries(Vec::new()).expect("empty fixture catalog");
    let error = bundle
        .verify(&empty, true)
        .expect_err("empty production catalog");
    assert!(error.detail().contains("nonempty Codex support catalog"));
}

#[test]
fn incomplete_retained_evidence_is_rejected() {
    let bundle = SyntheticBundle::new();
    let cell = bundle.targets.required_cells()[0];
    let retained = bundle
        .evidence_artifact_path(cell.target_triple, cell.platform_environment)
        .join("scenario-evidence/15-unsupported_host_artifact.evidence");
    fs::remove_file(retained).expect("remove retained scenario evidence");
    assert!(bundle.verify(&bundle.catalog, true).is_err());
}

#[test]
fn not_run_scenario_is_rejected_by_the_final_gate() {
    let bundle = SyntheticBundle::new();
    let cell = bundle.targets.required_cells()[0];
    bundle.rewrite_entry(cell.target_triple, cell.platform_environment, |entry| {
        entry.validation_evidence.validation_result = CodexReleaseValidationResult::Unavailable;
        entry.validation_evidence.scenario_results[0].status =
            CodexReleaseScenarioStatus::Unavailable;
        entry.validation_evidence.scenario_results[0].reason =
            RequiredNullable::some("runner_unavailable".to_owned());
        entry.validation_evidence.scenario_results[1].status = CodexReleaseScenarioStatus::NotRun;
        entry.validation_evidence.scenario_results[1].reason =
            RequiredNullable::some("prerequisite_unavailable".to_owned());
        entry.validation_evidence.scenario_results[1].evidence_digest = RequiredNullable::null();
        entry.validation_evidence.scenario_results[1].observed_at = RequiredNullable::null();
    });
    assert!(bundle.verify(&bundle.catalog, true).is_err());
}

#[test]
fn duplicate_release_evidence_is_rejected() {
    let bundle = SyntheticBundle::new();
    let cell = bundle.targets.required_cells()[0];
    let path = bundle.evidence_manifest_path(cell.target_triple, cell.platform_environment);
    let manifest = load_codex_release_evidence_manifest(&path).expect("cell manifest");
    let entry = manifest.entries()[0].clone();
    write_unchecked_manifest(&path, &[entry.clone(), entry]);
    assert!(bundle.verify(&bundle.catalog, true).is_err());
}

#[test]
fn evidence_from_another_source_revision_is_rejected() {
    let bundle = SyntheticBundle::new();
    let cell = bundle.targets.required_cells()[0];
    let other_revision = different_revision(&bundle.source_revision);
    bundle.rewrite_entry(cell.target_triple, cell.platform_environment, |entry| {
        entry.validation_evidence.source_revision = other_revision;
    });
    assert!(bundle.verify(&bundle.catalog, true).is_err());
}

#[test]
fn volicord_digest_mismatch_is_rejected_against_actual_binary_bytes() {
    let bundle = SyntheticBundle::new();
    let replacement = bundle.root.join("different-volicord");
    fs::write(&replacement, b"different real Volicord bytes\n").expect("replacement bytes");
    let replacement_digest =
        sha256_external_file(&bundle.context, &replacement, None).expect("replacement digest");
    let cell = bundle.targets.required_cells()[0];
    bundle.rewrite_entry(cell.target_triple, cell.platform_environment, |entry| {
        entry.validation_evidence.volicord_artifact_digest = replacement_digest;
    });
    assert!(bundle.verify(&bundle.catalog, true).is_err());
}

#[test]
fn codex_digest_absent_from_runtime_catalog_is_rejected() {
    let bundle = SyntheticBundle::new();
    let replacement = bundle.root.join("different-codex");
    fs::write(&replacement, b"different real Codex bytes\n").expect("replacement bytes");
    let replacement_digest =
        sha256_external_file(&bundle.context, &replacement, None).expect("replacement digest");
    let cell = bundle.targets.required_cells()[0];
    bundle.rewrite_entry(cell.target_triple, cell.platform_environment, |entry| {
        entry.codex_artifact_digest = replacement_digest.clone();
        entry.validation_evidence.codex_artifact_digest = replacement_digest;
    });
    assert!(bundle.verify(&bundle.catalog, true).is_err());
}

#[test]
fn codex_digest_mismatch_inside_release_evidence_is_rejected() {
    let bundle = SyntheticBundle::new();
    let replacement = bundle.root.join("mismatched-codex");
    fs::write(&replacement, b"mismatched real Codex bytes\n").expect("replacement bytes");
    let replacement_digest =
        sha256_external_file(&bundle.context, &replacement, None).expect("replacement digest");
    let cell = bundle.targets.required_cells()[0];
    bundle.rewrite_entry(cell.target_triple, cell.platform_environment, |entry| {
        entry.validation_evidence.codex_artifact_digest = replacement_digest;
    });
    assert!(bundle.verify(&bundle.catalog, true).is_err());
}

#[test]
fn missing_published_target_or_environment_cell_is_rejected() {
    let missing_target = SyntheticBundle::new();
    let target = missing_target.targets.published_targets()[0];
    fs::remove_dir_all(missing_target.build_root.join(build_artifact_name(
        target,
        RUN_ID,
        RUN_ATTEMPT,
    )))
    .expect("remove published target");
    assert!(missing_target
        .verify(&missing_target.catalog, true)
        .is_err());

    let missing_cell = SyntheticBundle::new();
    let wsl2 = missing_cell
        .targets
        .required_cells()
        .iter()
        .find(|cell| cell.platform_environment == PlatformEnvironment::Wsl2)
        .expect("WSL2 cell");
    fs::remove_dir_all(
        missing_cell.evidence_artifact_path(wsl2.target_triple, wsl2.platform_environment),
    )
    .expect("remove WSL2 cell");
    assert!(missing_cell.verify(&missing_cell.catalog, true).is_err());
}

struct SyntheticBundle {
    _temporary: TempDir,
    context: ValidationContext,
    root: std::path::PathBuf,
    build_root: std::path::PathBuf,
    evidence_root: std::path::PathBuf,
    source_revision: String,
    targets: ReleaseTargetContract,
    catalog: CodexSupportCatalog,
}

impl SyntheticBundle {
    fn new() -> Self {
        let temporary = tempfile::tempdir().expect("temporary release bundle");
        let root = fs::canonicalize(temporary.path()).expect("canonical temporary root");
        let build_root = root.join("builds");
        let evidence_root = root.join("evidence");
        let codex_root = root.join("codex");
        fs::create_dir(&build_root).expect("build root");
        fs::create_dir(&evidence_root).expect("evidence root");
        fs::create_dir(&codex_root).expect("Codex root");
        let scenario_driver = root.join("scenario-driver");
        fs::write(&scenario_driver, b"synthetic scenario driver bytes\n").expect("scenario driver");

        let context = validation_context();
        let repository = repository_root();
        let source_revision = git_head(&repository).expect("source revision");
        let targets = load_release_target_contract(&repository.join(RELEASE_TARGETS_PATH))
            .expect("release target contract");

        let capabilities = FIRST_RELEASE_CODEX_CAPABILITIES.to_vec();
        let mut support_entries = Vec::new();
        for cell in targets.required_cells() {
            let codex_path = codex_root.join(format!(
                "{}-{}",
                cell.target_triple,
                cell.platform_environment.as_str()
            ));
            fs::write(
                &codex_path,
                format!(
                    "synthetic Codex artifact for {}/{}\n",
                    cell.target_triple,
                    cell.platform_environment.as_str()
                ),
            )
            .expect("Codex artifact");
            support_entries.push(
                generate_support_entry(
                    &context,
                    &codex_path,
                    cell.target_triple,
                    cell.platform_environment,
                    cell.integration_profile,
                    &capabilities,
                )
                .expect("support entry from actual file"),
            );
        }
        support_entries.sort_by_key(|entry| {
            (
                entry.codex_artifact_digest.clone(),
                entry.target_triple,
                entry.platform_environment,
                entry.integration_profile,
            )
        });
        let catalog = CodexSupportCatalog::from_entries(support_entries)
            .expect("complete fixture support catalog");

        let mut build_digests = BTreeMap::new();
        for target in targets.published_targets() {
            let artifact = build_root.join(build_artifact_name(*target, RUN_ID, RUN_ATTEMPT));
            fs::create_dir(&artifact).expect("build artifact directory");
            let binary_name = release_binary_name(*target);
            let binary_path = artifact.join(binary_name);
            fs::write(
                &binary_path,
                format!("synthetic Volicord artifact for {target}\n"),
            )
            .expect("Volicord artifact");
            let digest = sha256_external_file(&context, &binary_path, None)
                .expect("Volicord artifact digest");
            fs::write(
                artifact.join("volicord.sha256"),
                format!("{digest}  {binary_name}\n"),
            )
            .expect("build digest metadata");
            fs::write(
                artifact.join("build-metadata.json"),
                format!(
                    "{{\"contract_id\":\"volicord.release-build-artifact\",\"target_triple\":\"{target}\",\"source_revision\":\"{source_revision}\",\"binary_name\":\"{binary_name}\",\"binary_sha256\":\"{digest}\"}}\n"
                ),
            )
            .expect("build metadata");
            build_digests.insert(*target, digest);
        }

        for cell in targets.required_cells() {
            let support = catalog
                .entries()
                .iter()
                .find(|entry| {
                    entry.target_triple == cell.target_triple
                        && entry.platform_environment == cell.platform_environment
                        && entry.integration_profile == cell.integration_profile
                })
                .expect("support entry for cell");
            let artifact = evidence_root.join(evidence_artifact_name(
                cell.target_triple,
                cell.platform_environment,
                RUN_ID,
                RUN_ATTEMPT,
            ));
            let retained = artifact.join("scenario-evidence");
            fs::create_dir(&artifact).expect("evidence artifact directory");
            fs::create_dir(&retained).expect("retained evidence directory");
            let mut entry = synthetic_passed_entry(
                cell.target_triple,
                cell.platform_environment,
                support.codex_artifact_digest.clone(),
                build_digests[&cell.target_triple].clone(),
                source_revision.clone(),
            );
            write_synthetic_retained_scenario_evidence(
                &context,
                &retained,
                &scenario_driver,
                &mut entry,
            )
            .expect("retained scenario evidence");
            let manifest = CodexReleaseEvidenceManifest::from_entries(vec![entry])
                .expect("cell evidence manifest");
            let mut bytes = serialize_codex_release_evidence_manifest(&manifest)
                .expect("canonical cell manifest");
            bytes.push(b'\n');
            fs::write(artifact.join("release-evidence.json"), bytes)
                .expect("release evidence manifest");
        }

        Self {
            _temporary: temporary,
            context,
            root,
            build_root,
            evidence_root,
            source_revision,
            targets,
            catalog,
        }
    }

    fn verify(
        &self,
        catalog: &CodexSupportCatalog,
        production_mode: bool,
    ) -> crate::error::ValidationResult<VerifiedReleaseIndex> {
        verify_publish_inputs_with_contracts(
            &self.context,
            &self.build_root,
            &self.evidence_root,
            &self.source_revision,
            RUN_ID,
            RUN_ATTEMPT,
            &self.targets,
            catalog,
            production_mode,
        )
    }

    fn evidence_artifact_path(
        &self,
        target: ReleaseTargetTriple,
        platform: PlatformEnvironment,
    ) -> std::path::PathBuf {
        self.evidence_root.join(evidence_artifact_name(
            target,
            platform,
            RUN_ID,
            RUN_ATTEMPT,
        ))
    }

    fn evidence_manifest_path(
        &self,
        target: ReleaseTargetTriple,
        platform: PlatformEnvironment,
    ) -> std::path::PathBuf {
        self.evidence_artifact_path(target, platform)
            .join("release-evidence.json")
    }

    fn rewrite_entry(
        &self,
        target: ReleaseTargetTriple,
        platform: PlatformEnvironment,
        mutate: impl FnOnce(&mut CodexReleaseEvidenceEntry),
    ) {
        let path = self.evidence_manifest_path(target, platform);
        let manifest = load_codex_release_evidence_manifest(&path).expect("cell manifest");
        let mut entry = manifest.entries()[0].clone();
        mutate(&mut entry);
        entry.validation_evidence.evidence_digest =
            compute_codex_release_evidence_digest(&entry.validation_evidence)
                .expect("mutated evidence digest");
        write_unchecked_manifest(&path, &[entry]);
    }
}

fn synthetic_passed_entry(
    target_triple: ReleaseTargetTriple,
    platform_environment: PlatformEnvironment,
    codex_artifact_digest: String,
    volicord_artifact_digest: String,
    source_revision: String,
) -> CodexReleaseEvidenceEntry {
    let scenario_ids = if platform_environment == PlatformEnvironment::Wsl2 {
        CodexReleaseScenarioId::BASE
            .into_iter()
            .chain(CodexReleaseScenarioId::WSL2_ADDITIONAL)
            .collect::<Vec<_>>()
    } else {
        CodexReleaseScenarioId::BASE.to_vec()
    };
    let scenario_results = scenario_ids
        .into_iter()
        .map(|scenario_id| CodexReleaseScenarioResult {
            scenario_id,
            status: CodexReleaseScenarioStatus::Passed,
            reason: RequiredNullable::null(),
            evidence_digest: RequiredNullable::null(),
            observed_at: RequiredNullable::some(OBSERVED_AT.to_owned()),
        })
        .collect();
    CodexReleaseEvidenceEntry {
        codex_artifact_digest: codex_artifact_digest.clone(),
        target_triple,
        platform_environment,
        observed_capabilities: FIRST_RELEASE_CODEX_CAPABILITIES.to_vec(),
        integration_profile: IntegrationProfile::Record,
        validation_evidence: CodexReleaseValidationEvidence {
            validation_result: CodexReleaseValidationResult::Passed,
            codex_artifact_digest,
            target_triple,
            platform_environment,
            observed_capabilities: FIRST_RELEASE_CODEX_CAPABILITIES.to_vec(),
            integration_profile: IntegrationProfile::Record,
            volicord_artifact_digest,
            source_revision,
            runner: CodexReleaseEvidenceRunner {
                runner_id: format!("synthetic-{}-runner", platform_environment.as_str()),
                target_triple,
                architecture: match target_triple.architecture() {
                    "x86_64" => CodexReleaseRunnerArchitecture::X86_64,
                    "aarch64" => CodexReleaseRunnerArchitecture::Aarch64,
                    _ => unreachable!("release target architecture is closed"),
                },
                os_release: format!("synthetic-{}-release", platform_environment.as_str()),
                environment_image: match platform_environment {
                    PlatformEnvironment::Linux => "ubuntu-24.04",
                    PlatformEnvironment::Macos => "macos-15",
                    PlatformEnvironment::NativeWindows => "windows-2022",
                    PlatformEnvironment::Wsl2 => PINNED_WSL2_ENVIRONMENT_IMAGE,
                }
                .to_owned(),
            },
            scenario_results,
            evidence_digest: String::new(),
            observed_at: OBSERVED_AT.to_owned(),
        },
    }
}

#[derive(Serialize)]
struct UncheckedManifest<'a> {
    contract_id: &'static str,
    entries: &'a [CodexReleaseEvidenceEntry],
}

fn write_unchecked_manifest(path: &Path, entries: &[CodexReleaseEvidenceEntry]) {
    let manifest = UncheckedManifest {
        contract_id: "volicord.codex-release-evidence-manifest",
        entries,
    };
    let mut bytes = serde_json::to_vec(&manifest).expect("unchecked manifest JSON");
    bytes.push(b'\n');
    fs::write(path, bytes).expect("rewrite cell manifest");
}

fn different_revision(source_revision: &str) -> String {
    let mut bytes = source_revision.as_bytes().to_vec();
    bytes[0] = if bytes[0] == b'0' { b'1' } else { b'0' };
    String::from_utf8(bytes).expect("hex revision remains UTF-8")
}

fn validation_context() -> ValidationContext {
    ValidationContext::from_process(Path::new(env!("CARGO_MANIFEST_DIR")))
        .expect("release-validation context")
}

fn repository_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("release-validation package is below repository root")
        .to_path_buf()
}

fn release_binary_name(target: ReleaseTargetTriple) -> &'static str {
    if target == ReleaseTargetTriple::X86_64PcWindowsMsvc {
        "volicord.exe"
    } else {
        "volicord"
    }
}
