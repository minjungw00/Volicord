use crate::{
    platforms::{PlatformCellDefinition, PlatformRunnerBoundary},
    scenarios::BASE_SCENARIOS,
    schema::PlatformEnvironment,
};

pub fn definition() -> PlatformCellDefinition {
    PlatformCellDefinition {
        platform: PlatformEnvironment::NativeWindows,
        runner_boundary: PlatformRunnerBoundary::NativeWindows,
        scenarios: BASE_SCENARIOS.to_vec(),
    }
}
