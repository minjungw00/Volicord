use crate::schema::CodexReleaseScenarioId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScenarioExpectation {
    CompleteSuccessfully,
    RejectUnsupportedHost,
    RejectUnsupportedHostArtifact,
    RejectWsl1,
    AcceptWsl2Ext4,
    RejectWsl2Drvfs,
    RejectWsl2CrossTopology,
    RejectNativeWindowsReceiptReuse,
    RejectStaleWsl2ProcessAndReceipt,
    PreserveObservedPathsWhenSuppressionUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScenarioDefinition {
    pub id: CodexReleaseScenarioId,
    pub expectation: ScenarioExpectation,
}

pub const BASE_SCENARIOS: [CodexReleaseScenarioId; 15] = CodexReleaseScenarioId::BASE;

pub const WSL2_ADDITIONAL_SCENARIOS: [CodexReleaseScenarioId; 6] =
    CodexReleaseScenarioId::WSL2_ADDITIONAL;

pub fn definition(id: CodexReleaseScenarioId) -> ScenarioDefinition {
    use CodexReleaseScenarioId as Id;
    use ScenarioExpectation as Expectation;

    let expectation = match id {
        Id::UnsupportedHost => Expectation::RejectUnsupportedHost,
        Id::UnsupportedHostArtifact => Expectation::RejectUnsupportedHostArtifact,
        Id::SuppressionUnavailable => Expectation::PreserveObservedPathsWhenSuppressionUnavailable,
        Id::WslShutdownRestart => Expectation::RejectStaleWsl2ProcessAndReceipt,
        Id::Wsl2Ext4Project => Expectation::AcceptWsl2Ext4,
        Id::Wsl2DrvfsRejection => Expectation::RejectWsl2Drvfs,
        Id::Wsl2CrossTopologyRejection => Expectation::RejectWsl2CrossTopology,
        Id::Wsl1Rejection => Expectation::RejectWsl1,
        Id::Wsl2NativeWindowsReceiptReuseRejection => Expectation::RejectNativeWindowsReceiptReuse,
        _ => Expectation::CompleteSuccessfully,
    };
    ScenarioDefinition { id, expectation }
}

pub fn scenarios_for_wsl2() -> Vec<CodexReleaseScenarioId> {
    BASE_SCENARIOS
        .into_iter()
        .chain(WSL2_ADDITIONAL_SCENARIOS)
        .collect()
}
