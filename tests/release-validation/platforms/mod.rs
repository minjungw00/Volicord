pub mod linux;
pub mod macos;
pub mod windows;
pub mod wsl2;

use volicord_types::{CodexReleaseScenarioId, PlatformEnvironment};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformRunnerBoundary {
    NativeLinux,
    NativeMacos,
    NativeWindows,
    PinnedUbuntuLtsWsl2,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformCellDefinition {
    pub platform: PlatformEnvironment,
    pub runner_boundary: PlatformRunnerBoundary,
    pub scenarios: Vec<CodexReleaseScenarioId>,
}

pub fn all() -> [PlatformCellDefinition; 4] {
    [
        linux::definition(),
        macos::definition(),
        windows::definition(),
        wsl2::definition(),
    ]
}
