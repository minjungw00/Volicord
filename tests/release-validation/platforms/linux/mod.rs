use crate::{
    platforms::{PlatformCellDefinition, PlatformRunnerBoundary},
    scenarios::BASE_SCENARIOS,
};
use volicord_types::PlatformEnvironment;

pub fn definition() -> PlatformCellDefinition {
    PlatformCellDefinition {
        platform: PlatformEnvironment::Linux,
        runner_boundary: PlatformRunnerBoundary::NativeLinux,
        scenarios: BASE_SCENARIOS.to_vec(),
    }
}
