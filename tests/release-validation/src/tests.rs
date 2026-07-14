use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::Duration as StdDuration,
};

use serde_json::Value;
use sha2::{Digest, Sha256};
use tempfile::TempDir;
use volicord_types::{
    host_feature_implementation_for_version, HostFeature, HostFeatureImplementation,
    HostFeatureSupportStatus, IntegrationProfile, REVIEWED_CODEX_HOST_VERSION,
};

use crate::{
    audit::{run_audit, AuditRequest},
    evaluation::evaluate_release_matrix,
    gate::{run_gate, GateRequest},
    io::{git_archive_sha256, parse_strict_json, sha256_bytes, ValidationContext},
    schema::{
        expected_assertion_ids, AuditVerdict, Candidate, CandidateBuildEnvironment, Cell,
        CellAssertion, CellEnvironment, GateVerdict, HostKind, ImplementationDisposition,
        RequiredNullable, RunState, AUDIT_SCHEMA, CANDIDATE_SCHEMA, CELL_INPUTS_DIGEST_DOMAIN,
        CELL_SCHEMA, MANIFEST_SCHEMA, SOURCE_ARCHIVE_ALGORITHM,
    },
};

const EVALUATED_AT: &str = "2026-01-01T02:00:00Z";
const TARGET: &str = "x86_64-unknown-linux-gnu";

#[test]
fn release_contract_identifiers_use_v2_without_changing_candidate_or_archive_v1() {
    assert_eq!(CANDIDATE_SCHEMA, "volicord-release-candidate-v1");
    assert_eq!(SOURCE_ARCHIVE_ALGORITHM, "git_archive_tar_sha256_v1");
    assert_eq!(CELL_SCHEMA, "volicord-host-release-cell-v2");
    assert_eq!(MANIFEST_SCHEMA, "volicord-host-release-manifest-v2");
    assert_eq!(AUDIT_SCHEMA, "volicord-host-release-audit-v2");
    assert_eq!(
        CELL_INPUTS_DIGEST_DOMAIN,
        b"volicord-host-release-cell-inputs-v2\0"
    );
}

#[test]
fn strict_json_rejects_duplicate_and_unknown_members() {
    let duplicate = br#"{"schema":"one","schema":"two"}"#;
    let error = parse_strict_json::<Value>(duplicate).expect_err("duplicate key must fail");
    assert!(error.detail().contains("duplicate JSON object member"));

    let unknown = br#"{
        "schema":"volicord-release-candidate-v1",
        "candidate_id":"candidate",
        "candidate_path":"/tmp/candidate",
        "source_revision":"0123456789012345678901234567890123456789",
        "source_clean":true,
        "source_archive_algorithm":"git_archive_tar_sha256_v1",
        "source_archive_sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "target_triple":"x86_64-unknown-linux-gnu",
        "release_profile":"release",
        "binary_sha256":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        "build_environment":{
            "runner_os":"linux",
            "runner_os_version":"test",
            "runner_arch":"x86_64",
            "git_version":"git test",
            "rustc_version":"rustc test",
            "cargo_version":"cargo test"
        },
        "recorded_at":"2026-01-01T00:00:00Z",
        "unexpected":true
    }"#;
    let error = parse_strict_json::<Candidate>(unknown).expect_err("unknown field must fail");
    assert!(error.detail().contains("unknown field"));
}

#[cfg(unix)]
#[test]
fn exact_matrix_gate_and_separate_audit_pass() {
    let fixture = Fixture::new();
    let manifest = fixture.run_gate("manifest.json", EVALUATED_AT);
    assert_eq!(manifest.verdict, GateVerdict::Pass);
    assert_eq!(manifest.cells.len(), 12);
    assert_eq!(manifest.requested_verified_claims.len(), 10);
    assert_eq!(
        manifest
            .cells
            .iter()
            .filter(|cell| cell.derived_status == HostFeatureSupportStatus::UnsupportedByHost)
            .count(),
        2
    );

    let audit = run_audit(
        &fixture.context,
        &AuditRequest {
            candidate_descriptor: fixture.candidate_descriptor.clone(),
            cell_directory: fixture.cell_directory.clone(),
            manifest: fixture.external.path().join("manifest.json"),
            audit_output: fixture.external.path().join("audit.json"),
            started_at: EVALUATED_AT.to_owned(),
            evaluated_at: EVALUATED_AT.to_owned(),
        },
    )
    .expect("independent audit");
    assert_eq!(audit.audit_verdict, AuditVerdict::Pass);
    assert_eq!(audit.schema, AUDIT_SCHEMA);
    assert_eq!(audit.recalculated_verdict, GateVerdict::Pass);
    assert_eq!(audit.recalculated_cells.len(), 12);
    assert_eq!(
        audit.cell_directory,
        fixture.cell_directory.to_string_lossy()
    );
    assert_eq!(audit.cell_inputs_sha256.len(), 64);
    assert_eq!(
        audit.cell_inputs_sha256,
        fixture_cell_inputs_digest(&fixture.cell_directory, CELL_INPUTS_DIGEST_DOMAIN)
    );
    assert_ne!(
        audit.cell_inputs_sha256,
        fixture_cell_inputs_digest(
            &fixture.cell_directory,
            b"volicord-host-release-cell-inputs-v1\0"
        )
    );
    assert!(audit.findings.is_empty());

    let repeated_audit = run_audit(
        &fixture.context,
        &AuditRequest {
            candidate_descriptor: fixture.candidate_descriptor.clone(),
            cell_directory: fixture.cell_directory.clone(),
            manifest: fixture.external.path().join("manifest.json"),
            audit_output: fixture.external.path().join("audit-repeat.json"),
            started_at: EVALUATED_AT.to_owned(),
            evaluated_at: EVALUATED_AT.to_owned(),
        },
    )
    .expect("repeated independent audit");
    assert_eq!(audit.cell_inputs_sha256, repeated_audit.cell_inputs_sha256);
}

#[cfg(unix)]
#[test]
fn reviewed_codex_version_matrix_is_enforced_by_gate_and_audit() {
    let mut fixture = Fixture::new();
    fixture.set_codex_host_version(REVIEWED_CODEX_HOST_VERSION);

    let manifest = fixture.run_gate("reviewed-codex.json", EVALUATED_AT);
    assert_eq!(manifest.schema, MANIFEST_SCHEMA);
    assert_eq!(manifest.verdict, GateVerdict::Pass);
    assert_eq!(manifest.requested_verified_claims.len(), 9);
    let expected = [
        HostFeatureImplementation::Implemented,
        HostFeatureImplementation::UnsupportedByHost,
        HostFeatureImplementation::Implemented,
        HostFeatureImplementation::Implemented,
        HostFeatureImplementation::UnsupportedByHost,
        HostFeatureImplementation::UnsupportedByHost,
    ];
    for (feature, expected) in HostFeature::ALL.into_iter().zip(expected) {
        let cell = manifest
            .cells
            .iter()
            .find(|cell| cell.raw.host_kind == HostKind::Codex && cell.raw.feature == feature)
            .expect("reviewed Codex cell");
        assert_eq!(
            cell.raw.host_version.as_ref().map(String::as_str),
            Some(REVIEWED_CODEX_HOST_VERSION)
        );
        assert_eq!(
            cell.raw.implementation_disposition,
            match expected {
                HostFeatureImplementation::Implemented => ImplementationDisposition::Implemented,
                HostFeatureImplementation::UnsupportedByHost => {
                    ImplementationDisposition::UnsupportedByHost
                }
            },
            "{}",
            feature.as_str()
        );
    }

    let audit = run_audit(
        &fixture.context,
        &AuditRequest {
            candidate_descriptor: fixture.candidate_descriptor.clone(),
            cell_directory: fixture.cell_directory.clone(),
            manifest: fixture.external.path().join("reviewed-codex.json"),
            audit_output: fixture.external.path().join("reviewed-codex-audit.json"),
            started_at: EVALUATED_AT.to_owned(),
            evaluated_at: EVALUATED_AT.to_owned(),
        },
    )
    .expect("reviewed Codex audit");
    assert_eq!(audit.audit_verdict, AuditVerdict::Pass);
    assert_eq!(audit.recalculated_verdict, GateVerdict::Pass);
}

#[cfg(unix)]
#[test]
fn historical_v1_cell_and_manifest_inputs_are_rejected() {
    let mut cell_fixture = Fixture::new();
    cell_fixture.cells[0].schema = "volicord-host-release-cell-v1".to_owned();
    cell_fixture.write_cell(0);
    let cell_manifest = cell_fixture.external.path().join("v1-cell-manifest.json");
    let cell_error = run_gate(
        &cell_fixture.context,
        &GateRequest {
            candidate_descriptor: cell_fixture.candidate_descriptor.clone(),
            cell_directory: cell_fixture.cell_directory.clone(),
            manifest_output: cell_manifest.clone(),
            evaluated_at: EVALUATED_AT.to_owned(),
        },
    )
    .expect_err("v1 cell input must be rejected");
    assert!(cell_error
        .detail()
        .contains("cell schema identifier mismatch"));
    assert!(!cell_manifest.exists());

    let manifest_fixture = Fixture::new();
    manifest_fixture.run_gate("v2-manifest.json", EVALUATED_AT);
    let v2_manifest_path = manifest_fixture.external.path().join("v2-manifest.json");
    let mut historical: Value =
        serde_json::from_slice(&fs::read(&v2_manifest_path).expect("v2 manifest bytes"))
            .expect("v2 manifest JSON");
    historical["schema"] = Value::String("volicord-host-release-manifest-v1".to_owned());
    let historical_path = manifest_fixture.external.path().join("v1-manifest.json");
    fs::write(
        &historical_path,
        serde_json::to_vec_pretty(&historical).expect("historical manifest bytes"),
    )
    .expect("historical manifest fixture");
    let audit_output = manifest_fixture
        .external
        .path()
        .join("v1-manifest-audit.json");
    let manifest_error = run_audit(
        &manifest_fixture.context,
        &AuditRequest {
            candidate_descriptor: manifest_fixture.candidate_descriptor.clone(),
            cell_directory: manifest_fixture.cell_directory.clone(),
            manifest: historical_path,
            audit_output: audit_output.clone(),
            started_at: EVALUATED_AT.to_owned(),
            evaluated_at: EVALUATED_AT.to_owned(),
        },
    )
    .expect_err("v1 manifest input must be rejected");
    assert!(manifest_error
        .detail()
        .contains("manifest schema identifier mismatch"));
    assert!(!audit_output.exists());
}

#[cfg(unix)]
#[test]
fn claimed_status_is_untrusted_but_mismatch_is_retained() {
    let mut fixture = Fixture::new();
    let index = fixture.implemented_cell_index();
    fixture.cells[index].claimed_status = HostFeatureSupportStatus::UnsupportedByHost;
    fixture.write_cell(index);

    let manifest = fixture.run_gate("claimed-mismatch.json", EVALUATED_AT);
    let cell = manifest
        .cells
        .iter()
        .find(|cell| cell.raw.matrix_key() == fixture.cells[index].matrix_key())
        .expect("mutated cell");
    assert_eq!(cell.derived_status, HostFeatureSupportStatus::Verified);
    assert!(cell
        .finding_codes
        .contains(&"claimed_status_mismatch".to_owned()));
    assert_eq!(manifest.verdict, GateVerdict::Pass);
}

#[cfg(unix)]
#[test]
fn explicitly_excluded_verified_implementation_is_a_downgrade() {
    let mut fixture = Fixture::new();
    let index = fixture.implemented_cell_index();
    fixture.cells[index].requested_verified = false;
    let excluded_key = fixture.cells[index].key();
    fixture.write_cell(index);

    let manifest = fixture.run_gate("explicit-exclusion.json", EVALUATED_AT);
    let excluded_cell = manifest
        .cells
        .iter()
        .find(|cell| cell.raw.key() == excluded_key)
        .expect("explicitly excluded cell");
    assert_eq!(
        excluded_cell.derived_status,
        HostFeatureSupportStatus::Verified
    );
    assert_eq!(manifest.verdict, GateVerdict::PassWithDowngrades);
    assert!(manifest.downgrades.contains(&excluded_key));
    assert!(!manifest.requested_verified_claims.contains(&excluded_key));

    let audit = run_audit(
        &fixture.context,
        &AuditRequest {
            candidate_descriptor: fixture.candidate_descriptor.clone(),
            cell_directory: fixture.cell_directory.clone(),
            manifest: fixture.external.path().join("explicit-exclusion.json"),
            audit_output: fixture
                .external
                .path()
                .join("explicit-exclusion-audit.json"),
            started_at: EVALUATED_AT.to_owned(),
            evaluated_at: EVALUATED_AT.to_owned(),
        },
    )
    .expect("independent audit of explicit exclusion");
    assert_eq!(audit.recalculated_verdict, GateVerdict::PassWithDowngrades);
    assert_eq!(audit.audit_verdict, AuditVerdict::Pass);
    assert!(audit.findings.is_empty());
}

#[cfg(unix)]
#[test]
fn freshness_is_half_open_at_exactly_twenty_four_hours() {
    let fixture = Fixture::new();
    let manifest = evaluate_release_matrix(
        &fixture.context,
        fixture.candidate.clone(),
        fixture.cells.clone(),
        "2026-01-02T01:00:00Z",
    )
    .expect("stale matrix still derives a manifest")
    .manifest;
    assert_eq!(manifest.verdict, GateVerdict::Fail);
    assert!(manifest.cells.iter().any(|cell| {
        cell.raw.implementation_disposition == ImplementationDisposition::Implemented
            && cell.derived_status == HostFeatureSupportStatus::ImplementedUnverified
            && cell
                .finding_codes
                .contains(&"cell_timestamp_not_fresh".to_owned())
    }));
}

#[cfg(unix)]
#[test]
fn assertion_membership_rejects_omission_addition_and_duplicate() {
    let fixture = Fixture::new();
    let index = fixture.implemented_cell_index();
    let mut variants = Vec::new();

    let mut omitted = fixture.cells.clone();
    omitted[index].assertions.remove(0);
    variants.push(omitted);

    let mut additional = fixture.cells.clone();
    additional[index].assertions.push(CellAssertion {
        assertion_id: "unexpected_assertion".to_owned(),
        passed: true,
        finding_codes: None,
    });
    additional[index]
        .assertions
        .sort_by(|left, right| left.assertion_id.cmp(&right.assertion_id));
    variants.push(additional);

    let mut duplicated = fixture.cells.clone();
    let duplicate_assertion = duplicated[index].assertions[0].clone();
    duplicated[index].assertions.push(duplicate_assertion);
    duplicated[index]
        .assertions
        .sort_by(|left, right| left.assertion_id.cmp(&right.assertion_id));
    variants.push(duplicated);

    for cells in variants {
        let error = evaluate_release_matrix(
            &fixture.context,
            fixture.candidate.clone(),
            cells,
            EVALUATED_AT,
        )
        .expect_err("assertion mutation must fail");
        assert!(error.detail().contains("exact sorted required set"));
    }
}

#[cfg(unix)]
#[test]
fn matrix_rejects_cross_host_version_aggregation() {
    let fixture = Fixture::new();
    let mut cells = fixture.cells.clone();
    let index = cells
        .iter()
        .position(|cell| {
            cell.host_kind == HostKind::Codex && cell.feature == HostFeature::NativeUserAction
        })
        .expect("Codex cell");
    cells[index].host_version = RequiredNullable::some("codex-other".to_owned());
    cells[index].environment.host_version = RequiredNullable::some("codex-other".to_owned());
    let error = evaluate_release_matrix(
        &fixture.context,
        fixture.candidate.clone(),
        cells,
        EVALUATED_AT,
    )
    .expect_err("mixed host versions must fail");
    assert!(error
        .detail()
        .contains("one exact host availability coordinate"));
}

#[cfg(unix)]
#[test]
fn matrix_rejects_missing_duplicate_and_extra_cells() {
    let fixture = Fixture::new();
    let mut missing = fixture.cells.clone();
    missing.pop();

    let mut duplicate = fixture.cells.clone();
    duplicate[0] = duplicate[1].clone();

    let mut extra = fixture.cells.clone();
    extra.push(extra[0].clone());

    for cells in [missing, duplicate, extra] {
        let error = evaluate_release_matrix(
            &fixture.context,
            fixture.candidate.clone(),
            cells,
            EVALUATED_AT,
        )
        .expect_err("non-exact matrix must fail");
        assert!(
            error.detail().contains("fixed twelve-cell matrix")
                || error.detail().contains("duplicate, missing, or additional")
        );
    }
}

#[cfg(unix)]
#[test]
fn audit_rejects_manifest_and_embedded_cell_mutations() {
    let fixture = Fixture::new();
    fixture.run_gate("original.json", EVALUATED_AT);
    let original_path = fixture.external.path().join("original.json");
    let original: Value =
        serde_json::from_slice(&fs::read(&original_path).expect("manifest bytes"))
            .expect("manifest JSON");
    for (suffix, mut value) in [("verdict", original.clone()), ("cell", original.clone())] {
        if suffix == "verdict" {
            value["verdict"] = Value::String("fail".to_owned());
        } else {
            value["cells"][0]["claimed_status"] =
                Value::String("implemented_unverified".to_owned());
        }
        let mutated_path = fixture
            .external
            .path()
            .join(format!("mutated-{suffix}.json"));
        fs::write(
            &mutated_path,
            serde_json::to_vec_pretty(&value).expect("mutated manifest"),
        )
        .expect("write mutated manifest");

        let audit = run_audit(
            &fixture.context,
            &AuditRequest {
                candidate_descriptor: fixture.candidate_descriptor.clone(),
                cell_directory: fixture.cell_directory.clone(),
                manifest: mutated_path,
                audit_output: fixture
                    .external
                    .path()
                    .join(format!("mutated-{suffix}-audit.json")),
                started_at: EVALUATED_AT.to_owned(),
                evaluated_at: EVALUATED_AT.to_owned(),
            },
        )
        .expect("audit should create an explicit failed result");
        assert_eq!(audit.audit_verdict, AuditVerdict::Fail);
        assert!(audit
            .findings
            .contains(&"invariant_failed:manifest_recalculation_exact".to_owned()));
        assert_eq!(audit.recalculated_verdict, GateVerdict::Pass);
    }
}

#[cfg(unix)]
#[test]
fn audit_reopens_evidence_and_detects_post_gate_mutation() {
    let fixture = Fixture::new();
    fixture.run_gate("evidence-manifest.json", EVALUATED_AT);
    let evidence_path = fixture.cells[fixture.implemented_cell_index()]
        .evidence_artifact_path
        .as_ref()
        .expect("implemented evidence path");
    fs::write(evidence_path, b"mutated after gate").expect("mutate evidence");

    let audit = run_audit(
        &fixture.context,
        &AuditRequest {
            candidate_descriptor: fixture.candidate_descriptor.clone(),
            cell_directory: fixture.cell_directory.clone(),
            manifest: fixture.external.path().join("evidence-manifest.json"),
            audit_output: fixture.external.path().join("evidence-audit.json"),
            started_at: EVALUATED_AT.to_owned(),
            evaluated_at: EVALUATED_AT.to_owned(),
        },
    )
    .expect("audit result");
    assert_eq!(audit.audit_verdict, AuditVerdict::Fail);
    assert!(audit.recalculated_cells.iter().any(|cell| {
        cell.finding_codes
            .contains(&"evidence_artifact_digest_mismatch".to_owned())
    }));
}

#[cfg(unix)]
#[test]
fn candidate_build_identity_mutations_fail_named_invariants() {
    let cases = [
        (
            BuildOverride::Git("1111111111111111111111111111111111111111"),
            "candidate_build_git_exact",
        ),
        (
            BuildOverride::Target("aarch64-unknown-linux-gnu"),
            "candidate_build_target_exact",
        ),
        (
            BuildOverride::Profile("dev"),
            "candidate_build_profile_exact",
        ),
        (BuildOverride::Tree("dirty"), "candidate_build_tree_clean"),
        (
            BuildOverride::MetadataSource("repository"),
            "candidate_build_metadata_source_environment",
        ),
        (
            BuildOverride::ProfileExact("false"),
            "candidate_build_profile_exact",
        ),
    ];
    for (build_override, expected_invariant) in cases {
        let mut fixture = Fixture::new();
        fixture.apply_build_override(build_override);
        let manifest = fixture.run_gate(&format!("{expected_invariant}.json"), EVALUATED_AT);
        assert_eq!(manifest.verdict, GateVerdict::Fail);
        assert!(
            manifest
                .invariant_findings
                .contains(&expected_invariant.to_owned()),
            "missing {expected_invariant}: {:?}",
            manifest.invariant_findings
        );
    }
}

#[cfg(unix)]
#[test]
fn candidate_binary_and_source_archive_mutations_fail_invariants() {
    let binary_fixture = Fixture::new();
    let mut bytes = fs::read(&binary_fixture.candidate_path).expect("candidate bytes");
    bytes.extend_from_slice(b"# post-descriptor mutation\n");
    fs::write(&binary_fixture.candidate_path, bytes).expect("mutated candidate");
    let binary_output = binary_fixture.external.path().join("binary-mutation.json");
    let error = run_gate(
        &binary_fixture.context,
        &GateRequest {
            candidate_descriptor: binary_fixture.candidate_descriptor.clone(),
            cell_directory: binary_fixture.cell_directory.clone(),
            manifest_output: binary_output.clone(),
            evaluated_at: EVALUATED_AT.to_owned(),
        },
    )
    .expect_err("pre-execution candidate digest mismatch must abort the gate");
    assert!(error.detail().contains("before execution"));
    assert!(!binary_output.exists());

    let mut source_fixture = Fixture::new();
    source_fixture.candidate.source_archive_sha256 = "0".repeat(64);
    source_fixture.write_candidate();
    let source_manifest = source_fixture.run_gate("source-mutation.json", EVALUATED_AT);
    assert_eq!(source_manifest.verdict, GateVerdict::Fail);
    assert!(source_manifest
        .invariant_findings
        .contains(&"source_archive_digest_exact".to_owned()));
}

#[cfg(unix)]
#[test]
fn candidate_digest_mismatch_never_executes_and_writes_no_manifest() {
    let fixture = Fixture::new();
    let marker = fixture
        .external
        .path()
        .join("unexpected-candidate-execution");
    let script = BuildFields::new(&fixture.candidate.source_revision)
        .script()
        .replacen("printf", &format!(": > '{}'\nprintf", marker.display()), 1);
    write_executable(&fixture.candidate_path, script.as_bytes());

    let manifest_output = fixture.external.path().join("digest-mismatch.json");
    let error = run_gate(
        &fixture.context,
        &GateRequest {
            candidate_descriptor: fixture.candidate_descriptor.clone(),
            cell_directory: fixture.cell_directory.clone(),
            manifest_output: manifest_output.clone(),
            evaluated_at: EVALUATED_AT.to_owned(),
        },
    )
    .expect_err("descriptor digest mismatch must abort before candidate execution");
    assert!(error.detail().contains("before execution"));
    assert!(!marker.exists());
    assert!(!manifest_output.exists());
}

#[cfg(unix)]
#[test]
fn candidate_executes_private_copy_and_path_replacement_fails_final_stability() {
    let mut fixture = Fixture::new();
    let marker = fixture.external.path().join("private-copy-running");
    let original_path = fixture.candidate_path.to_string_lossy();
    let prelude = format!(
        "[ \"$0\" != '{}' ] || exit 70\n[ -z \"${{HOME+x}}\" ] || exit 71\n: > '{}'\n/bin/sleep 0.25\n",
        original_path,
        marker.display(),
    );
    let script = BuildFields::new(&fixture.candidate.source_revision)
        .script()
        .replacen("printf", &format!("{prelude}printf"), 1);
    fixture.install_candidate_script(script);

    let replacement_path = fixture.external.path().join("replacement-candidate");
    let replacement_script = format!(
        "{}# replacement with a distinct digest\n",
        BuildFields::new(&fixture.candidate.source_revision).script()
    );
    write_executable(&replacement_path, replacement_script.as_bytes());
    let candidate_path = fixture.candidate_path.clone();
    let marker_for_thread = marker.clone();
    let replacement = thread::spawn(move || {
        for _ in 0..200 {
            if marker_for_thread.exists() {
                fs::rename(&replacement_path, &candidate_path)
                    .expect("replace candidate pathname atomically");
                return true;
            }
            thread::sleep(StdDuration::from_millis(5));
        }
        false
    });

    let manifest = fixture.run_gate("path-replaced.json", EVALUATED_AT);
    assert!(replacement.join().expect("replacement thread"));
    assert_eq!(manifest.verdict, GateVerdict::Fail);
    assert!(manifest
        .invariant_findings
        .contains(&"candidate_binary_final_stable".to_owned()));
    assert!(!manifest
        .invariant_findings
        .contains(&"candidate_build_git_exact".to_owned()));
}

#[cfg(unix)]
#[test]
fn structural_cell_and_evidence_failures_write_no_manifest() {
    let fixture = Fixture::new();
    let index = fixture.implemented_cell_index();

    fs::remove_file(&fixture.cell_paths[index]).expect("remove required cell");
    let missing_cell_output = fixture.external.path().join("missing-cell.json");
    let error = run_gate(
        &fixture.context,
        &GateRequest {
            candidate_descriptor: fixture.candidate_descriptor.clone(),
            cell_directory: fixture.cell_directory.clone(),
            manifest_output: missing_cell_output.clone(),
            evaluated_at: EVALUATED_AT.to_owned(),
        },
    )
    .expect_err("missing cell is a structural command failure");
    assert!(error.detail().contains("exactly twelve"));
    assert!(!missing_cell_output.exists());

    fixture.write_cell(index);
    fs::write(&fixture.cell_paths[index], b"{ malformed").expect("malformed cell");
    let malformed_cell_output = fixture.external.path().join("malformed-cell.json");
    run_gate(
        &fixture.context,
        &GateRequest {
            candidate_descriptor: fixture.candidate_descriptor.clone(),
            cell_directory: fixture.cell_directory.clone(),
            manifest_output: malformed_cell_output.clone(),
            evaluated_at: EVALUATED_AT.to_owned(),
        },
    )
    .expect_err("malformed cell is a structural command failure");
    assert!(!malformed_cell_output.exists());

    fixture.write_cell(index);
    let evidence_path = PathBuf::from(
        fixture.cells[index]
            .evidence_artifact_path
            .as_ref()
            .expect("implemented evidence"),
    );
    fs::remove_file(evidence_path).expect("remove required evidence");
    let missing_evidence_output = fixture.external.path().join("missing-evidence.json");
    run_gate(
        &fixture.context,
        &GateRequest {
            candidate_descriptor: fixture.candidate_descriptor.clone(),
            cell_directory: fixture.cell_directory.clone(),
            manifest_output: missing_evidence_output.clone(),
            evaluated_at: EVALUATED_AT.to_owned(),
        },
    )
    .expect_err("missing evidence is a structural command failure");
    assert!(!missing_evidence_output.exists());
}

#[cfg(unix)]
#[test]
fn present_null_host_cells_record_honest_not_run_downgrades() {
    let mut fixture = Fixture::new();
    let unavailable_host = HostKind::ClaudeCode;
    let mut expected_downgrades = 0usize;
    for index in 0..fixture.cells.len() {
        if fixture.cells[index].host_kind != unavailable_host {
            continue;
        }
        let cell = &mut fixture.cells[index];
        cell.host_version = RequiredNullable::null();
        cell.environment.host_version = RequiredNullable::null();
        cell.environment.host_executable_sha256 = RequiredNullable::null();
        cell.requested_verified = false;
        match cell.implementation_disposition {
            ImplementationDisposition::Implemented => {
                expected_downgrades += 1;
                cell.run_state = RunState::Ignored;
                cell.claimed_status = HostFeatureSupportStatus::ImplementedUnverified;
                for assertion in &mut cell.assertions {
                    assertion.passed = false;
                    assertion.finding_codes = Some(vec!["host_unavailable".to_owned()]);
                }
            }
            ImplementationDisposition::UnsupportedByHost => {
                cell.run_state = RunState::NotApplicable;
                cell.claimed_status = HostFeatureSupportStatus::UnsupportedByHost;
            }
        }
        fixture.write_cell(index);
    }

    let manifest = fixture.run_gate("host-unavailable.json", EVALUATED_AT);
    assert_eq!(manifest.verdict, GateVerdict::PassWithDowngrades);
    assert_eq!(
        manifest
            .cells
            .iter()
            .filter(|cell| {
                cell.raw.host_kind == unavailable_host
                    && cell.raw.implementation_disposition == ImplementationDisposition::Implemented
                    && cell.derived_status == HostFeatureSupportStatus::ImplementedUnverified
                    && cell.raw.run_state == RunState::Ignored
                    && cell.raw.evidence_artifact_path.as_ref().is_some()
                    && cell.raw.key().contains("/unavailable/")
            })
            .count(),
        expected_downgrades
    );
    assert_eq!(
        manifest
            .downgrades
            .iter()
            .filter(|key| key.starts_with("claude_code/unavailable/"))
            .count(),
        expected_downgrades
    );

    let requested_index = fixture
        .cells
        .iter()
        .position(|cell| {
            cell.host_kind == unavailable_host
                && cell.implementation_disposition == ImplementationDisposition::Implemented
        })
        .expect("unavailable implemented cell");
    fixture.cells[requested_index].requested_verified = true;
    let requested_key = fixture.cells[requested_index].key();
    fixture.write_cell(requested_index);
    let requested_manifest = fixture.run_gate("host-unavailable-requested.json", EVALUATED_AT);
    assert_eq!(requested_manifest.verdict, GateVerdict::Fail);
    assert!(requested_manifest
        .requested_verified_claims
        .contains(&requested_key));
    assert!(requested_manifest.downgrades.contains(&requested_key));
}

#[cfg(unix)]
#[test]
fn unavailable_implemented_cell_rejects_passing_assertions() {
    let mut fixture = Fixture::new();
    let unavailable_host = HostKind::ClaudeCode;
    for cell in fixture
        .cells
        .iter_mut()
        .filter(|cell| cell.host_kind == unavailable_host)
    {
        cell.host_version = RequiredNullable::null();
        cell.environment.host_version = RequiredNullable::null();
        cell.environment.host_executable_sha256 = RequiredNullable::null();
        cell.requested_verified = false;
        match cell.implementation_disposition {
            ImplementationDisposition::Implemented => {
                cell.run_state = RunState::Ignored;
                cell.claimed_status = HostFeatureSupportStatus::ImplementedUnverified;
                for assertion in &mut cell.assertions {
                    assertion.passed = false;
                    assertion.finding_codes = Some(vec!["host_unavailable".to_owned()]);
                }
            }
            ImplementationDisposition::UnsupportedByHost => {
                cell.run_state = RunState::NotApplicable;
                cell.claimed_status = HostFeatureSupportStatus::UnsupportedByHost;
            }
        }
    }
    let index = fixture
        .cells
        .iter()
        .position(|cell| {
            cell.host_kind == unavailable_host
                && cell.implementation_disposition == ImplementationDisposition::Implemented
        })
        .expect("unavailable implemented cell");
    fixture.cells[index].assertions[0].passed = true;
    fixture.cells[index].assertions[0].finding_codes = None;

    let error = evaluate_release_matrix(
        &fixture.context,
        fixture.candidate.clone(),
        fixture.cells.clone(),
        EVALUATED_AT,
    )
    .expect_err("an unavailable host cannot report a passing live assertion");
    assert!(error
        .detail()
        .contains("unavailable implemented cell requires every assertion to fail"));
}

#[cfg(unix)]
#[test]
fn static_unsupported_cells_cannot_request_verified() {
    let fixture = Fixture::new();
    let mut cells = fixture.cells.clone();
    let index = cells
        .iter()
        .position(|cell| {
            cell.implementation_disposition == ImplementationDisposition::UnsupportedByHost
        })
        .expect("static unsupported cell");
    cells[index].requested_verified = true;
    let error = evaluate_release_matrix(
        &fixture.context,
        fixture.candidate.clone(),
        cells,
        EVALUATED_AT,
    )
    .expect_err("static unsupported cannot request verified");
    assert!(error.detail().contains("cannot request verified"));
}

#[cfg(unix)]
#[test]
fn static_unsupported_coordinate_mismatches_fail_matrix_invariants() {
    let cases = [
        ("candidate", "all_cell_candidate_coordinates_exact"),
        ("environment", "all_cell_environment_coordinates_exact"),
    ];

    for (case, expected_invariant) in cases {
        let mut fixture = Fixture::new();
        let index = fixture
            .cells
            .iter()
            .position(|cell| {
                cell.implementation_disposition == ImplementationDisposition::UnsupportedByHost
            })
            .expect("static unsupported cell");
        match case {
            "candidate" => fixture.cells[index].candidate_id.push_str("-other"),
            "environment" => {
                fixture.cells[index].adapter_version.push_str("-other");
                fixture.cells[index]
                    .environment
                    .adapter_version
                    .push_str("-other");
            }
            _ => unreachable!("the cases table is exhaustive"),
        }
        fixture.write_cell(index);

        let manifest_name = format!("static-unsupported-{case}-mismatch.json");
        let manifest = fixture.run_gate(&manifest_name, EVALUATED_AT);
        assert_eq!(manifest.verdict, GateVerdict::Fail);
        assert!(
            manifest
                .invariant_findings
                .contains(&expected_invariant.to_owned()),
            "missing {expected_invariant}: {:?}",
            manifest.invariant_findings
        );

        let audit = run_audit(
            &fixture.context,
            &AuditRequest {
                candidate_descriptor: fixture.candidate_descriptor.clone(),
                cell_directory: fixture.cell_directory.clone(),
                manifest: fixture.external.path().join(manifest_name),
                audit_output: fixture
                    .external
                    .path()
                    .join(format!("static-unsupported-{case}-mismatch-audit.json")),
                started_at: EVALUATED_AT.to_owned(),
                evaluated_at: EVALUATED_AT.to_owned(),
            },
        )
        .expect("independent audit of the coordinate mismatch");
        assert_eq!(audit.audit_verdict, AuditVerdict::Fail);
        assert!(audit.invariant_results.iter().any(|invariant| {
            invariant.invariant_id == expected_invariant && !invariant.passed
        }));
    }
}

#[cfg(unix)]
#[test]
fn coherent_adapter_coordinate_mutations_fail_gate_and_independent_audit() {
    for coordinate in ["profile", "version"] {
        let mut fixture = Fixture::new();
        let index = fixture.implemented_cell_index();
        match coordinate {
            "profile" => {
                fixture.cells[index].adapter_profile = "wrong-profile".to_owned();
                fixture.cells[index].environment.adapter_profile = "wrong-profile".to_owned();
            }
            "version" => {
                fixture.cells[index].adapter_version = "wrong-build-id".to_owned();
                fixture.cells[index].environment.adapter_version = "wrong-build-id".to_owned();
            }
            _ => unreachable!("the coordinate cases are exhaustive"),
        }
        let cell_key = fixture.cells[index].key();
        let cell_host_kind = fixture.cells[index].host_kind;
        let cell_feature = fixture.cells[index].feature;
        fixture.write_cell(index);

        let manifest_name = format!("coherent-adapter-{coordinate}-mismatch.json");
        let manifest = fixture.run_gate(&manifest_name, EVALUATED_AT);
        assert_eq!(manifest.verdict, GateVerdict::Fail);
        assert!(manifest
            .invariant_findings
            .contains(&"all_cell_environment_coordinates_exact".to_owned()));
        let recalculated_cell = manifest
            .cells
            .iter()
            .find(|cell| cell.raw.key() == cell_key)
            .expect("mutated implemented cell");
        assert_eq!(
            recalculated_cell.derived_status,
            HostFeatureSupportStatus::ImplementedUnverified
        );
        assert!(recalculated_cell
            .finding_codes
            .contains(&"environment_coordinate_mismatch".to_owned()));

        let audit = run_audit(
            &fixture.context,
            &AuditRequest {
                candidate_descriptor: fixture.candidate_descriptor.clone(),
                cell_directory: fixture.cell_directory.clone(),
                manifest: fixture.external.path().join(manifest_name),
                audit_output: fixture
                    .external
                    .path()
                    .join(format!("coherent-adapter-{coordinate}-mismatch-audit.json")),
                started_at: EVALUATED_AT.to_owned(),
                evaluated_at: EVALUATED_AT.to_owned(),
            },
        )
        .expect("independent audit of coherent adapter-coordinate mutation");
        assert_eq!(audit.recalculated_verdict, GateVerdict::Fail);
        assert_eq!(audit.audit_verdict, AuditVerdict::Fail);
        assert!(audit.invariant_results.iter().any(|invariant| {
            invariant.invariant_id == "all_cell_environment_coordinates_exact" && !invariant.passed
        }));
        assert!(audit.recalculated_cells.iter().any(|cell| {
            cell.host_kind == cell_host_kind
                && cell.feature == cell_feature
                && cell
                    .finding_codes
                    .contains(&"environment_coordinate_mismatch".to_owned())
        }));
    }
}

#[cfg(unix)]
#[test]
fn audit_rejects_coherent_manifest_rewrite_not_backed_by_original_cells() {
    let fixture = Fixture::new();
    fixture.run_gate("original-for-rewrite.json", EVALUATED_AT);
    let mut rewritten_cells = fixture.cells.clone();
    let index = fixture.implemented_cell_index();
    rewritten_cells[index].claimed_status = HostFeatureSupportStatus::UnsupportedByHost;
    let coherent_rewrite = evaluate_release_matrix(
        &fixture.context,
        fixture.candidate.clone(),
        rewritten_cells,
        EVALUATED_AT,
    )
    .expect("coherent rewritten manifest")
    .manifest;
    let rewritten_path = fixture.external.path().join("coherent-rewrite.json");
    fs::write(
        &rewritten_path,
        serde_json::to_vec_pretty(&coherent_rewrite).expect("rewritten manifest JSON"),
    )
    .expect("write rewritten manifest");

    let audit = run_audit(
        &fixture.context,
        &AuditRequest {
            candidate_descriptor: fixture.candidate_descriptor.clone(),
            cell_directory: fixture.cell_directory.clone(),
            manifest: rewritten_path,
            audit_output: fixture.external.path().join("coherent-rewrite-audit.json"),
            started_at: EVALUATED_AT.to_owned(),
            evaluated_at: EVALUATED_AT.to_owned(),
        },
    )
    .expect("audit emits a failed artifact");
    assert_eq!(audit.audit_verdict, AuditVerdict::Fail);
    assert!(audit
        .findings
        .contains(&"invariant_failed:cell_inputs_match_manifest".to_owned()));
}

#[cfg(unix)]
#[test]
fn configured_exclusion_roots_are_normalized_before_overlap_checks() {
    use std::os::unix::fs::symlink;

    let source = tempfile::tempdir().expect("source root");
    let target = tempfile::tempdir().expect("target root");
    let docs = tempfile::tempdir().expect("docs root");
    let runtime = tempfile::tempdir().expect("runtime root");
    let target_file = target.path().join("candidate");
    let docs_file = docs.path().join("evidence");
    let runtime_file = runtime.path().join("cell.json");
    fs::write(&target_file, b"target").expect("target file");
    fs::write(&docs_file, b"docs").expect("docs file");
    fs::write(&runtime_file, b"runtime").expect("runtime file");

    let sibling =
        |path: &Path| PathBuf::from("..").join(path.file_name().expect("temporary directory name"));
    let runtime_link = source.path().join("runtime-link");
    symlink(runtime.path(), &runtime_link).expect("runtime symlink");
    let context = ValidationContext::new(
        source.path().to_path_buf(),
        sibling(target.path()),
        source.path().join(sibling(docs.path())),
        vec![sibling(runtime.path()), runtime_link],
    )
    .expect("normalized exclusion context");

    for (path, expected) in [
        (&target_file, "Cargo target directory"),
        (&docs_file, "maintained documentation"),
        (&runtime_file, "Volicord Runtime Home"),
    ] {
        let path = fs::canonicalize(path).expect("canonical excluded file");
        let error = context
            .validate_existing_file(&path)
            .expect_err("normalized exclusion root must not be escapable");
        assert!(error.detail().contains(expected), "{}", error.detail());
    }

    let symlink_parent = tempfile::tempdir().expect("symlink parent root");
    let anchor = symlink_parent.path().join("anchor");
    let escaped_target = symlink_parent.path().join("escaped-target");
    fs::create_dir(&anchor).expect("symlink anchor");
    fs::create_dir(&escaped_target).expect("escaped target root");
    let escaped_file = escaped_target.join("candidate");
    fs::write(&escaped_file, b"escaped").expect("escaped target file");
    let dot_link = source.path().join("dot-link");
    symlink(&anchor, &dot_link).expect("dot-component symlink");
    let context = ValidationContext::new(
        source.path().to_path_buf(),
        dot_link.join("..").join("escaped-target"),
        source.path().join("docs"),
        Vec::new(),
    )
    .expect("symlink-aware dot normalization");
    let escaped_file = fs::canonicalize(escaped_file).expect("canonical escaped file");
    let error = context
        .validate_existing_file(&escaped_file)
        .expect_err("symlink-aware dot root must remain excluded");
    assert!(error.detail().contains("Cargo target directory"));
}

#[cfg(unix)]
#[test]
fn create_new_output_refuses_overwrite() {
    let fixture = Fixture::new();
    fixture.run_gate("create-new.json", EVALUATED_AT);
    let error = run_gate(
        &fixture.context,
        &GateRequest {
            candidate_descriptor: fixture.candidate_descriptor.clone(),
            cell_directory: fixture.cell_directory.clone(),
            manifest_output: fixture.external.path().join("create-new.json"),
            evaluated_at: EVALUATED_AT.to_owned(),
        },
    )
    .expect_err("existing output must be rejected");
    assert!(error.detail().contains("output already exists"));
}

#[cfg(unix)]
#[test]
fn audit_rejects_a_manifest_path_that_cannot_be_recorded_exactly() {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};

    let fixture = Fixture::new();
    fixture.run_gate("utf8-manifest.json", EVALUATED_AT);
    let valid_manifest = fixture.external.path().join("utf8-manifest.json");
    let invalid_manifest = fixture
        .external
        .path()
        .join(OsString::from_vec(b"manifest-\xff.json".to_vec()));
    fs::rename(valid_manifest, &invalid_manifest).expect("rename manifest to a non-UTF-8 path");
    let audit_output = fixture.external.path().join("non-utf8-path-audit.json");

    let error = run_audit(
        &fixture.context,
        &AuditRequest {
            candidate_descriptor: fixture.candidate_descriptor.clone(),
            cell_directory: fixture.cell_directory.clone(),
            manifest: invalid_manifest,
            audit_output: audit_output.clone(),
            started_at: EVALUATED_AT.to_owned(),
            evaluated_at: EVALUATED_AT.to_owned(),
        },
    )
    .expect_err("a lossy manifest path must not enter the audit artifact");
    assert!(error.detail().contains("manifest path is not valid UTF-8"));
    assert!(!audit_output.exists());
}

#[cfg(unix)]
struct Fixture {
    _source: TempDir,
    external: TempDir,
    context: ValidationContext,
    candidate: Candidate,
    candidate_descriptor: PathBuf,
    candidate_path: PathBuf,
    cell_directory: PathBuf,
    cell_paths: Vec<PathBuf>,
    cells: Vec<Cell>,
}

#[cfg(unix)]
impl Fixture {
    fn new() -> Self {
        use std::os::unix::fs::PermissionsExt;

        let source = tempfile::tempdir().expect("source tempdir");
        run(source.path(), "git", &["init", "-q"]);
        run(
            source.path(),
            "git",
            &["config", "user.email", "test@example.invalid"],
        );
        run(
            source.path(),
            "git",
            &["config", "user.name", "Release Test"],
        );
        fs::write(source.path().join("source.txt"), b"release source\n").expect("source file");
        run(source.path(), "git", &["add", "source.txt"]);
        run(source.path(), "git", &["commit", "-q", "-m", "fixture"]);
        let source_revision = output(source.path(), "git", &["rev-parse", "HEAD"]);
        let source_checkout = fs::canonicalize(source.path()).expect("canonical source");
        let source_archive_sha256 =
            git_archive_sha256(&source_checkout, &source_revision).expect("source archive digest");

        let external = tempfile::tempdir().expect("external tempdir");
        let candidate_path = external.path().join("volicord-candidate");
        let build = BuildFields::new(&source_revision);
        let candidate_build_id = build.build_id();
        fs::write(&candidate_path, build.script()).expect("candidate script");
        let mut permissions = fs::metadata(&candidate_path)
            .expect("candidate metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&candidate_path, permissions).expect("candidate executable");
        let binary_sha256 = sha256_bytes(&fs::read(&candidate_path).expect("candidate bytes"));
        let candidate = Candidate {
            schema: CANDIDATE_SCHEMA.to_owned(),
            candidate_id: "candidate-test".to_owned(),
            candidate_path: candidate_path.to_string_lossy().into_owned(),
            source_revision: source_revision.clone(),
            source_clean: true,
            source_archive_algorithm: SOURCE_ARCHIVE_ALGORITHM.to_owned(),
            source_archive_sha256,
            target_triple: TARGET.to_owned(),
            release_profile: "release".to_owned(),
            binary_sha256,
            build_environment: CandidateBuildEnvironment {
                runner_os: "linux".to_owned(),
                runner_os_version: "test".to_owned(),
                runner_arch: "x86_64".to_owned(),
                git_version: "git test".to_owned(),
                rustc_version: "rustc test".to_owned(),
                cargo_version: "cargo test".to_owned(),
            },
            recorded_at: "2026-01-01T00:00:00Z".to_owned(),
        };
        let candidate_descriptor = external.path().join("candidate.json");
        fs::write(
            &candidate_descriptor,
            serde_json::to_vec_pretty(&candidate).expect("candidate JSON"),
        )
        .expect("candidate descriptor");
        let cell_directory = external.path().join("cells");
        let evidence_directory = external.path().join("evidence");
        fs::create_dir(&cell_directory).expect("cell directory");
        fs::create_dir(&evidence_directory).expect("evidence directory");
        let mut cells = Vec::new();
        let mut cell_paths = Vec::new();
        for host_kind in HostKind::ALL {
            for feature in HostFeature::ALL {
                let host_version = match host_kind {
                    HostKind::Codex => "codex-test-1",
                    HostKind::ClaudeCode => "claude-test-1",
                };
                let disposition = canonical_disposition(host_kind, Some(host_version), feature);
                let adapter_profile = expected_adapter_profile(feature).as_str();
                let evidence = if disposition == ImplementationDisposition::Implemented {
                    let path = evidence_directory.join(format!(
                        "{}--{}.bin",
                        host_kind.as_str(),
                        feature.as_str()
                    ));
                    let bytes = format!("{}/{} evidence", host_kind.as_str(), feature.as_str());
                    fs::write(&path, bytes.as_bytes()).expect("evidence file");
                    (
                        RequiredNullable::some(path.to_string_lossy().into_owned()),
                        RequiredNullable::some(sha256_bytes(bytes.as_bytes())),
                    )
                } else {
                    (RequiredNullable::null(), RequiredNullable::null())
                };
                let assertions = expected_assertion_ids(disposition, feature)
                    .into_iter()
                    .map(|assertion_id| CellAssertion {
                        assertion_id: assertion_id.to_owned(),
                        passed: true,
                        finding_codes: None,
                    })
                    .collect();
                let cell = Cell {
                    schema: CELL_SCHEMA.to_owned(),
                    candidate_id: candidate.candidate_id.clone(),
                    binary_sha256: candidate.binary_sha256.clone(),
                    source_revision: source_revision.clone(),
                    target_triple: TARGET.to_owned(),
                    release_profile: "release".to_owned(),
                    host_kind,
                    host_version: RequiredNullable::some(host_version.to_owned()),
                    adapter_profile: adapter_profile.to_owned(),
                    adapter_version: candidate_build_id.clone(),
                    feature,
                    implementation_disposition: disposition,
                    requested_verified: disposition == ImplementationDisposition::Implemented,
                    claimed_status: if disposition == ImplementationDisposition::Implemented {
                        HostFeatureSupportStatus::Verified
                    } else {
                        HostFeatureSupportStatus::UnsupportedByHost
                    },
                    run_state: if disposition == ImplementationDisposition::Implemented {
                        RunState::Completed
                    } else {
                        RunState::NotApplicable
                    },
                    started_at: "2026-01-01T01:00:00Z".to_owned(),
                    recorded_at: "2026-01-01T01:01:00Z".to_owned(),
                    environment: CellEnvironment {
                        runner_os: "linux".to_owned(),
                        runner_os_version: "test".to_owned(),
                        runner_arch: "x86_64".to_owned(),
                        host_executable_sha256: RequiredNullable::some(sha256_bytes(
                            host_version.as_bytes(),
                        )),
                        host_kind,
                        host_version: RequiredNullable::some(host_version.to_owned()),
                        adapter_profile: adapter_profile.to_owned(),
                        adapter_version: candidate_build_id.clone(),
                    },
                    assertions,
                    evidence_artifact_path: evidence.0,
                    evidence_artifact_sha256: evidence.1,
                };
                let cell_path = cell_directory.join(format!(
                    "{:02}--{}--{}.json",
                    cells.len(),
                    host_kind.as_str(),
                    feature.as_str()
                ));
                fs::write(
                    &cell_path,
                    serde_json::to_vec_pretty(&cell).expect("cell JSON"),
                )
                .expect("cell file");
                cells.push(cell);
                cell_paths.push(cell_path);
            }
        }
        let context = ValidationContext::new(
            source_checkout.clone(),
            source_checkout.join("target"),
            source_checkout.join("docs"),
            vec![source_checkout.join(".volicord")],
        )
        .expect("validation context");
        Self {
            _source: source,
            external,
            context,
            candidate,
            candidate_descriptor,
            candidate_path,
            cell_directory,
            cell_paths,
            cells,
        }
    }

    fn run_gate(&self, output_name: &str, evaluated_at: &str) -> crate::schema::ReleaseManifest {
        run_gate(
            &self.context,
            &GateRequest {
                candidate_descriptor: self.candidate_descriptor.clone(),
                cell_directory: self.cell_directory.clone(),
                manifest_output: self.external.path().join(output_name),
                evaluated_at: evaluated_at.to_owned(),
            },
        )
        .expect("release gate")
    }

    fn implemented_cell_index(&self) -> usize {
        self.cells
            .iter()
            .position(|cell| {
                cell.implementation_disposition == ImplementationDisposition::Implemented
            })
            .expect("implemented cell")
    }

    fn write_cell(&self, index: usize) {
        fs::write(
            &self.cell_paths[index],
            serde_json::to_vec_pretty(&self.cells[index]).expect("cell JSON"),
        )
        .expect("rewrite test cell");
    }

    fn write_candidate(&self) {
        fs::write(
            &self.candidate_descriptor,
            serde_json::to_vec_pretty(&self.candidate).expect("candidate JSON"),
        )
        .expect("rewrite candidate descriptor");
    }

    fn set_codex_host_version(&mut self, host_version: &str) {
        for index in 0..self.cells.len() {
            if self.cells[index].host_kind != HostKind::Codex {
                continue;
            }
            let feature = self.cells[index].feature;
            let disposition = canonical_disposition(HostKind::Codex, Some(host_version), feature);
            self.cells[index].host_version = RequiredNullable::some(host_version.to_owned());
            self.cells[index].environment.host_version =
                RequiredNullable::some(host_version.to_owned());
            self.cells[index].environment.host_executable_sha256 =
                RequiredNullable::some(sha256_bytes(host_version.as_bytes()));
            self.cells[index].implementation_disposition = disposition;
            self.cells[index].requested_verified =
                disposition == ImplementationDisposition::Implemented;
            self.cells[index].claimed_status =
                if disposition == ImplementationDisposition::Implemented {
                    HostFeatureSupportStatus::Verified
                } else {
                    HostFeatureSupportStatus::UnsupportedByHost
                };
            self.cells[index].run_state = if disposition == ImplementationDisposition::Implemented {
                RunState::Completed
            } else {
                RunState::NotApplicable
            };
            self.cells[index].assertions = expected_assertion_ids(disposition, feature)
                .into_iter()
                .map(|assertion_id| CellAssertion {
                    assertion_id: assertion_id.to_owned(),
                    passed: true,
                    finding_codes: None,
                })
                .collect();
            if disposition == ImplementationDisposition::UnsupportedByHost {
                self.cells[index].evidence_artifact_path = RequiredNullable::null();
                self.cells[index].evidence_artifact_sha256 = RequiredNullable::null();
            }
            self.write_cell(index);
        }
    }

    fn apply_build_override(&mut self, build_override: BuildOverride) {
        let (script, build_id) = {
            let mut build = BuildFields::new(&self.candidate.source_revision);
            build.apply(build_override);
            (build.script(), build.build_id())
        };
        self.install_candidate_script(script);
        for index in 0..self.cells.len() {
            self.cells[index].adapter_version.clone_from(&build_id);
            self.cells[index]
                .environment
                .adapter_version
                .clone_from(&build_id);
            self.write_cell(index);
        }
    }

    fn install_candidate_script(&mut self, script: String) {
        write_executable(&self.candidate_path, script.as_bytes());
        self.candidate.binary_sha256 =
            sha256_bytes(&fs::read(&self.candidate_path).expect("candidate bytes"));
        for index in 0..self.cells.len() {
            self.cells[index].binary_sha256 = self.candidate.binary_sha256.clone();
            self.write_cell(index);
        }
        self.write_candidate();
    }
}

#[cfg(unix)]
#[derive(Clone, Copy)]
enum BuildOverride {
    Git(&'static str),
    Target(&'static str),
    Profile(&'static str),
    Tree(&'static str),
    MetadataSource(&'static str),
    ProfileExact(&'static str),
}

#[cfg(unix)]
struct BuildFields<'a> {
    git: &'a str,
    target: &'a str,
    profile: &'a str,
    tree: &'a str,
    metadata_source: &'a str,
    profile_exact: &'a str,
}

#[cfg(unix)]
impl<'a> BuildFields<'a> {
    fn new(git: &'a str) -> Self {
        Self {
            git,
            target: TARGET,
            profile: "release",
            tree: "clean",
            metadata_source: "environment",
            profile_exact: "true",
        }
    }

    fn apply(&mut self, build_override: BuildOverride) {
        match build_override {
            BuildOverride::Git(value) => self.git = value,
            BuildOverride::Target(value) => self.target = value,
            BuildOverride::Profile(value) => self.profile = value,
            BuildOverride::Tree(value) => self.tree = value,
            BuildOverride::MetadataSource(value) => self.metadata_source = value,
            BuildOverride::ProfileExact(value) => self.profile_exact = value,
        }
    }

    fn script(&self) -> String {
        format!(
            "#!/bin/sh\n[ \"$#\" -eq 1 ] && [ \"$1\" = \"--version\" ] || exit 64\nprintf '%s\\n' 'volicord {} (build_id={})'\n",
            env!("CARGO_PKG_VERSION"),
            self.build_id(),
        )
    }

    fn build_id(&self) -> String {
        format!(
            "{};git={};tree={};metadata_source={};target={};profile={};profile_class=release;profile_exact={};opt=3;debug=false",
            env!("CARGO_PKG_VERSION"),
            self.git,
            self.tree,
            self.metadata_source,
            self.target,
            self.profile,
            self.profile_exact,
        )
    }
}

#[cfg(unix)]
const fn expected_adapter_profile(feature: HostFeature) -> IntegrationProfile {
    match feature {
        HostFeature::RecordFinalOutput => IntegrationProfile::Record,
        HostFeature::NativeUserAction
        | HostFeature::LocalWebUserChannel
        | HostFeature::VerifiedToolProducer
        | HostFeature::RegisteredConnectionObservation
        | HostFeature::DetectiveFinalOutput => IntegrationProfile::Detective,
    }
}

#[cfg(unix)]
fn canonical_disposition(
    host_kind: HostKind,
    host_version: Option<&str>,
    feature: HostFeature,
) -> ImplementationDisposition {
    match host_feature_implementation_for_version(host_kind.as_str(), host_version, feature) {
        HostFeatureImplementation::Implemented => ImplementationDisposition::Implemented,
        HostFeatureImplementation::UnsupportedByHost => {
            ImplementationDisposition::UnsupportedByHost
        }
    }
}

#[cfg(unix)]
fn fixture_cell_inputs_digest(cell_directory: &Path, domain: &[u8]) -> String {
    let mut paths = fs::read_dir(cell_directory)
        .expect("cell directory")
        .map(|entry| entry.expect("cell entry").path())
        .collect::<Vec<_>>();
    paths.sort_by(|left, right| {
        left.to_str()
            .expect("UTF-8 cell path")
            .as_bytes()
            .cmp(right.to_str().expect("UTF-8 cell path").as_bytes())
    });
    let mut preimage = domain.to_vec();
    for path in paths {
        let path_text = path.to_str().expect("UTF-8 cell path");
        let path_bytes = path_text.as_bytes();
        preimage.extend_from_slice(
            &u64::try_from(path_bytes.len())
                .expect("bounded cell path")
                .to_be_bytes(),
        );
        preimage.extend_from_slice(path_bytes);
        preimage.extend_from_slice(&Sha256::digest(fs::read(path).expect("cell bytes")));
    }
    sha256_bytes(&preimage)
}

#[cfg(unix)]
fn run(current_dir: &Path, program: &str, args: &[&str]) {
    let status = Command::new(program)
        .args(args)
        .current_dir(current_dir)
        .status()
        .expect("run fixture command");
    assert!(status.success(), "{program} {args:?} failed");
}

#[cfg(unix)]
fn output(current_dir: &Path, program: &str, args: &[&str]) -> String {
    let output = Command::new(program)
        .args(args)
        .current_dir(current_dir)
        .output()
        .expect("run fixture command");
    assert!(output.status.success(), "{program} {args:?} failed");
    String::from_utf8(output.stdout)
        .expect("UTF-8 command output")
        .trim()
        .to_owned()
}

#[cfg(unix)]
fn write_executable(path: &Path, bytes: &[u8]) {
    use std::os::unix::fs::PermissionsExt;

    fs::write(path, bytes).expect("write executable");
    let mut permissions = fs::metadata(path)
        .expect("executable metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("set executable permissions");
}
