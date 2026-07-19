use std::path::Path;

use serde::Serialize;

use crate::{
    guard_integration::{files::RetirementPlanStatus, FilePlanStatus, GuardIntegrationPlan},
    host_integration::{HostPlan, PlannedChange},
};

use super::{host_target_text, path_text};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum PlannedConnectionChangeKind {
    RuntimeHomeInitialization,
    ProjectRegistration,
    ManagedHostConfiguration,
    GuardManagedFile,
    GuardRegistrySetup,
    ModeTransition,
    ConnectionMembership,
}

impl PlannedConnectionChangeKind {
    const ALL: [Self; 7] = [
        Self::ConnectionMembership,
        Self::GuardManagedFile,
        Self::GuardRegistrySetup,
        Self::ManagedHostConfiguration,
        Self::ModeTransition,
        Self::ProjectRegistration,
        Self::RuntimeHomeInitialization,
    ];

    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::RuntimeHomeInitialization => "runtime_home_initialization",
            Self::ProjectRegistration => "project_registration",
            Self::ManagedHostConfiguration => "managed_host_configuration",
            Self::GuardManagedFile => "guard_managed_file",
            Self::GuardRegistrySetup => "guard_registry_setup",
            Self::ModeTransition => "mode_transition",
            Self::ConnectionMembership => "connection_membership",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum PlannedChangeOperation {
    Create,
    Update,
    Remove,
    Register,
    Rebind,
    Execute,
}

impl PlannedChangeOperation {
    const ALL: [Self; 6] = [
        Self::Create,
        Self::Execute,
        Self::Rebind,
        Self::Register,
        Self::Remove,
        Self::Update,
    ];

    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Update => "update",
            Self::Remove => "remove",
            Self::Register => "register",
            Self::Rebind => "rebind",
            Self::Execute => "execute",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct PlannedConnectionChange {
    kind: PlannedConnectionChangeKind,
    operation: PlannedChangeOperation,
    target: String,
}

impl PlannedConnectionChange {
    pub(super) fn new(
        kind: PlannedConnectionChangeKind,
        operation: PlannedChangeOperation,
        target: impl Into<String>,
    ) -> Self {
        debug_assert!(PlannedConnectionChangeKind::ALL.contains(&kind));
        debug_assert!(PlannedChangeOperation::ALL.contains(&operation));
        Self {
            kind,
            operation,
            target: target.into(),
        }
    }

    pub(super) const fn kind(&self) -> PlannedConnectionChangeKind {
        self.kind
    }

    pub(super) const fn operation(&self) -> PlannedChangeOperation {
        self.operation
    }

    pub(super) fn target(&self) -> &str {
        &self.target
    }
}

pub(super) struct InitPlannedChanges<'a> {
    pub(super) runtime_home: &'a Path,
    pub(super) repo_root: &'a Path,
    pub(super) profile_exists: bool,
    pub(super) project_exists: bool,
    pub(super) host_plan: &'a HostPlan,
    pub(super) integration: &'a GuardIntegrationPlan,
}

pub(super) fn plan_init_changes(input: InitPlannedChanges<'_>) -> Vec<PlannedConnectionChange> {
    let mut changes = Vec::new();
    if !input.profile_exists {
        changes.push(PlannedConnectionChange::new(
            PlannedConnectionChangeKind::RuntimeHomeInitialization,
            PlannedChangeOperation::Create,
            path_text(input.runtime_home),
        ));
    }
    if !input.project_exists {
        changes.extend([
            PlannedConnectionChange::new(
                PlannedConnectionChangeKind::ProjectRegistration,
                PlannedChangeOperation::Register,
                path_text(input.repo_root),
            ),
            PlannedConnectionChange::new(
                PlannedConnectionChangeKind::ConnectionMembership,
                PlannedChangeOperation::Register,
                path_text(input.repo_root),
            ),
            PlannedConnectionChange::new(
                PlannedConnectionChangeKind::GuardRegistrySetup,
                PlannedChangeOperation::Register,
                &input.integration.guard_installation_id,
            ),
        ]);
    } else if input.integration.migration_required {
        changes.push(PlannedConnectionChange::new(
            PlannedConnectionChangeKind::GuardRegistrySetup,
            PlannedChangeOperation::Rebind,
            &input.integration.guard_installation_id,
        ));
    }
    if let Some(operation) = host_change_operation(input.host_plan.change) {
        changes.push(PlannedConnectionChange::new(
            PlannedConnectionChangeKind::ManagedHostConfiguration,
            operation,
            host_target_text(&input.host_plan.target),
        ));
    }
    for file in &input.integration.generated_files {
        if let Some(operation) = generated_file_operation(file.status) {
            changes.push(PlannedConnectionChange::new(
                PlannedConnectionChangeKind::GuardManagedFile,
                operation,
                canonical_guard_target(input.repo_root, &file.path),
            ));
        }
    }
    for file in &input.integration.retired_files {
        if let Some(operation) = retired_file_operation(file.status) {
            changes.push(PlannedConnectionChange::new(
                PlannedConnectionChangeKind::GuardManagedFile,
                operation,
                canonical_guard_target(input.repo_root, &file.path),
            ));
        }
    }
    changes.sort_by(|left, right| {
        left.kind()
            .as_str()
            .cmp(right.kind().as_str())
            .then_with(|| left.operation().as_str().cmp(right.operation().as_str()))
            .then_with(|| left.target().cmp(right.target()))
    });
    changes.dedup();
    changes
}

fn host_change_operation(change: PlannedChange) -> Option<PlannedChangeOperation> {
    match change {
        PlannedChange::Create => Some(PlannedChangeOperation::Create),
        PlannedChange::Update => Some(PlannedChangeOperation::Update),
        PlannedChange::Remove => Some(PlannedChangeOperation::Remove),
        PlannedChange::Noop => None,
        PlannedChange::ExternalCommand => Some(PlannedChangeOperation::Execute),
    }
}

fn generated_file_operation(status: FilePlanStatus) -> Option<PlannedChangeOperation> {
    match status {
        FilePlanStatus::PlannedCreate => Some(PlannedChangeOperation::Create),
        FilePlanStatus::PlannedUpdate => Some(PlannedChangeOperation::Update),
        FilePlanStatus::Unchanged | FilePlanStatus::Created | FilePlanStatus::Updated => None,
    }
}

fn retired_file_operation(status: RetirementPlanStatus) -> Option<PlannedChangeOperation> {
    match status {
        RetirementPlanStatus::PlannedRemove => Some(PlannedChangeOperation::Remove),
        RetirementPlanStatus::PlannedUpdate => Some(PlannedChangeOperation::Update),
        RetirementPlanStatus::Unchanged
        | RetirementPlanStatus::Removed
        | RetirementPlanStatus::Updated => None,
    }
}

fn canonical_guard_target(repo_root: &Path, path: &Path) -> String {
    path.strip_prefix(repo_root)
        .ok()
        .filter(|relative| relative.components().next().is_some())
        .map(path_text)
        .unwrap_or_else(|| path_text(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planned_kind_and_operation_vocabularies_serialize_exactly() {
        let kinds = [
            "connection_membership",
            "guard_managed_file",
            "guard_registry_setup",
            "managed_host_configuration",
            "mode_transition",
            "project_registration",
            "runtime_home_initialization",
        ];
        for (kind, expected) in PlannedConnectionChangeKind::ALL.into_iter().zip(kinds) {
            assert_eq!(serde_json::to_value(kind).unwrap(), expected);
        }

        let operations = [
            "create", "execute", "rebind", "register", "remove", "update",
        ];
        for (operation, expected) in PlannedChangeOperation::ALL.into_iter().zip(operations) {
            assert_eq!(serde_json::to_value(operation).unwrap(), expected);
        }
    }
}
