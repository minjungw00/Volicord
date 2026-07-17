pub mod linux;
pub mod macos;
pub mod windows;
pub mod wsl2;

use std::path::Path;

use volicord_types::{
    CodexReleaseScenarioId, IntegrationProfile, PlatformEnvironment, ReleaseTargetTriple,
};

use crate::contracts::{load_release_target_contract, ReleaseCell};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformRunnerBoundary {
    NativeLinux,
    NativeMacos,
    NativeWindows,
    PinnedUbuntuLtsWsl2,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformCellDefinition {
    pub target_triple: ReleaseTargetTriple,
    pub platform: PlatformEnvironment,
    pub integration_profile: IntegrationProfile,
    pub runner_boundary: PlatformRunnerBoundary,
    pub scenarios: Vec<CodexReleaseScenarioId>,
}

pub fn all() -> Vec<PlatformCellDefinition> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("contracts/release-targets.json");
    let contract = load_release_target_contract(&path)
        .expect("checked-in release target contract must remain valid");
    contract
        .required_cells()
        .iter()
        .copied()
        .map(definition_for)
        .collect()
}

fn definition_for(cell: ReleaseCell) -> PlatformCellDefinition {
    match cell.platform_environment {
        PlatformEnvironment::Linux => linux::definition(cell.target_triple),
        PlatformEnvironment::Macos => macos::definition(cell.target_triple),
        PlatformEnvironment::NativeWindows => windows::definition(cell.target_triple),
        PlatformEnvironment::Wsl2 => wsl2::definition(cell.target_triple),
    }
}
