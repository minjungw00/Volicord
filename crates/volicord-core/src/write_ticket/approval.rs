use std::collections::BTreeSet;

use volicord_types::ids::{ProjectId, UserActionResolutionId};
use volicord_types::schema::{
    StateRecordRef, UserActionResolutionIdentity, UserActionResolutionRef, WriteTicketAttemptScope,
};
use volicord_types::values::{
    StateRecordKind, TaskControlLevel, UserActionKind, UserActionRequiredFor, UtcTimestamp,
};
use volicord_user_action_service::{
    accepted_current_user_authority, current_sensitive_approval, SensitiveApprovalRequirement,
    UserActionAuthority,
};

/// The exact current sensitive approvals for one canonical Write Ticket requirement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CurrentSensitiveApprovals {
    identities: BTreeSet<UserActionResolutionIdentity>,
    scope_candidate_identities: BTreeSet<UserActionResolutionIdentity>,
}

impl CurrentSensitiveApprovals {
    pub(crate) fn new(
        authorities: &[UserActionAuthority],
        requirement: &WriteTicketApprovalRequirement<'_>,
    ) -> Self {
        let mut identities = BTreeSet::new();
        let mut scope_candidate_identities = BTreeSet::new();
        for authority in authorities {
            if !requirement.has_current_owner(authority) {
                continue;
            }
            let Some(identity) = authority.resolution_identity() else {
                continue;
            };
            if accepted_current_user_authority(authority, UserActionKind::SensitiveApproval)
                && authority
                    .required_for
                    .contains(&UserActionRequiredFor::PrepareWrite)
            {
                scope_candidate_identities.insert(identity.clone());
            }
            if requirement.matches(authority) {
                identities.insert(identity);
            }
        }
        Self {
            identities,
            scope_candidate_identities,
        }
    }

    pub(crate) fn primary_basis(&self) -> Option<NonEmptyApprovalBasis> {
        self.identities
            .first()
            .cloned()
            .map(NonEmptyApprovalBasis::one)
    }
}

/// A validated, non-empty collection of full approval-resolution identities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NonEmptyApprovalBasis {
    identities: BTreeSet<UserActionResolutionIdentity>,
}

impl NonEmptyApprovalBasis {
    fn one(identity: UserActionResolutionIdentity) -> Self {
        Self {
            identities: BTreeSet::from([identity]),
        }
    }

    fn from_store_valid_refs(refs: &[UserActionResolutionRef]) -> Option<Self> {
        if refs.is_empty() {
            return None;
        }
        Some(Self {
            identities: refs.iter().map(UserActionResolutionRef::identity).collect(),
        })
    }

    pub(crate) fn first_resolution_id(&self) -> &UserActionResolutionId {
        &self
            .identities
            .first()
            .expect("non-empty approval basis retains an identity")
            .resolution_id
    }

    pub(crate) fn resolution_refs(
        &self,
        produced_at_state_version: u64,
    ) -> Vec<UserActionResolutionRef> {
        self.identities
            .iter()
            .map(|identity| {
                UserActionResolutionRef::new(
                    identity.project_id.clone(),
                    identity.task_id.clone(),
                    identity.resolution_id.clone(),
                    Some(produced_at_state_version),
                )
            })
            .collect()
    }

    pub(crate) fn state_refs(&self, produced_at_state_version: u64) -> Vec<StateRecordRef> {
        self.identities
            .iter()
            .map(|identity| {
                StateRecordRef::new(
                    StateRecordKind::UserActionResolution,
                    identity.resolution_id.as_str(),
                    identity.project_id.clone(),
                    Some(identity.task_id.clone()),
                    Some(produced_at_state_version),
                )
            })
            .collect()
    }
}

/// Canonical semantic requirement for a Write Ticket's approval basis.
pub(crate) struct WriteTicketApprovalRequirement<'a> {
    project_id: &'a ProjectId,
    scope_revision: u64,
    effective_control_level: TaskControlLevel,
    scope: &'a WriteTicketAttemptScope,
    normalized_paths: Vec<String>,
    observed_at: &'a UtcTimestamp,
}

impl<'a> WriteTicketApprovalRequirement<'a> {
    pub(crate) fn new(
        project_id: &'a ProjectId,
        scope_revision: u64,
        effective_control_level: TaskControlLevel,
        scope: &'a WriteTicketAttemptScope,
        observed_at: &'a UtcTimestamp,
    ) -> Self {
        Self {
            project_id,
            scope_revision,
            effective_control_level,
            scope,
            normalized_paths: scope
                .intended_paths
                .iter()
                .map(|path| path.as_str().to_owned())
                .collect(),
            observed_at,
        }
    }

    pub(crate) fn is_required(&self) -> bool {
        self.effective_control_level == TaskControlLevel::Sensitive
            || !self.scope.sensitive_categories.is_empty()
    }

    pub(crate) fn sensitive_requirement(&self) -> SensitiveApprovalRequirement<'_> {
        SensitiveApprovalRequirement {
            task_id: &self.scope.task_id,
            change_unit_id: &self.scope.change_unit_id,
            scope_revision: self.scope_revision,
            operation: &self.scope.intended_operation,
            normalized_paths: &self.normalized_paths,
            sensitive_categories: &self.scope.sensitive_categories,
            baseline_ref: self.scope.baseline_ref.as_ref(),
            required_for: UserActionRequiredFor::PrepareWrite,
            now: self.observed_at,
        }
    }

    fn has_current_owner(&self, authority: &UserActionAuthority) -> bool {
        authority.project_id == *self.project_id && authority.task_id == self.scope.task_id
    }

    fn matches(&self, authority: &UserActionAuthority) -> bool {
        self.has_current_owner(authority)
            && current_sensitive_approval(authority, &self.sensitive_requirement())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ApprovalBasisChangeReason {
    ApprovalNowRequired,
    NoCurrentResolution,
    ApprovalScopeChanged {
        resolution: UserActionResolutionIdentity,
    },
    BasisResolutionNoLongerCurrent {
        resolution: UserActionResolutionIdentity,
    },
}

#[must_use]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WriteTicketApprovalAssessment {
    NotRequired,
    Current { basis: NonEmptyApprovalBasis },
    Changed { reason: ApprovalBasisChangeReason },
}

/// Assesses one Store-valid persisted approval basis against the canonical current set.
pub(crate) fn assess_write_ticket_approval(
    requirement: &WriteTicketApprovalRequirement<'_>,
    current: &CurrentSensitiveApprovals,
    persisted_refs: &[UserActionResolutionRef],
) -> WriteTicketApprovalAssessment {
    let Some(basis) = NonEmptyApprovalBasis::from_store_valid_refs(persisted_refs) else {
        return if requirement.is_required() {
            WriteTicketApprovalAssessment::Changed {
                reason: ApprovalBasisChangeReason::ApprovalNowRequired,
            }
        } else {
            WriteTicketApprovalAssessment::NotRequired
        };
    };

    if basis
        .identities
        .iter()
        .all(|identity| current.identities.contains(identity))
    {
        return WriteTicketApprovalAssessment::Current { basis };
    }

    let missing = basis
        .identities
        .iter()
        .find(|identity| !current.identities.contains(*identity))
        .expect("a non-current basis has a missing identity")
        .clone();
    if current.scope_candidate_identities.contains(&missing) {
        WriteTicketApprovalAssessment::Changed {
            reason: ApprovalBasisChangeReason::ApprovalScopeChanged {
                resolution: missing,
            },
        }
    } else if current.identities.is_empty() {
        WriteTicketApprovalAssessment::Changed {
            reason: ApprovalBasisChangeReason::NoCurrentResolution,
        }
    } else {
        WriteTicketApprovalAssessment::Changed {
            reason: ApprovalBasisChangeReason::BasisResolutionNoLongerCurrent {
                resolution: missing,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use volicord_types::ids::{
        BaselineRef, ChangeUnitId, ProjectId, TaskId, UserActionOptionId, UserActionRequestId,
        UserActionResolutionId,
    };
    use volicord_types::product_path::ProductRelativePath;
    use volicord_types::schema::{
        RequiredNullable, SensitiveActionScope, UserActionBasis, UserActionBasisCoordinates,
        UserActionChoiceBasis, UserActionResolutionBody, UserActionResolutionRef,
        WriteTicketAttemptScope,
    };
    use volicord_types::values::{
        ActorSource, JudgmentResolutionOutcome, TaskControlLevel, UserActionBasisStatus,
        UserActionKind, UserActionOptionAction, UserActionRequiredFor, UserActionStatus,
        UserActionVerificationBasis, WriteTicketStatus,
    };
    use volicord_user_action_service::UserActionAuthority;

    use super::{
        assess_write_ticket_approval, ApprovalBasisChangeReason, CurrentSensitiveApprovals,
        WriteTicketApprovalAssessment, WriteTicketApprovalRequirement,
    };
    use crate::write_ticket::admission::{
        record_run_approval_admission, RecordRunApprovalAdmission,
    };
    use crate::write_ticket::current_validity::evaluate_current_write_ticket;
    use crate::write_ticket::planning::{reuse_approval_assessment, WriteTicketReuseApproval};
    use crate::write_ticket::read_model::{
        WriteTicketCurrentFacts, WriteTicketEvidenceFacts, WriteTicketTaskFacts,
        WriteTicketWorkflowFacts,
    };
    use crate::write_ticket::semantic::test_support::{stored_facts, timestamp};
    use crate::write_ticket::summary::{project_write_ticket_summary, WriteTicketSummaryInput};

    fn scope() -> WriteTicketAttemptScope {
        WriteTicketAttemptScope {
            task_id: TaskId::new("task-test"),
            change_unit_id: ChangeUnitId::new("change-test"),
            intended_operation: "edit".to_owned(),
            intended_paths: vec![
                ProductRelativePath::parse("src").expect("valid Product Repository path")
            ],
            product_file_write_intended: true,
            sensitive_categories: vec!["network".to_owned()],
            baseline_ref: Some(BaselineRef::new("baseline-test")),
        }
    }

    struct ApprovalSpec<'a> {
        project_id: &'a str,
        task_id: &'a str,
        change_unit_id: &'a str,
        scope_revision: u64,
        operation: &'a str,
        paths: &'a [&'a str],
        categories: &'a [&'a str],
        baseline_ref: &'a str,
    }

    fn approval(resolution_id: &str, spec: ApprovalSpec<'_>) -> UserActionAuthority {
        UserActionAuthority {
            project_id: ProjectId::new(spec.project_id),
            user_action_request_id: UserActionRequestId::new(format!("request-{resolution_id}")),
            user_action_resolution_id: Some(UserActionResolutionId::new(resolution_id)),
            task_id: TaskId::new(spec.task_id),
            action_kind: UserActionKind::SensitiveApproval,
            status: UserActionStatus::Resolved,
            required_for: vec![UserActionRequiredFor::PrepareWrite],
            affected_refs: Vec::new(),
            machine_action: Some(UserActionOptionAction::Accept),
            resolution_outcome: Some(JudgmentResolutionOutcome::Accepted),
            resolved_by_actor_source: Some(ActorSource::LocalUser),
            resolved_verification_basis: Some(UserActionVerificationBasis::CliDirectUserChannel),
            resolved_assurance_level: Some("direct_user_input".to_owned()),
            basis_status: UserActionBasisStatus::Current,
            basis: Some(UserActionBasis::Choice(Box::new(UserActionChoiceBasis {
                coordinates: UserActionBasisCoordinates {
                    task_id: TaskId::new(spec.task_id),
                    change_unit_id: RequiredNullable::some(ChangeUnitId::new(spec.change_unit_id)),
                    scope_revision: spec.scope_revision,
                    baseline_ref: RequiredNullable::some(BaselineRef::new(spec.baseline_ref)),
                    created_at_state_version: 6,
                    compatibility_status: UserActionBasisStatus::Current,
                },
                close_basis_revision: RequiredNullable::null(),
                result_refs: Vec::new(),
                residual_risk_ids: Vec::new(),
                sensitive_action_scope: RequiredNullable::some(SensitiveActionScope {
                    action_kind: spec.operation.to_owned(),
                    description: "Approve the exact test operation.".to_owned(),
                    intended_paths: spec.paths.iter().map(|path| (*path).to_owned()).collect(),
                    sensitive_categories: spec
                        .categories
                        .iter()
                        .map(|category| (*category).to_owned())
                        .collect(),
                    command_or_tool_summary: RequiredNullable::null(),
                    network_or_host_summary: RequiredNullable::null(),
                    secret_or_credential_summary: RequiredNullable::null(),
                    capability_claim: "test approval".to_owned(),
                    expires_at: RequiredNullable::null(),
                }),
            }))),
            resolution: Some(UserActionResolutionBody::Choice {
                selected_option_id: UserActionOptionId::new("accept"),
                machine_action: UserActionOptionAction::Accept,
                resolution_outcome: JudgmentResolutionOutcome::Accepted,
                note: RequiredNullable::null(),
                accepted_risk_ids: Vec::new(),
            }),
            expires_at: None,
        }
    }

    fn matching_approval(resolution_id: &str) -> UserActionAuthority {
        approval(
            resolution_id,
            ApprovalSpec {
                project_id: "project-test",
                task_id: "task-test",
                change_unit_id: "change-test",
                scope_revision: 3,
                operation: "edit",
                paths: &["src"],
                categories: &["network"],
                baseline_ref: "baseline-test",
            },
        )
    }

    fn approval_ref(resolution_id: &str) -> UserActionResolutionRef {
        UserActionResolutionRef::new(
            ProjectId::new("project-test"),
            TaskId::new("task-test"),
            UserActionResolutionId::new(resolution_id),
            Some(6),
        )
    }

    fn assessment(
        scope: &WriteTicketAttemptScope,
        scope_revision: u64,
        control: TaskControlLevel,
        authorities: &[UserActionAuthority],
        refs: &[UserActionResolutionRef],
    ) -> WriteTicketApprovalAssessment {
        let project_id = ProjectId::new("project-test");
        let now = timestamp("2026-07-29T00:05:00Z");
        let requirement =
            WriteTicketApprovalRequirement::new(&project_id, scope_revision, control, scope, &now);
        let current = CurrentSensitiveApprovals::new(authorities, &requirement);
        assess_write_ticket_approval(&requirement, &current, refs)
    }

    #[test]
    fn approval_owner_covers_the_complete_currentness_matrix() {
        let current = matching_approval("resolution-a");
        assert_eq!(
            assessment(
                &WriteTicketAttemptScope {
                    sensitive_categories: Vec::new(),
                    ..scope()
                },
                3,
                TaskControlLevel::Tracked,
                &[],
                &[],
            ),
            WriteTicketApprovalAssessment::NotRequired
        );
        assert_eq!(
            assessment(&scope(), 3, TaskControlLevel::Sensitive, &[], &[]),
            WriteTicketApprovalAssessment::Changed {
                reason: ApprovalBasisChangeReason::ApprovalNowRequired
            }
        );
        assert!(matches!(
            assessment(
                &scope(),
                3,
                TaskControlLevel::Sensitive,
                std::slice::from_ref(&current),
                &[approval_ref("resolution-a")],
            ),
            WriteTicketApprovalAssessment::Current { .. }
        ));
        assert_eq!(
            assessment(
                &scope(),
                3,
                TaskControlLevel::Sensitive,
                &[],
                &[approval_ref("resolution-a")],
            ),
            WriteTicketApprovalAssessment::Changed {
                reason: ApprovalBasisChangeReason::NoCurrentResolution
            }
        );

        let another_current = matching_approval("resolution-c");
        assert_eq!(
            assessment(
                &scope(),
                3,
                TaskControlLevel::Sensitive,
                &[current.clone(), another_current],
                &[approval_ref("resolution-a"), approval_ref("resolution-b")],
            ),
            WriteTicketApprovalAssessment::Changed {
                reason: ApprovalBasisChangeReason::BasisResolutionNoLongerCurrent {
                    resolution: approval_ref("resolution-b").identity()
                }
            }
        );

        let unrelated = approval(
            "resolution-unrelated",
            ApprovalSpec {
                project_id: "project-test",
                task_id: "task-test",
                change_unit_id: "change-test",
                scope_revision: 3,
                operation: "deploy",
                paths: &["ops"],
                categories: &["secrets"],
                baseline_ref: "baseline-test",
            },
        );
        assert!(matches!(
            assessment(
                &scope(),
                3,
                TaskControlLevel::Sensitive,
                &[current.clone(), unrelated],
                &[approval_ref("resolution-a")],
            ),
            WriteTicketApprovalAssessment::Current { .. }
        ));

        let other_project = approval(
            "resolution-other-project",
            ApprovalSpec {
                project_id: "project-other",
                task_id: "task-test",
                change_unit_id: "change-test",
                scope_revision: 3,
                operation: "edit",
                paths: &["src"],
                categories: &["network"],
                baseline_ref: "baseline-test",
            },
        );
        let other_task = approval(
            "resolution-other-task",
            ApprovalSpec {
                project_id: "project-test",
                task_id: "task-other",
                change_unit_id: "change-other",
                scope_revision: 3,
                operation: "edit",
                paths: &["src"],
                categories: &["network"],
                baseline_ref: "baseline-test",
            },
        );
        assert!(matches!(
            assessment(
                &scope(),
                3,
                TaskControlLevel::Sensitive,
                &[current.clone(), other_project, other_task],
                &[approval_ref("resolution-a")],
            ),
            WriteTicketApprovalAssessment::Current { .. }
        ));

        assert!(matches!(
            assessment(
                &scope(),
                3,
                TaskControlLevel::Sensitive,
                &[current, matching_approval("resolution-b")],
                &[approval_ref("resolution-a"), approval_ref("resolution-b")],
            ),
            WriteTicketApprovalAssessment::Current { .. }
        ));
    }

    #[test]
    fn scope_coordinates_report_semantic_approval_change() {
        let authority = matching_approval("resolution-a");
        let persisted = [approval_ref("resolution-a")];
        let mut changed_operation = scope();
        changed_operation.intended_operation = "deploy".to_owned();
        let mut changed_paths = scope();
        changed_paths.intended_paths =
            vec![ProductRelativePath::parse("docs").expect("valid path")];
        let mut changed_categories = scope();
        changed_categories.sensitive_categories = vec!["secrets".to_owned()];
        let mut changed_baseline = scope();
        changed_baseline.baseline_ref = Some(BaselineRef::new("baseline-other"));
        let unchanged_scope = scope();
        let cases = [
            (&unchanged_scope, 4),
            (&changed_operation, 3),
            (&changed_paths, 3),
            (&changed_categories, 3),
            (&changed_baseline, 3),
        ];

        for (changed_scope, revision) in cases {
            assert_eq!(
                assessment(
                    changed_scope,
                    revision,
                    TaskControlLevel::Sensitive,
                    std::slice::from_ref(&authority),
                    &persisted,
                ),
                WriteTicketApprovalAssessment::Changed {
                    reason: ApprovalBasisChangeReason::ApprovalScopeChanged {
                        resolution: approval_ref("resolution-a").identity()
                    }
                }
            );
        }
    }

    #[test]
    fn one_assessment_table_drives_summary_reuse_and_admission() {
        struct Case {
            scope: WriteTicketAttemptScope,
            control: TaskControlLevel,
            authorities: Vec<UserActionAuthority>,
            persisted_refs: Vec<UserActionResolutionRef>,
            expected_status: WriteTicketStatus,
            expected_reuse: bool,
            expected_admission: bool,
        }

        let cases = [
            Case {
                scope: scope(),
                control: TaskControlLevel::Sensitive,
                authorities: vec![matching_approval("resolution-a")],
                persisted_refs: vec![approval_ref("resolution-a")],
                expected_status: WriteTicketStatus::Active,
                expected_reuse: true,
                expected_admission: true,
            },
            Case {
                scope: WriteTicketAttemptScope {
                    sensitive_categories: Vec::new(),
                    ..scope()
                },
                control: TaskControlLevel::Tracked,
                authorities: Vec::new(),
                persisted_refs: Vec::new(),
                expected_status: WriteTicketStatus::Active,
                expected_reuse: true,
                expected_admission: true,
            },
            Case {
                scope: scope(),
                control: TaskControlLevel::Sensitive,
                authorities: Vec::new(),
                persisted_refs: vec![approval_ref("resolution-a")],
                expected_status: WriteTicketStatus::Invalidated,
                expected_reuse: false,
                expected_admission: false,
            },
        ];

        for case in cases {
            let assessment = assessment(
                &case.scope,
                3,
                case.control,
                &case.authorities,
                &case.persisted_refs,
            );
            let current = WriteTicketCurrentFacts {
                task: WriteTicketTaskFacts {
                    scope_revision: 3,
                    effective_control_level: case.control,
                    pending_policy_reevaluation: false,
                },
                workflow: WriteTicketWorkflowFacts {
                    write_authority_fingerprint: format!("sha256:{}", "0".repeat(64)),
                },
                sensitive_approvals: Vec::new(),
                observed_at: timestamp("2026-07-29T00:05:00Z"),
            };
            let evaluated = evaluate_current_write_ticket(
                stored_facts("ticket-conformance", WriteTicketStatus::Active, 7),
                &current,
                assessment.clone(),
            );
            let summary = project_write_ticket_summary(WriteTicketSummaryInput {
                evaluated: &evaluated,
                state_version: 8,
                evidence: &WriteTicketEvidenceFacts::default(),
                guarantee_display: None,
            });
            assert_eq!(summary.status, case.expected_status);
            assert_eq!(
                reuse_approval_assessment(assessment.clone()) == WriteTicketReuseApproval::Reusable,
                case.expected_reuse
            );
            assert_eq!(
                record_run_approval_admission(assessment) == RecordRunApprovalAdmission::Admitted,
                case.expected_admission
            );
        }
    }
}
