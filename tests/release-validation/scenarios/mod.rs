use serde::{Deserialize, Serialize};
use volicord_types::CodexReleaseScenarioId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScenarioBoundary {
    Core,
    McpStdio,
    Cli,
    ManagedHost,
    Platform,
}

impl ScenarioBoundary {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Core => "core",
            Self::McpStdio => "mcp_stdio",
            Self::Cli => "cli",
            Self::ManagedHost => "managed_host",
            Self::Platform => "platform",
        }
    }

    pub const fn projection(self) -> ScenarioProjection {
        match self {
            Self::Core => ScenarioProjection::CoreResponse,
            Self::McpStdio => ScenarioProjection::McpStructuredContent,
            Self::Cli => ScenarioProjection::CliJson,
            Self::ManagedHost => ScenarioProjection::ManagedHostState,
            Self::Platform => ScenarioProjection::PlatformResult,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScenarioProjection {
    CoreResponse,
    McpStructuredContent,
    CliJson,
    ManagedHostState,
    PlatformResult,
}

impl ScenarioProjection {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CoreResponse => "core_response",
            Self::McpStructuredContent => "mcp_structured_content",
            Self::CliJson => "cli_json",
            Self::ManagedHostState => "managed_host_state",
            Self::PlatformResult => "platform_result",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScenarioFixture {
    NoInstallation,
    RuntimeHomeAbsent,
    PersonalBindingAbsent,
    SharedBindingAbsent,
    CurrentManagedBinding,
    DriftedManagedConfiguration,
    RepairableManagedConfigurationDrift,
    InstalledManagedBinding,
    SymlinkedManagedPath,
    RestartedCodexProcess,
    MovedProductRepository,
    RecordWorkflowReady,
    SuppressionProviderUnavailable,
    UnsupportedHostSelected,
    UnregisteredHostArtifact,
    StaleWsl2ProcessAndReceipt,
    Wsl2Ext4Topology,
    Wsl2DrvfsTopology,
    Wsl2CrossTopology,
    Wsl1Environment,
    NativeWindowsReceiptInWsl2,
}

impl ScenarioFixture {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NoInstallation => "no_installation",
            Self::RuntimeHomeAbsent => "runtime_home_absent",
            Self::PersonalBindingAbsent => "personal_binding_absent",
            Self::SharedBindingAbsent => "shared_binding_absent",
            Self::CurrentManagedBinding => "current_managed_binding",
            Self::DriftedManagedConfiguration => "drifted_managed_configuration",
            Self::RepairableManagedConfigurationDrift => "repairable_managed_configuration_drift",
            Self::InstalledManagedBinding => "installed_managed_binding",
            Self::SymlinkedManagedPath => "symlinked_managed_path",
            Self::RestartedCodexProcess => "restarted_codex_process",
            Self::MovedProductRepository => "moved_product_repository",
            Self::RecordWorkflowReady => "record_workflow_ready",
            Self::SuppressionProviderUnavailable => "suppression_provider_unavailable",
            Self::UnsupportedHostSelected => "unsupported_host_selected",
            Self::UnregisteredHostArtifact => "unregistered_host_artifact",
            Self::StaleWsl2ProcessAndReceipt => "stale_wsl2_process_and_receipt",
            Self::Wsl2Ext4Topology => "wsl2_ext4_topology",
            Self::Wsl2DrvfsTopology => "wsl2_drvfs_topology",
            Self::Wsl2CrossTopology => "wsl2_cross_topology",
            Self::Wsl1Environment => "wsl1_environment",
            Self::NativeWindowsReceiptInWsl2 => "native_windows_receipt_in_wsl2",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScenarioDomainDisposition {
    Completed,
    Rejected,
    Warning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScenarioOutcomeCode {
    InstallationCompleted,
    RuntimeHomeCreated,
    PersonalManagedBindingInstalled,
    SharedManagedBindingInstalled,
    ReceiptCurrent,
    ConfigurationDriftDetected,
    ConfigurationRepaired,
    ManagedBindingRemoved,
    CanonicalPathRulesEnforced,
    StaleReceiptRejected,
    MovedProjectBindingRejected,
    RecordWriteCompleted,
    ObservedPathsPreserved,
    UnsupportedHostRejected,
    UnsupportedHostArtifactRejected,
    StaleWsl2ProcessAndReceiptRejected,
    Wsl2Ext4Accepted,
    Wsl2DrvfsRejected,
    Wsl2CrossTopologyRejected,
    Wsl1Rejected,
    NativeWindowsReceiptReuseRejected,
}

impl ScenarioOutcomeCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InstallationCompleted => "installation_completed",
            Self::RuntimeHomeCreated => "runtime_home_created",
            Self::PersonalManagedBindingInstalled => "personal_managed_binding_installed",
            Self::SharedManagedBindingInstalled => "shared_managed_binding_installed",
            Self::ReceiptCurrent => "receipt_current",
            Self::ConfigurationDriftDetected => "configuration_drift_detected",
            Self::ConfigurationRepaired => "configuration_repaired",
            Self::ManagedBindingRemoved => "managed_binding_removed",
            Self::CanonicalPathRulesEnforced => "canonical_path_rules_enforced",
            Self::StaleReceiptRejected => "stale_receipt_rejected",
            Self::MovedProjectBindingRejected => "moved_project_binding_rejected",
            Self::RecordWriteCompleted => "record_write_completed",
            Self::ObservedPathsPreserved => "observed_paths_preserved",
            Self::UnsupportedHostRejected => "unsupported_host_rejected",
            Self::UnsupportedHostArtifactRejected => "unsupported_host_artifact_rejected",
            Self::StaleWsl2ProcessAndReceiptRejected => "stale_wsl2_process_and_receipt_rejected",
            Self::Wsl2Ext4Accepted => "wsl2_ext4_accepted",
            Self::Wsl2DrvfsRejected => "wsl2_drvfs_rejected",
            Self::Wsl2CrossTopologyRejected => "wsl2_cross_topology_rejected",
            Self::Wsl1Rejected => "wsl1_rejected",
            Self::NativeWindowsReceiptReuseRejected => "native_windows_receipt_reuse_rejected",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioDefinition {
    pub id: CodexReleaseScenarioId,
    pub fixture: ScenarioFixture,
    pub boundary: ScenarioBoundary,
    pub projection: ScenarioProjection,
    pub expectation: ScenarioExpectation,
    pub disposition: ScenarioDomainDisposition,
    pub outcome_code: ScenarioOutcomeCode,
    pub observed_paths_preserved: Option<bool>,
}

pub const BASE_SCENARIOS: [CodexReleaseScenarioId; 15] = CodexReleaseScenarioId::BASE;

pub const WSL2_ADDITIONAL_SCENARIOS: [CodexReleaseScenarioId; 6] =
    CodexReleaseScenarioId::WSL2_ADDITIONAL;

pub fn definition(id: CodexReleaseScenarioId) -> ScenarioDefinition {
    use CodexReleaseScenarioId as Id;
    use ScenarioBoundary as Boundary;
    use ScenarioDomainDisposition as Disposition;
    use ScenarioExpectation as Expectation;
    use ScenarioFixture as Fixture;
    use ScenarioOutcomeCode as Outcome;

    let (fixture, boundary, expectation, disposition, outcome_code, observed_paths_preserved) =
        match id {
            Id::FreshInstall => (
                Fixture::NoInstallation,
                Boundary::Cli,
                Expectation::CompleteSuccessfully,
                Disposition::Completed,
                Outcome::InstallationCompleted,
                None,
            ),
            Id::RuntimeHomeCreation => (
                Fixture::RuntimeHomeAbsent,
                Boundary::Cli,
                Expectation::CompleteSuccessfully,
                Disposition::Completed,
                Outcome::RuntimeHomeCreated,
                None,
            ),
            Id::PersonalManagedBinding => (
                Fixture::PersonalBindingAbsent,
                Boundary::Cli,
                Expectation::CompleteSuccessfully,
                Disposition::Completed,
                Outcome::PersonalManagedBindingInstalled,
                None,
            ),
            Id::SharedManagedBinding => (
                Fixture::SharedBindingAbsent,
                Boundary::Cli,
                Expectation::CompleteSuccessfully,
                Disposition::Completed,
                Outcome::SharedManagedBindingInstalled,
                None,
            ),
            Id::ReceiptCreateAndValidate => (
                Fixture::CurrentManagedBinding,
                Boundary::ManagedHost,
                Expectation::CompleteSuccessfully,
                Disposition::Completed,
                Outcome::ReceiptCurrent,
                None,
            ),
            Id::ConfigurationDriftDetection => (
                Fixture::DriftedManagedConfiguration,
                Boundary::ManagedHost,
                Expectation::CompleteSuccessfully,
                Disposition::Completed,
                Outcome::ConfigurationDriftDetected,
                None,
            ),
            Id::RepairAfterDrift => (
                Fixture::RepairableManagedConfigurationDrift,
                Boundary::Cli,
                Expectation::CompleteSuccessfully,
                Disposition::Completed,
                Outcome::ConfigurationRepaired,
                None,
            ),
            Id::SafeUninstall => (
                Fixture::InstalledManagedBinding,
                Boundary::Cli,
                Expectation::CompleteSuccessfully,
                Disposition::Completed,
                Outcome::ManagedBindingRemoved,
                None,
            ),
            Id::SymlinkAndCanonicalPath => (
                Fixture::SymlinkedManagedPath,
                Boundary::Platform,
                Expectation::CompleteSuccessfully,
                Disposition::Completed,
                Outcome::CanonicalPathRulesEnforced,
                None,
            ),
            Id::CodexRestart => (
                Fixture::RestartedCodexProcess,
                Boundary::ManagedHost,
                Expectation::CompleteSuccessfully,
                Disposition::Completed,
                Outcome::StaleReceiptRejected,
                None,
            ),
            Id::ProjectMove => (
                Fixture::MovedProductRepository,
                Boundary::ManagedHost,
                Expectation::CompleteSuccessfully,
                Disposition::Completed,
                Outcome::MovedProjectBindingRejected,
                None,
            ),
            Id::RecordWriteWorkflow => (
                Fixture::RecordWorkflowReady,
                Boundary::McpStdio,
                Expectation::CompleteSuccessfully,
                Disposition::Completed,
                Outcome::RecordWriteCompleted,
                None,
            ),
            Id::SuppressionUnavailable => (
                Fixture::SuppressionProviderUnavailable,
                Boundary::Core,
                Expectation::PreserveObservedPathsWhenSuppressionUnavailable,
                Disposition::Warning,
                Outcome::ObservedPathsPreserved,
                Some(true),
            ),
            Id::UnsupportedHost => (
                Fixture::UnsupportedHostSelected,
                Boundary::Cli,
                Expectation::RejectUnsupportedHost,
                Disposition::Rejected,
                Outcome::UnsupportedHostRejected,
                None,
            ),
            Id::UnsupportedHostArtifact => (
                Fixture::UnregisteredHostArtifact,
                Boundary::ManagedHost,
                Expectation::RejectUnsupportedHostArtifact,
                Disposition::Rejected,
                Outcome::UnsupportedHostArtifactRejected,
                None,
            ),
            Id::WslShutdownRestart => (
                Fixture::StaleWsl2ProcessAndReceipt,
                Boundary::Platform,
                Expectation::RejectStaleWsl2ProcessAndReceipt,
                Disposition::Rejected,
                Outcome::StaleWsl2ProcessAndReceiptRejected,
                None,
            ),
            Id::Wsl2Ext4Project => (
                Fixture::Wsl2Ext4Topology,
                Boundary::Platform,
                Expectation::AcceptWsl2Ext4,
                Disposition::Completed,
                Outcome::Wsl2Ext4Accepted,
                None,
            ),
            Id::Wsl2DrvfsRejection => (
                Fixture::Wsl2DrvfsTopology,
                Boundary::Platform,
                Expectation::RejectWsl2Drvfs,
                Disposition::Rejected,
                Outcome::Wsl2DrvfsRejected,
                None,
            ),
            Id::Wsl2CrossTopologyRejection => (
                Fixture::Wsl2CrossTopology,
                Boundary::Platform,
                Expectation::RejectWsl2CrossTopology,
                Disposition::Rejected,
                Outcome::Wsl2CrossTopologyRejected,
                None,
            ),
            Id::Wsl1Rejection => (
                Fixture::Wsl1Environment,
                Boundary::Platform,
                Expectation::RejectWsl1,
                Disposition::Rejected,
                Outcome::Wsl1Rejected,
                None,
            ),
            Id::Wsl2NativeWindowsReceiptReuseRejection => (
                Fixture::NativeWindowsReceiptInWsl2,
                Boundary::ManagedHost,
                Expectation::RejectNativeWindowsReceiptReuse,
                Disposition::Rejected,
                Outcome::NativeWindowsReceiptReuseRejected,
                None,
            ),
        };
    ScenarioDefinition {
        id,
        fixture,
        boundary,
        projection: boundary.projection(),
        expectation,
        disposition,
        outcome_code,
        observed_paths_preserved,
    }
}

pub fn scenarios_for_wsl2() -> Vec<CodexReleaseScenarioId> {
    BASE_SCENARIOS
        .into_iter()
        .chain(WSL2_ADDITIONAL_SCENARIOS)
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use serde::Serialize;

    use super::*;

    #[test]
    fn every_catalog_entry_has_one_exact_repository_owned_contract() {
        let mut fixtures = BTreeSet::new();
        let mut outcomes = BTreeSet::new();
        for id in BASE_SCENARIOS.into_iter().chain(WSL2_ADDITIONAL_SCENARIOS) {
            let definition = definition(id);
            assert_eq!(definition.id, id);
            assert_eq!(definition.projection, definition.boundary.projection());
            assert!(fixtures.insert(definition.fixture.as_str()));
            assert!(outcomes.insert(definition.outcome_code.as_str()));
            assert_eq!(
                definition.observed_paths_preserved,
                (id == CodexReleaseScenarioId::SuppressionUnavailable).then_some(true)
            );
            assert_serialized_name(definition.fixture, definition.fixture.as_str());
            assert_serialized_name(definition.boundary, definition.boundary.as_str());
            assert_serialized_name(definition.projection, definition.projection.as_str());
            assert_serialized_name(definition.outcome_code, definition.outcome_code.as_str());
        }
    }

    fn assert_serialized_name(value: impl Serialize, expected: &str) {
        assert_eq!(
            serde_json::to_value(value).expect("serialize closed scenario value"),
            serde_json::Value::String(expected.to_owned())
        );
    }
}
