use std::collections::BTreeSet;

use volicord_types::{
    AcceptanceCriterion, AcceptanceCriterionId, ArtifactAvailability, ArtifactIntegrityStatus,
    CloseReadinessBlocker, CloseReadinessBlockerCategory, EvidenceCoverageItem,
    EvidenceCoverageState, EvidenceGateState, EvidenceGateSummary, EvidenceRequirement,
    EvidenceStatus, EvidenceSummary, EvidenceTarget, StateRecordKind, StateRecordRef,
};

use super::evidence::{
    evidence_item_has_no_support, evidence_status_for_items, unique_artifact_refs,
    unique_state_record_refs,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CloseEvidenceRunFacts {
    pub(crate) project_id: String,
    pub(crate) task_id: String,
    pub(crate) change_unit_id: Option<String>,
    pub(crate) scope_revision: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CloseEvidenceSummaryFacts {
    pub(crate) task_project_id: String,
    pub(crate) task_id: String,
    pub(crate) task_change_unit_id: Option<String>,
    pub(crate) task_scope_revision: u64,
    pub(crate) summary_change_unit_id: Option<String>,
    pub(crate) updated_by_run_declared: bool,
    pub(crate) updated_by_run: Option<CloseEvidenceRunFacts>,
    pub(crate) updated_by_run_ref: Option<StateRecordRef>,
    pub(crate) coverage_items: Vec<EvidenceCoverageItem>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum CloseEvidenceIssueKind {
    Missing,
    Unsupported,
    Stale,
    AgentReportOnly,
    InsufficientProvenance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CloseEvidenceObservationDisposition {
    StrongSupported,
    UnsupportedRelevance,
    CooperativeAgentReport,
    Weak,
    Stale,
}

pub(crate) fn interpret_close_evidence_item(
    item: &EvidenceCoverageItem,
    required_criterion_ids: &BTreeSet<String>,
    has_current_close_basis: bool,
    observation_dispositions: &[CloseEvidenceObservationDisposition],
) -> Option<CloseEvidenceIssueKind> {
    let EvidenceTarget::AcceptanceCriterion {
        acceptance_criterion_id,
    } = &item.target
    else {
        return None;
    };
    if !required_criterion_ids.contains(acceptance_criterion_id.as_str()) {
        return None;
    }
    if item.coverage_state != EvidenceCoverageState::Supported {
        return Some(if item.coverage_state == EvidenceCoverageState::Stale {
            CloseEvidenceIssueKind::Stale
        } else if evidence_item_has_no_support(item) {
            CloseEvidenceIssueKind::Missing
        } else {
            CloseEvidenceIssueKind::Unsupported
        });
    }
    if !has_current_close_basis {
        return Some(CloseEvidenceIssueKind::Missing);
    }
    if observation_dispositions.is_empty() {
        return Some(CloseEvidenceIssueKind::InsufficientProvenance);
    }
    if observation_dispositions.contains(&CloseEvidenceObservationDisposition::StrongSupported) {
        return None;
    }
    if observation_dispositions.contains(&CloseEvidenceObservationDisposition::UnsupportedRelevance)
    {
        return Some(CloseEvidenceIssueKind::Unsupported);
    }
    let has_cooperative = observation_dispositions
        .contains(&CloseEvidenceObservationDisposition::CooperativeAgentReport);
    let has_weak = observation_dispositions.contains(&CloseEvidenceObservationDisposition::Weak);
    let has_stale = observation_dispositions.contains(&CloseEvidenceObservationDisposition::Stale);
    Some(if has_cooperative && !has_weak {
        CloseEvidenceIssueKind::AgentReportOnly
    } else if has_stale && !has_cooperative && !has_weak {
        CloseEvidenceIssueKind::Stale
    } else {
        CloseEvidenceIssueKind::InsufficientProvenance
    })
}

pub(crate) fn required_acceptance_criterion_ids(
    acceptance_criteria: &[AcceptanceCriterion],
) -> BTreeSet<String> {
    acceptance_criteria
        .iter()
        .filter(|criterion| criterion.evidence_requirement == EvidenceRequirement::Required)
        .map(|criterion| criterion.acceptance_criterion_id.as_str().to_owned())
        .collect()
}

pub(crate) fn evidence_summary_with_required_criteria(
    summary: Option<EvidenceSummary>,
    acceptance_criteria: &[AcceptanceCriterion],
) -> Option<EvidenceSummary> {
    let required = required_acceptance_criterion_ids(acceptance_criteria);
    evidence_summary_with_required_ids(summary, &required)
}

pub(crate) fn evidence_summary_with_required_ids(
    summary: Option<EvidenceSummary>,
    required: &BTreeSet<String>,
) -> Option<EvidenceSummary> {
    if summary.is_none() && required.is_empty() {
        return None;
    }
    let mut summary = summary.unwrap_or(EvidenceSummary {
        evidence_state: None,
        status: EvidenceStatus::Unknown,
        coverage_items: Vec::new(),
        artifact_refs: Vec::new(),
        observation_refs: Vec::new(),
        updated_by_run_ref: None,
    });
    for acceptance_criterion_id in required {
        if !summary.coverage_items.iter().any(|item| {
            matches!(
                &item.target,
                EvidenceTarget::AcceptanceCriterion {
                    acceptance_criterion_id: existing
                } if existing.as_str() == acceptance_criterion_id
            )
        }) {
            summary.coverage_items.push(EvidenceCoverageItem {
                target: EvidenceTarget::AcceptanceCriterion {
                    acceptance_criterion_id: AcceptanceCriterionId::new(acceptance_criterion_id),
                },
                coverage_state: EvidenceCoverageState::Unsupported,
                supporting_run_refs: Vec::new(),
                observation_refs: Vec::new(),
                supporting_artifact_refs: Vec::new(),
                gap_refs: Vec::new(),
            });
        }
    }
    summary.status = evidence_status_for_items(&summary.coverage_items);
    Some(summary)
}

pub(crate) fn project_close_evidence_summary(
    mut facts: CloseEvidenceSummaryFacts,
    required_criterion_ids: &BTreeSet<String>,
) -> Option<EvidenceSummary> {
    let evidence_scope_is_stale = if facts.updated_by_run_declared {
        facts.updated_by_run.as_ref().is_none_or(|run| {
            run.project_id != facts.task_project_id
                || run.task_id != facts.task_id
                || run.scope_revision != facts.task_scope_revision
                || run.change_unit_id != facts.task_change_unit_id
                || facts.summary_change_unit_id != facts.task_change_unit_id
        })
    } else {
        false
    };
    for item in &mut facts.coverage_items {
        if item.coverage_state == EvidenceCoverageState::Supported
            && (evidence_scope_is_stale
                || item.supporting_artifact_refs.iter().any(|artifact_ref| {
                    artifact_ref.availability != ArtifactAvailability::Available
                        || artifact_ref.integrity_status != ArtifactIntegrityStatus::Verified
                }))
        {
            item.coverage_state = EvidenceCoverageState::Stale;
        }
    }
    let summary = if facts.coverage_items.is_empty() {
        None
    } else {
        let artifact_refs = unique_artifact_refs(
            facts
                .coverage_items
                .iter()
                .flat_map(|item| item.supporting_artifact_refs.clone())
                .collect(),
        );
        let observation_refs = unique_state_record_refs(
            facts
                .coverage_items
                .iter()
                .flat_map(|item| item.observation_refs.clone())
                .collect(),
        );
        Some(EvidenceSummary {
            evidence_state: None,
            status: evidence_status_for_items(&facts.coverage_items),
            coverage_items: facts.coverage_items,
            artifact_refs,
            observation_refs,
            updated_by_run_ref: facts.updated_by_run_ref,
        })
    };
    evidence_summary_with_required_ids(summary, required_criterion_ids)
}

pub(crate) fn evaluate_evidence_gate(
    acceptance_criteria: &[AcceptanceCriterion],
    evidence_summary: Option<&EvidenceSummary>,
    close_blockers: &[CloseReadinessBlocker],
) -> EvidenceGateSummary {
    let required_ids = acceptance_criteria
        .iter()
        .filter(|criterion| criterion.evidence_requirement == EvidenceRequirement::Required)
        .map(|criterion| criterion.acceptance_criterion_id.as_str())
        .collect::<BTreeSet<_>>();
    let optional_ids = acceptance_criteria
        .iter()
        .filter(|criterion| criterion.evidence_requirement == EvidenceRequirement::Optional)
        .map(|criterion| criterion.acceptance_criterion_id.as_str())
        .collect::<BTreeSet<_>>();

    if required_ids.is_empty() && optional_ids.is_empty() {
        return EvidenceGateSummary {
            state: EvidenceGateState::NotRequired,
        };
    }

    let coverage_items = evidence_summary
        .map(|summary| summary.coverage_items.as_slice())
        .unwrap_or_default();
    let criterion_item = |criterion_id: &str| {
        coverage_items.iter().find(|item| {
            matches!(
                &item.target,
                EvidenceTarget::AcceptanceCriterion {
                    acceptance_criterion_id
                } if acceptance_criterion_id.as_str() == criterion_id
            )
        })
    };
    let required_items = coverage_items.iter().filter(|item| {
        matches!(
            &item.target,
            EvidenceTarget::AcceptanceCriterion {
                acceptance_criterion_id
            } if required_ids.contains(acceptance_criterion_id.as_str())
        )
    });
    let required_artifact_ids = required_items
        .clone()
        .flat_map(|item| item.supporting_artifact_refs.iter())
        .map(|artifact_ref| artifact_ref.artifact_id.as_str())
        .collect::<BTreeSet<_>>();

    let has_blocking_evidence_condition = close_blockers.iter().any(|blocker| {
        blocker.category == CloseReadinessBlockerCategory::Evidence
            || (blocker.category == CloseReadinessBlockerCategory::ArtifactAvailability
                && blocker.related_refs.iter().any(|record_ref| {
                    record_ref.record_kind == StateRecordKind::Artifact
                        && required_artifact_ids.contains(record_ref.record_id.as_str())
                }))
            || (blocker.category == CloseReadinessBlockerCategory::EvidenceProvenance
                && blocker.code != "evidence_provenance_stale")
    }) || required_items
        .clone()
        .any(|item| item.coverage_state == EvidenceCoverageState::Contradicted);
    if has_blocking_evidence_condition {
        return EvidenceGateSummary {
            state: EvidenceGateState::Blocked,
        };
    }

    let has_stale_evidence = close_blockers.iter().any(|blocker| {
        blocker.category == CloseReadinessBlockerCategory::EvidenceProvenance
            && blocker.code == "evidence_provenance_stale"
    }) || required_items
        .clone()
        .any(|item| item.coverage_state == EvidenceCoverageState::Stale);
    if has_stale_evidence {
        return EvidenceGateSummary {
            state: EvidenceGateState::Stale,
        };
    }

    let item_is_sufficient =
        |item: &EvidenceCoverageItem| item.coverage_state == EvidenceCoverageState::Supported;
    let item_has_recorded_evidence =
        |item: &EvidenceCoverageItem| !evidence_item_has_no_support(item);
    let has_evidence_claim_blocker = close_blockers
        .iter()
        .any(|blocker| blocker.category == CloseReadinessBlockerCategory::EvidenceClaim);

    if !required_ids.is_empty() {
        if !has_evidence_claim_blocker
            && required_ids
                .iter()
                .all(|criterion_id| criterion_item(criterion_id).is_some_and(item_is_sufficient))
        {
            return EvidenceGateSummary {
                state: EvidenceGateState::Sufficient,
            };
        }
        let any_required_evidence = required_ids.iter().any(|criterion_id| {
            criterion_item(criterion_id).is_some_and(item_has_recorded_evidence)
        });
        return EvidenceGateSummary {
            state: if any_required_evidence {
                EvidenceGateState::Partial
            } else {
                EvidenceGateState::RequiredMissing
            },
        };
    }

    let optional_items = optional_ids
        .iter()
        .filter_map(|criterion_id| criterion_item(criterion_id))
        .filter(|item| item_has_recorded_evidence(item))
        .collect::<Vec<_>>();
    if optional_items.is_empty() {
        return EvidenceGateSummary {
            state: EvidenceGateState::OptionalNone,
        };
    }
    EvidenceGateSummary {
        state: if optional_items.iter().all(|item| item_is_sufficient(item)) {
            EvidenceGateState::Sufficient
        } else {
            EvidenceGateState::Partial
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use volicord_types::{
        ArtifactAvailability, ArtifactId, ArtifactIntegrityStatus, ArtifactRef,
        EvidenceCoverageState, ProjectId, RecordId, RedactionState, RequiredNullable,
        StateRecordRef, TaskId,
    };

    fn criterion(id: &str, requirement: EvidenceRequirement) -> AcceptanceCriterion {
        AcceptanceCriterion {
            acceptance_criterion_id: AcceptanceCriterionId::new(id),
            statement: id.to_owned(),
            evidence_requirement: requirement,
        }
    }

    fn coverage(id: &str, state: EvidenceCoverageState, support: bool) -> EvidenceCoverageItem {
        EvidenceCoverageItem {
            target: EvidenceTarget::AcceptanceCriterion {
                acceptance_criterion_id: AcceptanceCriterionId::new(id),
            },
            coverage_state: state,
            supporting_run_refs: support
                .then(|| StateRecordRef {
                    record_kind: StateRecordKind::Run,
                    record_id: RecordId::new(format!("run_{id}")),
                    project_id: volicord_types::ProjectId::new("project_gate"),
                    task_id: Some(TaskId::new("task_gate")).into(),
                    produced_at_state_version: RequiredNullable::null(),
                })
                .into_iter()
                .collect(),
            observation_refs: Vec::new(),
            supporting_artifact_refs: Vec::new(),
            gap_refs: Vec::new(),
        }
    }

    fn summary(items: Vec<EvidenceCoverageItem>) -> EvidenceSummary {
        EvidenceSummary {
            evidence_state: None,
            status: evidence_status_for_items(&items),
            coverage_items: items,
            artifact_refs: Vec::new(),
            observation_refs: Vec::new(),
            updated_by_run_ref: None,
        }
    }

    fn blocker(category: CloseReadinessBlockerCategory, code: &str) -> CloseReadinessBlocker {
        CloseReadinessBlocker {
            category,
            code: code.to_owned(),
            message: code.to_owned(),
            related_refs: Vec::new(),
            next_actions: Vec::new(),
        }
    }

    fn coverage_with_artifact(
        id: &str,
        state: EvidenceCoverageState,
        artifact_id: &str,
    ) -> EvidenceCoverageItem {
        let mut item = coverage(id, state, true);
        item.supporting_artifact_refs.push(ArtifactRef {
            artifact_id: ArtifactId::new(artifact_id),
            project_id: ProjectId::new("project_gate"),
            task_id: TaskId::new("task_gate"),
            display_name: artifact_id.to_owned(),
            content_type: RequiredNullable::null(),
            sha256: RequiredNullable::null(),
            size_bytes: RequiredNullable::null(),
            integrity_status: ArtifactIntegrityStatus::Verified,
            redaction_state: RedactionState::None,
            availability: ArtifactAvailability::Missing,
            created_by_run_ref: RequiredNullable::null(),
            created_by_actor_source: RequiredNullable::null(),
            storage_ref: RequiredNullable::null(),
        });
        item
    }

    fn artifact_blocker(artifact_id: &str) -> CloseReadinessBlocker {
        let mut blocker = blocker(
            CloseReadinessBlockerCategory::ArtifactAvailability,
            "artifact_unavailable",
        );
        blocker.related_refs.push(StateRecordRef {
            record_kind: StateRecordKind::Artifact,
            record_id: RecordId::new(artifact_id),
            project_id: ProjectId::new("project_gate"),
            task_id: Some(TaskId::new("task_gate")).into(),
            produced_at_state_version: Some(1).into(),
        });
        blocker
    }

    #[test]
    fn close_evidence_interpretation_distinguishes_current_policy_outcomes() {
        let required = BTreeSet::from(["criterion_required".to_owned()]);
        let supported = coverage("criterion_required", EvidenceCoverageState::Supported, true);
        assert_eq!(
            interpret_close_evidence_item(
                &supported,
                &required,
                true,
                &[CloseEvidenceObservationDisposition::StrongSupported],
            ),
            None
        );
        assert_eq!(
            interpret_close_evidence_item(
                &supported,
                &required,
                true,
                &[CloseEvidenceObservationDisposition::UnsupportedRelevance],
            ),
            Some(CloseEvidenceIssueKind::Unsupported)
        );
        assert_eq!(
            interpret_close_evidence_item(
                &supported,
                &required,
                true,
                &[CloseEvidenceObservationDisposition::CooperativeAgentReport],
            ),
            Some(CloseEvidenceIssueKind::AgentReportOnly)
        );
        assert_eq!(
            interpret_close_evidence_item(
                &supported,
                &required,
                true,
                &[CloseEvidenceObservationDisposition::Stale],
            ),
            Some(CloseEvidenceIssueKind::Stale)
        );
        assert_eq!(
            interpret_close_evidence_item(
                &supported,
                &required,
                true,
                &[CloseEvidenceObservationDisposition::Weak],
            ),
            Some(CloseEvidenceIssueKind::InsufficientProvenance)
        );
    }

    #[test]
    fn close_evidence_summary_projection_applies_scope_and_required_target_policy() {
        let required = BTreeSet::from(["criterion_required".to_owned()]);
        let facts = CloseEvidenceSummaryFacts {
            task_project_id: "project_gate".to_owned(),
            task_id: "task_gate".to_owned(),
            task_change_unit_id: Some("change_gate".to_owned()),
            task_scope_revision: 4,
            summary_change_unit_id: Some("change_gate".to_owned()),
            updated_by_run_declared: true,
            updated_by_run: Some(CloseEvidenceRunFacts {
                project_id: "project_gate".to_owned(),
                task_id: "task_gate".to_owned(),
                change_unit_id: Some("change_gate".to_owned()),
                scope_revision: 4,
            }),
            updated_by_run_ref: None,
            coverage_items: vec![coverage(
                "criterion_required",
                EvidenceCoverageState::Supported,
                true,
            )],
        };
        let current = project_close_evidence_summary(facts.clone(), &required).unwrap();
        assert_eq!(
            current.coverage_items[0].coverage_state,
            EvidenceCoverageState::Supported
        );

        let mut stale = facts;
        stale.updated_by_run.as_mut().unwrap().scope_revision = 3;
        let stale = project_close_evidence_summary(stale, &required).unwrap();
        assert_eq!(
            stale.coverage_items[0].coverage_state,
            EvidenceCoverageState::Stale
        );

        let missing = project_close_evidence_summary(
            CloseEvidenceSummaryFacts {
                task_project_id: "project_gate".to_owned(),
                task_id: "task_gate".to_owned(),
                task_change_unit_id: Some("change_gate".to_owned()),
                task_scope_revision: 4,
                summary_change_unit_id: None,
                updated_by_run_declared: false,
                updated_by_run: None,
                updated_by_run_ref: None,
                coverage_items: Vec::new(),
            },
            &required,
        )
        .unwrap();
        assert_eq!(
            missing.coverage_items[0].coverage_state,
            EvidenceCoverageState::Unsupported
        );
    }

    #[test]
    fn evidence_gate_matrix_uses_current_close_policy() {
        struct Case {
            name: &'static str,
            criteria: Vec<AcceptanceCriterion>,
            summary: Option<EvidenceSummary>,
            blockers: Vec<CloseReadinessBlocker>,
            expected: EvidenceGateState,
        }
        let cases = vec![
            Case {
                name: "not_required",
                criteria: vec![criterion("criterion", EvidenceRequirement::NotRequired)],
                summary: None,
                blockers: Vec::new(),
                expected: EvidenceGateState::NotRequired,
            },
            Case {
                name: "optional_none",
                criteria: vec![criterion("criterion", EvidenceRequirement::Optional)],
                summary: None,
                blockers: Vec::new(),
                expected: EvidenceGateState::OptionalNone,
            },
            Case {
                name: "optional_supported",
                criteria: vec![criterion("criterion", EvidenceRequirement::Optional)],
                summary: Some(summary(vec![coverage(
                    "criterion",
                    EvidenceCoverageState::Supported,
                    true,
                )])),
                blockers: Vec::new(),
                expected: EvidenceGateState::Sufficient,
            },
            Case {
                name: "optional_stale",
                criteria: vec![criterion("criterion", EvidenceRequirement::Optional)],
                summary: Some(summary(vec![coverage(
                    "criterion",
                    EvidenceCoverageState::Stale,
                    true,
                )])),
                blockers: Vec::new(),
                expected: EvidenceGateState::Partial,
            },
            Case {
                name: "required_missing",
                criteria: vec![criterion("criterion", EvidenceRequirement::Required)],
                summary: None,
                blockers: vec![blocker(
                    CloseReadinessBlockerCategory::EvidenceClaim,
                    "evidence_claim_missing",
                )],
                expected: EvidenceGateState::RequiredMissing,
            },
            Case {
                name: "required_partial",
                criteria: vec![criterion("criterion", EvidenceRequirement::Required)],
                summary: Some(summary(vec![coverage(
                    "criterion",
                    EvidenceCoverageState::Partial,
                    true,
                )])),
                blockers: vec![blocker(
                    CloseReadinessBlockerCategory::EvidenceClaim,
                    "evidence_claim_unsupported",
                )],
                expected: EvidenceGateState::Partial,
            },
            Case {
                name: "one_supported_one_missing",
                criteria: vec![
                    criterion("criterion", EvidenceRequirement::Required),
                    criterion("criterion_missing", EvidenceRequirement::Required),
                ],
                summary: Some(summary(vec![coverage(
                    "criterion",
                    EvidenceCoverageState::Supported,
                    true,
                )])),
                blockers: vec![blocker(
                    CloseReadinessBlockerCategory::EvidenceClaim,
                    "evidence_claim_missing",
                )],
                expected: EvidenceGateState::Partial,
            },
            Case {
                name: "required_sufficient",
                criteria: vec![criterion("criterion", EvidenceRequirement::Required)],
                summary: Some(summary(vec![coverage(
                    "criterion",
                    EvidenceCoverageState::Supported,
                    true,
                )])),
                blockers: Vec::new(),
                expected: EvidenceGateState::Sufficient,
            },
            Case {
                name: "required_ignores_optional_contradiction",
                criteria: vec![
                    criterion("criterion", EvidenceRequirement::Required),
                    criterion("criterion_optional", EvidenceRequirement::Optional),
                ],
                summary: Some(summary(vec![
                    coverage("criterion", EvidenceCoverageState::Supported, true),
                    coverage(
                        "criterion_optional",
                        EvidenceCoverageState::Contradicted,
                        true,
                    ),
                ])),
                blockers: Vec::new(),
                expected: EvidenceGateState::Sufficient,
            },
            Case {
                name: "required_not_applicable_without_support",
                criteria: vec![criterion("criterion", EvidenceRequirement::Required)],
                summary: Some(summary(vec![coverage(
                    "criterion",
                    EvidenceCoverageState::NotApplicable,
                    false,
                )])),
                blockers: vec![blocker(
                    CloseReadinessBlockerCategory::EvidenceClaim,
                    "evidence_claim_missing",
                )],
                expected: EvidenceGateState::RequiredMissing,
            },
            Case {
                name: "supported_with_close_claim_blocker",
                criteria: vec![criterion("criterion", EvidenceRequirement::Required)],
                summary: Some(summary(vec![coverage(
                    "criterion",
                    EvidenceCoverageState::Supported,
                    true,
                )])),
                blockers: vec![blocker(
                    CloseReadinessBlockerCategory::EvidenceClaim,
                    "evidence_claim_missing",
                )],
                expected: EvidenceGateState::Partial,
            },
            Case {
                name: "stale",
                criteria: vec![criterion("criterion", EvidenceRequirement::Required)],
                summary: Some(summary(vec![coverage(
                    "criterion",
                    EvidenceCoverageState::Stale,
                    true,
                )])),
                blockers: vec![blocker(
                    CloseReadinessBlockerCategory::EvidenceProvenance,
                    "evidence_provenance_stale",
                )],
                expected: EvidenceGateState::Stale,
            },
            Case {
                name: "contradicted",
                criteria: vec![criterion("criterion", EvidenceRequirement::Required)],
                summary: Some(summary(vec![coverage(
                    "criterion",
                    EvidenceCoverageState::Contradicted,
                    true,
                )])),
                blockers: Vec::new(),
                expected: EvidenceGateState::Blocked,
            },
            Case {
                name: "insufficient_provenance",
                criteria: vec![criterion("criterion", EvidenceRequirement::Required)],
                summary: Some(summary(vec![coverage(
                    "criterion",
                    EvidenceCoverageState::Supported,
                    true,
                )])),
                blockers: vec![blocker(
                    CloseReadinessBlockerCategory::EvidenceProvenance,
                    "evidence_provenance_insufficient",
                )],
                expected: EvidenceGateState::Blocked,
            },
            Case {
                name: "required_artifact_unavailable",
                criteria: vec![criterion("criterion", EvidenceRequirement::Required)],
                summary: Some(summary(vec![coverage_with_artifact(
                    "criterion",
                    EvidenceCoverageState::Supported,
                    "artifact_required",
                )])),
                blockers: vec![artifact_blocker("artifact_required")],
                expected: EvidenceGateState::Blocked,
            },
            Case {
                name: "unrelated_artifact_unavailable",
                criteria: vec![criterion("criterion", EvidenceRequirement::Required)],
                summary: Some(summary(vec![coverage(
                    "criterion",
                    EvidenceCoverageState::Supported,
                    true,
                )])),
                blockers: vec![artifact_blocker("artifact_unrelated")],
                expected: EvidenceGateState::Sufficient,
            },
        ];
        for case in cases {
            assert_eq!(
                evaluate_evidence_gate(&case.criteria, case.summary.as_ref(), &case.blockers).state,
                case.expected,
                "case {}",
                case.name
            );
        }
    }
}
