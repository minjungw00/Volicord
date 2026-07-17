use crate::{
    platforms::{PlatformCellDefinition, PlatformRunnerBoundary},
    scenarios::BASE_SCENARIOS,
};
use volicord_types::{IntegrationProfile, PlatformEnvironment, ReleaseTargetTriple};

pub fn definition(target_triple: ReleaseTargetTriple) -> PlatformCellDefinition {
    assert_eq!(target_triple, ReleaseTargetTriple::X86_64PcWindowsMsvc);
    PlatformCellDefinition {
        target_triple,
        platform: PlatformEnvironment::NativeWindows,
        integration_profile: IntegrationProfile::Record,
        runner_boundary: PlatformRunnerBoundary::NativeWindows,
        scenarios: BASE_SCENARIOS.to_vec(),
    }
}
