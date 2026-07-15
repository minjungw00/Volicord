use std::{
    fs,
    path::{Path, PathBuf},
};

use chrono::{DateTime, Duration, Utc};
use sha2::{Digest, Sha256};
use volicord_types::{
    evaluate_host_feature_support_for_version, host_feature_implementation_for_version,
    CurrentRuntimeReadiness, ExactLiveEvidenceState, HostFeature, HostFeatureEvaluationInput,
    HostFeatureSupportStatus, IntegrationProfile, REVIEWED_CODEX_HOST_VERSION,
    REVIEWED_CODEX_MCP_CLIENT_NAME,
};

use crate::{
    error::{ValidationError, ValidationResult},
    evaluation::{
        cli_implementation, parse_canonical_timestamp, validate_candidate_shape,
        validate_cell_shape, validate_manifest_container, validate_matrix_shape, validate_sha256,
    },
    io::{
        git_archive_sha256, git_head, git_is_clean, inspect_candidate_artifact, parse_strict_json,
        read_bounded_external_file, read_strict_json, sha256_bytes, sha256_external_file,
        write_json_create_new, ResultRootLease, ValidationContext, MAX_AUDIT_JSON_BYTES,
        MAX_CANDIDATE_JSON_BYTES, MAX_CELL_JSON_BYTES, MAX_EVIDENCE_BYTES, MAX_MANIFEST_JSON_BYTES,
    },
    schema::{
        AuditExclusion, AuditInvariantResult, AuditVerdict, Candidate, Cell, GateVerdict, HostKind,
        ImplementationDisposition, ManifestCell, RecalculatedCell, ReleaseAudit, ReleaseManifest,
        RunState, AUDIT_SCHEMA, CELL_INPUTS_DIGEST_DOMAIN, MANIFEST_SCHEMA, MAX_FINDING_CODES,
    },
};

#[derive(Debug, Clone)]
pub struct AuditRequest {
    pub candidate_descriptor: PathBuf,
    pub cell_directory: PathBuf,
    pub manifest: PathBuf,
    pub audit_output: PathBuf,
    pub started_at: String,
    pub evaluated_at: String,
}

pub fn run_audit(
    context: &ValidationContext,
    request: &AuditRequest,
) -> ValidationResult<ReleaseAudit> {
    ResultRootLease::prevalidate_summary_output(
        context,
        &request.cell_directory,
        &request.audit_output,
    )?;
    let lease =
        ResultRootLease::acquire_shared_for_cell_directory(context, &request.cell_directory)?;
    let started_at = parse_canonical_timestamp("audit.started_at", &request.started_at)?;
    let evaluated_at = parse_canonical_timestamp("audit.evaluated_at", &request.evaluated_at)?;
    if started_at > evaluated_at {
        return Err(ValidationError::new(
            "audit started_at must not be after evaluated_at",
        ));
    }
    let manifest_path = request
        .manifest
        .to_str()
        .ok_or_else(|| ValidationError::new("audit manifest path is not valid UTF-8"))?
        .to_owned();

    let candidate: Candidate = read_strict_json(
        context,
        &request.candidate_descriptor,
        MAX_CANDIDATE_JSON_BYTES,
    )?;
    let manifest_bytes =
        read_bounded_external_file(context, &request.manifest, MAX_MANIFEST_JSON_BYTES)?;
    let manifest_sha256 = sha256_bytes(&manifest_bytes);
    let manifest: ReleaseManifest = parse_strict_json(&manifest_bytes)?;
    validate_manifest_container(&manifest)?;

    let (original_cells, cell_inputs_sha256) =
        independently_read_cell_directory(context, &request.cell_directory)?;
    lease.validate_attached(context)?;
    let cell_directory = request
        .cell_directory
        .to_str()
        .ok_or_else(|| ValidationError::new("audit cell directory is not valid UTF-8"))?
        .to_owned();
    let mut manifest_raw_cells = manifest
        .cells
        .iter()
        .map(|cell| cell.raw.clone())
        .collect::<Vec<_>>();
    let mut sorted_original_cells = original_cells.clone();
    manifest_raw_cells.sort_by_key(Cell::matrix_key);
    sorted_original_cells.sort_by_key(Cell::matrix_key);
    let cell_inputs_match_manifest = sorted_original_cells == manifest_raw_cells;
    let original_recalculation = independently_recalculate(
        context,
        candidate.clone(),
        original_cells.clone(),
        &manifest.evaluated_at,
    )?;
    let current_recalculation = independently_recalculate(
        context,
        candidate.clone(),
        original_cells,
        &request.evaluated_at,
    )?;
    let candidate_sha256 = sha256_external_file(
        context,
        std::path::Path::new(&candidate.candidate_path),
        None,
    )?;

    let candidate_matches_manifest = manifest.candidate == candidate;
    let manifest_recalculation_exact = original_recalculation.manifest == manifest;
    let current_projection_agrees = same_projection(&current_recalculation.manifest, &manifest);
    let mut invariant_results = current_recalculation
        .invariant_results
        .iter()
        .map(|(invariant_id, passed)| AuditInvariantResult {
            invariant_id: invariant_id.clone(),
            passed: *passed,
        })
        .collect::<Vec<_>>();
    invariant_results.extend([
        audit_candidate_digest_invariant(&candidate_sha256, &candidate.binary_sha256),
        AuditInvariantResult {
            invariant_id: "candidate_descriptor_matches_manifest".to_owned(),
            passed: candidate_matches_manifest,
        },
        AuditInvariantResult {
            invariant_id: "cell_inputs_match_manifest".to_owned(),
            passed: cell_inputs_match_manifest,
        },
        AuditInvariantResult {
            invariant_id: "manifest_current_projection_agrees".to_owned(),
            passed: current_projection_agrees,
        },
        AuditInvariantResult {
            invariant_id: "manifest_recalculation_exact".to_owned(),
            passed: manifest_recalculation_exact,
        },
    ]);
    invariant_results.sort_by(|left, right| left.invariant_id.cmp(&right.invariant_id));

    let exclusions: Vec<AuditExclusion> = Vec::new();
    let (findings, audit_verdict) = decide_audit(
        &invariant_results,
        current_recalculation.manifest.verdict,
        &exclusions,
    );
    let recalculated_cells = current_recalculation
        .manifest
        .cells
        .iter()
        .map(RecalculatedCell::from)
        .collect();
    let audit = ReleaseAudit {
        schema: AUDIT_SCHEMA.to_owned(),
        manifest_path,
        manifest_sha256,
        cell_directory,
        cell_inputs_sha256,
        candidate_path: candidate.candidate_path,
        candidate_sha256,
        started_at: request.started_at.clone(),
        evaluated_at: request.evaluated_at.clone(),
        invariant_results,
        recalculated_cells,
        findings,
        exclusions,
        recalculated_verdict: current_recalculation.manifest.verdict,
        audit_verdict,
    };
    write_json_create_new(context, &request.audit_output, &audit, MAX_AUDIT_JSON_BYTES)?;
    lease.validate_attached(context)?;
    Ok(audit)
}

fn audit_candidate_digest_invariant(
    candidate_sha256: &str,
    descriptor_sha256: &str,
) -> AuditInvariantResult {
    AuditInvariantResult {
        invariant_id: "audit_candidate_binary_digest_exact".to_owned(),
        passed: candidate_sha256 == descriptor_sha256,
    }
}

fn decide_audit(
    invariant_results: &[AuditInvariantResult],
    recalculated_verdict: GateVerdict,
    exclusions: &[AuditExclusion],
) -> (Vec<String>, AuditVerdict) {
    let mut findings = invariant_results
        .iter()
        .filter(|result| !result.passed)
        .map(|result| format!("invariant_failed:{}", result.invariant_id))
        .collect::<Vec<_>>();
    if recalculated_verdict == GateVerdict::Fail {
        findings.push("recalculated_gate_verdict_fail".to_owned());
    }
    findings.sort();
    findings.dedup();
    let audit_verdict = if findings.is_empty()
        && exclusions.is_empty()
        && recalculated_verdict != GateVerdict::Fail
    {
        AuditVerdict::Pass
    } else {
        AuditVerdict::Fail
    };
    (findings, audit_verdict)
}

fn independently_read_cell_directory(
    context: &ValidationContext,
    cell_directory: &Path,
) -> ValidationResult<(Vec<Cell>, String)> {
    context.validate_existing_directory(cell_directory)?;
    let mut paths = fs::read_dir(cell_directory)?
        .map(|entry| {
            let path = entry.map_err(ValidationError::from)?.path();
            let exact = path.to_str().ok_or_else(|| {
                ValidationError::new(format!(
                    "audit cell path is not valid UTF-8: {}",
                    path.display()
                ))
            })?;
            Ok((exact.to_owned(), path))
        })
        .collect::<ValidationResult<Vec<_>>>()?;
    paths.sort_by(|left, right| left.0.as_bytes().cmp(right.0.as_bytes()));
    if paths.len() != 12 {
        return Err(ValidationError::new(
            "audit cell directory must contain exactly twelve JSON files",
        ));
    }

    let mut digest_preimage = Vec::from(CELL_INPUTS_DIGEST_DOMAIN);
    let mut cells = Vec::with_capacity(12);
    for (exact_path, path) in paths {
        if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
            return Err(ValidationError::new(format!(
                "audit cell directory entry must be a .json file: {}",
                path.display()
            )));
        }
        let bytes = read_bounded_external_file(context, &path, MAX_CELL_JSON_BYTES)?;
        let path_bytes = exact_path.as_bytes();
        let path_length = u64::try_from(path_bytes.len())
            .map_err(|_| ValidationError::new("audit cell path length overflow"))?;
        digest_preimage.extend_from_slice(&path_length.to_be_bytes());
        digest_preimage.extend_from_slice(path_bytes);
        digest_preimage.extend_from_slice(Sha256::digest(&bytes).as_slice());
        cells.push(parse_strict_json(&bytes)?);
    }
    Ok((cells, sha256_bytes(&digest_preimage)))
}

fn same_projection(current: &ReleaseManifest, recorded: &ReleaseManifest) -> bool {
    current.candidate == recorded.candidate
        && current.cells == recorded.cells
        && current.requested_verified_claims == recorded.requested_verified_claims
        && current.downgrades == recorded.downgrades
        && current.invariant_findings == recorded.invariant_findings
        && current.verdict == recorded.verdict
}

struct IndependentRecalculation {
    manifest: ReleaseManifest,
    invariant_results: Vec<(String, bool)>,
}

fn independently_recalculate(
    context: &ValidationContext,
    candidate: Candidate,
    cells: Vec<Cell>,
    evaluated_at: &str,
) -> ValidationResult<IndependentRecalculation> {
    let evaluated_at_value = parse_canonical_timestamp("audit.recalculated_at", evaluated_at)?;
    let candidate_recorded_at = validate_candidate_shape(&candidate)?;
    validate_matrix_shape(&cells)?;

    let all_cell_candidate_coordinates_exact = cells.iter().all(|cell| {
        cell.candidate_id == candidate.candidate_id
            && cell.binary_sha256 == candidate.binary_sha256
            && cell.source_revision == candidate.source_revision
            && cell.target_triple == candidate.target_triple
            && cell.release_profile == candidate.release_profile
    });
    let artifact = inspect_candidate_artifact(
        context,
        Path::new(&candidate.candidate_path),
        &candidate.binary_sha256,
    )?;
    let all_cell_environment_coordinates_exact = cells
        .iter()
        .all(|cell| independent_environment_coordinates_exact(cell, &artifact.build.build_id));
    let single_host_client_identity_per_host =
        independent_single_host_client_identity_per_host(&cells);
    let current_head = git_head(context.source_checkout())?;
    let source_clean = git_is_clean(context.source_checkout())?;
    let actual_archive_sha256 =
        git_archive_sha256(context.source_checkout(), &candidate.source_revision)?;
    let invariant_results = vec![
        (
            "candidate_binary_digest_exact".to_owned(),
            artifact.sha256_before == candidate.binary_sha256,
        ),
        (
            "candidate_binary_private_copy_exact".to_owned(),
            artifact.private_copy_sha256 == candidate.binary_sha256,
        ),
        (
            "candidate_binary_final_stable".to_owned(),
            artifact.sha256_after_held == artifact.sha256_before
                && artifact.sha256_after_path.as_deref() == Some(artifact.sha256_before.as_str())
                && artifact.path_identity_stable,
        ),
        (
            "candidate_build_git_exact".to_owned(),
            artifact.build.git_commit == candidate.source_revision,
        ),
        (
            "candidate_build_metadata_source_environment".to_owned(),
            artifact.build.metadata_source == "environment",
        ),
        (
            "candidate_build_package_version_exact".to_owned(),
            artifact.build.package_version == env!("CARGO_PKG_VERSION"),
        ),
        (
            "candidate_build_profile_exact".to_owned(),
            artifact.build.profile == candidate.release_profile
                && artifact.build.profile == "release"
                && artifact.build.profile_class == "release"
                && artifact.build.profile_exact == "true",
        ),
        (
            "candidate_build_target_exact".to_owned(),
            artifact.build.target == candidate.target_triple,
        ),
        (
            "candidate_build_tree_clean".to_owned(),
            artifact.build.tree == "clean",
        ),
        (
            "candidate_recorded_before_evaluation".to_owned(),
            candidate_recorded_at <= evaluated_at_value,
        ),
        (
            "all_cell_candidate_coordinates_exact".to_owned(),
            all_cell_candidate_coordinates_exact,
        ),
        (
            "all_cell_environment_coordinates_exact".to_owned(),
            all_cell_environment_coordinates_exact,
        ),
        ("fixed_twelve_cell_matrix".to_owned(), true),
        ("single_host_version_per_host".to_owned(), true),
        (
            "single_host_client_identity_per_host".to_owned(),
            single_host_client_identity_per_host,
        ),
        (
            "source_archive_digest_exact".to_owned(),
            actual_archive_sha256 == candidate.source_archive_sha256,
        ),
        ("source_checkout_clean".to_owned(), source_clean),
        (
            "source_revision_exact".to_owned(),
            current_head == candidate.source_revision,
        ),
    ];
    let mut invariant_findings = invariant_results
        .iter()
        .filter_map(|(id, passed)| (!passed).then_some(id.clone()))
        .collect::<Vec<_>>();
    invariant_findings.sort();

    let mut recalculated_cells = cells
        .into_iter()
        .map(|cell| {
            independently_recalculate_cell(
                context,
                &candidate,
                &artifact.build.build_id,
                candidate_recorded_at,
                evaluated_at_value,
                cell,
            )
        })
        .collect::<ValidationResult<Vec<_>>>()?;
    recalculated_cells.sort_by_key(|cell| cell.raw.key());
    let mut requested_verified_claims = recalculated_cells
        .iter()
        .filter(|cell| cell.raw.requested_verified)
        .map(|cell| cell.raw.key())
        .collect::<Vec<_>>();
    requested_verified_claims.sort();
    let mut downgrades = recalculated_cells
        .iter()
        .filter(|cell| {
            cell.raw.implementation_disposition == ImplementationDisposition::Implemented
                && (!cell.raw.requested_verified
                    || cell.derived_status != HostFeatureSupportStatus::Verified)
        })
        .map(|cell| cell.raw.key())
        .collect::<Vec<_>>();
    downgrades.sort();
    let requested_claim_failed = recalculated_cells.iter().any(|cell| {
        cell.raw.requested_verified && cell.derived_status != HostFeatureSupportStatus::Verified
    });
    let verdict = if !invariant_findings.is_empty() || requested_claim_failed {
        GateVerdict::Fail
    } else if downgrades.is_empty() {
        GateVerdict::Pass
    } else {
        GateVerdict::PassWithDowngrades
    };
    Ok(IndependentRecalculation {
        manifest: ReleaseManifest {
            schema: MANIFEST_SCHEMA.to_owned(),
            candidate,
            evaluated_at: evaluated_at.to_owned(),
            cells: recalculated_cells,
            requested_verified_claims,
            downgrades,
            invariant_findings,
            verdict,
        },
        invariant_results,
    })
}

fn independently_recalculate_cell(
    context: &ValidationContext,
    candidate: &Candidate,
    candidate_build_id: &str,
    candidate_recorded_at: DateTime<Utc>,
    evaluated_at: DateTime<Utc>,
    cell: Cell,
) -> ValidationResult<ManifestCell> {
    validate_cell_shape(&cell)?;
    let started_at = parse_canonical_timestamp("audit.cell.started_at", &cell.started_at)?;
    let recorded_at = parse_canonical_timestamp("audit.cell.recorded_at", &cell.recorded_at)?;
    let mut finding_codes = Vec::new();
    let candidate_coordinates_exact = cell.candidate_id == candidate.candidate_id
        && cell.binary_sha256 == candidate.binary_sha256
        && cell.source_revision == candidate.source_revision
        && cell.target_triple == candidate.target_triple
        && cell.release_profile == candidate.release_profile;
    if !candidate_coordinates_exact {
        finding_codes.push("candidate_coordinate_mismatch".to_owned());
    }
    let environment_coordinates_exact =
        independent_environment_coordinates_exact(&cell, candidate_build_id);
    if !environment_coordinates_exact {
        finding_codes.push("environment_coordinate_mismatch".to_owned());
    }
    let client_identity_exact = match (cell.client_name.as_ref(), cell.client_version.as_ref()) {
        (None, None) => {
            if cell.implementation_disposition == ImplementationDisposition::Implemented {
                finding_codes.push("client_identity_missing".to_owned());
            }
            false
        }
        (Some(client_name), Some(client_version)) => {
            let exact = independent_client_identity_coordinates_exact(&cell)
                && cell.host_version.as_ref() == Some(client_version)
                && (cell.host_kind != HostKind::Codex
                    || cell.host_version.as_ref().map(String::as_str)
                        != Some(REVIEWED_CODEX_HOST_VERSION)
                    || (client_name == REVIEWED_CODEX_MCP_CLIENT_NAME
                        && client_version == REVIEWED_CODEX_HOST_VERSION));
            if !exact {
                finding_codes.push("client_identity_mismatch".to_owned());
            }
            exact
        }
        _ => unreachable!("validated client identity group is all strings or all null"),
    };
    let timestamp_current = started_at <= recorded_at
        && recorded_at <= evaluated_at
        && evaluated_at
            < started_at
                .checked_add_signed(Duration::hours(24))
                .ok_or_else(|| ValidationError::new("audit freshness timestamp overflow"))?;
    if !timestamp_current {
        finding_codes.push("cell_timestamp_not_fresh".to_owned());
    }
    let candidate_precedes_cell = candidate_recorded_at <= started_at;
    if !candidate_precedes_cell {
        finding_codes.push("candidate_recorded_after_cell_start".to_owned());
    }
    let expected_implementation = host_feature_implementation_for_version(
        cell.host_kind.as_str(),
        cell.host_version.as_ref().map(String::as_str),
        cell.feature,
    );
    if expected_implementation != cli_implementation(cell.implementation_disposition) {
        return Err(ValidationError::new(format!(
            "{} implementation_disposition does not match the independent canonical audit",
            cell.matrix_key()
        )));
    }
    let evidence_exact = match cell.implementation_disposition {
        ImplementationDisposition::UnsupportedByHost => {
            if cell.run_state != RunState::NotApplicable
                || cell.evidence_artifact_path.as_ref().is_some()
                || cell.evidence_artifact_sha256.as_ref().is_some()
            {
                return Err(ValidationError::new(format!(
                    "{} unsupported audit cell has live evidence coordinates",
                    cell.matrix_key()
                )));
            }
            true
        }
        ImplementationDisposition::Implemented => {
            if cell.run_state == RunState::NotApplicable {
                return Err(ValidationError::new(format!(
                    "{} implemented audit cell is not_applicable",
                    cell.matrix_key()
                )));
            }
            let evidence_path = cell.evidence_artifact_path.as_ref().ok_or_else(|| {
                ValidationError::new("implemented audit cell omitted evidence_artifact_path")
            })?;
            let evidence_sha256 = cell.evidence_artifact_sha256.as_ref().ok_or_else(|| {
                ValidationError::new("implemented audit cell omitted evidence_artifact_sha256")
            })?;
            validate_sha256("audit.cell.evidence_artifact_sha256", evidence_sha256)?;
            let actual =
                sha256_external_file(context, Path::new(evidence_path), Some(MAX_EVIDENCE_BYTES))?;
            if actual == *evidence_sha256 {
                true
            } else {
                finding_codes.push("evidence_artifact_digest_mismatch".to_owned());
                false
            }
        }
    };
    let assertions_pass = cell.assertions.iter().all(|assertion| assertion.passed);
    for assertion in cell.assertions.iter().filter(|assertion| !assertion.passed) {
        finding_codes.push(format!("assertion_failed:{}", assertion.assertion_id));
        if let Some(codes) = &assertion.finding_codes {
            finding_codes.extend(codes.iter().cloned());
        }
    }
    if cell.run_state != RunState::Completed
        && cell.implementation_disposition == ImplementationDisposition::Implemented
    {
        finding_codes.push(format!("run_state:{}", cell.run_state.as_str()));
    }
    let current = cell.implementation_disposition == ImplementationDisposition::Implemented
        && cell.run_state == RunState::Completed
        && timestamp_current
        && candidate_precedes_cell
        && candidate_coordinates_exact
        && environment_coordinates_exact
        && client_identity_exact
        && evidence_exact
        && assertions_pass;
    let derived_status = evaluate_host_feature_support_for_version(
        cell.host_kind.as_str(),
        cell.host_version.as_ref().map(String::as_str),
        cell.feature,
        HostFeatureEvaluationInput::new(
            if current {
                ExactLiveEvidenceState::Current
            } else {
                ExactLiveEvidenceState::StaleOrMismatched
            },
            CurrentRuntimeReadiness::Ready,
        ),
    );
    if cell.claimed_status != derived_status {
        finding_codes.push("claimed_status_mismatch".to_owned());
    }
    finding_codes.sort();
    finding_codes.dedup();
    if finding_codes.len() > MAX_FINDING_CODES {
        return Err(ValidationError::new("derived cell findings exceed bound"));
    }
    Ok(ManifestCell {
        raw: cell,
        derived_status,
        finding_codes,
    })
}

fn independent_environment_coordinates_exact(cell: &Cell, candidate_build_id: &str) -> bool {
    let adapter_profile = independent_expected_adapter_profile(cell.feature).as_str();
    cell.environment.host_kind == cell.host_kind
        && cell.environment.host_version == cell.host_version
        && cell.environment.client_name == cell.client_name
        && cell.environment.client_version == cell.client_version
        && cell.adapter_profile == adapter_profile
        && cell.environment.adapter_profile == adapter_profile
        && cell.adapter_version == candidate_build_id
        && cell.environment.adapter_version == candidate_build_id
}

fn independent_client_identity_coordinates_exact(cell: &Cell) -> bool {
    cell.environment.client_name == cell.client_name
        && cell.environment.client_version == cell.client_version
}

fn independent_single_host_client_identity_per_host(cells: &[Cell]) -> bool {
    HostKind::ALL.into_iter().all(|host| {
        cells
            .iter()
            .filter(|cell| cell.host_kind == host)
            .filter_map(|cell| {
                cell.client_name
                    .as_ref()
                    .zip(cell.client_version.as_ref())
                    .map(|(name, version)| (name.as_str(), version.as_str()))
            })
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            <= 1
    })
}

const fn independent_expected_adapter_profile(feature: HostFeature) -> IntegrationProfile {
    match feature {
        HostFeature::RecordFinalOutput => IntegrationProfile::Record,
        HostFeature::NativeUserAction
        | HostFeature::LocalWebUserChannel
        | HostFeature::VerifiedToolProducer
        | HostFeature::RegisteredConnectionObservation
        | HostFeature::DetectiveFinalOutput => IntegrationProfile::Detective,
    }
}

#[cfg(test)]
mod decision_tests {
    use super::*;

    #[test]
    fn final_candidate_digest_mismatch_forces_failed_audit() {
        let invariant = audit_candidate_digest_invariant(&"1".repeat(64), &"2".repeat(64));
        assert!(!invariant.passed);

        let (findings, verdict) =
            decide_audit(std::slice::from_ref(&invariant), GateVerdict::Pass, &[]);
        assert_eq!(verdict, AuditVerdict::Fail);
        assert_eq!(
            findings,
            vec!["invariant_failed:audit_candidate_binary_digest_exact"]
        );
    }
}
