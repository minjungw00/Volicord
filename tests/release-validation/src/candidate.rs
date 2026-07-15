use std::{
    path::{Path, PathBuf},
    process::Command,
};

use crate::{
    error::{ValidationError, ValidationResult},
    evaluation::{
        candidate_artifact_invariant_results, candidate_source_invariant_results, canonical_now,
        validate_candidate_id, validate_candidate_shape,
    },
    io::{
        candidate_artifact_still_stable, git_archive_sha256, git_head, git_is_clean,
        inspect_candidate_artifact, sha256_external_file, write_json_create_new, ValidationContext,
        MAX_CANDIDATE_JSON_BYTES,
    },
    schema::{Candidate, CandidateBuildEnvironment, CANDIDATE_SCHEMA, SOURCE_ARCHIVE_ALGORITHM},
};

const MAX_PRODUCED_BUILD_ENVIRONMENT_COORDINATE_BYTES: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateRequest {
    pub candidate_id: String,
    pub candidate_path: PathBuf,
    pub candidate_output: PathBuf,
}

pub fn create_candidate(
    context: &ValidationContext,
    request: &CandidateRequest,
) -> ValidationResult<Candidate> {
    create_candidate_with_environment(context, request, measure_build_environment)
}

pub(crate) fn create_candidate_with_environment<F>(
    context: &ValidationContext,
    request: &CandidateRequest,
    measure_environment: F,
) -> ValidationResult<Candidate>
where
    F: FnOnce() -> ValidationResult<CandidateBuildEnvironment>,
{
    validate_candidate_id(&request.candidate_id)?;
    context.validate_existing_file(&request.candidate_path)?;
    context.validate_new_output(&request.candidate_output)?;
    let candidate_path = exact_utf8_path("candidate path", &request.candidate_path)?;

    let source_revision = git_head(context.source_checkout())?;
    if !git_is_clean(context.source_checkout())? {
        return Err(ValidationError::new(
            "source checkout must be clean before candidate creation",
        ));
    }
    let source_archive_sha256 = git_archive_sha256(context.source_checkout(), &source_revision)?;
    let binary_sha256 = sha256_external_file(context, &request.candidate_path, None)?;
    let artifact = inspect_candidate_artifact(context, &request.candidate_path, &binary_sha256)?;
    let build_environment = measure_environment()?;

    if !candidate_artifact_still_stable(
        context,
        &request.candidate_path,
        &binary_sha256,
        &artifact,
    )? {
        return Err(ValidationError::new(
            "candidate binary changed or its path was replaced after build-identity inspection",
        ));
    }
    let final_source_revision = git_head(context.source_checkout())?;
    let final_source_clean = git_is_clean(context.source_checkout())?;
    let final_source_archive_sha256 =
        git_archive_sha256(context.source_checkout(), &final_source_revision)?;

    let candidate = Candidate {
        schema: CANDIDATE_SCHEMA.to_owned(),
        candidate_id: request.candidate_id.clone(),
        candidate_path,
        source_revision,
        source_clean: true,
        source_archive_algorithm: SOURCE_ARCHIVE_ALGORITHM.to_owned(),
        source_archive_sha256,
        target_triple: artifact.build.target.clone(),
        release_profile: "release".to_owned(),
        binary_sha256,
        build_environment,
        recorded_at: canonical_now(),
    };
    validate_candidate_shape(&candidate)?;

    let failed_invariants = candidate_artifact_invariant_results(&candidate, &artifact)
        .into_iter()
        .chain(candidate_source_invariant_results(
            &candidate,
            &final_source_revision,
            final_source_clean,
            &final_source_archive_sha256,
        ))
        .filter_map(|(invariant, passed)| (!passed).then_some(invariant))
        .collect::<Vec<_>>();
    if !failed_invariants.is_empty() {
        return Err(ValidationError::new(format!(
            "release candidate creation failed invariants: {}",
            failed_invariants.join(", ")
        )));
    }

    write_json_create_new(
        context,
        &request.candidate_output,
        &candidate,
        MAX_CANDIDATE_JSON_BYTES,
    )?;
    Ok(candidate)
}

fn exact_utf8_path(label: &str, path: &Path) -> ValidationResult<String> {
    path.to_str().map(str::to_owned).ok_or_else(|| {
        ValidationError::new(format!(
            "{label} does not have an exact UTF-8 representation"
        ))
    })
}

fn measure_build_environment() -> ValidationResult<CandidateBuildEnvironment> {
    let (runner_os, runner_os_version, runner_arch) = measure_runner_environment()?;
    Ok(CandidateBuildEnvironment {
        runner_os,
        runner_os_version,
        runner_arch,
        git_version: command_coordinate("git_version", "git", &["--version"])?,
        rustc_version: command_coordinate("rustc_version", "rustc", &["--version"])?,
        cargo_version: command_coordinate("cargo_version", "cargo", &["--version"])?,
    })
}

#[cfg(unix)]
fn measure_runner_environment() -> ValidationResult<(String, String, String)> {
    Ok((
        command_coordinate("runner_os", "uname", &["-s"])?,
        command_coordinate("runner_os_version", "uname", &["-r"])?,
        command_coordinate("runner_arch", "uname", &["-m"])?,
    ))
}

#[cfg(windows)]
fn measure_runner_environment() -> ValidationResult<(String, String, String)> {
    Ok((
        std::env::consts::OS.to_owned(),
        command_coordinate("runner_os_version", "cmd", &["/C", "ver"])?,
        std::env::consts::ARCH.to_owned(),
    ))
}

#[cfg(not(any(unix, windows)))]
fn measure_runner_environment() -> ValidationResult<(String, String, String)> {
    Err(ValidationError::new(
        "candidate build-environment measurement is unsupported on this platform",
    ))
}

fn command_coordinate(label: &str, program: &str, args: &[&str]) -> ValidationResult<String> {
    let output = Command::new(program).args(args).output().map_err(|error| {
        ValidationError::new(format!("cannot measure {label} with {program}: {error}"))
    })?;
    if !output.status.success() {
        return Err(ValidationError::new(format!(
            "cannot measure {label}: {program} exited with {}",
            output.status
        )));
    }
    validate_environment_coordinate(label, &output.stdout)
}

fn validate_environment_coordinate(label: &str, stdout: &[u8]) -> ValidationResult<String> {
    let value = std::str::from_utf8(stdout)
        .map_err(|_| ValidationError::new(format!("measured {label} is not UTF-8")))?
        .trim()
        .to_owned();
    if value.len() > MAX_PRODUCED_BUILD_ENVIRONMENT_COORDINATE_BYTES {
        return Err(ValidationError::new(format!(
            "measured {label} exceeds the {MAX_PRODUCED_BUILD_ENVIRONMENT_COORDINATE_BYTES}-byte bound"
        )));
    }
    if value.is_empty() || value.chars().any(char::is_control) {
        return Err(ValidationError::new(format!(
            "measured {label} must be non-empty control-free UTF-8"
        )));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn environment_coordinate_accepts_512_bytes_and_rejects_513_or_control_text() {
        assert_eq!(
            validate_environment_coordinate("test", &vec![b'x'; 512])
                .expect("exact environment coordinate bound")
                .len(),
            512
        );
        assert!(validate_environment_coordinate("test", &vec![b'x'; 513]).is_err());
        assert!(validate_environment_coordinate("test", b"   ").is_err());
        assert!(validate_environment_coordinate("test", b"valid\ninvalid").is_err());
    }
}
