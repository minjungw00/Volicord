use super::*;

fn criterion(id: &str, evidence_requirement: EvidenceRequirement) -> AcceptanceCriterion {
    AcceptanceCriterion {
        acceptance_criterion_id: AcceptanceCriterionId::new(id),
        statement: format!("Criterion {id}"),
        evidence_requirement,
    }
}

fn coverage(id: &str, coverage_state: EvidenceCoverageState) -> EvidenceCoverageItem {
    EvidenceCoverageItem {
        target: EvidenceTarget::AcceptanceCriterion {
            acceptance_criterion_id: AcceptanceCriterionId::new(id),
        },
        coverage_state,
        supporting_run_refs: vec![test_state_record_ref(
            StateRecordKind::Run,
            &format!("run_{id}"),
            PROJECT_ID,
            "task_evidence_gate",
            Some(1),
        )],
        observation_refs: Vec::new(),
        supporting_artifact_refs: Vec::new(),
        gap_refs: Vec::new(),
    }
}

fn coverage_without_support(
    id: &str,
    coverage_state: EvidenceCoverageState,
) -> EvidenceCoverageItem {
    let mut item = coverage(id, coverage_state);
    item.supporting_run_refs.clear();
    item
}

fn coverage_with_artifact(
    id: &str,
    coverage_state: EvidenceCoverageState,
    artifact_id: &str,
) -> EvidenceCoverageItem {
    let mut item = coverage(id, coverage_state);
    item.supporting_artifact_refs.push(ArtifactRef {
        artifact_id: ArtifactId::new(artifact_id),
        project_id: ProjectId::new(PROJECT_ID),
        task_id: TaskId::new("task_evidence_gate"),
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

fn evidence_summary(items: Vec<EvidenceCoverageItem>) -> EvidenceSummary {
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
        control_surface: None,
        can_resolve_in_chat: false,
        outside_chat_action_required: false,
        related_refs: Vec::new(),
        next_actions: Vec::new(),
    }
}

fn artifact_blocker(artifact_id: &str) -> CloseReadinessBlocker {
    let mut blocker = blocker(
        CloseReadinessBlockerCategory::ArtifactAvailability,
        "artifact_unavailable",
    );
    blocker.related_refs.push(test_state_record_ref(
        StateRecordKind::Artifact,
        artifact_id,
        PROJECT_ID,
        "task_evidence_gate",
        Some(1),
    ));
    blocker
}

#[test]
fn evidence_gate_state_table_matches_criterion_and_close_policy() {
    struct Case {
        name: &'static str,
        criteria: Vec<AcceptanceCriterion>,
        evidence: Option<EvidenceSummary>,
        blockers: Vec<CloseReadinessBlocker>,
        expected: EvidenceGateState,
    }

    let cases = vec![
        Case {
            name: "not_required",
            criteria: vec![criterion("criterion_1", EvidenceRequirement::NotRequired)],
            evidence: None,
            blockers: Vec::new(),
            expected: EvidenceGateState::NotRequired,
        },
        Case {
            name: "optional_none",
            criteria: vec![criterion("criterion_1", EvidenceRequirement::Optional)],
            evidence: None,
            blockers: Vec::new(),
            expected: EvidenceGateState::OptionalNone,
        },
        Case {
            name: "optional_supported",
            criteria: vec![criterion("criterion_1", EvidenceRequirement::Optional)],
            evidence: Some(evidence_summary(vec![coverage(
                "criterion_1",
                EvidenceCoverageState::Supported,
            )])),
            blockers: Vec::new(),
            expected: EvidenceGateState::Sufficient,
        },
        Case {
            name: "optional_stale_is_partial_not_blocking",
            criteria: vec![criterion("criterion_1", EvidenceRequirement::Optional)],
            evidence: Some(evidence_summary(vec![coverage(
                "criterion_1",
                EvidenceCoverageState::Stale,
            )])),
            blockers: Vec::new(),
            expected: EvidenceGateState::Partial,
        },
        Case {
            name: "required_missing",
            criteria: vec![criterion("criterion_1", EvidenceRequirement::Required)],
            evidence: None,
            blockers: vec![blocker(
                CloseReadinessBlockerCategory::EvidenceClaim,
                "evidence_claim_missing",
            )],
            expected: EvidenceGateState::RequiredMissing,
        },
        Case {
            name: "partial_coverage",
            criteria: vec![criterion("criterion_1", EvidenceRequirement::Required)],
            evidence: Some(evidence_summary(vec![coverage(
                "criterion_1",
                EvidenceCoverageState::Partial,
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
                criterion("criterion_1", EvidenceRequirement::Required),
                criterion("criterion_2", EvidenceRequirement::Required),
            ],
            evidence: Some(evidence_summary(vec![coverage(
                "criterion_1",
                EvidenceCoverageState::Supported,
            )])),
            blockers: vec![blocker(
                CloseReadinessBlockerCategory::EvidenceClaim,
                "evidence_claim_missing",
            )],
            expected: EvidenceGateState::Partial,
        },
        Case {
            name: "sufficient",
            criteria: vec![criterion("criterion_1", EvidenceRequirement::Required)],
            evidence: Some(evidence_summary(vec![coverage(
                "criterion_1",
                EvidenceCoverageState::Supported,
            )])),
            blockers: Vec::new(),
            expected: EvidenceGateState::Sufficient,
        },
        Case {
            name: "required_supported_ignores_optional_contradiction",
            criteria: vec![
                criterion("criterion_1", EvidenceRequirement::Required),
                criterion("criterion_2", EvidenceRequirement::Optional),
            ],
            evidence: Some(evidence_summary(vec![
                coverage("criterion_1", EvidenceCoverageState::Supported),
                coverage("criterion_2", EvidenceCoverageState::Contradicted),
            ])),
            blockers: Vec::new(),
            expected: EvidenceGateState::Sufficient,
        },
        Case {
            name: "required_not_applicable_is_not_sufficient",
            criteria: vec![criterion("criterion_1", EvidenceRequirement::Required)],
            evidence: Some(evidence_summary(vec![coverage_without_support(
                "criterion_1",
                EvidenceCoverageState::NotApplicable,
            )])),
            blockers: vec![blocker(
                CloseReadinessBlockerCategory::EvidenceClaim,
                "evidence_claim_missing",
            )],
            expected: EvidenceGateState::RequiredMissing,
        },
        Case {
            name: "supported_without_current_close_basis_is_not_sufficient",
            criteria: vec![criterion("criterion_1", EvidenceRequirement::Required)],
            evidence: Some(evidence_summary(vec![coverage(
                "criterion_1",
                EvidenceCoverageState::Supported,
            )])),
            blockers: vec![blocker(
                CloseReadinessBlockerCategory::EvidenceClaim,
                "evidence_claim_missing",
            )],
            expected: EvidenceGateState::Partial,
        },
        Case {
            name: "stale",
            criteria: vec![criterion("criterion_1", EvidenceRequirement::Required)],
            evidence: Some(evidence_summary(vec![coverage(
                "criterion_1",
                EvidenceCoverageState::Stale,
            )])),
            blockers: vec![blocker(
                CloseReadinessBlockerCategory::EvidenceProvenance,
                "evidence_provenance_stale",
            )],
            expected: EvidenceGateState::Stale,
        },
        Case {
            name: "contradicted",
            criteria: vec![criterion("criterion_1", EvidenceRequirement::Required)],
            evidence: Some(evidence_summary(vec![coverage(
                "criterion_1",
                EvidenceCoverageState::Contradicted,
            )])),
            blockers: vec![blocker(
                CloseReadinessBlockerCategory::EvidenceClaim,
                "evidence_claim_unsupported",
            )],
            expected: EvidenceGateState::Blocked,
        },
        Case {
            name: "insufficient_provenance",
            criteria: vec![criterion("criterion_1", EvidenceRequirement::Required)],
            evidence: Some(evidence_summary(vec![coverage(
                "criterion_1",
                EvidenceCoverageState::Supported,
            )])),
            blockers: vec![blocker(
                CloseReadinessBlockerCategory::EvidenceProvenance,
                "evidence_provenance_insufficient",
            )],
            expected: EvidenceGateState::Blocked,
        },
        Case {
            name: "artifact_unavailable",
            criteria: vec![criterion("criterion_1", EvidenceRequirement::Required)],
            evidence: Some(evidence_summary(vec![coverage_with_artifact(
                "criterion_1",
                EvidenceCoverageState::Supported,
                "artifact_required",
            )])),
            blockers: vec![artifact_blocker("artifact_required")],
            expected: EvidenceGateState::Blocked,
        },
        Case {
            name: "unrelated_close_result_artifact_does_not_block_evidence_gate",
            criteria: vec![criterion("criterion_1", EvidenceRequirement::Required)],
            evidence: Some(evidence_summary(vec![coverage(
                "criterion_1",
                EvidenceCoverageState::Supported,
            )])),
            blockers: vec![artifact_blocker("artifact_unrelated_result")],
            expected: EvidenceGateState::Sufficient,
        },
    ];

    for case in cases {
        let actual = evaluate_evidence_gate(&case.criteria, case.evidence.as_ref(), &case.blockers);
        assert_eq!(actual.state, case.expected, "case {}", case.name);
    }
}
