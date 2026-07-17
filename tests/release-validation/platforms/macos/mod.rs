use crate::{
    platforms::{PlatformCellDefinition, PlatformRunnerBoundary},
    scenarios::BASE_SCENARIOS,
};
use volicord_types::PlatformEnvironment;

pub fn definition() -> PlatformCellDefinition {
    PlatformCellDefinition {
        platform: PlatformEnvironment::Macos,
        runner_boundary: PlatformRunnerBoundary::NativeMacos,
        scenarios: BASE_SCENARIOS.to_vec(),
    }
}
