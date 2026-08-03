use crate::artifact::persistent_artifact_is_verified_current;
use crate::identity::allocate_risk_id;
use crate::pipeline::{CorePipelineError, CoreService};
use crate::policy::evidence::state_record_ref_identity_key;
use crate::policy::evidence_target::run_record_matches_close_basis_context;
use crate::record_refs::state_ref;
use crate::recording::RecordRunInput;
use crate::task_state::{normalize_display_string_list, normalize_display_text};
use crate::write_ticket::{normalized_string_set, AdmissibleStoredWriteTicket};
use std::collections::{BTreeMap, BTreeSet};
use volicord_store::core_pipeline::{ChangeUnitStatus, CoreProjectStore, RunRecord, TaskRecord};
use volicord_types::schema::{
    ArtifactRef, CurrentCloseBasis, RequiredNullable, ResidualRisk, SensitiveActionRequirement,
    StateRecordRef,
};
use volicord_types::values::{StateRecordKind, UtcTimestamp};
#[derive(Debug)]
pub(crate) enum RecordRunCloseBasisError {
    Core(CorePipelineError),
    Validation {
        field: &'static str,
        message: &'static str,
    },
}

impl From<CorePipelineError> for RecordRunCloseBasisError {
    fn from(error: CorePipelineError) -> Self {
        Self::Core(error)
    }
}

impl From<serde_json::Error> for RecordRunCloseBasisError {
    fn from(error: serde_json::Error) -> Self {
        Self::Core(CorePipelineError::from(error))
    }
}

fn close_basis_validation_error<T>(
    field: &'static str,
    message: &'static str,
) -> Result<T, RecordRunCloseBasisError> {
    Err(RecordRunCloseBasisError::Validation { field, message })
}

fn close_basis_store_error(error: volicord_store::error::StoreError) -> RecordRunCloseBasisError {
    RecordRunCloseBasisError::Core(CorePipelineError::from(error))
}

pub(crate) struct RecordRunCloseBasisContext<'a> {
    pub(crate) service: &'a CoreService,
    pub(crate) store: &'a CoreProjectStore<'a>,
    pub(crate) request: &'a RecordRunInput,
    pub(crate) task: &'a TaskRecord,
    pub(crate) run_ref: &'a StateRecordRef,
    pub(crate) write_ticket_scope: Option<&'a AdmissibleStoredWriteTicket>,
    pub(crate) evidence_summary_ref: Option<StateRecordRef>,
    pub(crate) registered_artifacts: &'a [ArtifactRef],
    pub(crate) close_basis_revision: u64,
    pub(crate) snapshot_state_version: u64,
    pub(crate) now: &'a UtcTimestamp,
}

pub(super) struct CloseBasisRefResolutionContext<'a> {
    pub(super) store: &'a CoreProjectStore<'a>,
    pub(super) request: &'a RecordRunInput,
    pub(super) current_scope_revision: u64,
    pub(super) field: &'static str,
    pub(super) run_ref: &'a StateRecordRef,
    pub(super) evidence_summary_ref: Option<&'a StateRecordRef>,
    pub(super) registered_artifacts: &'a [ArtifactRef],
    pub(super) snapshot_state_version: u64,
}

pub(crate) fn build_record_run_close_basis(
    context: RecordRunCloseBasisContext<'_>,
) -> Result<Option<CurrentCloseBasis>, RecordRunCloseBasisError> {
    let RecordRunCloseBasisContext {
        service,
        store,
        request,
        task,
        run_ref,
        write_ticket_scope,
        evidence_summary_ref,
        registered_artifacts,
        close_basis_revision,
        snapshot_state_version,
        now,
    } = context;
    let Some(assessment) = request.close_assessment() else {
        return Ok(None);
    };
    if assessment.result_summary.trim().is_empty() {
        return close_basis_validation_error(
            "close_assessment.result_summary",
            "close_assessment.result_summary must not be empty",
        );
    }

    let mut result_refs = assessment.result_refs.clone();
    result_refs.push(run_ref.clone());
    result_refs.push(canonical_close_basis_ref(
        request,
        StateRecordKind::ChangeUnit,
        request.change_unit_id().as_str(),
        snapshot_state_version,
    ));
    if let Some(ref evidence_summary_ref) = evidence_summary_ref {
        result_refs.push(evidence_summary_ref.clone());
    }
    let result_refs = canonicalize_close_basis_refs(
        CloseBasisRefResolutionContext {
            store,
            request,
            current_scope_revision: task.scope_revision,
            field: "close_assessment.result_refs",
            run_ref,
            evidence_summary_ref: evidence_summary_ref.as_ref(),
            registered_artifacts,
            snapshot_state_version,
        },
        &result_refs,
    )?;

    if request.dry_run().is_requested() {
        for risk in &assessment.residual_risks {
            validate_residual_risk_input(
                CloseBasisRefResolutionContext {
                    store,
                    request,
                    current_scope_revision: task.scope_revision,
                    field: "close_assessment.residual_risks[].source_refs",
                    run_ref,
                    evidence_summary_ref: evidence_summary_ref.as_ref(),
                    registered_artifacts,
                    snapshot_state_version,
                },
                risk,
            )?;
        }
        return Ok(None);
    }

    let mut allocated_risk_ids = BTreeSet::new();
    let mut residual_risks = Vec::new();
    for risk in &assessment.residual_risks {
        let source_refs = validate_residual_risk_input(
            CloseBasisRefResolutionContext {
                store,
                request,
                current_scope_revision: task.scope_revision,
                field: "close_assessment.residual_risks[].source_refs",
                run_ref,
                evidence_summary_ref: evidence_summary_ref.as_ref(),
                registered_artifacts,
                snapshot_state_version,
            },
            risk,
        )?;
        let risk_id = allocate_risk_id(service.durable_id_generator(), &allocated_risk_ids)
            .map_err(RecordRunCloseBasisError::Core)?;
        allocated_risk_ids.insert(risk_id.as_str().to_owned());
        residual_risks.push(ResidualRisk {
            risk_id,
            summary: normalize_display_text(&risk.summary),
            consequence: normalize_display_text(&risk.consequence),
            acceptance_required: risk.acceptance_required,
            source_refs,
        });
    }
    let sensitive_action_requirements =
        current_sensitive_action_requirements(store, request, task, run_ref, write_ticket_scope)?;
    let derived_sensitive_categories = sensitive_category_summary(&sensitive_action_requirements);
    let caller_sensitive_categories =
        normalize_display_string_list(&assessment.sensitive_categories);
    if caller_sensitive_categories != derived_sensitive_categories {
        return close_basis_validation_error(
            "close_assessment.sensitive_categories",
            "close_assessment.sensitive_categories must match Core-derived sensitive requirements",
        );
    }

    Ok(Some(CurrentCloseBasis {
        close_basis_revision,
        scope_revision: task.scope_revision,
        task_id: request.task_id().clone(),
        change_unit_id: request.change_unit_id().clone(),
        baseline_ref: Some(request.baseline_ref().clone()).into(),
        result_summary: normalize_display_text(&assessment.result_summary),
        result_refs,
        evidence_refs: evidence_summary_ref.iter().cloned().collect(),
        evidence_summary_ref: evidence_summary_ref.into(),
        residual_risks,
        sensitive_categories: derived_sensitive_categories,
        sensitive_action_requirements,
        recovery_constraints: normalize_display_string_list(&assessment.recovery_constraints),
        source_run_ref: RequiredNullable::some(run_ref.clone()),
        shaping_checkpoint_ref: RequiredNullable::null(),
        shaping_decision_application_refs: Vec::new(),
        updated_at: now.clone(),
    }))
}

pub(super) fn current_sensitive_action_requirements(
    store: &CoreProjectStore,
    request: &RecordRunInput,
    task: &TaskRecord,
    run_ref: &StateRecordRef,
    write_ticket_scope: Option<&AdmissibleStoredWriteTicket>,
) -> Result<Vec<SensitiveActionRequirement>, RecordRunCloseBasisError> {
    let mut requirements = previous_current_sensitive_action_requirements(store, request, task)?;
    if let Some(ticket) = write_ticket_scope {
        if let Some(requirement) = sensitive_action_requirement_from_write_ticket(run_ref, ticket)?
        {
            requirements.push(requirement);
        }
    }
    sorted_unique_sensitive_requirements(requirements)
}

pub(super) fn previous_current_sensitive_action_requirements(
    store: &CoreProjectStore,
    request: &RecordRunInput,
    task: &TaskRecord,
) -> Result<Vec<SensitiveActionRequirement>, RecordRunCloseBasisError> {
    let task_revision = store
        .task_revision_record(request.task_id())
        .map_err(close_basis_store_error)?;
    let Some(previous_basis) = task_revision.and_then(|record| record.current_close_basis) else {
        return Ok(Vec::new());
    };
    if previous_basis.task_id == *request.task_id()
        && previous_basis.change_unit_id == *request.change_unit_id()
        && previous_basis.scope_revision == task.scope_revision
        && previous_basis.close_basis_revision == task.close_basis_revision
        && previous_basis.baseline_ref.as_ref() == Some(request.baseline_ref())
    {
        Ok(previous_basis.sensitive_action_requirements)
    } else {
        Ok(Vec::new())
    }
}

pub(super) fn sensitive_action_requirement_from_write_ticket(
    run_ref: &StateRecordRef,
    ticket: &AdmissibleStoredWriteTicket,
) -> Result<Option<SensitiveActionRequirement>, RecordRunCloseBasisError> {
    let semantic = ticket.semantic_facts();
    let scope = semantic.attempt_scope();
    let sensitive_categories = normalized_string_set(&scope.sensitive_categories);
    let validity_basis = semantic.validity_basis();
    if sensitive_categories.is_empty() && validity_basis.approval_basis_refs.is_empty() {
        return Ok(None);
    }
    let action_kind = scope.intended_operation.trim().to_owned();
    if action_kind.is_empty() {
        return Err(RecordRunCloseBasisError::Core(
            CorePipelineError::Invariant {
                detail: format!(
                    "write ticket `{}` has an empty intended operation after Store decoding",
                    ticket.write_ticket_id()
                ),
            },
        ));
    }
    let normalized_paths = scope
        .intended_paths
        .iter()
        .map(|path| path.as_str().to_owned())
        .collect();
    Ok(Some(SensitiveActionRequirement {
        action_kind,
        normalized_paths,
        sensitive_categories,
        baseline_ref: scope.baseline_ref.clone().into(),
        change_unit_id: scope.change_unit_id.clone(),
        source_run_ref: run_ref.clone(),
        source_write_ticket_ref: state_ref(
            StateRecordKind::WriteTicket,
            ticket.write_ticket_id().as_str(),
            semantic.project_id(),
            Some(&validity_basis.task_id),
            Some(
                run_ref
                    .produced_at_state_version
                    .as_ref()
                    .copied()
                    .unwrap_or(semantic.basis_state_version()),
            ),
        ),
    }))
}

pub(super) fn sorted_unique_sensitive_requirements(
    requirements: Vec<SensitiveActionRequirement>,
) -> Result<Vec<SensitiveActionRequirement>, RecordRunCloseBasisError> {
    let mut unique = BTreeMap::new();
    for requirement in requirements {
        unique
            .entry(sensitive_requirement_key(&requirement)?)
            .or_insert(requirement);
    }
    Ok(unique.into_values().collect())
}

pub(super) fn sensitive_requirement_key(
    requirement: &SensitiveActionRequirement,
) -> Result<(String, String, String, Option<String>, String), RecordRunCloseBasisError> {
    Ok((
        requirement.action_kind.clone(),
        serde_json::to_string(&requirement.normalized_paths)?,
        serde_json::to_string(&requirement.sensitive_categories)?,
        requirement
            .baseline_ref
            .as_ref()
            .map(|baseline_ref| baseline_ref.as_str().to_owned()),
        requirement.change_unit_id.as_str().to_owned(),
    ))
}

pub(super) fn sensitive_category_summary(
    requirements: &[SensitiveActionRequirement],
) -> Vec<String> {
    requirements
        .iter()
        .flat_map(|requirement| requirement.sensitive_categories.iter().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub(super) fn validate_residual_risk_input(
    context: CloseBasisRefResolutionContext<'_>,
    risk: &volicord_types::schema::ResidualRiskInput,
) -> Result<Vec<StateRecordRef>, RecordRunCloseBasisError> {
    if risk.summary.trim().is_empty() {
        return close_basis_validation_error(
            "close_assessment.residual_risks.summary",
            "residual risk summary must not be empty",
        );
    }
    if risk.consequence.trim().is_empty() {
        return close_basis_validation_error(
            "close_assessment.residual_risks.consequence",
            "residual risk consequence must not be empty",
        );
    }
    canonicalize_close_basis_refs(context, &risk.source_refs)
}

pub(super) fn canonicalize_close_basis_refs(
    context: CloseBasisRefResolutionContext<'_>,
    refs: &[StateRecordRef],
) -> Result<Vec<StateRecordRef>, RecordRunCloseBasisError> {
    let mut normalized = BTreeMap::new();
    for record_ref in refs {
        let normalized_ref = resolve_close_basis_ref(&context, record_ref)?;
        let key = close_basis_ref_identity_key(&normalized_ref);
        normalized.entry(key).or_insert(normalized_ref);
    }
    Ok(normalized.into_values().collect())
}

pub(super) fn resolve_close_basis_ref(
    context: &CloseBasisRefResolutionContext<'_>,
    record_ref: &StateRecordRef,
) -> Result<StateRecordRef, RecordRunCloseBasisError> {
    let request = context.request;
    if record_ref.record_id.as_str().trim().is_empty() {
        return close_basis_validation_error(
            context.field,
            "close assessment refs must use non-empty record_id values",
        );
    }
    if !matches!(
        record_ref.record_kind,
        StateRecordKind::Run
            | StateRecordKind::Artifact
            | StateRecordKind::EvidenceSummary
            | StateRecordKind::ChangeUnit
    ) {
        return close_basis_validation_error(
            context.field,
            "close assessment refs may only use run, artifact, evidence_summary, or change_unit record_kind",
        );
    }
    if record_ref.project_id != *request.project_id() {
        return close_basis_validation_error(
            context.field,
            "close assessment refs must belong to the request project",
        );
    }
    if record_ref.task_id.as_ref() != Some(request.task_id()) {
        return close_basis_validation_error(
            context.field,
            "close assessment refs must belong to the request Task",
        );
    }

    match record_ref.record_kind {
        StateRecordKind::Run => resolve_close_basis_run_ref(context, record_ref),
        StateRecordKind::ChangeUnit => resolve_close_basis_change_unit_ref(context, record_ref),
        StateRecordKind::EvidenceSummary => {
            resolve_close_basis_evidence_summary_ref(context, record_ref)
        }
        StateRecordKind::Artifact => resolve_close_basis_artifact_ref(context, record_ref),
        _ => unreachable!("unsupported close-basis record kind rejected above"),
    }
}

pub(super) fn resolve_close_basis_run_ref(
    context: &CloseBasisRefResolutionContext<'_>,
    record_ref: &StateRecordRef,
) -> Result<StateRecordRef, RecordRunCloseBasisError> {
    let request = context.request;
    if record_ref.record_id == context.run_ref.record_id {
        return Ok(context.run_ref.clone());
    }
    let record = context
        .store
        .run_record(record_ref.record_id.as_str())
        .map_err(close_basis_store_error)?;
    let compatible = match record.as_ref() {
        Some(record) => run_record_is_close_basis_compatible(context, record)?,
        None => false,
    };
    if !compatible {
        return close_basis_validation_error(
            context.field,
            "Run refs in close_assessment must exist for the request Task, current Change Unit, current scope revision, and current baseline",
        );
    }
    let record = record.expect("compatible run record is present");
    Ok(canonical_close_basis_ref(
        request,
        StateRecordKind::Run,
        &record.run_id,
        context.snapshot_state_version,
    ))
}

pub(super) fn run_record_is_close_basis_compatible(
    context: &CloseBasisRefResolutionContext<'_>,
    record: &RunRecord,
) -> Result<bool, RecordRunCloseBasisError> {
    let Some(change_unit_id) = record.change_unit_id.as_deref() else {
        return Ok(false);
    };
    if !run_record_matches_close_basis_context(
        record,
        context.request.project_id(),
        context.request.task_id(),
        context.request.change_unit_id().as_str(),
        context.current_scope_revision,
        Some(context.request.baseline_ref().as_str()),
    ) {
        return Ok(false);
    }
    Ok(context
        .store
        .current_change_unit(context.request.task_id())
        .map_err(close_basis_store_error)?
        .as_ref()
        .is_some_and(|record| {
            record.change_unit_id == change_unit_id
                && record.status == ChangeUnitStatus::Active
                && record.is_current
        }))
}

pub(super) fn resolve_close_basis_change_unit_ref(
    context: &CloseBasisRefResolutionContext<'_>,
    record_ref: &StateRecordRef,
) -> Result<StateRecordRef, RecordRunCloseBasisError> {
    let request = context.request;
    let record = context
        .store
        .change_unit_record(request.task_id(), record_ref.record_id.as_str())
        .map_err(close_basis_store_error)?;
    if record.as_ref().is_none_or(|record| {
        record.project_id != request.project_id().as_str()
            || record.task_id != request.task_id().as_str()
            || record.change_unit_id != request.change_unit_id().as_str()
            || record.status != ChangeUnitStatus::Active
            || !record.is_current
    }) {
        return close_basis_validation_error(
            context.field,
            "Change Unit refs in close_assessment must identify the current Change Unit",
        );
    }
    let record = record.expect("current Change Unit record is present");
    Ok(canonical_close_basis_ref(
        request,
        StateRecordKind::ChangeUnit,
        &record.change_unit_id,
        context.snapshot_state_version,
    ))
}

pub(super) fn resolve_close_basis_evidence_summary_ref(
    context: &CloseBasisRefResolutionContext<'_>,
    record_ref: &StateRecordRef,
) -> Result<StateRecordRef, RecordRunCloseBasisError> {
    let request = context.request;
    if context
        .evidence_summary_ref
        .is_some_and(|summary_ref| summary_ref.record_id == record_ref.record_id)
    {
        return Ok(context
            .evidence_summary_ref
            .expect("checked evidence summary ref is present")
            .clone());
    }
    let record = context
        .store
        .evidence_summary_record(record_ref.record_id.as_str())
        .map_err(close_basis_store_error)?;
    let latest = context
        .store
        .latest_evidence_summary(request.task_id())
        .map_err(close_basis_store_error)?;
    if record.as_ref().is_none_or(|record| {
        record.project_id != request.project_id().as_str()
            || record.task_id != request.task_id().as_str()
            || latest
                .as_ref()
                .is_none_or(|latest| latest.evidence_summary_id != record.evidence_summary_id)
    }) {
        return close_basis_validation_error(
            context.field,
            "Evidence Summary refs in close_assessment must identify the current Task evidence summary",
        );
    }
    let record = record.expect("current Evidence Summary record is present");
    Ok(canonical_close_basis_ref(
        request,
        StateRecordKind::EvidenceSummary,
        &record.evidence_summary_id,
        record.produced_at_state_version,
    ))
}

pub(super) fn resolve_close_basis_artifact_ref(
    context: &CloseBasisRefResolutionContext<'_>,
    record_ref: &StateRecordRef,
) -> Result<StateRecordRef, RecordRunCloseBasisError> {
    let request = context.request;
    if context
        .registered_artifacts
        .iter()
        .any(|artifact| artifact.artifact_id.as_str() == record_ref.record_id.as_str())
    {
        return Ok(canonical_close_basis_ref(
            request,
            StateRecordKind::Artifact,
            record_ref.record_id.as_str(),
            context.snapshot_state_version,
        ));
    }
    let record = context
        .store
        .artifact_record(record_ref.record_id.as_str())
        .map_err(close_basis_store_error)?;
    let owner_link_exists = context
        .store
        .artifact_has_task_owner_link(record_ref.record_id.as_str(), request.task_id().as_str())
        .map_err(close_basis_store_error)?;
    if record
        .as_ref()
        .map(|record| {
            let available = persistent_artifact_is_verified_current(context.store, record)?;
            Ok::<_, CorePipelineError>(
                record.project_id == request.project_id().as_str()
                    && record.task_id == request.task_id().as_str()
                    && available
                    && owner_link_exists,
            )
        })
        .transpose()?
        .unwrap_or(false)
    {
        let record = record.expect("verified artifact record is present");
        Ok(canonical_close_basis_ref(
            request,
            StateRecordKind::Artifact,
            &record.artifact_id,
            context.snapshot_state_version,
        ))
    } else {
        close_basis_validation_error(
            context.field,
            "Artifact refs in close_assessment must identify verified available artifacts owned by the request Task",
        )
    }
}

pub(super) fn canonical_close_basis_ref(
    request: &RecordRunInput,
    record_kind: StateRecordKind,
    record_id: &str,
    snapshot_state_version: u64,
) -> StateRecordRef {
    state_ref(
        record_kind,
        record_id,
        request.project_id(),
        Some(request.task_id()),
        Some(snapshot_state_version),
    )
}

pub(super) fn close_basis_ref_identity_key(
    record_ref: &StateRecordRef,
) -> (String, String, String) {
    state_record_ref_identity_key(record_ref)
}
