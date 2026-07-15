use std::{collections::BTreeSet, path::Path, time::SystemTime};

use chrono::{DateTime, Duration, SecondsFormat, Timelike, Utc};
use volicord_types::{
    canonical_codex_host_version, evaluate_host_feature_support_for_version,
    host_feature_implementation_for_version, validate_managed_mcp_client_info_field,
    CurrentRuntimeReadiness, ExactLiveEvidenceState, HostFeature, HostFeatureEvaluationInput,
    HostFeatureImplementation, HostFeatureSupportStatus, IntegrationProfile,
    ManagedMcpClientInfoField, REVIEWED_CODEX_HOST_VERSION, REVIEWED_CODEX_MCP_CLIENT_NAME,
};

use crate::{
    error::{ValidationError, ValidationResult},
    io::{
        git_archive_sha256, git_head, git_is_clean, inspect_candidate_artifact,
        sha256_external_file, ValidationContext, MAX_EVIDENCE_BYTES,
    },
    schema::{
        expected_assertion_ids, Candidate, Cell, GateVerdict, HostKind, ImplementationDisposition,
        ManifestCell, ReleaseManifest, RunState, CANDIDATE_SCHEMA, CELL_SCHEMA, MANIFEST_SCHEMA,
        MAX_FINDING_CODES, SOURCE_ARCHIVE_ALGORITHM,
    },
};

const MAX_TEXT_BYTES: usize = 512;
const MAX_OPAQUE_ID_BYTES: usize = 256;

#[derive(Debug, Clone)]
pub struct EvaluationResult {
    pub manifest: ReleaseManifest,
    pub invariant_results: Vec<(String, bool)>,
}

pub fn canonical_now() -> String {
    DateTime::<Utc>::from(SystemTime::now())
        .with_nanosecond(0)
        .expect("zero nanoseconds is valid")
        .to_rfc3339_opts(SecondsFormat::Secs, true)
}

pub fn parse_canonical_timestamp(field: &str, text: &str) -> ValidationResult<DateTime<Utc>> {
    let parsed = DateTime::parse_from_rfc3339(text)
        .map_err(|_| ValidationError::new(format!("{field} must be canonical UTC RFC 3339")))?
        .with_timezone(&Utc);
    if parsed.to_rfc3339_opts(SecondsFormat::Secs, true) != text {
        return Err(ValidationError::new(format!(
            "{field} must use canonical UTC second precision"
        )));
    }
    Ok(parsed)
}

pub fn evaluate_release_matrix(
    context: &ValidationContext,
    candidate: Candidate,
    cells: Vec<Cell>,
    evaluated_at: &str,
) -> ValidationResult<EvaluationResult> {
    let evaluated_at_value = parse_canonical_timestamp("evaluated_at", evaluated_at)?;
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
        .all(|cell| environment_coordinates_exact(cell, &artifact.build.build_id));
    let single_host_client_identity_per_host = single_host_client_identity_per_host(&cells);
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

    let mut evaluated_cells = cells
        .into_iter()
        .map(|cell| {
            evaluate_cell(
                context,
                &candidate,
                &artifact.build.build_id,
                candidate_recorded_at,
                evaluated_at_value,
                cell,
            )
        })
        .collect::<ValidationResult<Vec<_>>>()?;
    evaluated_cells.sort_by_key(|cell| cell.raw.key());

    let mut requested_verified_claims = evaluated_cells
        .iter()
        .filter(|cell| cell.raw.requested_verified)
        .map(|cell| cell.raw.key())
        .collect::<Vec<_>>();
    requested_verified_claims.sort();

    let mut downgrades = evaluated_cells
        .iter()
        .filter(|cell| {
            cell.raw.implementation_disposition == ImplementationDisposition::Implemented
                && (!cell.raw.requested_verified
                    || cell.derived_status != HostFeatureSupportStatus::Verified)
        })
        .map(|cell| cell.raw.key())
        .collect::<Vec<_>>();
    downgrades.sort();

    let requested_claim_failed = evaluated_cells.iter().any(|cell| {
        cell.raw.requested_verified && cell.derived_status != HostFeatureSupportStatus::Verified
    });
    let verdict = if !invariant_findings.is_empty() || requested_claim_failed {
        GateVerdict::Fail
    } else if !downgrades.is_empty() {
        GateVerdict::PassWithDowngrades
    } else {
        GateVerdict::Pass
    };

    Ok(EvaluationResult {
        manifest: ReleaseManifest {
            schema: MANIFEST_SCHEMA.to_owned(),
            candidate,
            evaluated_at: evaluated_at.to_owned(),
            cells: evaluated_cells,
            requested_verified_claims,
            downgrades,
            invariant_findings,
            verdict,
        },
        invariant_results,
    })
}

pub fn validate_manifest_container(manifest: &ReleaseManifest) -> ValidationResult<()> {
    if manifest.schema != MANIFEST_SCHEMA {
        return Err(ValidationError::new("manifest schema identifier mismatch"));
    }
    parse_canonical_timestamp("manifest.evaluated_at", &manifest.evaluated_at)?;
    if manifest.cells.len() != 12 {
        return Err(ValidationError::new(
            "manifest must contain exactly twelve cells",
        ));
    }
    let cell_keys = manifest
        .cells
        .iter()
        .map(|cell| cell.raw.key())
        .collect::<Vec<_>>();
    require_sorted_unique("manifest.cells", &cell_keys, 12)?;
    require_sorted_unique(
        "manifest.requested_verified_claims",
        &manifest.requested_verified_claims,
        12,
    )?;
    require_sorted_unique("manifest.downgrades", &manifest.downgrades, 12)?;
    require_sorted_unique(
        "manifest.invariant_findings",
        &manifest.invariant_findings,
        MAX_FINDING_CODES,
    )?;
    for cell in &manifest.cells {
        require_sorted_unique(
            "manifest.cells[].finding_codes",
            &cell.finding_codes,
            MAX_FINDING_CODES,
        )?;
    }
    Ok(())
}

pub(crate) fn validate_candidate_shape(candidate: &Candidate) -> ValidationResult<DateTime<Utc>> {
    if candidate.schema != CANDIDATE_SCHEMA {
        return Err(ValidationError::new("candidate schema identifier mismatch"));
    }
    if !candidate.source_clean {
        return Err(ValidationError::new("candidate source_clean must be true"));
    }
    if candidate.source_archive_algorithm != SOURCE_ARCHIVE_ALGORITHM {
        return Err(ValidationError::new(
            "candidate source_archive_algorithm mismatch",
        ));
    }
    if candidate.release_profile != "release" {
        return Err(ValidationError::new(
            "candidate release_profile must be the exact maintained release profile",
        ));
    }
    validate_bounded_text(
        "candidate.candidate_id",
        &candidate.candidate_id,
        MAX_OPAQUE_ID_BYTES,
    )?;
    validate_revision(&candidate.source_revision)?;
    validate_sha256(
        "candidate.source_archive_sha256",
        &candidate.source_archive_sha256,
    )?;
    validate_sha256("candidate.binary_sha256", &candidate.binary_sha256)?;
    for (field, value) in [
        ("candidate.target_triple", &candidate.target_triple),
        ("candidate.release_profile", &candidate.release_profile),
        (
            "candidate.build_environment.runner_os",
            &candidate.build_environment.runner_os,
        ),
        (
            "candidate.build_environment.runner_os_version",
            &candidate.build_environment.runner_os_version,
        ),
        (
            "candidate.build_environment.runner_arch",
            &candidate.build_environment.runner_arch,
        ),
        (
            "candidate.build_environment.git_version",
            &candidate.build_environment.git_version,
        ),
        (
            "candidate.build_environment.rustc_version",
            &candidate.build_environment.rustc_version,
        ),
        (
            "candidate.build_environment.cargo_version",
            &candidate.build_environment.cargo_version,
        ),
    ] {
        validate_bounded_text(field, value, MAX_TEXT_BYTES)?;
    }
    let path = Path::new(&candidate.candidate_path);
    if candidate.candidate_path.len() > 4096 {
        return Err(ValidationError::new("candidate_path exceeds path bound"));
    }
    if path.as_os_str().is_empty() {
        return Err(ValidationError::new("candidate_path must not be empty"));
    }
    parse_canonical_timestamp("candidate.recorded_at", &candidate.recorded_at)
}

pub(crate) fn validate_matrix_shape(cells: &[Cell]) -> ValidationResult<()> {
    if cells.len() != HostKind::ALL.len() * HostFeature::ALL.len() {
        return Err(ValidationError::new(
            "cell input must contain exactly the fixed twelve-cell matrix",
        ));
    }
    let actual = cells.iter().map(Cell::matrix_key).collect::<BTreeSet<_>>();
    let expected = HostKind::ALL
        .into_iter()
        .flat_map(|host| {
            HostFeature::ALL
                .into_iter()
                .map(move |feature| format!("{}/{}", host.as_str(), feature.as_str()))
        })
        .collect::<BTreeSet<_>>();
    if actual != expected {
        return Err(ValidationError::new(
            "cell input has a duplicate, missing, or additional matrix coordinate",
        ));
    }
    for cell in cells {
        validate_cell_shape(cell)?;
    }
    for host in HostKind::ALL {
        let availability_coordinates = cells
            .iter()
            .filter(|cell| cell.host_kind == host)
            .map(|cell| {
                (
                    cell.host_version.as_ref().map(String::as_str),
                    cell.environment.host_version.as_ref().map(String::as_str),
                    cell.environment
                        .host_executable_sha256
                        .as_ref()
                        .map(String::as_str),
                )
            })
            .collect::<BTreeSet<_>>();
        if availability_coordinates.len() != 1 {
            return Err(ValidationError::new(format!(
                "all {} cells must use one exact host availability coordinate",
                host.as_str()
            )));
        }
    }
    Ok(())
}

fn single_host_client_identity_per_host(cells: &[Cell]) -> bool {
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
            .collect::<BTreeSet<_>>()
            .len()
            <= 1
    })
}

fn evaluate_cell(
    context: &ValidationContext,
    candidate: &Candidate,
    candidate_build_id: &str,
    candidate_recorded_at: DateTime<Utc>,
    evaluated_at: DateTime<Utc>,
    cell: Cell,
) -> ValidationResult<ManifestCell> {
    validate_cell_shape(&cell)?;
    let started_at = parse_canonical_timestamp("cell.started_at", &cell.started_at)?;
    let recorded_at = parse_canonical_timestamp("cell.recorded_at", &cell.recorded_at)?;
    let mut finding_codes = Vec::new();

    let candidate_coordinates_exact = cell.candidate_id == candidate.candidate_id
        && cell.binary_sha256 == candidate.binary_sha256
        && cell.source_revision == candidate.source_revision
        && cell.target_triple == candidate.target_triple
        && cell.release_profile == candidate.release_profile;
    if !candidate_coordinates_exact {
        finding_codes.push("candidate_coordinate_mismatch".to_owned());
    }

    let environment_coordinates_exact = environment_coordinates_exact(&cell, candidate_build_id);
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
            let exact = client_identity_coordinates_exact(&cell)
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
                .ok_or_else(|| ValidationError::new("cell freshness timestamp overflow"))?;
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
    let declared_implementation = cli_implementation(cell.implementation_disposition);
    if expected_implementation != declared_implementation {
        return Err(ValidationError::new(format!(
            "{} implementation_disposition does not match the canonical adapter evaluator",
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
                    "{} unsupported cell must be not_applicable with null evidence coordinates",
                    cell.matrix_key()
                )));
            }
            true
        }
        ImplementationDisposition::Implemented => {
            if cell.run_state == RunState::NotApplicable {
                return Err(ValidationError::new(format!(
                    "{} implemented cell cannot use not_applicable",
                    cell.matrix_key()
                )));
            }
            let path = cell.evidence_artifact_path.as_ref().ok_or_else(|| {
                ValidationError::new(format!(
                    "{} implemented cell requires evidence_artifact_path",
                    cell.matrix_key()
                ))
            })?;
            let expected_digest = cell.evidence_artifact_sha256.as_ref().ok_or_else(|| {
                ValidationError::new(format!(
                    "{} implemented cell requires evidence_artifact_sha256",
                    cell.matrix_key()
                ))
            })?;
            validate_sha256("cell.evidence_artifact_sha256", expected_digest)?;
            let actual_digest =
                sha256_external_file(context, Path::new(path), Some(MAX_EVIDENCE_BYTES))?;
            if actual_digest != *expected_digest {
                finding_codes.push("evidence_artifact_digest_mismatch".to_owned());
                false
            } else {
                true
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

    let exact_live_evidence = cell.implementation_disposition
        == ImplementationDisposition::Implemented
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
            if exact_live_evidence {
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

fn environment_coordinates_exact(cell: &Cell, candidate_build_id: &str) -> bool {
    let adapter_profile = expected_adapter_profile(cell.feature).as_str();
    cell.environment.host_kind == cell.host_kind
        && cell.environment.host_version == cell.host_version
        && cell.environment.client_name == cell.client_name
        && cell.environment.client_version == cell.client_version
        && cell.adapter_profile == adapter_profile
        && cell.environment.adapter_profile == adapter_profile
        && cell.adapter_version == candidate_build_id
        && cell.environment.adapter_version == candidate_build_id
}

fn client_identity_coordinates_exact(cell: &Cell) -> bool {
    cell.environment.client_name == cell.client_name
        && cell.environment.client_version == cell.client_version
}

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

pub(crate) fn validate_cell_shape(cell: &Cell) -> ValidationResult<()> {
    if cell.schema != CELL_SCHEMA {
        return Err(ValidationError::new("cell schema identifier mismatch"));
    }
    validate_bounded_text("cell.candidate_id", &cell.candidate_id, MAX_OPAQUE_ID_BYTES)?;
    validate_sha256("cell.binary_sha256", &cell.binary_sha256)?;
    validate_revision(&cell.source_revision)?;
    for (field, value) in [
        ("cell.target_triple", &cell.target_triple),
        ("cell.release_profile", &cell.release_profile),
        ("cell.adapter_profile", &cell.adapter_profile),
        ("cell.adapter_version", &cell.adapter_version),
        ("cell.environment.runner_os", &cell.environment.runner_os),
        (
            "cell.environment.runner_os_version",
            &cell.environment.runner_os_version,
        ),
        (
            "cell.environment.runner_arch",
            &cell.environment.runner_arch,
        ),
        (
            "cell.environment.adapter_profile",
            &cell.environment.adapter_profile,
        ),
        (
            "cell.environment.adapter_version",
            &cell.environment.adapter_version,
        ),
    ] {
        validate_bounded_text(field, value, MAX_TEXT_BYTES)?;
    }
    let host_available = match (
        cell.host_version.as_ref(),
        cell.environment.host_version.as_ref(),
        cell.environment.host_executable_sha256.as_ref(),
    ) {
        (Some(host_version), Some(environment_host_version), Some(host_sha256)) => {
            validate_bounded_text("cell.host_version", host_version, MAX_TEXT_BYTES)?;
            validate_bounded_text(
                "cell.environment.host_version",
                environment_host_version,
                MAX_TEXT_BYTES,
            )?;
            validate_sha256("cell.environment.host_executable_sha256", host_sha256)?;
            true
        }
        (None, None, None) => {
            let required_run_state = match cell.implementation_disposition {
                ImplementationDisposition::Implemented => RunState::Ignored,
                ImplementationDisposition::UnsupportedByHost => RunState::NotApplicable,
            };
            if cell.run_state != required_run_state {
                return Err(ValidationError::new(format!(
                    "a null host availability coordinate requires run_state={}",
                    required_run_state.as_str()
                )));
            }
            if cell.implementation_disposition == ImplementationDisposition::Implemented
                && cell.assertions.iter().any(|assertion| assertion.passed)
            {
                return Err(ValidationError::new(
                    "an unavailable implemented cell requires every assertion to fail",
                ));
            }
            false
        }
        _ => {
            return Err(ValidationError::new(
                "host_version, environment.host_version, and environment.host_executable_sha256 must be all strings or all null",
            ))
        }
    };
    for (field, version) in [
        ("cell.host_version", cell.host_version.as_ref()),
        (
            "cell.environment.host_version",
            cell.environment.host_version.as_ref(),
        ),
    ] {
        if cell.host_kind == HostKind::Codex
            && version.is_some_and(|value| canonical_codex_host_version(value).is_none())
        {
            return Err(ValidationError::new(format!(
                "{field} must be a canonical bare Codex version"
            )));
        }
    }
    match (
        cell.client_name.as_ref(),
        cell.client_version.as_ref(),
        cell.environment.client_name.as_ref(),
        cell.environment.client_version.as_ref(),
    ) {
        (
            Some(client_name),
            Some(client_version),
            Some(environment_client_name),
            Some(environment_client_version),
        ) => {
            if !host_available {
                return Err(ValidationError::new(
                    "non-null client identity requires non-null host availability",
                ));
            }
            for (field, kind, value) in [
                (
                    "cell.client_name",
                    ManagedMcpClientInfoField::Name,
                    client_name,
                ),
                (
                    "cell.client_version",
                    ManagedMcpClientInfoField::Version,
                    client_version,
                ),
                (
                    "cell.environment.client_name",
                    ManagedMcpClientInfoField::Name,
                    environment_client_name,
                ),
                (
                    "cell.environment.client_version",
                    ManagedMcpClientInfoField::Version,
                    environment_client_version,
                ),
            ] {
                validate_managed_mcp_client_info_field(kind, value).map_err(|error| {
                    ValidationError::new(format!("{field} is invalid: {error}"))
                })?;
            }
        }
        (None, None, None, None) => {}
        _ => {
            return Err(ValidationError::new(
                "client_name, client_version, environment.client_name, and environment.client_version must be all strings or all null",
            ))
        }
    }
    if cell.implementation_disposition == ImplementationDisposition::UnsupportedByHost
        && cell.requested_verified
    {
        return Err(ValidationError::new(
            "unsupported_by_host cells cannot request verified",
        ));
    }
    parse_canonical_timestamp("cell.started_at", &cell.started_at)?;
    parse_canonical_timestamp("cell.recorded_at", &cell.recorded_at)?;

    let expected = expected_assertion_ids(cell.implementation_disposition, cell.feature);
    let actual = cell
        .assertions
        .iter()
        .map(|assertion| assertion.assertion_id.as_str())
        .collect::<Vec<_>>();
    if actual != expected {
        return Err(ValidationError::new(format!(
            "{} assertions must be the exact sorted required set",
            cell.matrix_key()
        )));
    }
    for assertion in &cell.assertions {
        validate_stable_code("cell.assertions[].assertion_id", &assertion.assertion_id)?;
        if let Some(codes) = &assertion.finding_codes {
            if codes.is_empty() {
                return Err(ValidationError::new(
                    "present assertion finding_codes must not be empty",
                ));
            }
            require_sorted_unique("cell.assertions[].finding_codes", codes, MAX_FINDING_CODES)?;
            for code in codes {
                validate_stable_code("cell.assertions[].finding_codes[]", code)?;
            }
        }
    }
    if cell.implementation_disposition == ImplementationDisposition::UnsupportedByHost
        && !cell.assertions[0].passed
    {
        return Err(ValidationError::new(
            "static_unsupported_by_host assertion must pass",
        ));
    }
    Ok(())
}

pub(crate) fn cli_implementation(
    disposition: ImplementationDisposition,
) -> HostFeatureImplementation {
    match disposition {
        ImplementationDisposition::Implemented => HostFeatureImplementation::Implemented,
        ImplementationDisposition::UnsupportedByHost => {
            HostFeatureImplementation::UnsupportedByHost
        }
    }
}

fn validate_revision(value: &str) -> ValidationResult<()> {
    if !matches!(value.len(), 40 | 64)
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(ValidationError::new(
            "source_revision must be lowercase 40- or 64-hex",
        ));
    }
    Ok(())
}

pub(crate) fn validate_sha256(field: &str, value: &str) -> ValidationResult<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(ValidationError::new(format!(
            "{field} must be lowercase 64-hex"
        )));
    }
    Ok(())
}

fn validate_bounded_text(field: &str, value: &str, max_bytes: usize) -> ValidationResult<()> {
    if value.is_empty() || value.len() > max_bytes || value.chars().any(char::is_control) {
        return Err(ValidationError::new(format!(
            "{field} must be non-empty, bounded, control-free UTF-8"
        )));
    }
    Ok(())
}

fn validate_stable_code(field: &str, value: &str) -> ValidationResult<()> {
    if value.is_empty()
        || value.len() > 128
        || !value.bytes().enumerate().all(|(index, byte)| {
            if index == 0 {
                byte.is_ascii_lowercase() || byte.is_ascii_digit()
            } else {
                byte.is_ascii_lowercase()
                    || byte.is_ascii_digit()
                    || matches!(byte, b'.' | b'_' | b':' | b'-')
            }
        })
    {
        return Err(ValidationError::new(format!(
            "{field} is not a stable bounded finding/assertion code"
        )));
    }
    Ok(())
}

fn require_sorted_unique<T: Ord>(
    field: &str,
    values: &[T],
    max_len: usize,
) -> ValidationResult<()> {
    if values.len() > max_len {
        return Err(ValidationError::new(format!("{field} exceeds bound")));
    }
    if values.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(ValidationError::new(format!(
            "{field} must be bytewise sorted and duplicate-free"
        )));
    }
    Ok(())
}
