use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    error::{ValidationError, ValidationResult},
    evaluation::evaluate_release_matrix,
    io::{
        read_strict_json, write_json_create_new, ResultRootLease, ValidationContext,
        MAX_CANDIDATE_JSON_BYTES, MAX_CELL_JSON_BYTES, MAX_MANIFEST_JSON_BYTES,
    },
    schema::{Candidate, Cell, ReleaseManifest},
};

#[derive(Debug, Clone)]
pub struct GateRequest {
    pub candidate_descriptor: PathBuf,
    pub cell_directory: PathBuf,
    pub manifest_output: PathBuf,
    pub evaluated_at: String,
}

pub fn run_gate(
    context: &ValidationContext,
    request: &GateRequest,
) -> ValidationResult<ReleaseManifest> {
    ResultRootLease::prevalidate_summary_output(
        context,
        &request.cell_directory,
        &request.manifest_output,
    )?;
    let lease =
        ResultRootLease::acquire_shared_for_cell_directory(context, &request.cell_directory)?;
    let candidate: Candidate = read_strict_json(
        context,
        &request.candidate_descriptor,
        MAX_CANDIDATE_JSON_BYTES,
    )?;
    let cells = read_cell_directory(context, &request.cell_directory)?;
    lease.validate_attached(context)?;
    let evaluation = evaluate_release_matrix(context, candidate, cells, &request.evaluated_at)?;
    write_json_create_new(
        context,
        &request.manifest_output,
        &evaluation.manifest,
        MAX_MANIFEST_JSON_BYTES,
    )?;
    lease.validate_attached(context)?;
    Ok(evaluation.manifest)
}

fn read_cell_directory(
    context: &ValidationContext,
    cell_directory: &Path,
) -> ValidationResult<Vec<Cell>> {
    context.validate_existing_directory(cell_directory)?;
    let mut paths = fs::read_dir(cell_directory)?
        .map(|entry| {
            entry
                .map(|entry| entry.path())
                .map_err(ValidationError::from)
        })
        .collect::<ValidationResult<Vec<_>>>()?;
    paths.sort();
    if paths.len() != 12 {
        return Err(ValidationError::new(
            "cell directory must contain exactly twelve JSON files",
        ));
    }
    let mut cells = Vec::with_capacity(12);
    for path in paths {
        if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
            return Err(ValidationError::new(format!(
                "cell directory entry must be a .json file: {}",
                path.display()
            )));
        }
        cells.push(read_strict_json(context, &path, MAX_CELL_JSON_BYTES)?);
    }
    Ok(cells)
}
