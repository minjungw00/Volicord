//! Typed operational diagnostics for Volicord-owned administrative boundaries.

mod actions;
mod definitions;
mod facts;
mod persistence;
mod projection;
mod subjects;

pub use actions::OperationalCheckState;
pub use definitions::{
    GuardDiagnostic, InstallationDiagnostic, OperationalDiagnostic,
    OperationalDiagnosticDefinition, RevisionDiagnostic, ToolVerificationDiagnostic,
    TrustDiagnostic,
};
pub use facts::{
    GuardArtifactFacts, GuardEventFacts, GuardInstallationFacts, GuardPhaseFacts, GuardProbeFacts,
    InstallationFacts, IntegrationRevisionFacts, ManagedConfigurationFacts, TrustFacts,
    VerificationToolFacts,
};
pub(crate) use persistence::{reconcile_current_findings_for_scope, CurrentOperationalOwner};
pub(crate) use projection::{
    current_connection_finding, current_report_findings, current_report_findings_with_overlay,
    occurrence_finding, DiagnosticFindingOverlay,
};
pub(crate) use subjects::guard_artifact_kind;
pub use subjects::{
    GuardEventSubject, GuardInstallationSubject, GuardManagedArtifactSubject, GuardPhaseSubject,
    GuardVerificationAttemptSubject, InstallationSubject, IntegrationRevisionSubject,
    ManagedConfigurationTarget, OperationalSubject, ProductRepositorySubject, TrustSubject,
    VerificationToolSubject,
};
