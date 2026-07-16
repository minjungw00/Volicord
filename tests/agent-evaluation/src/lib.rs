mod catalog;
mod model;
mod runner;

use std::{error::Error, fmt};

pub use catalog::{
    embedded_catalog_text, fixture_catalog_digest, load_embedded_catalog, repository_seed_digest,
    validate_catalog, validate_relative_path,
};
pub use model::*;
pub use runner::{
    build_schedule, evaluate_live_criteria, fixture_evaluation, live_config_example_path,
    load_live_config, materialize_repository, pending_criteria, run_live, run_live_with_driver,
    validate_live_config, validate_schedule_matrix, write_result_create_new, CommandDriver,
    DriverFailure, TrialDriver, MAX_REPETITIONS,
};

pub type HarnessResult<T> = Result<T, HarnessError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HarnessError {
    message: String,
}

impl HarnessError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for HarnessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for HarnessError {}

pub fn result_schema_text() -> &'static str {
    include_str!("../schemas/result.schema.json")
}
