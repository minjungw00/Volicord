//! UserAction construction, materialization, authority, and projection services.

use super::{
    artifact_ref_from_verified_record, checked_derived_expiration, decision_rejected_response,
    decode_required_json, normalize_display_text, parse_owner_storage_value,
    persistent_artifact_is_verified_current, state_ref, store_error_response,
    stored_refs_to_state_refs, validation_plan_error, PlanError, StoredScope,
};
use crate::pipeline::{CorePipelineError, CoreResult, CoreService, VerifiedInvocationContext};
use crate::policy::close_readiness::{
    current_acceptance_required_risk_ids, is_terminal_lifecycle, UserActionAuthority,
};
use crate::policy::user_action_relevance::{
    user_action_blocks_operation, user_action_keeps_task_waiting, UserActionOperationContext,
};
use crate::policy::write_ticket::normalize_sensitive_action_scope;
use chrono::Duration;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use volicord_store::core_pipeline::{
    ChangeUnitRecord, CoreProjectStore, CoreStorageMutation, EffectiveUserActionRecord,
    ProjectStateHeader, TaskRecord, UserActionMutation, UserActionRequestInsert,
};
use volicord_store::StoreError;
use volicord_types::ids::{
    ArtifactId, BaselineRef, ChangeUnitId, DurableIdKind, ProjectId, RiskId, TaskId,
    UnrecordedChangeId, UserActionOptionId, UserActionRequestId,
};
use volicord_types::schema::{
    AgentSafeUserActionRequestSummary, ArtifactRef, EvidenceTarget, PersistedUserActionRequest,
    PersistedUserActionResolution, RequiredNullable, StateRecordRef, ToolEnvelope, UserActionBasis,
    UserActionBasisCoordinates, UserActionChoiceBasis, UserActionChoiceDraft,
    UserActionChoiceRequestBody, UserActionDraft, UserActionEvidenceObservationBasis,
    UserActionEvidenceObservationDraft, UserActionEvidenceObservationRequestBody, UserActionOption,
    UserActionOptionInput, UserActionRequest, UserActionRequestBody, UserActionResolutionBody,
    USER_ACTION_EVIDENCE_OBSERVATION_TTL_MINUTES,
};
use volicord_types::values::{
    JudgmentKind, JudgmentResolutionOutcome, MethodName, StateRecordKind, UserActionBasisStatus,
    UserActionKind, UserActionOptionAction, UserActionRequiredFor, UserActionStatus, UtcTimestamp,
};

/// Typed semantic intent supplied by a Core method that needs one current UserAction.
#[derive(Debug, Clone)]
pub(super) struct UserActionIntent {
    pub(super) task_id: TaskId,
    pub(super) change_unit_id: Option<ChangeUnitId>,
    pub(super) action: UserActionDraft,
    pub(super) required_for: Vec<UserActionRequiredFor>,
    pub(super) expires_at: RequiredNullable<UtcTimestamp>,
}

/// Current domain facts used to validate and construct one canonical UserAction.
pub(super) struct UserActionConstructionInput<'a> {
    pub(super) store: &'a CoreProjectStore<'a>,
    pub(super) project_state: &'a ProjectStateHeader,
    pub(super) envelope: &'a ToolEnvelope,
    pub(super) task: &'a TaskRecord,
    pub(super) current_change_unit: Option<&'a ChangeUnitRecord>,
    pub(super) operation_now: &'a UtcTimestamp,
    pub(super) intent: UserActionIntent,
}

/// Canonical typed UserAction ready for boundary materialization.
#[derive(Debug, Clone)]
pub(super) struct ConstructedUserAction {
    pub(super) task_id: TaskId,
    pub(super) coordinate_change_unit_id: Option<ChangeUnitId>,
    pub(super) body: UserActionRequestBody,
    pub(super) basis: UserActionBasis,
    pub(super) required_for: Vec<UserActionRequiredFor>,
    pub(super) expires_at: RequiredNullable<UtcTimestamp>,
    pub(super) created_at: UtcTimestamp,
}

/// Current Core operation that owns the newly constructed UserAction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum UserActionOrigin {
    DirectRequest,
    Reconciliation {
        unrecorded_change_id: UnrecordedChangeId,
    },
}

impl UserActionOrigin {
    fn source_method(&self) -> MethodName {
        match self {
            Self::DirectRequest => MethodName::RequestUserAction,
            Self::Reconciliation { .. } => MethodName::ReconcileChanges,
        }
    }

    fn metadata_json(&self) -> CoreResult<String> {
        #[derive(Serialize)]
        #[serde(deny_unknown_fields)]
        struct EmptyMetadata {}

        #[derive(Serialize)]
        #[serde(deny_unknown_fields)]
        struct ReconciliationMetadata<'a> {
            created_by: &'a str,
            unrecorded_change_id: &'a UnrecordedChangeId,
        }

        match self {
            Self::DirectRequest => {
                serde_json::to_string(&EmptyMetadata {}).map_err(CorePipelineError::from)
            }
            Self::Reconciliation {
                unrecorded_change_id,
            } => serde_json::to_string(&ReconciliationMetadata {
                created_by: MethodName::ReconcileChanges.as_str(),
                unrecorded_change_id,
            })
            .map_err(CorePipelineError::from),
        }
    }
}

/// Inputs that add operation identity and Store ownership to a constructed UserAction.
pub(super) struct UserActionMaterializationInput<'a> {
    pub(super) service: &'a CoreService,
    pub(super) store: &'a CoreProjectStore<'a>,
    pub(super) project_state: &'a ProjectStateHeader,
    pub(super) verified_invocation: &'a VerifiedInvocationContext,
    pub(super) envelope: &'a ToolEnvelope,
    pub(super) origin: UserActionOrigin,
    pub(super) constructed: ConstructedUserAction,
}

/// One typed public request plus the exact Store mutation that persists it.
#[derive(Debug, Clone)]
pub(super) struct MaterializedUserActionRequest {
    pub(super) request_ref: StateRecordRef,
    pub(super) public_request: UserActionRequest,
    pub(super) effective: EffectiveUserActionRecord,
    pub(super) mutation: CoreStorageMutation,
}

/// Validates semantic intent and current facts, then constructs the canonical typed action.
pub(super) fn construct_user_action(
    input: UserActionConstructionInput<'_>,
) -> Result<ConstructedUserAction, PlanError> {
    let UserActionConstructionInput {
        store,
        project_state,
        envelope,
        task,
        current_change_unit,
        operation_now,
        intent,
    } = input;
    let UserActionIntent {
        task_id,
        change_unit_id,
        action,
        required_for,
        expires_at,
    } = intent;

    action.validate_bounds().map_err(|error| {
        PlanError::Response(Box::new(
            super::validation_rejected(
                envelope.dry_run,
                Some(project_state.state_version),
                error.field(),
                error.message(),
            )
            .expect("user-action validation response should serialize"),
        ))
    })?;
    if required_for.is_empty() {
        return user_action_validation_error(
            envelope.dry_run,
            Some(project_state.state_version),
            "required_for",
            "required_for must contain at least one bounded operation",
        );
    }
    if required_for
        .iter()
        .enumerate()
        .any(|(index, target)| required_for[..index].contains(target))
    {
        return user_action_validation_error(
            envelope.dry_run,
            Some(project_state.state_version),
            "required_for",
            "required_for must not contain duplicate operation targets",
        );
    }
    if task.task_id != task_id.as_str() {
        return Err(PlanError::Core(CorePipelineError::Store(
            StoreError::corrupt_owner_state_value("tasks", &task.task_id, "task_id"),
        )));
    }
    validate_choice_affected_refs(
        &action,
        &envelope.project_id,
        &task_id,
        envelope.dry_run,
        project_state.state_version,
    )?;
    validate_required_for_compatibility(
        action.action_kind(),
        &required_for,
        envelope.dry_run,
        project_state.state_version,
    )?;

    let effective_expires_at = if matches!(&action, UserActionDraft::EvidenceObservation(_)) {
        if expires_at.is_some() {
            return user_action_validation_error(
                envelope.dry_run,
                Some(project_state.state_version),
                "expires_at",
                "evidence-observation actions require caller expires_at to be null",
            );
        }
        RequiredNullable::some(checked_derived_expiration(
            operation_now,
            Duration::minutes(USER_ACTION_EVIDENCE_OBSERVATION_TTL_MINUTES),
            envelope.dry_run,
            Some(project_state.state_version),
            "expires_at",
        )?)
    } else {
        expires_at
    };
    if effective_expires_at
        .as_ref()
        .is_some_and(|value| value.ensure_canonical_rfc3339_representable().is_err())
    {
        return user_action_validation_error(
            envelope.dry_run,
            Some(project_state.state_version),
            "expires_at",
            "expires_at must be representable as a canonical four-digit RFC 3339 timestamp",
        );
    }
    if effective_expires_at
        .as_ref()
        .is_some_and(|value| value <= operation_now)
    {
        return user_action_validation_error(
            envelope.dry_run,
            Some(project_state.state_version),
            "expires_at",
            "expires_at must be later than the request timestamp",
        );
    }

    if matches!(&action, UserActionDraft::EvidenceObservation(_))
        && (current_change_unit.is_none() || scope_baseline_is_missing(task)?)
    {
        return user_action_validation_error(
            envelope.dry_run,
            Some(project_state.state_version),
            "action",
            "evidence-observation actions require a current Change Unit and baseline",
        );
    }
    if let Some(change_unit_id) = change_unit_id.as_ref() {
        if store
            .change_unit_record(&task_id, change_unit_id.as_str())
            .map_err(CorePipelineError::from)?
            .is_none()
        {
            return user_action_validation_error(
                envelope.dry_run,
                Some(project_state.state_version),
                "change_unit_id",
                "change_unit_id must identify a Change Unit owned by the Task",
            );
        }
    }
    let action_needs_current_change_unit = matches!(
        action.action_kind(),
        UserActionKind::SensitiveApproval
            | UserActionKind::FinalAcceptance
            | UserActionKind::ResidualRiskAcceptance
            | UserActionKind::EvidenceObservation
    );
    if action_needs_current_change_unit {
        let Some(current) = current_change_unit else {
            return user_action_validation_error(
                envelope.dry_run,
                Some(project_state.state_version),
                "change_unit_id",
                "this action kind requires the current active Change Unit",
            );
        };
        if change_unit_id
            .as_ref()
            .is_some_and(|requested| requested.as_str() != current.change_unit_id)
        {
            return user_action_validation_error(
                envelope.dry_run,
                Some(project_state.state_version),
                "change_unit_id",
                "change_unit_id must match the current active Change Unit",
            );
        }
    }

    let coordinate_change_unit_id = change_unit_id.or_else(|| {
        current_change_unit.map(|record| ChangeUnitId::new(record.change_unit_id.clone()))
    });
    let scope = StoredScope::from_task(task)?;
    let coordinates = UserActionBasisCoordinates {
        task_id: task_id.clone(),
        change_unit_id: coordinate_change_unit_id.clone().into(),
        scope_revision: task.scope_revision,
        baseline_ref: scope.baseline_ref.map(BaselineRef::new).into(),
        created_at_state_version: project_state.state_version,
        compatibility_status: UserActionBasisStatus::Current,
    };
    let (body, basis) = canonical_request_body_and_basis(
        store,
        project_state,
        envelope,
        &task_id,
        &action,
        coordinates,
    )?;
    body.capture_form().map_err(|error| {
        PlanError::Response(Box::new(
            super::validation_rejected(
                envelope.dry_run,
                Some(project_state.state_version),
                error.field(),
                error.message(),
            )
            .expect("user-action validation response should serialize"),
        ))
    })?;

    Ok(ConstructedUserAction {
        task_id,
        coordinate_change_unit_id,
        body,
        basis,
        required_for,
        expires_at: effective_expires_at,
        created_at: operation_now.clone(),
    })
}

/// Adds canonical identity and serializes typed action values at the Store boundary.
pub(super) fn materialize_user_action_request(
    input: UserActionMaterializationInput<'_>,
) -> Result<MaterializedUserActionRequest, PlanError> {
    let UserActionMaterializationInput {
        service,
        store,
        project_state,
        verified_invocation,
        envelope,
        origin,
        constructed,
    } = input;
    let ConstructedUserAction {
        task_id,
        coordinate_change_unit_id,
        body,
        basis,
        required_for,
        expires_at,
        created_at,
    } = constructed;
    let action_kind = body.action_kind();
    let Some(source_idempotency_key) = envelope.idempotency_key.as_ref() else {
        return user_action_validation_error(
            envelope.dry_run,
            Some(project_state.state_version),
            "envelope.idempotency_key",
            "a committed user-action request requires an idempotency key",
        );
    };
    let source_method = origin.source_method();
    let metadata_json = origin.metadata_json().map_err(PlanError::Core)?;
    let request_id = allocate_user_action_request_id(service, store).map_err(PlanError::Core)?;
    let request_ref = state_ref(
        StateRecordKind::UserActionRequest,
        request_id.as_str(),
        &envelope.project_id,
        Some(&task_id),
        Some(project_state.state_version + 1),
    );
    let persisted = PersistedUserActionRequest {
        body: body.clone(),
        required_for: required_for.clone(),
        expires_at: expires_at.clone(),
    };
    let request_json = serde_json::to_string(&persisted)?;
    let basis_json = serde_json::to_string(&basis)?;
    let required_for_json = serde_json::to_string(&required_for)?;
    let requested_by_actor_source = verified_invocation.actor_source.to_canonical_string();
    let requested_at = created_at.to_string();
    let stored_expires_at = expires_at.as_ref().map(ToString::to_string);
    let public_request = UserActionRequest {
        user_action_request_id: request_id.clone(),
        project_id: envelope.project_id.clone(),
        task_id: task_id.clone(),
        change_unit_id: coordinate_change_unit_id.clone().into(),
        action_kind,
        status: UserActionStatus::Pending,
        body,
        basis,
        required_for,
        user_action_resolution_ref: RequiredNullable::null(),
        expires_at,
        created_at,
    };
    let effective = EffectiveUserActionRecord {
        request: volicord_store::core_pipeline::UserActionRequestRecord {
            project_id: envelope.project_id.as_str().to_owned(),
            user_action_request_id: request_id.as_str().to_owned(),
            task_id: task_id.as_str().to_owned(),
            change_unit_id: coordinate_change_unit_id
                .as_ref()
                .map(|id| id.as_str().to_owned()),
            action_kind,
            request_json: request_json.clone(),
            basis_json: basis_json.clone(),
            basis_status: UserActionBasisStatus::Current,
            required_for_json: required_for_json.clone(),
            requested_by_actor_source: requested_by_actor_source.clone(),
            source_method: source_method.as_str().to_owned(),
            source_idempotency_key: source_idempotency_key.as_str().to_owned(),
            requested_at: requested_at.clone(),
            expires_at: stored_expires_at.clone(),
            metadata_json: metadata_json.clone(),
        },
        resolution: None,
        status: UserActionStatus::Pending,
    };
    let mutation = CoreStorageMutation::UserAction(UserActionMutation::InsertRequest(
        UserActionRequestInsert {
            user_action_request_id: request_id.as_str().to_owned(),
            task_id: task_id.as_str().to_owned(),
            change_unit_id: coordinate_change_unit_id.map(ChangeUnitId::into_inner),
            action_kind,
            request_json,
            basis_json,
            basis_status: UserActionBasisStatus::Current,
            required_for_json,
            requested_by_actor_source,
            source_method: source_method.as_str().to_owned(),
            source_idempotency_key: source_idempotency_key.as_str().to_owned(),
            requested_at,
            expires_at: stored_expires_at,
            metadata_json,
        },
    ));
    Ok(MaterializedUserActionRequest {
        request_ref,
        public_request,
        effective,
        mutation,
    })
}

fn allocate_user_action_request_id(
    service: &CoreService,
    store: &CoreProjectStore,
) -> CoreResult<UserActionRequestId> {
    service
        .allocate_generated_id(DurableIdKind::UserActionRequest, |candidate| {
            store
                .user_action_request_id_exists(candidate)
                .map_err(CorePipelineError::from)
        })
        .map(UserActionRequestId::new)
}

fn canonical_request_body_and_basis(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    envelope: &ToolEnvelope,
    task_id: &TaskId,
    action: &UserActionDraft,
    coordinates: UserActionBasisCoordinates,
) -> Result<(UserActionRequestBody, UserActionBasis), PlanError> {
    match action {
        UserActionDraft::Choice(choice) => {
            let UserActionChoiceDraft {
                judgment_kind,
                presentation,
                question,
                options,
                context,
                affected_refs,
                sensitive_action_scope,
            } = choice.as_ref();
            let options = canonical_choice_options(
                *judgment_kind,
                options.as_ref().map(Vec::as_slice).unwrap_or_default(),
                envelope.locale.as_ref().map(String::as_str),
                envelope.dry_run,
                project_state.state_version,
            )?;
            if normalize_display_text(question).is_empty()
                || normalize_display_text(&context.summary).is_empty()
            {
                return user_action_validation_error(
                    envelope.dry_run,
                    Some(project_state.state_version),
                    "action.question",
                    "choice question and context summary must be non-empty",
                );
            }
            if *judgment_kind != JudgmentKind::SensitiveApproval && sensitive_action_scope.is_some()
            {
                return user_action_validation_error(
                    envelope.dry_run,
                    Some(project_state.state_version),
                    "action.sensitive_action_scope",
                    "sensitive_action_scope is only valid for sensitive approval",
                );
            }
            let sensitive_action_scope = sensitive_action_scope
                .as_ref()
                .map(|scope| {
                    normalize_sensitive_action_scope(&store.project_record().repo_root, scope)
                        .map_err(|_| {
                            PlanError::Response(Box::new(
                                super::validation_rejected(
                                    envelope.dry_run,
                                    Some(project_state.state_version),
                                    "action.sensitive_action_scope.intended_paths",
                                    "sensitive action paths must stay within the Product Repository",
                                )
                                .expect("validation response should serialize"),
                            ))
                        })
                })
                .transpose()?;
            if *judgment_kind == JudgmentKind::SensitiveApproval && sensitive_action_scope.is_none()
            {
                return user_action_validation_error(
                    envelope.dry_run,
                    Some(project_state.state_version),
                    "action.sensitive_action_scope",
                    "sensitive approval requires a bounded sensitive action scope",
                );
            }
            let close_coordinates =
                choice_close_coordinates(store, project_state, envelope, task_id, *judgment_kind)?;
            Ok((
                UserActionRequestBody::Choice(Box::new(UserActionChoiceRequestBody {
                    judgment_kind: *judgment_kind,
                    presentation: *presentation,
                    question: normalize_display_text(question),
                    options,
                    context: context.clone(),
                    affected_refs: affected_refs.clone(),
                    sensitive_action_scope: sensitive_action_scope.clone().into(),
                })),
                UserActionBasis::Choice(Box::new(UserActionChoiceBasis {
                    coordinates,
                    close_basis_revision: close_coordinates.close_basis_revision.into(),
                    result_refs: close_coordinates.result_refs,
                    residual_risk_ids: close_coordinates.residual_risk_ids,
                    sensitive_action_scope: sensitive_action_scope.into(),
                })),
            ))
        }
        UserActionDraft::EvidenceObservation(observation) => {
            let UserActionEvidenceObservationDraft {
                question,
                context_summary,
                target_candidates,
                artifact_candidate_ids,
            } = observation;
            if normalize_display_text(question).is_empty()
                || normalize_display_text(context_summary).is_empty()
            {
                return user_action_validation_error(
                    envelope.dry_run,
                    Some(project_state.state_version),
                    "action.question",
                    "observation question and context summary must be non-empty",
                );
            }
            if target_candidates.iter().collect::<BTreeSet<_>>().len() != target_candidates.len() {
                return user_action_validation_error(
                    envelope.dry_run,
                    Some(project_state.state_version),
                    "action.target_candidates",
                    "target candidates must not contain duplicates",
                );
            }
            if artifact_candidate_ids.iter().collect::<BTreeSet<_>>().len()
                != artifact_candidate_ids.len()
            {
                return user_action_validation_error(
                    envelope.dry_run,
                    Some(project_state.state_version),
                    "action.artifact_candidate_ids",
                    "artifact candidates must not contain duplicates",
                );
            }
            for target in target_candidates {
                validate_user_action_target(
                    store,
                    project_state,
                    envelope,
                    task_id,
                    target,
                    "action.target_candidates",
                )?;
            }
            let artifact_candidates = canonical_user_action_artifacts(
                store,
                project_state,
                envelope,
                task_id,
                artifact_candidate_ids,
                "action.artifact_candidate_ids",
            )?;
            Ok((
                UserActionRequestBody::EvidenceObservation(
                    UserActionEvidenceObservationRequestBody {
                        question: normalize_display_text(question),
                        context_summary: normalize_display_text(context_summary),
                        target_candidates: target_candidates.clone(),
                        artifact_candidates: artifact_candidates.clone(),
                    },
                ),
                UserActionBasis::EvidenceObservation(UserActionEvidenceObservationBasis {
                    coordinates,
                    target_candidates: target_candidates.clone(),
                    artifact_candidates,
                }),
            ))
        }
    }
}

fn canonical_choice_options(
    judgment_kind: JudgmentKind,
    caller_options: &[UserActionOptionInput],
    locale: Option<&str>,
    dry_run: bool,
    state_version: u64,
) -> Result<Vec<UserActionOption>, PlanError> {
    let authority_bearing = matches!(
        judgment_kind,
        JudgmentKind::ScopeDecision
            | JudgmentKind::SensitiveApproval
            | JudgmentKind::FinalAcceptance
            | JudgmentKind::ResidualRiskAcceptance
            | JudgmentKind::Cancellation
    );
    if authority_bearing {
        if !caller_options.is_empty() {
            return user_action_validation_error(
                dry_run,
                Some(state_version),
                "action.options",
                "authority-bearing actions use only Core-owned options",
            );
        }
        return Ok([
            UserActionOptionAction::Accept,
            UserActionOptionAction::Reject,
            UserActionOptionAction::Defer,
        ]
        .into_iter()
        .map(|machine_action| {
            let (label, description, consequence) =
                authority_option_copy(judgment_kind, machine_action, locale);
            UserActionOption {
                option_id: UserActionOptionId::new(match machine_action {
                    UserActionOptionAction::Accept => "accept",
                    UserActionOptionAction::Reject => "reject",
                    UserActionOptionAction::Defer => "defer",
                }),
                label,
                description,
                consequence,
                machine_action,
                resolution_outcome: machine_action.resolution_outcome(),
                is_default: machine_action == UserActionOptionAction::Accept,
            }
        })
        .collect());
    }
    if caller_options.is_empty() {
        return user_action_validation_error(
            dry_run,
            Some(state_version),
            "action.options",
            "product and technical choices require at least one caller-authored option",
        );
    }
    let mut ids = BTreeSet::new();
    if caller_options
        .iter()
        .any(|option| !ids.insert(option.option_id.as_str().to_owned()))
    {
        return user_action_validation_error(
            dry_run,
            Some(state_version),
            "action.options",
            "choice option IDs must be unique",
        );
    }
    if caller_options
        .iter()
        .filter(|option| option.is_default)
        .count()
        > 1
    {
        return user_action_validation_error(
            dry_run,
            Some(state_version),
            "action.options",
            "choice options may contain at most one default",
        );
    }
    Ok(caller_options
        .iter()
        .map(|option| UserActionOption {
            option_id: option.option_id.clone(),
            label: option.label.clone(),
            description: option.description.clone(),
            consequence: option.consequence.clone(),
            machine_action: UserActionOptionAction::Accept,
            resolution_outcome: JudgmentResolutionOutcome::Accepted,
            is_default: option.is_default,
        })
        .collect())
}

fn authority_option_copy(
    judgment_kind: JudgmentKind,
    action: UserActionOptionAction,
    locale: Option<&str>,
) -> (String, String, String) {
    let korean = locale
        .map(|locale| locale.to_ascii_lowercase().replace('_', "-"))
        .is_some_and(|locale| locale == "ko" || locale.starts_with("ko-"));
    let subject_en = match judgment_kind {
        JudgmentKind::ScopeDecision => "scope decision",
        JudgmentKind::SensitiveApproval => "sensitive action",
        JudgmentKind::FinalAcceptance => "final acceptance",
        JudgmentKind::ResidualRiskAcceptance => "residual risk",
        JudgmentKind::Cancellation => "task cancellation",
        JudgmentKind::ProductDecision => "product decision",
        JudgmentKind::TechnicalDecision => "technical decision",
    };
    let subject_ko = match judgment_kind {
        JudgmentKind::ScopeDecision => "범위 결정",
        JudgmentKind::SensitiveApproval => "민감 작업",
        JudgmentKind::FinalAcceptance => "최종 수락",
        JudgmentKind::ResidualRiskAcceptance => "잔여 위험",
        JudgmentKind::Cancellation => "작업 취소",
        JudgmentKind::ProductDecision => "제품 결정",
        JudgmentKind::TechnicalDecision => "기술 결정",
    };
    if korean {
        let (label, verb, outcome) = match action {
            UserActionOptionAction::Accept => ("수락", "수락합니다", "수락됨"),
            UserActionOptionAction::Reject => ("거부", "거부합니다", "거부됨"),
            UserActionOptionAction::Defer => ("보류", "나중으로 보류합니다", "보류됨"),
        };
        (
            label.to_owned(),
            format!("현재 근거에 따라 {subject_ko}을(를) {verb}."),
            format!("이 사용자 작업은 {outcome} 상태로 해결됩니다."),
        )
    } else {
        let (label, verb, outcome) = match action {
            UserActionOptionAction::Accept => ("Accept", "Accept", "accepted"),
            UserActionOptionAction::Reject => ("Reject", "Reject", "rejected"),
            UserActionOptionAction::Defer => ("Defer", "Defer", "deferred"),
        };
        (
            label.to_owned(),
            format!("{verb} the {subject_en} on the current basis."),
            format!("This user action resolves as {outcome}."),
        )
    }
}

struct ChoiceCloseCoordinates {
    close_basis_revision: Option<u64>,
    result_refs: Vec<StateRecordRef>,
    residual_risk_ids: Vec<RiskId>,
}

fn choice_close_coordinates(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    envelope: &ToolEnvelope,
    task_id: &TaskId,
    judgment_kind: JudgmentKind,
) -> Result<ChoiceCloseCoordinates, PlanError> {
    if !matches!(
        judgment_kind,
        JudgmentKind::FinalAcceptance | JudgmentKind::ResidualRiskAcceptance
    ) {
        return Ok(ChoiceCloseCoordinates {
            close_basis_revision: None,
            result_refs: Vec::new(),
            residual_risk_ids: Vec::new(),
        });
    }
    let close_basis = store
        .task_revision_record(task_id)
        .map_err(CorePipelineError::from)?
        .and_then(|record| record.current_close_basis)
        .ok_or_else(|| {
            PlanError::Response(Box::new(decision_rejected_response(
                envelope,
                Some(project_state.state_version),
                "a current close basis is required for this user action",
            )))
        })?;
    Ok(ChoiceCloseCoordinates {
        close_basis_revision: Some(close_basis.close_basis_revision),
        result_refs: close_basis.result_refs.clone(),
        residual_risk_ids: current_acceptance_required_risk_ids(&close_basis)
            .into_iter()
            .collect(),
    })
}

pub(super) fn validate_user_action_target(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    envelope: &ToolEnvelope,
    task_id: &TaskId,
    target: &EvidenceTarget,
    field: &'static str,
) -> Result<(), PlanError> {
    let current = match target {
        EvidenceTarget::AcceptanceCriterion {
            acceptance_criterion_id,
        } => store
            .acceptance_criterion_record(acceptance_criterion_id.as_str())
            .map_err(CorePipelineError::from)?
            .is_some_and(|record| record.task_id == task_id.as_str() && record.status == "active"),
        EvidenceTarget::SupplementalClaim {
            evidence_claim_id,
            statement,
        } => store
            .evidence_claim_record(task_id, evidence_claim_id.as_str())
            .map_err(CorePipelineError::from)?
            .is_some_and(|record| record.statement == normalize_display_text(statement)),
    };
    if current {
        Ok(())
    } else {
        user_action_validation_error(
            envelope.dry_run,
            Some(project_state.state_version),
            field,
            "target must identify a current acceptance criterion or supplemental claim",
        )
    }
}

pub(super) fn canonical_user_action_artifacts(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    envelope: &ToolEnvelope,
    task_id: &TaskId,
    artifact_ids: &[ArtifactId],
    field: &'static str,
) -> Result<Vec<ArtifactRef>, PlanError> {
    let mut canonical = BTreeMap::new();
    for artifact_id in artifact_ids {
        let record = store
            .artifact_record(artifact_id.as_str())
            .map_err(CorePipelineError::from)?;
        let owner_link = store
            .artifact_has_task_owner_link(artifact_id.as_str(), task_id.as_str())
            .map_err(CorePipelineError::from)?;
        let Some(record) = record else {
            return user_action_validation_error(
                envelope.dry_run,
                Some(project_state.state_version),
                field,
                "artifact candidates must identify current persistent Task artifacts",
            );
        };
        if record.project_id != envelope.project_id.as_str()
            || record.task_id != task_id.as_str()
            || !owner_link
            || !persistent_artifact_is_verified_current(store, &record)?
        {
            return user_action_validation_error(
                envelope.dry_run,
                Some(project_state.state_version),
                field,
                "artifact candidates must be verified current artifacts owned by this Task",
            );
        }
        let artifact_ref = artifact_ref_from_verified_record(
            store,
            &record,
            None,
            Some(project_state.state_version),
        )?;
        canonical.insert(artifact_id.as_str().to_owned(), artifact_ref);
    }
    Ok(canonical.into_values().collect())
}

fn validate_choice_affected_refs(
    action: &UserActionDraft,
    project_id: &ProjectId,
    task_id: &TaskId,
    dry_run: bool,
    state_version: u64,
) -> Result<(), PlanError> {
    let UserActionDraft::Choice(choice) = action else {
        return Ok(());
    };
    for affected_ref in &choice.affected_refs {
        if affected_ref.project_id != *project_id {
            return user_action_validation_error(
                dry_run,
                Some(state_version),
                "action.affected_refs.project_id",
                "affected_refs must belong to the request project",
            );
        }
        let task_record_mismatch = affected_ref.record_kind == StateRecordKind::Task
            && affected_ref.record_id.as_str() != task_id.as_str();
        let task_context_mismatch = affected_ref
            .task_id
            .as_ref()
            .is_some_and(|affected_task_id| affected_task_id != task_id);
        if task_record_mismatch || task_context_mismatch {
            return user_action_validation_error(
                dry_run,
                Some(state_version),
                "action.affected_refs.task_id",
                "task-scoped affected_refs must belong to the request Task",
            );
        }
    }
    Ok(())
}

fn validate_required_for_compatibility(
    action_kind: UserActionKind,
    required_for: &[UserActionRequiredFor],
    dry_run: bool,
    state_version: u64,
) -> Result<(), PlanError> {
    if required_for
        .iter()
        .copied()
        .all(|target| action_kind.is_compatible_with_required_for(target))
    {
        Ok(())
    } else {
        user_action_validation_error(
            dry_run,
            Some(state_version),
            "required_for",
            "required_for contains an operation incompatible with the action kind",
        )
    }
}

fn scope_baseline_is_missing(task: &TaskRecord) -> Result<bool, PlanError> {
    Ok(StoredScope::from_task(task)?.baseline_ref.is_none())
}

pub(super) fn user_action_validation_error<T>(
    dry_run: bool,
    state_version: Option<u64>,
    field: &'static str,
    message: &'static str,
) -> Result<T, PlanError> {
    validation_plan_error(dry_run, state_version, field, message)
}

/// Decodes one effective stored record into the authority facts used by Core policy.
pub(super) fn user_action_authority_from_record(
    record: &EffectiveUserActionRecord,
) -> CoreResult<UserActionAuthority> {
    let request: PersistedUserActionRequest = decode_required_json(
        "user_action_requests",
        record.request.user_action_request_id.clone(),
        "request_json",
        Some(&record.request.request_json),
    )?;
    let basis: UserActionBasis = decode_required_json(
        "user_action_requests",
        record.request.user_action_request_id.clone(),
        "basis_json",
        Some(&record.request.basis_json),
    )?;
    if request.body.action_kind() != record.request.action_kind
        || basis.compatibility_status() != record.request.basis_status
    {
        return Err(CorePipelineError::Store(
            StoreError::corrupt_owner_state_json(
                "user_action_requests",
                record.request.user_action_request_id.clone(),
                "request_json",
            ),
        ));
    }
    let resolution = record
        .resolution
        .as_ref()
        .map(|resolution| {
            let body: PersistedUserActionResolution = decode_required_json(
                "user_action_resolutions",
                resolution.user_action_resolution_id.clone(),
                "resolution_json",
                Some(&resolution.resolution_json),
            )?;
            body.validate().map_err(|_| {
                CorePipelineError::Store(StoreError::corrupt_owner_state_json(
                    "user_action_resolutions",
                    resolution.user_action_resolution_id.clone(),
                    "resolution_json",
                ))
            })?;
            if resolution.action_kind != record.request.action_kind {
                return Err(CorePipelineError::Store(
                    StoreError::corrupt_owner_state_value(
                        "user_action_resolutions",
                        resolution.user_action_resolution_id.clone(),
                        "action_kind",
                    ),
                ));
            }
            Ok(body)
        })
        .transpose()?;
    if record.status == UserActionStatus::Resolved && record.resolution.is_none() {
        return Err(CorePipelineError::Store(
            StoreError::corrupt_owner_state_value(
                "user_action_requests",
                record.request.user_action_request_id.clone(),
                "resolution",
            ),
        ));
    }
    let (machine_action, resolution_outcome) = match resolution.as_ref() {
        Some(UserActionResolutionBody::Choice {
            machine_action,
            resolution_outcome,
            ..
        }) => (Some(*machine_action), Some(*resolution_outcome)),
        _ => (None, None),
    };
    let affected_refs = request.body.affected_refs().to_vec();
    let expires_at = request.expires_at.into_option();
    let resolution_id = record
        .resolution
        .as_ref()
        .map(|resolution| resolution.user_action_resolution_id.clone());
    let resolved_by_actor_source = record
        .resolution
        .as_ref()
        .map(|resolution| {
            parse_owner_storage_value(
                "user_action_resolutions",
                resolution.user_action_resolution_id.clone(),
                "resolved_by_actor_source",
                &resolution.resolved_by_actor_source,
            )
        })
        .transpose()?;
    Ok(UserActionAuthority {
        user_action_request_id: record.request.user_action_request_id.clone(),
        user_action_resolution_id: resolution_id,
        task_id: TaskId::new(record.request.task_id.clone()),
        action_kind: record.request.action_kind,
        status: record.status,
        required_for: request.required_for,
        affected_refs,
        machine_action,
        resolution_outcome,
        resolved_by_actor_source,
        resolved_verification_basis: record
            .resolution
            .as_ref()
            .map(|resolution| resolution.resolved_verification_basis.clone()),
        resolved_assurance_level: record
            .resolution
            .as_ref()
            .map(|resolution| resolution.resolved_assurance_level.clone()),
        basis_status: record.request.basis_status,
        basis: Some(basis),
        resolution,
        expires_at,
    })
}

/// Projects a just-constructed pending request into method-neutral authority facts.
pub(super) fn user_action_authority_from_state(request: &UserActionRequest) -> UserActionAuthority {
    UserActionAuthority {
        user_action_request_id: request.user_action_request_id.as_str().to_owned(),
        user_action_resolution_id: None,
        task_id: request.task_id.clone(),
        action_kind: request.action_kind,
        status: request.status,
        required_for: request.required_for.clone(),
        affected_refs: request.body.affected_refs().to_vec(),
        machine_action: None,
        resolution_outcome: None,
        resolved_by_actor_source: None,
        resolved_verification_basis: None,
        resolved_assurance_level: None,
        basis_status: request.basis.compatibility_status(),
        basis: Some(request.basis.clone()),
        resolution: None,
        expires_at: request.expires_at.as_ref().cloned(),
    }
}

/// Loads resolved authority facts for one judgment kind.
pub(super) fn resolved_user_action_authorities_for_plan(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    envelope: &ToolEnvelope,
    task_id: &TaskId,
    judgment_kind: JudgmentKind,
    now: &UtcTimestamp,
) -> Result<Vec<UserActionAuthority>, PlanError> {
    store
        .resolved_user_action_records(task_id, judgment_kind.into(), now)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                envelope,
                project_state,
                error,
            )))
        })?
        .iter()
        .map(user_action_authority_from_record)
        .collect::<CoreResult<Vec<_>>>()
        .map_err(PlanError::Core)
}

/// Strictly decodes one effective stored record into its public typed request.
pub(super) fn user_action_from_record(
    record: &EffectiveUserActionRecord,
    state_version: u64,
) -> CoreResult<UserActionRequest> {
    let persisted: PersistedUserActionRequest = decode_required_json(
        "user_action_requests",
        record.request.user_action_request_id.clone(),
        "request_json",
        Some(&record.request.request_json),
    )?;
    let basis: UserActionBasis = decode_required_json(
        "user_action_requests",
        record.request.user_action_request_id.clone(),
        "basis_json",
        Some(&record.request.basis_json),
    )?;
    if persisted.body.action_kind() != record.request.action_kind
        || basis.compatibility_status() != record.request.basis_status
    {
        return Err(CorePipelineError::Store(
            StoreError::corrupt_owner_state_json(
                "user_action_requests",
                record.request.user_action_request_id.clone(),
                "request_json",
            ),
        ));
    }
    let project_id = ProjectId::new(record.request.project_id.clone());
    let task_id = TaskId::new(record.request.task_id.clone());
    let resolution_ref = record.resolution.as_ref().map(|resolution| {
        state_ref(
            StateRecordKind::UserActionResolution,
            &resolution.user_action_resolution_id,
            &project_id,
            Some(&task_id),
            Some(state_version),
        )
    });
    Ok(UserActionRequest {
        user_action_request_id: UserActionRequestId::new(
            record.request.user_action_request_id.clone(),
        ),
        project_id,
        task_id,
        change_unit_id: record
            .request
            .change_unit_id
            .clone()
            .map(ChangeUnitId::new)
            .into(),
        action_kind: record.request.action_kind,
        status: record.status,
        body: persisted.body,
        basis,
        required_for: persisted.required_for,
        user_action_resolution_ref: resolution_ref.into(),
        expires_at: persisted.expires_at,
        created_at: parse_owner_storage_value(
            "user_action_requests",
            record.request.user_action_request_id.clone(),
            "requested_at",
            &record.request.requested_at,
        )?,
    })
}

/// Returns adapter-neutral application guidance for pending actions.
pub(super) fn pending_user_action_instruction() -> String {
    "Resolve pending user actions through the User Channel.".to_owned()
}

/// Loads all current pending UserAction authority facts for a Task.
pub(super) fn pending_user_action_authorities_for_plan(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    envelope: &ToolEnvelope,
    task_id: &TaskId,
    now: &UtcTimestamp,
) -> Result<Vec<UserActionAuthority>, PlanError> {
    store
        .pending_user_action_records(task_id, now)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                envelope,
                project_state,
                error,
            )))
        })?
        .iter()
        .map(user_action_authority_from_record)
        .collect::<CoreResult<Vec<_>>>()
        .map_err(PlanError::Core)
}

/// Projects pending refs that block one typed Core operation.
pub(super) fn pending_user_action_refs_for_operation(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    envelope: &ToolEnvelope,
    now: &UtcTimestamp,
    context: &UserActionOperationContext<'_>,
) -> Result<Vec<StateRecordRef>, PlanError> {
    Ok(pending_user_action_authorities_for_plan(
        store,
        project_state,
        envelope,
        context.task_id,
        now,
    )?
    .iter()
    .filter(|authority| user_action_blocks_operation(authority, context))
    .map(|authority| {
        state_ref(
            StateRecordKind::UserActionRequest,
            &authority.user_action_request_id,
            &envelope.project_id,
            Some(context.task_id),
            Some(project_state.state_version),
        )
    })
    .collect())
}

/// Loads every current resolved UserAction authority for a Task.
pub(super) fn resolved_user_action_authorities_for_all_kinds(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    envelope: &ToolEnvelope,
    task_id: &TaskId,
    now: &UtcTimestamp,
) -> Result<Vec<UserActionAuthority>, PlanError> {
    store
        .user_action_records_for_task(task_id, now)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                envelope,
                project_state,
                error,
            )))
        })?
        .into_iter()
        .filter(|record| record.status == UserActionStatus::Resolved)
        .map(|record| user_action_authority_from_record(&record))
        .collect::<CoreResult<Vec<_>>>()
        .map_err(PlanError::Core)
}

/// Reduces pending request refs to the only projection allowed in agent results.
pub(super) fn agent_safe_pending_user_action_summaries(
    refs: impl IntoIterator<Item = StateRecordRef>,
) -> Vec<AgentSafeUserActionRequestSummary> {
    refs.into_iter()
        .map(|record_ref| {
            AgentSafeUserActionRequestSummary::pending(UserActionRequestId::new(
                record_ref.record_id.as_str(),
            ))
        })
        .collect()
}

/// Reads current pending refs for a projected Task state.
pub(super) fn projected_pending_user_action_refs(
    store: &CoreProjectStore,
    task_id: &TaskId,
    state_version: u64,
    now: &UtcTimestamp,
) -> Result<Vec<StateRecordRef>, PlanError> {
    Ok(stored_refs_to_state_refs(
        store
            .pending_user_action_refs(task_id, state_version, now)
            .map_err(CorePipelineError::from)?,
    ))
}

/// Derives the Task lifecycle phase after applying pending UserAction facts.
pub(super) fn projected_user_action_lifecycle_phase(
    project_state: &ProjectStateHeader,
    task: &TaskRecord,
    current_change_unit: Option<&ChangeUnitRecord>,
    pending_authorities: &[UserActionAuthority],
) -> Option<&'static str> {
    if project_state.active_task_id.as_deref() != Some(task.task_id.as_str())
        || is_terminal_lifecycle(&task.lifecycle_phase)
    {
        return None;
    }

    let task_id = TaskId::new(task.task_id.clone());
    let current_change_unit_id =
        current_change_unit.map(|record| ChangeUnitId::new(record.change_unit_id.clone()));
    let waits_for_user = pending_authorities.iter().any(|authority| {
        user_action_keeps_task_waiting(
            authority,
            &task_id,
            current_change_unit_id.as_ref(),
            task.scope_revision,
        )
    });
    let next_phase = if waits_for_user {
        "waiting_user"
    } else if task.lifecycle_phase == "waiting_user" {
        if current_change_unit.is_some() {
            "ready"
        } else {
            "shaping"
        }
    } else {
        return None;
    };

    (task.lifecycle_phase != next_phase).then_some(next_phase)
}
