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
    HostFeatureSupportStatus, IntegrationProfile, MAX_MANAGED_MCP_CLIENT_INFO_FIELD_BYTES,
    REVIEWED_CODEX_HOST_VERSION, REVIEWED_CODEX_MCP_CLIENT_NAME,
};

use crate::{
    audit::{run_audit, AuditRequest},
    evaluation::evaluate_release_matrix,
    gate::{run_gate, GateRequest},
    io::{
        git_archive_sha256, parse_strict_json, sha256_bytes, ResultRootLease, ValidationContext,
        RELEASE_RESULT_ROOT_ACTIVE_STATE, RELEASE_RESULT_ROOT_LOCK_NAME,
    },
    schema::{
        expected_assertion_ids, AuditVerdict, Candidate, CandidateBuildEnvironment, Cell,
        CellAssertion, CellEnvironment, GateVerdict, HostKind, ImplementationDisposition,
        ReleaseAudit, RequiredNullable, RunState, AUDIT_SCHEMA, CANDIDATE_SCHEMA,
        CELL_INPUTS_DIGEST_DOMAIN, CELL_SCHEMA, MANIFEST_SCHEMA, MAX_FINDING_CODES,
        SOURCE_ARCHIVE_ALGORITHM,
    },
};

const EVALUATED_AT: &str = "2026-01-01T02:00:00Z";
const TARGET: &str = "x86_64-unknown-linux-gnu";

#[test]
fn release_contract_identifiers_use_v3_without_changing_candidate_or_archive_v1() {
    assert_eq!(CANDIDATE_SCHEMA, "volicord-release-candidate-v1");
    assert_eq!(SOURCE_ARCHIVE_ALGORITHM, "git_archive_tar_sha256_v1");
    assert_eq!(CELL_SCHEMA, "volicord-host-release-cell-v3");
    assert_eq!(MANIFEST_SCHEMA, "volicord-host-release-manifest-v3");
    assert_eq!(AUDIT_SCHEMA, "volicord-host-release-audit-v3");
    assert_eq!(
        CELL_INPUTS_DIGEST_DOMAIN,
        b"volicord-host-release-cell-inputs-v3\0"
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
    assert!(audit.invariant_results.iter().any(|invariant| {
        invariant.invariant_id == "single_host_version_per_host" && invariant.passed
    }));
    assert!(audit.invariant_results.iter().any(|invariant| {
        invariant.invariant_id == "single_host_client_identity_per_host" && invariant.passed
    }));
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
    assert_ne!(
        audit.cell_inputs_sha256,
        fixture_cell_inputs_digest(
            &fixture.cell_directory,
            b"volicord-host-release-cell-inputs-v2\0"
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
            cell.raw.client_name.as_ref().map(String::as_str),
            Some(REVIEWED_CODEX_MCP_CLIENT_NAME)
        );
        assert_eq!(
            cell.raw.client_version.as_ref().map(String::as_str),
            Some(REVIEWED_CODEX_HOST_VERSION)
        );
        assert_eq!(cell.raw.environment.client_name, cell.raw.client_name);
        assert_eq!(cell.raw.environment.client_version, cell.raw.client_version);
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
    assert!(audit.recalculated_cells.iter().all(|cell| {
        cell.host_kind != HostKind::Codex
            || (cell.client_name.as_ref().map(String::as_str)
                == Some(REVIEWED_CODEX_MCP_CLIENT_NAME)
                && cell.client_version.as_ref().map(String::as_str)
                    == Some(REVIEWED_CODEX_HOST_VERSION))
    }));
}

#[cfg(unix)]
#[test]
fn implemented_client_identity_missing_downgrades_gate_and_independent_audit() {
    let mut fixture = Fixture::new();
    let index = fixture.implemented_cell_index();
    let cell_key = fixture.cells[index].key();
    fixture.cells[index].client_name = RequiredNullable::null();
    fixture.cells[index].client_version = RequiredNullable::null();
    fixture.cells[index].environment.client_name = RequiredNullable::null();
    fixture.cells[index].environment.client_version = RequiredNullable::null();
    fixture.write_cell(index);

    let manifest = fixture.run_gate("client-identity-missing.json", EVALUATED_AT);
    assert_eq!(manifest.verdict, GateVerdict::Fail);
    assert!(!manifest
        .invariant_findings
        .contains(&"single_host_client_identity_per_host".to_owned()));
    let cell = manifest
        .cells
        .iter()
        .find(|cell| cell.raw.key() == cell_key)
        .expect("missing-client cell");
    assert_eq!(
        cell.derived_status,
        HostFeatureSupportStatus::ImplementedUnverified
    );
    assert!(cell
        .finding_codes
        .contains(&"client_identity_missing".to_owned()));
    assert!(!cell
        .finding_codes
        .contains(&"client_identity_mismatch".to_owned()));

    let audit = fixture.run_audit(
        "client-identity-missing.json",
        "client-identity-missing-audit.json",
    );
    assert_eq!(audit.recalculated_verdict, GateVerdict::Fail);
    assert_eq!(audit.audit_verdict, AuditVerdict::Fail);
    let recalculated = audit
        .recalculated_cells
        .iter()
        .find(|cell| {
            cell.host_kind == fixture.cells[index].host_kind
                && cell.feature == fixture.cells[index].feature
        })
        .expect("independently recalculated missing-client cell");
    assert!(recalculated
        .finding_codes
        .contains(&"client_identity_missing".to_owned()));
}

#[cfg(unix)]
#[test]
fn static_unsupported_cells_may_omit_client_identity_with_non_null_host() {
    let mut fixture = Fixture::new();
    fixture.set_codex_host_version(REVIEWED_CODEX_HOST_VERSION);
    for index in 0..fixture.cells.len() {
        if fixture.cells[index].host_kind == HostKind::Codex
            && fixture.cells[index].implementation_disposition
                == ImplementationDisposition::UnsupportedByHost
        {
            fixture.cells[index].client_name = RequiredNullable::null();
            fixture.cells[index].client_version = RequiredNullable::null();
            fixture.cells[index].environment.client_name = RequiredNullable::null();
            fixture.cells[index].environment.client_version = RequiredNullable::null();
            fixture.write_cell(index);
        }
    }

    let manifest = fixture.run_gate("static-unsupported-null-client.json", EVALUATED_AT);
    assert_eq!(manifest.verdict, GateVerdict::Pass);
    assert!(manifest
        .cells
        .iter()
        .filter(|cell| {
            cell.raw.host_kind == HostKind::Codex
                && cell.raw.implementation_disposition
                    == ImplementationDisposition::UnsupportedByHost
        })
        .all(|cell| {
            cell.raw.host_version.as_ref().is_some()
                && cell.raw.client_name.as_ref().is_none()
                && cell.raw.client_version.as_ref().is_none()
                && cell.derived_status == HostFeatureSupportStatus::UnsupportedByHost
                && !cell
                    .finding_codes
                    .iter()
                    .any(|code| code.starts_with("client_identity_"))
        }));

    let audit = fixture.run_audit(
        "static-unsupported-null-client.json",
        "static-unsupported-null-client-audit.json",
    );
    assert_eq!(audit.audit_verdict, AuditVerdict::Pass);
    assert_eq!(audit.recalculated_verdict, GateVerdict::Pass);
}

#[cfg(unix)]
#[test]
fn client_identity_shape_rejects_partial_null_hostless_and_invalid_fields() {
    let fixture = Fixture::new();
    let index = fixture.implemented_cell_index();

    let mut partial = fixture.cells.clone();
    partial[index].environment.client_version = RequiredNullable::null();
    let partial_error = evaluate_release_matrix(
        &fixture.context,
        fixture.candidate.clone(),
        partial,
        EVALUATED_AT,
    )
    .expect_err("a partial-null client quartet must fail structurally");
    assert!(partial_error
        .detail()
        .contains("must be all strings or all null"));

    let mut hostless = fixture.cells.clone();
    hostless[index].host_version = RequiredNullable::null();
    hostless[index].environment.host_version = RequiredNullable::null();
    hostless[index].environment.host_executable_sha256 = RequiredNullable::null();
    hostless[index].run_state = RunState::Ignored;
    hostless[index].requested_verified = false;
    for assertion in &mut hostless[index].assertions {
        assertion.passed = false;
        assertion.finding_codes = Some(vec!["host_unavailable".to_owned()]);
    }
    let hostless_error = evaluate_release_matrix(
        &fixture.context,
        fixture.candidate.clone(),
        hostless,
        EVALUATED_AT,
    )
    .expect_err("non-null client identity cannot exist without host availability");
    assert!(hostless_error
        .detail()
        .contains("non-null client identity requires non-null host availability"));

    for invalid in [
        "   ".to_owned(),
        "x".repeat(MAX_MANAGED_MCP_CLIENT_INFO_FIELD_BYTES + 1),
    ] {
        let mut invalid_cells = fixture.cells.clone();
        invalid_cells[index].client_name = RequiredNullable::some(invalid.clone());
        invalid_cells[index].environment.client_name = RequiredNullable::some(invalid);
        let error = evaluate_release_matrix(
            &fixture.context,
            fixture.candidate.clone(),
            invalid_cells,
            EVALUATED_AT,
        )
        .expect_err("invalid managed client identity text must fail structurally");
        assert!(error.detail().contains("cell.client_name is invalid"));
    }
}

#[cfg(unix)]
#[test]
fn mismatched_client_identity_downgrades_gate_and_independent_audit() {
    for case in [
        "version_differs_from_host",
        "reviewed_codex_name",
        "environment_copy",
    ] {
        let mut fixture = Fixture::new();
        let host_kind = match case {
            "version_differs_from_host" => {
                fixture.set_host_client_identity(
                    HostKind::ClaudeCode,
                    Some("claude-code-mcp-client"),
                    Some("different-client-version"),
                );
                HostKind::ClaudeCode
            }
            "reviewed_codex_name" => {
                fixture.set_codex_host_version(REVIEWED_CODEX_HOST_VERSION);
                fixture.set_host_client_identity(
                    HostKind::Codex,
                    Some("wrong-codex-client"),
                    Some(REVIEWED_CODEX_HOST_VERSION),
                );
                HostKind::Codex
            }
            "environment_copy" => {
                for index in 0..fixture.cells.len() {
                    if fixture.cells[index].host_kind == HostKind::ClaudeCode {
                        fixture.cells[index].environment.client_name =
                            RequiredNullable::some("wrong-environment-client".to_owned());
                        fixture.write_cell(index);
                    }
                }
                HostKind::ClaudeCode
            }
            _ => unreachable!("client mismatch cases are exhaustive"),
        };

        let manifest_name = format!("client-identity-mismatch-{case}.json");
        let manifest = fixture.run_gate(&manifest_name, EVALUATED_AT);
        assert_eq!(manifest.verdict, GateVerdict::Fail, "{case}");
        assert!(!manifest
            .invariant_findings
            .contains(&"single_host_client_identity_per_host".to_owned()));
        assert!(manifest.cells.iter().any(|cell| {
            cell.raw.host_kind == host_kind
                && cell.raw.implementation_disposition == ImplementationDisposition::Implemented
                && cell.derived_status == HostFeatureSupportStatus::ImplementedUnverified
                && cell
                    .finding_codes
                    .contains(&"client_identity_mismatch".to_owned())
        }));

        let audit_name = format!("client-identity-mismatch-{case}-audit.json");
        let audit = fixture.run_audit(&manifest_name, &audit_name);
        assert_eq!(audit.recalculated_verdict, GateVerdict::Fail, "{case}");
        assert_eq!(audit.audit_verdict, AuditVerdict::Fail, "{case}");
        assert!(audit.recalculated_cells.iter().any(|cell| {
            cell.host_kind == host_kind
                && cell.derived_status == HostFeatureSupportStatus::ImplementedUnverified
                && cell
                    .finding_codes
                    .contains(&"client_identity_mismatch".to_owned())
        }));
    }
}

#[cfg(unix)]
#[test]
fn divergent_non_null_client_identity_fails_gate_and_independent_audit_invariant() {
    let mut fixture = Fixture::new();
    let index = fixture
        .cells
        .iter()
        .position(|cell| cell.host_kind == HostKind::ClaudeCode)
        .expect("Claude Code cell");
    let cell_key = fixture.cells[index].key();
    fixture.cells[index].client_name = RequiredNullable::some("other-claude-client".to_owned());
    fixture.cells[index].environment.client_name =
        RequiredNullable::some("other-claude-client".to_owned());
    fixture.write_cell(index);

    let manifest = fixture.run_gate("divergent-client-identity.json", EVALUATED_AT);
    assert_eq!(manifest.verdict, GateVerdict::Fail);
    assert!(manifest
        .invariant_findings
        .contains(&"single_host_client_identity_per_host".to_owned()));
    let cell = manifest
        .cells
        .iter()
        .find(|cell| cell.raw.key() == cell_key)
        .expect("divergent-client cell");
    assert_eq!(cell.derived_status, HostFeatureSupportStatus::Verified);
    assert!(!cell
        .finding_codes
        .contains(&"client_identity_mismatch".to_owned()));

    let audit = fixture.run_audit(
        "divergent-client-identity.json",
        "divergent-client-identity-audit.json",
    );
    assert_eq!(audit.recalculated_verdict, GateVerdict::Fail);
    assert_eq!(audit.audit_verdict, AuditVerdict::Fail);
    assert!(audit.invariant_results.iter().any(|invariant| {
        invariant.invariant_id == "single_host_version_per_host" && invariant.passed
    }));
    assert!(audit.invariant_results.iter().any(|invariant| {
        invariant.invariant_id == "single_host_client_identity_per_host" && !invariant.passed
    }));
}

#[cfg(unix)]
#[test]
fn raw_codex_probe_envelope_is_rejected_as_a_noncanonical_host_version() {
    let mut fixture = Fixture::new();
    fixture.set_codex_host_version("codex-cli 0.144.4");
    let output = fixture.external.path().join("raw-codex-version.json");
    let error = run_gate(
        &fixture.context,
        &GateRequest {
            candidate_descriptor: fixture.candidate_descriptor.clone(),
            cell_directory: fixture.cell_directory.clone(),
            manifest_output: output.clone(),
            evaluated_at: EVALUATED_AT.to_owned(),
        },
    )
    .expect_err("a raw Codex probe envelope must not become a release coordinate");
    assert!(error
        .detail()
        .contains("must be a canonical bare Codex version"));
    assert!(!output.exists());
}

#[cfg(unix)]
#[test]
fn codex_environment_version_uses_top_level_host_kind_for_gate_and_audit_shape() {
    let mut fixture = Fixture::new();
    let clean_manifest_name = "clean-before-codex-environment-mutation.json";
    fixture.run_gate(clean_manifest_name, EVALUATED_AT);

    for index in 0..fixture.cells.len() {
        if fixture.cells[index].host_kind != HostKind::Codex {
            continue;
        }
        fixture.cells[index].environment.host_kind = HostKind::ClaudeCode;
        fixture.cells[index].environment.host_version =
            RequiredNullable::some("codex-cli 0.144.4".to_owned());
        fixture.write_cell(index);
    }

    let manifest_output = fixture
        .external
        .path()
        .join("invalid-codex-environment-version.json");
    let gate_error = run_gate(
        &fixture.context,
        &GateRequest {
            candidate_descriptor: fixture.candidate_descriptor.clone(),
            cell_directory: fixture.cell_directory.clone(),
            manifest_output: manifest_output.clone(),
            evaluated_at: EVALUATED_AT.to_owned(),
        },
    )
    .expect_err("a Codex cell must validate its environment version as a Codex coordinate");
    assert!(gate_error
        .detail()
        .contains("cell.environment.host_version must be a canonical bare Codex version"));
    assert!(!manifest_output.exists());

    let audit_output = fixture
        .external
        .path()
        .join("invalid-codex-environment-version-audit.json");
    let audit_error = run_audit(
        &fixture.context,
        &AuditRequest {
            candidate_descriptor: fixture.candidate_descriptor.clone(),
            cell_directory: fixture.cell_directory.clone(),
            manifest: fixture.external.path().join(clean_manifest_name),
            audit_output: audit_output.clone(),
            started_at: EVALUATED_AT.to_owned(),
            evaluated_at: EVALUATED_AT.to_owned(),
        },
    )
    .expect_err("independent audit must reject the same malformed Codex cell shape");
    assert!(audit_error
        .detail()
        .contains("cell.environment.host_version must be a canonical bare Codex version"));
    assert!(!audit_output.exists());
}

#[cfg(unix)]
#[test]
fn historical_v1_and_v2_cell_and_manifest_inputs_are_rejected() {
    for version in ["v1", "v2"] {
        let mut cell_fixture = Fixture::new();
        cell_fixture.cells[0].schema = format!("volicord-host-release-cell-{version}");
        cell_fixture.write_cell(0);
        let cell_manifest = cell_fixture
            .external
            .path()
            .join(format!("{version}-cell-manifest.json"));
        let cell_error = run_gate(
            &cell_fixture.context,
            &GateRequest {
                candidate_descriptor: cell_fixture.candidate_descriptor.clone(),
                cell_directory: cell_fixture.cell_directory.clone(),
                manifest_output: cell_manifest.clone(),
                evaluated_at: EVALUATED_AT.to_owned(),
            },
        )
        .expect_err("historical cell input must be rejected");
        assert!(cell_error
            .detail()
            .contains("cell schema identifier mismatch"));
        assert!(!cell_manifest.exists());
    }

    let manifest_fixture = Fixture::new();
    manifest_fixture.run_gate("v3-manifest.json", EVALUATED_AT);
    let v3_manifest_path = manifest_fixture.external.path().join("v3-manifest.json");
    let canonical: Value =
        serde_json::from_slice(&fs::read(&v3_manifest_path).expect("v3 manifest bytes"))
            .expect("v3 manifest JSON");
    for version in ["v1", "v2"] {
        let mut historical = canonical.clone();
        historical["schema"] = Value::String(format!("volicord-host-release-manifest-{version}"));
        let historical_path = manifest_fixture
            .external
            .path()
            .join(format!("{version}-manifest.json"));
        fs::write(
            &historical_path,
            serde_json::to_vec_pretty(&historical).expect("historical manifest bytes"),
        )
        .expect("historical manifest fixture");
        let audit_output = manifest_fixture
            .external
            .path()
            .join(format!("{version}-manifest-audit.json"));
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
        .expect_err("historical manifest input must be rejected");
        assert!(manifest_error
            .detail()
            .contains("manifest schema identifier mismatch"));
        assert!(!audit_output.exists());
    }
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
fn derived_finding_code_bound_is_enforced_by_gate_and_independent_audit() {
    let mut fixture = Fixture::new();
    fixture.run_gate("finding-bound-baseline.json", EVALUATED_AT);

    let index = fixture.implemented_cell_index();
    fixture.cells[index].assertions[0].passed = false;
    fixture.cells[index].assertions[0].finding_codes = Some(
        (0..MAX_FINDING_CODES)
            .map(|code| format!("failed_assertion_{code:02}"))
            .collect(),
    );
    fixture.cells[index].claimed_status = HostFeatureSupportStatus::ImplementedUnverified;
    fixture.write_cell(index);

    let gate_output = fixture.external.path().join("finding-bound-manifest.json");
    let gate_error = run_gate(
        &fixture.context,
        &GateRequest {
            candidate_descriptor: fixture.candidate_descriptor.clone(),
            cell_directory: fixture.cell_directory.clone(),
            manifest_output: gate_output.clone(),
            evaluated_at: EVALUATED_AT.to_owned(),
        },
    )
    .expect_err("derived gate findings over the bound must fail");
    assert_eq!(gate_error.detail(), "derived cell findings exceed bound");
    assert!(!gate_output.exists());

    let audit_output = fixture.external.path().join("finding-bound-audit.json");
    let audit_error = run_audit(
        &fixture.context,
        &AuditRequest {
            candidate_descriptor: fixture.candidate_descriptor.clone(),
            cell_directory: fixture.cell_directory.clone(),
            manifest: fixture.external.path().join("finding-bound-baseline.json"),
            audit_output: audit_output.clone(),
            started_at: EVALUATED_AT.to_owned(),
            evaluated_at: EVALUATED_AT.to_owned(),
        },
    )
    .expect_err("independently derived audit findings over the bound must fail");
    assert_eq!(audit_error.detail(), "derived cell findings exceed bound");
    assert!(!audit_output.exists());
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
fn gate_and_audit_reject_nonclean_or_staged_cells_and_never_adopt_orphan_evidence() {
    let fixture = Fixture::new();
    fixture.run_gate("baseline-manifest.json", EVALUATED_AT);
    let manifest = fixture.external.path().join("baseline-manifest.json");

    let missing_index = fixture.implemented_cell_index();
    fs::remove_file(&fixture.cell_paths[missing_index]).expect("remove one committed cell");
    let missing_audit_output = fixture.external.path().join("missing-audit.json");
    let error = run_audit(
        &fixture.context,
        &AuditRequest {
            candidate_descriptor: fixture.candidate_descriptor.clone(),
            cell_directory: fixture.cell_directory.clone(),
            manifest: manifest.clone(),
            audit_output: missing_audit_output.clone(),
            started_at: EVALUATED_AT.to_owned(),
            evaluated_at: EVALUATED_AT.to_owned(),
        },
    )
    .expect_err("a missing committed cell must be an audit structural failure");
    assert!(error.detail().contains("exactly twelve"));
    assert!(!missing_audit_output.exists());
    fixture.write_cell(missing_index);

    let cell_stage = fixture
        .cell_directory
        .join(".volicord-live-stage-uncommitted");
    fs::write(&cell_stage, b"private stage").expect("private cell stage");
    let staged_gate_output = fixture.external.path().join("staged-gate.json");
    let error = run_gate(
        &fixture.context,
        &GateRequest {
            candidate_descriptor: fixture.candidate_descriptor.clone(),
            cell_directory: fixture.cell_directory.clone(),
            manifest_output: staged_gate_output.clone(),
            evaluated_at: EVALUATED_AT.to_owned(),
        },
    )
    .expect_err("an extra cell stage must be a gate structural failure");
    assert!(error.detail().contains("exactly twelve"));
    assert!(!staged_gate_output.exists());

    let staged_audit_output = fixture.external.path().join("staged-audit.json");
    run_audit(
        &fixture.context,
        &AuditRequest {
            candidate_descriptor: fixture.candidate_descriptor.clone(),
            cell_directory: fixture.cell_directory.clone(),
            manifest: manifest.clone(),
            audit_output: staged_audit_output.clone(),
            started_at: EVALUATED_AT.to_owned(),
            evaluated_at: EVALUATED_AT.to_owned(),
        },
    )
    .expect_err("an extra cell stage must be an audit structural failure");
    assert!(!staged_audit_output.exists());
    assert_eq!(
        fs::read(&cell_stage).expect("preserved cell stage"),
        b"private stage"
    );
    fs::remove_file(&cell_stage).expect("remove test-only stage between cases");

    let evidence_directory = fixture
        .cell_directory
        .parent()
        .expect("result root")
        .join("evidence");
    let orphan_evidence = evidence_directory.join("unreferenced-orphan.bin");
    fs::write(&orphan_evidence, b"unreferenced evidence").expect("orphan evidence");
    let orphan_manifest = fixture.run_gate("orphan-manifest.json", EVALUATED_AT);
    assert_eq!(orphan_manifest.verdict, GateVerdict::Pass);
    let orphan_audit = run_audit(
        &fixture.context,
        &AuditRequest {
            candidate_descriptor: fixture.candidate_descriptor.clone(),
            cell_directory: fixture.cell_directory.clone(),
            manifest: fixture.external.path().join("orphan-manifest.json"),
            audit_output: fixture.external.path().join("orphan-audit.json"),
            started_at: EVALUATED_AT.to_owned(),
            evaluated_at: EVALUATED_AT.to_owned(),
        },
    )
    .expect("unreferenced evidence remains outside the audit input set");
    assert_eq!(orphan_audit.audit_verdict, AuditVerdict::Pass);
    assert_eq!(
        fs::read(&orphan_evidence).expect("preserved orphan evidence"),
        b"unreferenced evidence"
    );

    let result_root = fixture.cell_directory.parent().expect("result root");
    fs::write(
        result_root.join(RELEASE_RESULT_ROOT_LOCK_NAME),
        RELEASE_RESULT_ROOT_ACTIVE_STATE,
    )
    .expect("simulate a non-clean state after final-name installation");
    let active_gate_output = fixture.external.path().join("active-gate.json");
    let error = run_gate(
        &fixture.context,
        &GateRequest {
            candidate_descriptor: fixture.candidate_descriptor.clone(),
            cell_directory: fixture.cell_directory.clone(),
            manifest_output: active_gate_output.clone(),
            evaluated_at: EVALUATED_AT.to_owned(),
        },
    )
    .expect_err("a full final-name set under active state must fail the gate");
    assert!(error.detail().contains("incomplete prior publication"));
    assert!(!active_gate_output.exists());

    let active_audit_output = fixture.external.path().join("active-audit.json");
    let error = run_audit(
        &fixture.context,
        &AuditRequest {
            candidate_descriptor: fixture.candidate_descriptor.clone(),
            cell_directory: fixture.cell_directory.clone(),
            manifest,
            audit_output: active_audit_output.clone(),
            started_at: EVALUATED_AT.to_owned(),
            evaluated_at: EVALUATED_AT.to_owned(),
        },
    )
    .expect_err("a full final-name set under active state must fail the audit");
    assert!(error.detail().contains("incomplete prior publication"));
    assert!(!active_audit_output.exists());
}

#[cfg(unix)]
#[test]
fn gate_and_audit_outputs_cannot_mutate_their_cell_or_evidence_input_sets() {
    let fixture = Fixture::new();
    let evidence_directory = fixture
        .cell_directory
        .parent()
        .expect("result root")
        .join("evidence");

    for forbidden_manifest in [
        fixture.cell_directory.join("manifest.json"),
        evidence_directory.join("manifest.json"),
    ] {
        let error = run_gate(
            &fixture.context,
            &GateRequest {
                candidate_descriptor: fixture.candidate_descriptor.clone(),
                cell_directory: fixture.cell_directory.clone(),
                manifest_output: forbidden_manifest.clone(),
                evaluated_at: EVALUATED_AT.to_owned(),
            },
        )
        .expect_err("manifest output inside a live input directory must fail");
        assert!(error
            .detail()
            .contains("outside the live cell and evidence"));
        assert!(!forbidden_manifest.exists());
    }
    assert_eq!(
        fs::read_dir(&fixture.cell_directory)
            .expect("cell directory")
            .count(),
        12
    );

    fixture.run_gate("valid-output-separation-manifest.json", EVALUATED_AT);
    let forbidden_audit = evidence_directory.join("audit.json");
    let error = run_audit(
        &fixture.context,
        &AuditRequest {
            candidate_descriptor: fixture.candidate_descriptor.clone(),
            cell_directory: fixture.cell_directory.clone(),
            manifest: fixture
                .external
                .path()
                .join("valid-output-separation-manifest.json"),
            audit_output: forbidden_audit.clone(),
            started_at: EVALUATED_AT.to_owned(),
            evaluated_at: EVALUATED_AT.to_owned(),
        },
    )
    .expect_err("audit output inside a live input directory must fail");
    assert!(error
        .detail()
        .contains("outside the live cell and evidence"));
    assert!(!forbidden_audit.exists());
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
        cell.client_name = RequiredNullable::null();
        cell.client_version = RequiredNullable::null();
        cell.environment.host_version = RequiredNullable::null();
        cell.environment.client_name = RequiredNullable::null();
        cell.environment.client_version = RequiredNullable::null();
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
        cell.client_name = RequiredNullable::null();
        cell.client_version = RequiredNullable::null();
        cell.environment.host_version = RequiredNullable::null();
        cell.environment.client_name = RequiredNullable::null();
        cell.environment.client_version = RequiredNullable::null();
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
    assert_eq!(
        context.target_directory(),
        fs::canonicalize(target.path()).expect("canonical target root")
    );

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

#[test]
fn additional_runtime_home_exclusion_rejects_inputs_and_outputs_without_partial_mutation() {
    let source = tempfile::tempdir().expect("source root");
    let external = tempfile::tempdir().expect("external root");
    let runtime_home = external.path().join("observed-runtime-home");
    fs::create_dir(&runtime_home).expect("observed runtime home");
    let input = runtime_home.join("candidate");
    fs::write(&input, b"candidate").expect("runtime-home input");
    let output = runtime_home.join("cell.json");
    let new_directory = runtime_home.join("evidence");
    let mut context = ValidationContext::new(
        source.path().to_path_buf(),
        source.path().join("target"),
        source.path().join("docs"),
        Vec::new(),
    )
    .expect("validation context");

    context
        .validate_existing_file(&input)
        .expect("unregistered runtime root is initially external");
    context
        .validate_new_output(&output)
        .expect("unregistered runtime output is initially external");
    context
        .validate_new_directory(&new_directory)
        .expect("unregistered runtime directory is initially external");

    let error = context
        .add_runtime_home(Path::new("relative-runtime-home"))
        .expect_err("an observed Runtime Home exclusion must be absolute");
    assert!(error.detail().contains("must be absolute"));
    context
        .validate_existing_file(&input)
        .expect("failed extension must not partially mutate the context");
    context
        .validate_new_output(&output)
        .expect("failed extension must leave output policy unchanged");

    context
        .add_runtime_home(&runtime_home)
        .expect("absolute observed Runtime Home exclusion");
    for error in [
        context
            .validate_existing_file(&input)
            .expect_err("Runtime Home input must be rejected"),
        context
            .validate_new_output(&output)
            .expect_err("Runtime Home output must be rejected"),
        context
            .validate_new_directory(&new_directory)
            .expect_err("Runtime Home directory must be rejected"),
    ] {
        assert!(
            error.detail().contains("Volicord Runtime Home"),
            "{}",
            error.detail()
        );
    }
    assert!(!output.exists());
    assert!(!new_directory.exists());
}

#[test]
fn new_directory_validation_rejects_an_ancestor_of_an_excluded_runtime_without_creating_it() {
    let source = tempfile::tempdir().expect("source root");
    let external = tempfile::tempdir().expect("external root");
    let new_directory = external.path().join("release-records");
    let future_runtime_home = new_directory.join("runtime-home");
    let mut context = ValidationContext::new(
        source.path().to_path_buf(),
        source.path().join("target"),
        source.path().join("docs"),
        Vec::new(),
    )
    .expect("validation context");
    context
        .add_runtime_home(&future_runtime_home)
        .expect("missing observed Runtime Home root is normalized from its existing prefix");

    let error = context
        .validate_new_directory(&new_directory)
        .expect_err("a directory containing an excluded Runtime Home must be rejected");
    assert!(error.detail().contains("Volicord Runtime Home"));
    assert!(!new_directory.exists());
}

#[cfg(unix)]
#[test]
fn result_root_lease_process_helper() {
    let Some(source) = std::env::var_os("VOLICORD_TEST_LEASE_SOURCE") else {
        return;
    };
    let cell_path = PathBuf::from(
        std::env::var_os("VOLICORD_TEST_LEASE_CELL").expect("lease helper cell path"),
    );
    let ready_path = PathBuf::from(
        std::env::var_os("VOLICORD_TEST_LEASE_READY").expect("lease helper ready path"),
    );
    let release_path = PathBuf::from(
        std::env::var_os("VOLICORD_TEST_LEASE_RELEASE").expect("lease helper release path"),
    );
    let source = PathBuf::from(source);
    let context = ValidationContext::new(
        source.clone(),
        source.join("target"),
        source.join("docs"),
        Vec::new(),
    )
    .expect("lease helper validation context");
    let mut lease = ResultRootLease::acquire_exclusive_for_cell_path(&context, &cell_path)
        .expect("lease helper exclusive acquisition");
    lease
        .begin_publication_attempt()
        .expect("lease helper active state");
    fs::write(&ready_path, b"ready").expect("lease helper ready marker");
    for _ in 0..500 {
        if release_path.exists() {
            return;
        }
        thread::sleep(StdDuration::from_millis(10));
    }
    panic!("lease helper release marker was not observed");
}

#[cfg(unix)]
#[test]
fn result_root_lease_serializes_processes_and_durably_rejects_failed_roots() {
    fn create_result_root(parent: &Path, name: &str) -> (PathBuf, PathBuf, PathBuf) {
        let root = parent.join(name);
        let cells = root.join("cells");
        let evidence = root.join("evidence");
        fs::create_dir_all(&cells).expect("cell directory");
        fs::create_dir(&evidence).expect("evidence directory");
        (root, cells, evidence)
    }

    let source = tempfile::tempdir().expect("lease source root");
    let external = tempfile::tempdir().expect("lease external root");
    let context = ValidationContext::new(
        source.path().to_path_buf(),
        source.path().join("target"),
        source.path().join("docs"),
        Vec::new(),
    )
    .expect("lease validation context");

    let (failed_root, failed_cells, failed_evidence) =
        create_result_root(external.path(), "failed-result-root");
    let failed_cell = failed_cells.join("first.json");
    let ready_path = external.path().join("lease-helper-ready");
    let release_path = external.path().join("lease-helper-release");
    let mut child = Command::new(std::env::current_exe().expect("current test executable"))
        .args([
            "tests::result_root_lease_process_helper",
            "--exact",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("VOLICORD_TEST_LEASE_SOURCE", source.path())
        .env("VOLICORD_TEST_LEASE_CELL", &failed_cell)
        .env("VOLICORD_TEST_LEASE_READY", &ready_path)
        .env("VOLICORD_TEST_LEASE_RELEASE", &release_path)
        .spawn()
        .expect("spawn lease helper");
    for _ in 0..500 {
        if ready_path.exists() {
            break;
        }
        assert!(
            child.try_wait().expect("poll lease helper").is_none(),
            "lease helper exited before acquiring its lease"
        );
        thread::sleep(StdDuration::from_millis(10));
    }
    assert!(ready_path.exists(), "lease helper never reported readiness");
    let contention = ResultRootLease::acquire_exclusive_for_cell_path(
        &context,
        &failed_cells.join("second.json"),
    )
    .expect_err("a second process must not acquire the active result root");
    assert!(contention.detail().contains("cannot acquire Exclusive"));
    fs::write(&release_path, b"release").expect("release lease helper");
    assert!(child.wait().expect("wait for lease helper").success());
    assert_eq!(
        fs::read_dir(&failed_cells).expect("failed cells").count(),
        0
    );
    assert_eq!(
        fs::read_dir(&failed_evidence)
            .expect("failed evidence")
            .count(),
        0
    );
    let stale_active = ResultRootLease::acquire_exclusive_for_cell_path(
        &context,
        &failed_cells.join("second.json"),
    )
    .expect_err("stale active state must poison same-root recovery");
    assert!(stale_active
        .detail()
        .contains("incomplete prior publication"));
    let stale_shared = ResultRootLease::acquire_shared_for_cell_directory(&context, &failed_cells)
        .expect_err("gate and audit shared leases must reject stale active state");
    assert!(stale_shared
        .detail()
        .contains("incomplete prior publication"));

    let (fresh_root, fresh_cells, _) = create_result_root(external.path(), "fresh-result-root");
    let mut fresh =
        ResultRootLease::acquire_exclusive_for_cell_path(&context, &fresh_cells.join("first.json"))
            .expect("fresh root acquisition");
    fresh
        .begin_publication_attempt()
        .expect("fresh root active state");
    fresh
        .complete_publication_attempt()
        .expect("fresh root clean completion");
    drop(fresh);
    drop(
        ResultRootLease::acquire_exclusive_for_cell_path(
            &context,
            &fresh_cells.join("second.json"),
        )
        .expect("clean completed root can accept its next serialized cell"),
    );
    let mut shared = ResultRootLease::acquire_shared_for_cell_directory(&context, &fresh_cells)
        .expect("shared clean-root lease");
    let shared_transition = shared
        .begin_publication_attempt()
        .expect_err("a shared lease must never mutate publication state");
    assert!(shared_transition
        .detail()
        .contains("shared result-root lease"));
    drop(shared);

    fs::write(fresh_root.join(RELEASE_RESULT_ROOT_LOCK_NAME), b"")
        .expect("simulate interrupted state rewrite");
    let empty_state =
        ResultRootLease::acquire_exclusive_for_cell_path(&context, &fresh_cells.join("third.json"))
            .expect_err("an existing empty lease state must not be reinitialized");
    assert!(empty_state.detail().contains("missing or malformed"));

    let (_, staged_cells, _) = create_result_root(external.path(), "staged-result-root");
    fs::write(staged_cells.join(".volicord-live-stage-fixture"), b"stage")
        .expect("private cell stage");
    let staged = ResultRootLease::acquire_exclusive_for_cell_path(
        &context,
        &staged_cells.join("first.json"),
    )
    .expect_err("a private cell stage must poison producer reuse");
    assert!(staged.detail().contains("cannot adopt pre-existing"));

    let (_, orphan_cells, orphan_evidence) =
        create_result_root(external.path(), "orphan-result-root");
    fs::write(orphan_evidence.join("orphan.json"), b"orphan").expect("orphan evidence");
    let orphan = ResultRootLease::acquire_exclusive_for_cell_path(
        &context,
        &orphan_cells.join("first.json"),
    )
    .expect_err("orphan evidence must poison producer reuse");
    assert!(orphan.detail().contains("cannot adopt pre-existing"));

    assert!(failed_root.join(RELEASE_RESULT_ROOT_LOCK_NAME).exists());
}

#[cfg(unix)]
#[test]
fn result_root_lease_rejects_a_shape_invalid_prior_cell_before_the_next_attempt() {
    let fixture = Fixture::new();
    let result_root = fixture.external.path().join("shape-invalid-result-root");
    let cells = result_root.join("cells");
    let evidence = result_root.join("evidence");
    fs::create_dir_all(&cells).expect("cell directory");
    fs::create_dir(&evidence).expect("evidence directory");

    drop(
        ResultRootLease::acquire_exclusive_for_cell_path(
            &fixture.context,
            &cells.join("lease-bootstrap-not-published.json"),
        )
        .expect("initialize clean result-root state"),
    );

    let mut invalid = fixture
        .cells
        .iter()
        .find(|cell| cell.evidence_artifact_path.as_ref().is_none())
        .expect("fixture includes a static unsupported cell")
        .clone();
    invalid.schema = "wrong-cell-schema".to_owned();
    fs::write(
        cells.join("prior.json"),
        serde_json::to_vec_pretty(&invalid).expect("invalid prior cell JSON"),
    )
    .expect("invalid prior cell");

    let error = ResultRootLease::acquire_exclusive_for_cell_path(
        &fixture.context,
        &cells.join("next.json"),
    )
    .expect_err("a shape-invalid prior cell must poison producer reuse");
    assert!(error.detail().contains("cell schema identifier mismatch"));
}

#[cfg(unix)]
#[test]
fn new_directory_validation_requires_an_existing_canonical_symlink_free_parent_and_absent_leaf() {
    use std::os::unix::fs::symlink;

    let source = tempfile::tempdir().expect("source root");
    let external = tempfile::tempdir().expect("external root");
    let parent = external.path().join("real-parent");
    fs::create_dir(&parent).expect("real output parent");
    let context = ValidationContext::new(
        source.path().to_path_buf(),
        source.path().join("target"),
        source.path().join("docs"),
        Vec::new(),
    )
    .expect("validation context");

    let accepted = parent.join("new-evidence");
    context
        .validate_new_directory(&accepted)
        .expect("an absent directory under a canonical real parent is valid");
    assert!(!accepted.exists());

    let existing = parent.join("existing-evidence");
    fs::create_dir(&existing).expect("existing output leaf");
    let error = context
        .validate_new_directory(&existing)
        .expect_err("an existing directory must be rejected");
    assert!(error.detail().contains("directory already exists"));

    let missing_parent_output = external.path().join("missing-parent").join("evidence");
    context
        .validate_new_directory(&missing_parent_output)
        .expect_err("a missing parent must be rejected");
    assert!(!missing_parent_output.exists());

    let parent_alias = external.path().join("parent-alias");
    symlink(&parent, &parent_alias).expect("output-parent symlink");
    let aliased_output = parent_alias.join("aliased-evidence");
    let error = context
        .validate_new_directory(&aliased_output)
        .expect_err("a symlinked parent must be rejected");
    assert!(error.detail().contains("symbolic links are not allowed"));
    assert!(!parent.join("aliased-evidence").exists());
}

#[test]
fn empty_home_uses_nonempty_userprofile_for_runtime_home_exclusion() {
    let user_profile = tempfile::tempdir().expect("USERPROFILE root");
    let runtime_home = user_profile.path().join(".volicord");
    fs::create_dir(&runtime_home).expect("USERPROFILE Runtime Home");
    let input = runtime_home.join("candidate");
    fs::write(&input, b"candidate").expect("Runtime Home input");
    let context = ValidationContext::from_process_environment(
        Path::new(env!("CARGO_MANIFEST_DIR")),
        None,
        Some(std::ffi::OsString::new()),
        Some(user_profile.path().as_os_str().to_os_string()),
    )
    .expect("process validation context");

    let error = context
        .validate_existing_file(&input)
        .expect_err("nonempty USERPROFILE must replace an empty HOME");
    assert!(error.detail().contains("Volicord Runtime Home"));
}

#[cfg(unix)]
#[test]
fn accepted_artifact_paths_require_exact_utf8_without_creating_outputs() {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};

    let source = tempfile::tempdir().expect("source root");
    let external = tempfile::tempdir().expect("external root");
    let context = ValidationContext::new(
        source.path().to_path_buf(),
        source.path().join("target"),
        source.path().join("docs"),
        Vec::new(),
    )
    .expect("validation context");
    let input = external
        .path()
        .join(OsString::from_vec(b"candidate-\xff".to_vec()));
    let output = external
        .path()
        .join(OsString::from_vec(b"cell-\xfe.json".to_vec()));
    let new_directory = external
        .path()
        .join(OsString::from_vec(b"evidence-\xfd".to_vec()));
    fs::write(&input, b"candidate").expect("non-UTF-8 input fixture");

    for error in [
        context
            .validate_existing_file(&input)
            .expect_err("non-UTF-8 input path must fail"),
        context
            .validate_new_output(&output)
            .expect_err("non-UTF-8 output path must fail"),
        context
            .validate_new_directory(&new_directory)
            .expect_err("non-UTF-8 directory path must fail"),
    ] {
        assert!(error.detail().contains("not valid UTF-8"));
    }
    assert!(!output.exists());
    assert!(!new_directory.exists());
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
        let context = ValidationContext::new(
            source_checkout.clone(),
            source_checkout.join("target"),
            source_checkout.join("docs"),
            vec![source_checkout.join(".volicord")],
        )
        .expect("validation context");
        drop(
            ResultRootLease::acquire_exclusive_for_cell_path(
                &context,
                &cell_directory.join("lease-bootstrap-not-published.json"),
            )
            .expect("initialize result-root lease"),
        );
        let mut cells = Vec::new();
        let mut cell_paths = Vec::new();
        for host_kind in HostKind::ALL {
            for feature in HostFeature::ALL {
                let host_version = match host_kind {
                    HostKind::Codex => "codex-test-1",
                    HostKind::ClaudeCode => "claude-test-1",
                };
                let client_name = match host_kind {
                    HostKind::Codex => REVIEWED_CODEX_MCP_CLIENT_NAME,
                    HostKind::ClaudeCode => "claude-code-mcp-client",
                };
                let client_version = host_version;
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
                    client_name: RequiredNullable::some(client_name.to_owned()),
                    client_version: RequiredNullable::some(client_version.to_owned()),
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
                        client_name: RequiredNullable::some(client_name.to_owned()),
                        client_version: RequiredNullable::some(client_version.to_owned()),
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

    fn run_audit(&self, manifest_name: &str, audit_name: &str) -> ReleaseAudit {
        run_audit(
            &self.context,
            &AuditRequest {
                candidate_descriptor: self.candidate_descriptor.clone(),
                cell_directory: self.cell_directory.clone(),
                manifest: self.external.path().join(manifest_name),
                audit_output: self.external.path().join(audit_name),
                started_at: EVALUATED_AT.to_owned(),
                evaluated_at: EVALUATED_AT.to_owned(),
            },
        )
        .expect("independent release audit")
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
            self.cells[index].client_name =
                RequiredNullable::some(REVIEWED_CODEX_MCP_CLIENT_NAME.to_owned());
            self.cells[index].client_version = RequiredNullable::some(host_version.to_owned());
            self.cells[index].environment.client_name =
                RequiredNullable::some(REVIEWED_CODEX_MCP_CLIENT_NAME.to_owned());
            self.cells[index].environment.client_version =
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

    fn set_host_client_identity(
        &mut self,
        host_kind: HostKind,
        client_name: Option<&str>,
        client_version: Option<&str>,
    ) {
        for index in 0..self.cells.len() {
            if self.cells[index].host_kind != host_kind {
                continue;
            }
            self.cells[index].client_name = RequiredNullable(client_name.map(str::to_owned));
            self.cells[index].client_version = RequiredNullable(client_version.map(str::to_owned));
            self.cells[index].environment.client_name =
                RequiredNullable(client_name.map(str::to_owned));
            self.cells[index].environment.client_version =
                RequiredNullable(client_version.map(str::to_owned));
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
