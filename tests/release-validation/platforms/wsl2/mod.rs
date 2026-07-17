use crate::{
    platforms::{PlatformCellDefinition, PlatformRunnerBoundary},
    scenarios::{scenarios_for_wsl2, ScenarioExpectation},
    schema::{CodexReleaseScenarioId, PlatformEnvironment},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WslGeneration {
    Wsl1,
    Wsl2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentEnvironment {
    Wsl2Distribution,
    NativeWindows,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilesystemBoundary {
    DistributionExt4,
    Drvfs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Wsl2TopologyScenario {
    pub scenario_id: CodexReleaseScenarioId,
    pub generation: WslGeneration,
    pub codex: ComponentEnvironment,
    pub volicord: ComponentEnvironment,
    pub product_repository: FilesystemBoundary,
    pub runtime_home: FilesystemBoundary,
    pub receipt_origin: ComponentEnvironment,
    pub expectation: ScenarioExpectation,
}

pub const TOPOLOGY_SCENARIOS: [Wsl2TopologyScenario; 5] = [
    Wsl2TopologyScenario {
        scenario_id: CodexReleaseScenarioId::Wsl2Ext4Project,
        generation: WslGeneration::Wsl2,
        codex: ComponentEnvironment::Wsl2Distribution,
        volicord: ComponentEnvironment::Wsl2Distribution,
        product_repository: FilesystemBoundary::DistributionExt4,
        runtime_home: FilesystemBoundary::DistributionExt4,
        receipt_origin: ComponentEnvironment::Wsl2Distribution,
        expectation: ScenarioExpectation::AcceptWsl2Ext4,
    },
    Wsl2TopologyScenario {
        scenario_id: CodexReleaseScenarioId::Wsl2DrvfsRejection,
        generation: WslGeneration::Wsl2,
        codex: ComponentEnvironment::Wsl2Distribution,
        volicord: ComponentEnvironment::Wsl2Distribution,
        product_repository: FilesystemBoundary::Drvfs,
        runtime_home: FilesystemBoundary::DistributionExt4,
        receipt_origin: ComponentEnvironment::Wsl2Distribution,
        expectation: ScenarioExpectation::RejectWsl2Drvfs,
    },
    Wsl2TopologyScenario {
        scenario_id: CodexReleaseScenarioId::Wsl2CrossTopologyRejection,
        generation: WslGeneration::Wsl2,
        codex: ComponentEnvironment::NativeWindows,
        volicord: ComponentEnvironment::Wsl2Distribution,
        product_repository: FilesystemBoundary::DistributionExt4,
        runtime_home: FilesystemBoundary::DistributionExt4,
        receipt_origin: ComponentEnvironment::Wsl2Distribution,
        expectation: ScenarioExpectation::RejectWsl2CrossTopology,
    },
    Wsl2TopologyScenario {
        scenario_id: CodexReleaseScenarioId::Wsl1Rejection,
        generation: WslGeneration::Wsl1,
        codex: ComponentEnvironment::Wsl2Distribution,
        volicord: ComponentEnvironment::Wsl2Distribution,
        product_repository: FilesystemBoundary::DistributionExt4,
        runtime_home: FilesystemBoundary::DistributionExt4,
        receipt_origin: ComponentEnvironment::Wsl2Distribution,
        expectation: ScenarioExpectation::RejectWsl1,
    },
    Wsl2TopologyScenario {
        scenario_id: CodexReleaseScenarioId::Wsl2NativeWindowsReceiptReuseRejection,
        generation: WslGeneration::Wsl2,
        codex: ComponentEnvironment::Wsl2Distribution,
        volicord: ComponentEnvironment::Wsl2Distribution,
        product_repository: FilesystemBoundary::DistributionExt4,
        runtime_home: FilesystemBoundary::DistributionExt4,
        receipt_origin: ComponentEnvironment::NativeWindows,
        expectation: ScenarioExpectation::RejectNativeWindowsReceiptReuse,
    },
];

pub fn definition() -> PlatformCellDefinition {
    PlatformCellDefinition {
        platform: PlatformEnvironment::Wsl2,
        runner_boundary: PlatformRunnerBoundary::PinnedUbuntuLtsWsl2,
        scenarios: scenarios_for_wsl2(),
    }
}
