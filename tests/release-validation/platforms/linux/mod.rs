use crate::{
    platforms::{PlatformCellDefinition, PlatformRunnerBoundary},
    scenarios::BASE_SCENARIOS,
};
use volicord_types::{IntegrationProfile, PlatformEnvironment, ReleaseTargetTriple};

pub fn definition(target_triple: ReleaseTargetTriple) -> PlatformCellDefinition {
    assert!(matches!(
        target_triple,
        ReleaseTargetTriple::X86_64UnknownLinuxGnu | ReleaseTargetTriple::Aarch64UnknownLinuxGnu
    ));
    PlatformCellDefinition {
        target_triple,
        platform: PlatformEnvironment::Linux,
        integration_profile: IntegrationProfile::Record,
        runner_boundary: PlatformRunnerBoundary::NativeLinux,
        scenarios: BASE_SCENARIOS.to_vec(),
    }
}
