use crate::{
    platforms::{PlatformCellDefinition, PlatformRunnerBoundary},
    scenarios::BASE_SCENARIOS,
};
use volicord_types::{IntegrationProfile, PlatformEnvironment, ReleaseTargetTriple};

pub fn definition(target_triple: ReleaseTargetTriple) -> PlatformCellDefinition {
    assert!(matches!(
        target_triple,
        ReleaseTargetTriple::Aarch64AppleDarwin | ReleaseTargetTriple::X86_64AppleDarwin
    ));
    PlatformCellDefinition {
        target_triple,
        platform: PlatformEnvironment::Macos,
        integration_profile: IntegrationProfile::Record,
        runner_boundary: PlatformRunnerBoundary::NativeMacos,
        scenarios: BASE_SCENARIOS.to_vec(),
    }
}
