use super::blockers::close_blocker;
use super::change_control::task_ref_for_close;
use super::facts::{required_criteria_for_close_context, CloseReadinessFacts};
use super::guidance::{close_guidance, CloseGuidance};
use super::service::CloseReadinessRequest;
use crate::methods::evidence_facts::{
    projected_evidence_observation_provenance_facts, stored_evidence_observation_capture_relevance,
    stored_evidence_observation_provenance_facts,
};
use crate::methods::{
    change_unit_ref, parse_owner_storage_value, persistent_artifact_is_verified_current, state_ref,
    store_error_plan, PlanError,
};
use crate::pipeline::CorePipelineError;
use crate::policy::close_readiness_evidence::{
    interpret_close_evidence_item, CloseEvidenceIssueKind, CloseEvidenceObservationDisposition,
};
use crate::policy::evidence::{
    evidence_item_related_refs, state_record_ref_identity_key, unique_state_record_refs,
};
use crate::policy::evidence_provenance::{classify_evidence_provenance, EvidenceProvenanceClass};
use crate::policy::evidence_relevance::capture_relevance_is_unsupported;
use crate::policy::evidence_target::{
    projected_observation_matches_close_basis, stored_observation_matches_close_basis,
    EvidenceObservationBasis,
};
use std::collections::{BTreeMap, BTreeSet};
use volicord_store::core_pipeline::{CoreProjectStore, ProjectStateHeader};
use volicord_types::ids::BaselineRef;
use volicord_types::schema::{
    CloseReadinessBlocker, EvidenceCoverageItem, EvidenceTarget, StateRecordRef,
};
use volicord_types::values::{
    ArtifactAvailability, ArtifactIntegrityStatus, CloseReadinessBlockerCategory,
    EvidenceCoverageState, EvidenceRelevanceStatus, RedactionState, StateRecordKind,
};

pub(super) fn completion_blockers(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseReadinessRequest,
    context: &CloseReadinessFacts,
) -> Result<Vec<CloseReadinessBlocker>, PlanError> {
    let change_unit_ref = context.current_change_unit.as_ref().map(|record| {
        change_unit_ref(
            &request.envelope.project_id,
            &request.task_id,
            record,
            project_state.state_version,
        )
    });
    let mut blockers =
        close_evidence_blockers(store, project_state, request, context, change_unit_ref)?;
    let unavailable_artifacts =
        unavailable_close_artifact_refs(store, project_state, request, context)?;
    if !unavailable_artifacts.is_empty() {
        let task_ref = task_ref_for_close(request, project_state.state_version);
        blockers.push(close_blocker(
            CloseReadinessBlockerCategory::ArtifactAvailability,
            "artifact_unavailable",
            "A required close artifact is missing, unavailable, or incompatible with storage.",
            unavailable_artifacts,
            vec![close_guidance(
                CloseGuidance::RepairArtifact,
                vec![task_ref],
            )],
        ));
    }
    Ok(blockers)
}

fn evidence_target_required_by(target: &EvidenceTarget, required: &BTreeSet<String>) -> bool {
    matches!(
        target,
        EvidenceTarget::AcceptanceCriterion {
            acceptance_criterion_id
        } if required.contains(acceptance_criterion_id.as_str())
    )
}

#[derive(Debug)]
struct CloseEvidenceIssue {
    kind: CloseEvidenceIssueKind,
    related_refs: Vec<StateRecordRef>,
}

fn close_evidence_blockers(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseReadinessRequest,
    context: &CloseReadinessFacts,
    change_unit_ref: Option<StateRecordRef>,
) -> Result<Vec<CloseReadinessBlocker>, PlanError> {
    let Some(summary) = context.evidence_summary.as_ref() else {
        return Ok(Vec::new());
    };
    let mut grouped: BTreeMap<CloseEvidenceIssueKind, Vec<StateRecordRef>> = BTreeMap::new();
    for item in &summary.coverage_items {
        if let Some(issue) =
            close_evidence_issue_for_item(store, project_state, request, context, item)?
        {
            grouped
                .entry(issue.kind)
                .or_default()
                .extend(issue.related_refs);
        }
    }

    let required_refs = change_unit_ref.into_iter().collect::<Vec<_>>();
    let mut blockers = Vec::new();
    for kind in [
        CloseEvidenceIssueKind::Missing,
        CloseEvidenceIssueKind::Unsupported,
        CloseEvidenceIssueKind::Stale,
        CloseEvidenceIssueKind::AgentReportOnly,
        CloseEvidenceIssueKind::InsufficientProvenance,
    ] {
        let Some(related_refs) = grouped.remove(&kind) else {
            continue;
        };
        let category = match kind {
            CloseEvidenceIssueKind::Missing | CloseEvidenceIssueKind::Unsupported => {
                CloseReadinessBlockerCategory::EvidenceClaim
            }
            CloseEvidenceIssueKind::Stale
            | CloseEvidenceIssueKind::AgentReportOnly
            | CloseEvidenceIssueKind::InsufficientProvenance => {
                CloseReadinessBlockerCategory::EvidenceProvenance
            }
        };
        let (code, message) = match kind {
            CloseEvidenceIssueKind::Missing => (
                "evidence_claim_missing",
                "One or more required close evidence claims are missing.",
            ),
            CloseEvidenceIssueKind::Unsupported => (
                "evidence_claim_unsupported",
                "One or more required close evidence claims are unsupported.",
            ),
            CloseEvidenceIssueKind::Stale => (
                "evidence_provenance_stale",
                "Evidence provenance exists but is stale against the current close basis.",
            ),
            CloseEvidenceIssueKind::AgentReportOnly => (
                "evidence_agent_report_only",
                "Required close evidence is supported only by cooperative agent reports.",
            ),
            CloseEvidenceIssueKind::InsufficientProvenance => (
                "evidence_provenance_insufficient",
                "Required close evidence lacks sufficient source provenance.",
            ),
        };
        blockers.push(close_blocker(
            category,
            code,
            message,
            unique_state_record_refs(related_refs),
            vec![close_guidance(
                CloseGuidance::RecordRequiredEvidence,
                required_refs.clone(),
            )],
        ));
    }
    Ok(blockers)
}

fn close_evidence_issue_for_item(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseReadinessRequest,
    context: &CloseReadinessFacts,
    item: &EvidenceCoverageItem,
) -> Result<Option<CloseEvidenceIssue>, PlanError> {
    let EvidenceTarget::AcceptanceCriterion {
        acceptance_criterion_id,
    } = &item.target
    else {
        return Ok(None);
    };
    let required_criteria = required_criteria_for_close_context(context)?;
    if !required_criteria.contains(acceptance_criterion_id.as_str()) {
        return Ok(None);
    }
    let Some(basis) = context.current_close_basis.as_ref() else {
        return Ok(
            interpret_close_evidence_item(item, required_criteria, false, &[]).map(|kind| {
                CloseEvidenceIssue {
                    kind,
                    related_refs: evidence_item_related_refs(item),
                }
            }),
        );
    };
    if item.coverage_state != EvidenceCoverageState::Supported || item.observation_refs.is_empty() {
        return Ok(
            interpret_close_evidence_item(item, required_criteria, true, &[]).map(|kind| {
                CloseEvidenceIssue {
                    kind,
                    related_refs: evidence_item_related_refs(item),
                }
            }),
        );
    }

    let mut dispositions = Vec::new();
    let evidence_state_version = basis
        .evidence_summary_ref
        .as_ref()
        .and_then(|record_ref| record_ref.produced_at_state_version.as_ref().copied());
    for observation_ref in &item.observation_refs {
        if observation_ref.record_kind != StateRecordKind::EvidenceObservation
            || observation_ref.project_id != request.envelope.project_id
            || observation_ref.task_id.as_ref() != Some(&request.task_id)
        {
            dispositions.push(CloseEvidenceObservationDisposition::Weak);
            continue;
        }
        if evidence_state_version.is_some_and(|state_version| {
            observation_ref.produced_at_state_version.as_ref() != Some(&state_version)
        }) {
            dispositions.push(CloseEvidenceObservationDisposition::Stale);
            continue;
        }
        if let Some(observation) =
            context
                .projected_evidence_observations
                .iter()
                .find(|observation| {
                    observation.observation_id.as_str() == observation_ref.record_id.as_str()
                })
        {
            if observation.project_id != request.envelope.project_id
                || observation.task_id != request.task_id
                || !projected_observation_matches_close_basis(observation, basis, &item.target)
            {
                dispositions.push(CloseEvidenceObservationDisposition::Stale);
                continue;
            }
            if capture_relevance_is_unsupported(
                observation.producer_anchor.producer_kind,
                &observation.relevance_assessment,
            ) {
                dispositions.push(CloseEvidenceObservationDisposition::UnsupportedRelevance);
                continue;
            }
            let facts = projected_evidence_observation_provenance_facts(
                store,
                observation,
                &EvidenceObservationBasis {
                    project_id: &request.envelope.project_id,
                    task_id: &request.task_id,
                    change_unit_id: basis.change_unit_id.as_str(),
                    scope_revision: basis.scope_revision,
                    baseline_ref: basis.baseline_ref.as_ref().map(BaselineRef::as_str),
                    target: &item.target,
                    now: &context.now,
                },
                &context.projected_artifacts,
            )?;
            dispositions.push(match classify_evidence_provenance(&facts) {
                EvidenceProvenanceClass::Strong => {
                    CloseEvidenceObservationDisposition::StrongSupported
                }
                EvidenceProvenanceClass::CooperativeAgentReport => {
                    CloseEvidenceObservationDisposition::CooperativeAgentReport
                }
                EvidenceProvenanceClass::Weak => CloseEvidenceObservationDisposition::Weak,
            });
            continue;
        }
        let record = store
            .evidence_observation_record(observation_ref.record_id.as_str())
            .map_err(|error| store_error_plan(&request.envelope, project_state, error))?;
        let Some(record) = record else {
            dispositions.push(CloseEvidenceObservationDisposition::Weak);
            continue;
        };
        if record.project_id != request.envelope.project_id.as_str()
            || record.task_id != request.task_id.as_str()
            || !stored_observation_matches_close_basis(&record, basis, &item.target)
        {
            dispositions.push(CloseEvidenceObservationDisposition::Stale);
            continue;
        }
        if stored_evidence_observation_capture_relevance(&record)?
            .is_some_and(|status| status != EvidenceRelevanceStatus::Supported)
        {
            dispositions.push(CloseEvidenceObservationDisposition::UnsupportedRelevance);
            continue;
        }
        let facts = stored_evidence_observation_provenance_facts(
            store,
            &record,
            &EvidenceObservationBasis {
                project_id: &request.envelope.project_id,
                task_id: &request.task_id,
                change_unit_id: basis.change_unit_id.as_str(),
                scope_revision: basis.scope_revision,
                baseline_ref: basis.baseline_ref.as_ref().map(BaselineRef::as_str),
                target: &item.target,
                now: &context.now,
            },
        )?;
        dispositions.push(match classify_evidence_provenance(&facts) {
            EvidenceProvenanceClass::Strong => CloseEvidenceObservationDisposition::StrongSupported,
            EvidenceProvenanceClass::CooperativeAgentReport => {
                CloseEvidenceObservationDisposition::CooperativeAgentReport
            }
            EvidenceProvenanceClass::Weak => CloseEvidenceObservationDisposition::Weak,
        });
    }

    Ok(
        interpret_close_evidence_item(item, required_criteria, true, &dispositions).map(|kind| {
            CloseEvidenceIssue {
                kind,
                related_refs: evidence_item_related_refs(item),
            }
        }),
    )
}

fn unavailable_close_artifact_refs(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseReadinessRequest,
    context: &CloseReadinessFacts,
) -> Result<Vec<StateRecordRef>, PlanError> {
    let mut seen = BTreeSet::new();
    let mut unavailable = Vec::new();
    let required_criteria = required_criteria_for_close_context(context)?;
    if let Some(evidence_summary) = context.evidence_summary.as_ref() {
        for artifact_ref in evidence_summary
            .coverage_items
            .iter()
            .filter(|item| evidence_target_required_by(&item.target, required_criteria))
            .flat_map(|item| item.supporting_artifact_refs.iter())
        {
            let state_ref = state_ref(
                StateRecordKind::Artifact,
                artifact_ref.artifact_id.as_str(),
                &request.envelope.project_id,
                Some(&request.task_id),
                Some(project_state.state_version),
            );
            if !seen.insert(state_record_ref_identity_key(&state_ref)) {
                continue;
            }
            if artifact_ref.availability != ArtifactAvailability::Available {
                unavailable.push(state_ref);
                continue;
            }
            if context.projected_artifacts.iter().any(|projected| {
                projected == artifact_ref
                    && projected.integrity_status == ArtifactIntegrityStatus::Verified
            }) {
                continue;
            }
            let stored = store
                .artifact_record(artifact_ref.artifact_id.as_str())
                .map_err(|error| store_error_plan(&request.envelope, project_state, error))?;
            let Some(stored) = stored else {
                unavailable.push(state_ref);
                continue;
            };
            let owner_link_exists = store
                .artifact_has_task_owner_link(
                    artifact_ref.artifact_id.as_str(),
                    request.task_id.as_str(),
                )
                .map_err(|error| store_error_plan(&request.envelope, project_state, error))?;
            let stored_available = persistent_artifact_is_verified_current(store, &stored)?;
            let stored_redaction_state: RedactionState = parse_owner_storage_value(
                "artifacts",
                stored.artifact_id.clone(),
                "redaction_state",
                &stored.redaction_state,
            )?;
            let artifact_sha256 = artifact_ref.sha256.as_ref();
            let artifact_size_bytes = artifact_ref.size_bytes.as_ref().copied();
            if stored.project_id != request.envelope.project_id.as_str()
                || stored.task_id != request.task_id.as_str()
                || !stored_available
                || artifact_ref.integrity_status != ArtifactIntegrityStatus::Verified
                || stored.sha256.as_deref() != artifact_sha256.map(String::as_str)
                || stored.size_bytes != artifact_size_bytes
                || stored_redaction_state != artifact_ref.redaction_state
                || !owner_link_exists
            {
                unavailable.push(state_ref);
            }
        }
    }
    if let Some(basis) = context.current_close_basis.as_ref() {
        for record_ref in basis
            .result_refs
            .iter()
            .chain(
                basis
                    .residual_risks
                    .iter()
                    .flat_map(|risk| risk.source_refs.iter()),
            )
            .filter(|record_ref| record_ref.record_kind == StateRecordKind::Artifact)
        {
            if !seen.insert(state_record_ref_identity_key(record_ref)) {
                continue;
            }
            if close_basis_artifact_ref_unavailable(
                store,
                request,
                record_ref,
                project_state,
                context,
            )? {
                unavailable.push(record_ref.clone());
            }
        }
    }
    Ok(unavailable)
}

fn close_basis_artifact_ref_unavailable(
    store: &CoreProjectStore,
    request: &CloseReadinessRequest,
    record_ref: &StateRecordRef,
    project_state: &ProjectStateHeader,
    context: &CloseReadinessFacts,
) -> Result<bool, PlanError> {
    if let Some(artifact_ref) = context
        .projected_artifacts
        .iter()
        .find(|artifact_ref| artifact_ref.artifact_id.as_str() == record_ref.record_id.as_str())
    {
        return Ok(record_ref.project_id != request.envelope.project_id
            || record_ref.task_id.as_ref() != Some(&request.task_id)
            || artifact_ref.project_id != request.envelope.project_id
            || artifact_ref.task_id != request.task_id
            || artifact_ref.availability != ArtifactAvailability::Available
            || artifact_ref.integrity_status != ArtifactIntegrityStatus::Verified);
    }
    let stored = store
        .artifact_record(record_ref.record_id.as_str())
        .map_err(|error| store_error_plan(&request.envelope, project_state, error))?;
    let owner_link_exists = store
        .artifact_has_task_owner_link(record_ref.record_id.as_str(), request.task_id.as_str())
        .map_err(|error| store_error_plan(&request.envelope, project_state, error))?;
    Ok(stored
        .as_ref()
        .map(|record| {
            let available = persistent_artifact_is_verified_current(store, record)?;
            let unavailable = record.project_id != request.envelope.project_id.as_str()
                || record.task_id != request.task_id.as_str()
                || !available
                || !owner_link_exists;
            Ok::<_, CorePipelineError>(unavailable)
        })
        .transpose()?
        .unwrap_or(true))
}

#[cfg(test)]
#[path = "tests/evidence.rs"]
mod tests;
