//! Canonical request projections for current workflow action forms.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;
use volicord_types::methods::{WorkflowActionAdmissionClass, PUBLIC_METHOD_CONTRACTS};
use volicord_types::schema::{
    JsonObject, WorkflowRecordShapingCheckpointSubmissionContract,
    WorkflowTransitionSubmissionContract, WorkflowUpdateScopeSubmissionContract,
};
use volicord_types::tool_names::AgentToolId;
use volicord_types::values::MethodName;
use volicord_types::values::WorkflowActionSemanticVariant;

use crate::tool_contracts::mcp_tool_contract;

/// One fixed authority value copied in the public MCP request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionFormFixedArgumentDescriptor {
    pub authority: &'static str,
    pub path_pattern: &'static str,
}

/// One caller-authored request slot that is not fixed by current authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionFormAuthoredInputDescriptor {
    pub path_pattern: &'static str,
}

/// One authority coordinate bound without a caller-controlled wire field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionFormInjectedAuthorityDescriptor {
    pub authority: &'static str,
    pub canonical_request_target: &'static str,
}

/// One exact semantic-variant projection from an action form to an MCP request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionFormRequestProjectionDescriptor {
    submission_variant: ActionFormSubmissionVariant,
    pub method: MethodName,
    pub selected_semantic_variant: WorkflowActionSemanticVariant,
    pub semantic_variant_selector: Option<ActionFormSemanticVariantSelector>,
    pub fixed_arguments: &'static [ActionFormFixedArgumentDescriptor],
    pub required_agent_inputs: &'static [ActionFormAuthoredInputDescriptor],
    pub optional_agent_inputs: &'static [ActionFormAuthoredInputDescriptor],
    pub injected_authorities: &'static [ActionFormInjectedAuthorityDescriptor],
    pub core_current_authorities: &'static [&'static str],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ActionFormSubmissionVariant {
    CheckpointCreate,
    CheckpointReplace,
    UpdateKeep,
    UpdateGeneralCreate,
    UpdateGeneralReplace,
    UpdateAdvisorCreate,
    UpdateAdvisorReplace,
    FinalizeAdvice,
    AdvanceTask,
    PrepareEvidenceCapture,
    PrepareWrite,
    StageArtifact,
    RecordRun,
    RequestUserAction,
    ReconcileChanges,
    CheckClose,
    CloseTask,
}

/// Exact public discriminator value selecting one method-owned semantic variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionFormSemanticVariantSelector {
    pub path: &'static str,
    pub value: &'static str,
}

const CHECKPOINT_CREATE_SELECTOR: ActionFormSemanticVariantSelector =
    ActionFormSemanticVariantSelector {
        path: "/checkpoint_operation/operation",
        value: "create_initial",
    };
const CHECKPOINT_REPLACE_SELECTOR: ActionFormSemanticVariantSelector =
    ActionFormSemanticVariantSelector {
        path: "/checkpoint_operation/operation",
        value: "replace_current",
    };
const UPDATE_SCOPE_KEEP_SELECTOR: ActionFormSemanticVariantSelector =
    ActionFormSemanticVariantSelector {
        path: "/change_unit/operation",
        value: "keep_current",
    };
const UPDATE_SCOPE_CREATE_SELECTOR: ActionFormSemanticVariantSelector =
    ActionFormSemanticVariantSelector {
        path: "/change_unit/operation",
        value: "create_current",
    };
const UPDATE_SCOPE_REPLACE_SELECTOR: ActionFormSemanticVariantSelector =
    ActionFormSemanticVariantSelector {
        path: "/change_unit/operation",
        value: "replace_current",
    };

const TASK: ActionFormFixedArgumentDescriptor = ActionFormFixedArgumentDescriptor {
    authority: "task",
    path_pattern: "/task_id",
};
const EXPECTED_STATE_VERSION: ActionFormInjectedAuthorityDescriptor =
    ActionFormInjectedAuthorityDescriptor {
        authority: "project_state.state_version",
        canonical_request_target: "/envelope/expected_state_version",
    };
const PROJECT_ROUTE: ActionFormInjectedAuthorityDescriptor =
    ActionFormInjectedAuthorityDescriptor {
        authority: "project",
        canonical_request_target: "/envelope/project_id",
    };
const COMMON_INJECTED: &[ActionFormInjectedAuthorityDescriptor] =
    &[PROJECT_ROUTE, EXPECTED_STATE_VERSION];
const READ_INJECTED: &[ActionFormInjectedAuthorityDescriptor] = &[PROJECT_ROUTE];

const CHECKPOINT_CREATE_FIXED: &[ActionFormFixedArgumentDescriptor] = &[
    TASK,
    ActionFormFixedArgumentDescriptor {
        authority: "checkpoint_operation.kind",
        path_pattern: "/checkpoint_operation/operation",
    },
    ActionFormFixedArgumentDescriptor {
        authority: "scope_revision",
        path_pattern: "/scope_revision",
    },
    ActionFormFixedArgumentDescriptor {
        authority: "baseline",
        path_pattern: "/baseline_ref",
    },
];
const CHECKPOINT_REPLACE_FIXED: &[ActionFormFixedArgumentDescriptor] = &[
    TASK,
    ActionFormFixedArgumentDescriptor {
        authority: "checkpoint_operation.kind",
        path_pattern: "/checkpoint_operation/operation",
    },
    ActionFormFixedArgumentDescriptor {
        authority: "checkpoint.expected_current_checkpoint_id",
        path_pattern: "/checkpoint_operation/expected_current_checkpoint_id",
    },
    ActionFormFixedArgumentDescriptor {
        authority: "checkpoint.retired_non_authorizing_requests",
        path_pattern: "/checkpoint_operation/retired_non_authorizing_request_refs",
    },
    ActionFormFixedArgumentDescriptor {
        authority: "checkpoint.carry_forward_applications",
        path_pattern: "/checkpoint_operation/carry_forward_application_refs",
    },
    ActionFormFixedArgumentDescriptor {
        authority: "checkpoint.stale_application",
        path_pattern: "/checkpoint_operation/stale_authority_actions/*/stale_application_ref",
    },
    ActionFormFixedArgumentDescriptor {
        authority: "scope_revision",
        path_pattern: "/scope_revision",
    },
    ActionFormFixedArgumentDescriptor {
        authority: "baseline",
        path_pattern: "/baseline_ref",
    },
];
const CHECKPOINT_REQUIRED: &[ActionFormAuthoredInputDescriptor] = &[
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/summary",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/implementation_boundary",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/gaps",
    },
];
const CHECKPOINT_REPLACE_REQUIRED: &[ActionFormAuthoredInputDescriptor] = &[
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/summary",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/implementation_boundary",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/gaps",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/checkpoint_operation/stale_authority_actions/*/action",
    },
];
const CHECKPOINT_OPTIONAL: &[ActionFormAuthoredInputDescriptor] = &[
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/source_refs",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/evidence_refs",
    },
];
const CHECKPOINT_REPLACE_OPTIONAL: &[ActionFormAuthoredInputDescriptor] = &[
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/source_refs",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/evidence_refs",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/checkpoint_operation/stale_authority_actions/*/successor_gap",
    },
];

const UPDATE_SCOPE_FIXED: &[ActionFormFixedArgumentDescriptor] = &[
    TASK,
    ActionFormFixedArgumentDescriptor {
        authority: "scope_decision_resolutions",
        path_pattern: "/related_scope_decision_refs",
    },
    ActionFormFixedArgumentDescriptor {
        authority: "change_unit.operation",
        path_pattern: "/change_unit/operation",
    },
];
const ADVISOR_UPDATE_SCOPE_FIXED: &[ActionFormFixedArgumentDescriptor] = &[
    TASK,
    ActionFormFixedArgumentDescriptor {
        authority: "scope_decision_resolutions",
        path_pattern: "/related_scope_decision_refs",
    },
    ActionFormFixedArgumentDescriptor {
        authority: "change_unit.operation",
        path_pattern: "/change_unit/operation",
    },
    ActionFormFixedArgumentDescriptor {
        authority: "advisor.affected_paths",
        path_pattern: "/change_unit/affected_paths",
    },
    ActionFormFixedArgumentDescriptor {
        authority: "advisor.effect_contract",
        path_pattern: "/change_unit/effect_contract",
    },
];
const UPDATE_SCOPE_REQUIRED: &[ActionFormAuthoredInputDescriptor] = &[
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/baseline_ref",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/goal_summary",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/scope_update",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/scope_boundary",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/non_goals",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/acceptance_criteria",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/autonomy_boundary",
    },
];
const GENERAL_UPDATE_SCOPE_REQUIRED: &[ActionFormAuthoredInputDescriptor] = &[
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/baseline_ref",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/goal_summary",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/scope_update",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/scope_boundary",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/non_goals",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/acceptance_criteria",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/autonomy_boundary",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/change_unit/scope_summary",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/change_unit/affected_paths",
    },
];
const ADVISOR_UPDATE_SCOPE_REQUIRED: &[ActionFormAuthoredInputDescriptor] = &[
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/baseline_ref",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/goal_summary",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/scope_update",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/scope_boundary",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/non_goals",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/acceptance_criteria",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/autonomy_boundary",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/change_unit/scope_summary",
    },
];
const GENERAL_UPDATE_SCOPE_OPTIONAL: &[ActionFormAuthoredInputDescriptor] = &[
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/change_unit/affected_areas",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/change_unit/constraints",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/change_unit/effect_contract",
    },
];
const ADVISOR_UPDATE_SCOPE_OPTIONAL: &[ActionFormAuthoredInputDescriptor] = &[
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/change_unit/affected_areas",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/change_unit/constraints",
    },
];

const FINALIZE_FIXED: &[ActionFormFixedArgumentDescriptor] = &[
    TASK,
    ActionFormFixedArgumentDescriptor {
        authority: "shaping_checkpoint",
        path_pattern: "/shaping_checkpoint_id",
    },
    ActionFormFixedArgumentDescriptor {
        authority: "change_unit",
        path_pattern: "/change_unit_id",
    },
    ActionFormFixedArgumentDescriptor {
        authority: "scope_revision",
        path_pattern: "/scope_revision",
    },
    ActionFormFixedArgumentDescriptor {
        authority: "baseline",
        path_pattern: "/baseline_ref",
    },
    ActionFormFixedArgumentDescriptor {
        authority: "user_action_resolutions",
        path_pattern: "/user_action_resolution_ids",
    },
];
const FINALIZE_REQUIRED: &[ActionFormAuthoredInputDescriptor] =
    &[ActionFormAuthoredInputDescriptor {
        path_pattern: "/result_summary",
    }];
const FINALIZE_OPTIONAL: &[ActionFormAuthoredInputDescriptor] = &[
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/result_refs",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/evidence_refs",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/residual_risks",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/recovery_constraints",
    },
];

const NO_INPUTS: &[ActionFormAuthoredInputDescriptor] = &[];
const IMPLEMENTATION_BASIS_FIXED: &[ActionFormFixedArgumentDescriptor] = &[
    TASK,
    ActionFormFixedArgumentDescriptor {
        authority: "change_unit",
        path_pattern: "/change_unit_id",
    },
    ActionFormFixedArgumentDescriptor {
        authority: "baseline",
        path_pattern: "/baseline_ref",
    },
];
const PREPARE_EVIDENCE_REQUIRED: &[ActionFormAuthoredInputDescriptor] = &[
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/target",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/capture",
    },
];
const PREPARE_WRITE_REQUIRED: &[ActionFormAuthoredInputDescriptor] = &[
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/intended_operation",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/intended_paths",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/product_file_write_intended",
    },
];
const PREPARE_WRITE_OPTIONAL: &[ActionFormAuthoredInputDescriptor] =
    &[ActionFormAuthoredInputDescriptor {
        path_pattern: "/sensitive_categories",
    }];
const TASK_FIXED: &[ActionFormFixedArgumentDescriptor] = &[TASK];
const STAGE_REQUIRED: &[ActionFormAuthoredInputDescriptor] = &[
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/display_name",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/content_type",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/redaction_state",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/safe_bytes_or_notice",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/expected_sha256",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/expected_size_bytes",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/relation_hint",
    },
];
const RECORD_RUN_FIXED: &[ActionFormFixedArgumentDescriptor] = &[
    TASK,
    ActionFormFixedArgumentDescriptor {
        authority: "change_unit",
        path_pattern: "/change_unit_id",
    },
    ActionFormFixedArgumentDescriptor {
        authority: "baseline",
        path_pattern: "/baseline_ref",
    },
];
const RECORD_RUN_REQUIRED: &[ActionFormAuthoredInputDescriptor] = &[
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/kind",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/run_id",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/write_ticket_id",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/performed_operation",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/summary",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/observed_changes",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/close_assessment",
    },
];
const RECORD_RUN_OPTIONAL: &[ActionFormAuthoredInputDescriptor] = &[
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/artifact_inputs",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/evidence_updates",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/evidence_observations",
    },
];
const REQUEST_USER_ACTION_FIXED: &[ActionFormFixedArgumentDescriptor] = &[
    ActionFormFixedArgumentDescriptor {
        authority: "request.operation",
        path_pattern: "/request/operation",
    },
    ActionFormFixedArgumentDescriptor {
        authority: "task",
        path_pattern: "/request/task_id",
    },
    ActionFormFixedArgumentDescriptor {
        authority: "change_unit",
        path_pattern: "/request/change_unit_id",
    },
];
const REQUEST_USER_ACTION_REQUIRED: &[ActionFormAuthoredInputDescriptor] = &[
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/request/action",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/request/required_for",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/request/expires_at",
    },
];
const RECONCILE_OPTIONAL: &[ActionFormAuthoredInputDescriptor] =
    &[ActionFormAuthoredInputDescriptor {
        path_pattern: "/resolution_requests",
    }];
const CLOSE_REQUIRED: &[ActionFormAuthoredInputDescriptor] = &[
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/intent",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/close_reason",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/superseding_task_id",
    },
    ActionFormAuthoredInputDescriptor {
        path_pattern: "/user_note",
    },
];
const PROJECTIONS: &[ActionFormRequestProjectionDescriptor] = &[
    ActionFormRequestProjectionDescriptor {
        submission_variant: ActionFormSubmissionVariant::CheckpointCreate,
        method: MethodName::RecordShapingCheckpoint,
        selected_semantic_variant: WorkflowActionSemanticVariant::CreateInitial,
        semantic_variant_selector: Some(CHECKPOINT_CREATE_SELECTOR),
        fixed_arguments: CHECKPOINT_CREATE_FIXED,
        required_agent_inputs: CHECKPOINT_REQUIRED,
        optional_agent_inputs: CHECKPOINT_OPTIONAL,
        injected_authorities: COMMON_INJECTED,
        core_current_authorities: &[],
    },
    ActionFormRequestProjectionDescriptor {
        submission_variant: ActionFormSubmissionVariant::CheckpointReplace,
        method: MethodName::RecordShapingCheckpoint,
        selected_semantic_variant: WorkflowActionSemanticVariant::ReplaceCurrent,
        semantic_variant_selector: Some(CHECKPOINT_REPLACE_SELECTOR),
        fixed_arguments: CHECKPOINT_REPLACE_FIXED,
        required_agent_inputs: CHECKPOINT_REPLACE_REQUIRED,
        optional_agent_inputs: CHECKPOINT_REPLACE_OPTIONAL,
        injected_authorities: COMMON_INJECTED,
        core_current_authorities: &["checkpoint.predecessor_lineage"],
    },
    ActionFormRequestProjectionDescriptor {
        submission_variant: ActionFormSubmissionVariant::UpdateKeep,
        method: MethodName::UpdateScope,
        selected_semantic_variant: WorkflowActionSemanticVariant::KeepCurrentChangeUnit,
        semantic_variant_selector: Some(UPDATE_SCOPE_KEEP_SELECTOR),
        fixed_arguments: UPDATE_SCOPE_FIXED,
        required_agent_inputs: UPDATE_SCOPE_REQUIRED,
        optional_agent_inputs: NO_INPUTS,
        injected_authorities: COMMON_INJECTED,
        core_current_authorities: &["scope_revision", "current_change_unit", "current_baseline"],
    },
    ActionFormRequestProjectionDescriptor {
        submission_variant: ActionFormSubmissionVariant::UpdateGeneralCreate,
        method: MethodName::UpdateScope,
        selected_semantic_variant: WorkflowActionSemanticVariant::CreateCurrentChangeUnit,
        semantic_variant_selector: Some(UPDATE_SCOPE_CREATE_SELECTOR),
        fixed_arguments: UPDATE_SCOPE_FIXED,
        required_agent_inputs: GENERAL_UPDATE_SCOPE_REQUIRED,
        optional_agent_inputs: GENERAL_UPDATE_SCOPE_OPTIONAL,
        injected_authorities: COMMON_INJECTED,
        core_current_authorities: &["scope_revision", "current_change_unit", "current_baseline"],
    },
    ActionFormRequestProjectionDescriptor {
        submission_variant: ActionFormSubmissionVariant::UpdateGeneralReplace,
        method: MethodName::UpdateScope,
        selected_semantic_variant: WorkflowActionSemanticVariant::ReplaceCurrentChangeUnit,
        semantic_variant_selector: Some(UPDATE_SCOPE_REPLACE_SELECTOR),
        fixed_arguments: UPDATE_SCOPE_FIXED,
        required_agent_inputs: GENERAL_UPDATE_SCOPE_REQUIRED,
        optional_agent_inputs: GENERAL_UPDATE_SCOPE_OPTIONAL,
        injected_authorities: COMMON_INJECTED,
        core_current_authorities: &["scope_revision", "current_change_unit", "current_baseline"],
    },
    ActionFormRequestProjectionDescriptor {
        submission_variant: ActionFormSubmissionVariant::UpdateAdvisorCreate,
        method: MethodName::UpdateScope,
        selected_semantic_variant: WorkflowActionSemanticVariant::CreateCurrentChangeUnit,
        semantic_variant_selector: Some(UPDATE_SCOPE_CREATE_SELECTOR),
        fixed_arguments: ADVISOR_UPDATE_SCOPE_FIXED,
        required_agent_inputs: ADVISOR_UPDATE_SCOPE_REQUIRED,
        optional_agent_inputs: ADVISOR_UPDATE_SCOPE_OPTIONAL,
        injected_authorities: COMMON_INJECTED,
        core_current_authorities: &["scope_revision", "current_change_unit", "current_baseline"],
    },
    ActionFormRequestProjectionDescriptor {
        submission_variant: ActionFormSubmissionVariant::UpdateAdvisorReplace,
        method: MethodName::UpdateScope,
        selected_semantic_variant: WorkflowActionSemanticVariant::ReplaceCurrentChangeUnit,
        semantic_variant_selector: Some(UPDATE_SCOPE_REPLACE_SELECTOR),
        fixed_arguments: ADVISOR_UPDATE_SCOPE_FIXED,
        required_agent_inputs: ADVISOR_UPDATE_SCOPE_REQUIRED,
        optional_agent_inputs: ADVISOR_UPDATE_SCOPE_OPTIONAL,
        injected_authorities: COMMON_INJECTED,
        core_current_authorities: &["scope_revision", "current_change_unit", "current_baseline"],
    },
    ActionFormRequestProjectionDescriptor {
        submission_variant: ActionFormSubmissionVariant::FinalizeAdvice,
        method: MethodName::FinalizeAdvice,
        selected_semantic_variant: WorkflowActionSemanticVariant::FinalizeAdvice,
        semantic_variant_selector: None,
        fixed_arguments: FINALIZE_FIXED,
        required_agent_inputs: FINALIZE_REQUIRED,
        optional_agent_inputs: FINALIZE_OPTIONAL,
        injected_authorities: COMMON_INJECTED,
        core_current_authorities: &[],
    },
    ActionFormRequestProjectionDescriptor {
        submission_variant: ActionFormSubmissionVariant::AdvanceTask,
        method: MethodName::AdvanceTask,
        selected_semantic_variant: WorkflowActionSemanticVariant::AdvanceTask,
        semantic_variant_selector: None,
        fixed_arguments: FINALIZE_FIXED,
        required_agent_inputs: NO_INPUTS,
        optional_agent_inputs: NO_INPUTS,
        injected_authorities: COMMON_INJECTED,
        core_current_authorities: &[],
    },
    ActionFormRequestProjectionDescriptor {
        submission_variant: ActionFormSubmissionVariant::PrepareEvidenceCapture,
        method: MethodName::PrepareEvidenceCapture,
        selected_semantic_variant: WorkflowActionSemanticVariant::PrepareEvidenceCapture,
        semantic_variant_selector: None,
        fixed_arguments: IMPLEMENTATION_BASIS_FIXED,
        required_agent_inputs: PREPARE_EVIDENCE_REQUIRED,
        optional_agent_inputs: NO_INPUTS,
        injected_authorities: COMMON_INJECTED,
        core_current_authorities: &[],
    },
    ActionFormRequestProjectionDescriptor {
        submission_variant: ActionFormSubmissionVariant::PrepareWrite,
        method: MethodName::PrepareWrite,
        selected_semantic_variant: WorkflowActionSemanticVariant::PrepareWrite,
        semantic_variant_selector: None,
        fixed_arguments: IMPLEMENTATION_BASIS_FIXED,
        required_agent_inputs: PREPARE_WRITE_REQUIRED,
        optional_agent_inputs: PREPARE_WRITE_OPTIONAL,
        injected_authorities: COMMON_INJECTED,
        core_current_authorities: &[],
    },
    ActionFormRequestProjectionDescriptor {
        submission_variant: ActionFormSubmissionVariant::StageArtifact,
        method: MethodName::StageArtifact,
        selected_semantic_variant: WorkflowActionSemanticVariant::StageArtifact,
        semantic_variant_selector: None,
        fixed_arguments: TASK_FIXED,
        required_agent_inputs: STAGE_REQUIRED,
        optional_agent_inputs: NO_INPUTS,
        injected_authorities: COMMON_INJECTED,
        core_current_authorities: &[],
    },
    ActionFormRequestProjectionDescriptor {
        submission_variant: ActionFormSubmissionVariant::RecordRun,
        method: MethodName::RecordRun,
        selected_semantic_variant: WorkflowActionSemanticVariant::RecordRun,
        semantic_variant_selector: None,
        fixed_arguments: RECORD_RUN_FIXED,
        required_agent_inputs: RECORD_RUN_REQUIRED,
        optional_agent_inputs: RECORD_RUN_OPTIONAL,
        injected_authorities: COMMON_INJECTED,
        core_current_authorities: &[],
    },
    ActionFormRequestProjectionDescriptor {
        submission_variant: ActionFormSubmissionVariant::RequestUserAction,
        method: MethodName::RequestUserAction,
        selected_semantic_variant: WorkflowActionSemanticVariant::RequestUserAction,
        semantic_variant_selector: None,
        fixed_arguments: REQUEST_USER_ACTION_FIXED,
        required_agent_inputs: REQUEST_USER_ACTION_REQUIRED,
        optional_agent_inputs: NO_INPUTS,
        injected_authorities: COMMON_INJECTED,
        core_current_authorities: &[],
    },
    ActionFormRequestProjectionDescriptor {
        submission_variant: ActionFormSubmissionVariant::ReconcileChanges,
        method: MethodName::ReconcileChanges,
        selected_semantic_variant: WorkflowActionSemanticVariant::ReconcileChanges,
        semantic_variant_selector: None,
        fixed_arguments: TASK_FIXED,
        required_agent_inputs: NO_INPUTS,
        optional_agent_inputs: RECONCILE_OPTIONAL,
        injected_authorities: COMMON_INJECTED,
        core_current_authorities: &[],
    },
    ActionFormRequestProjectionDescriptor {
        submission_variant: ActionFormSubmissionVariant::CheckClose,
        method: MethodName::CheckClose,
        selected_semantic_variant: WorkflowActionSemanticVariant::CheckClose,
        semantic_variant_selector: None,
        fixed_arguments: TASK_FIXED,
        required_agent_inputs: NO_INPUTS,
        optional_agent_inputs: NO_INPUTS,
        injected_authorities: READ_INJECTED,
        core_current_authorities: &["project_state.state_version"],
    },
    ActionFormRequestProjectionDescriptor {
        submission_variant: ActionFormSubmissionVariant::CloseTask,
        method: MethodName::CloseTask,
        selected_semantic_variant: WorkflowActionSemanticVariant::CloseTask,
        semantic_variant_selector: None,
        fixed_arguments: TASK_FIXED,
        required_agent_inputs: CLOSE_REQUIRED,
        optional_agent_inputs: NO_INPUTS,
        injected_authorities: COMMON_INJECTED,
        core_current_authorities: &[],
    },
];

pub fn action_form_request_projection_descriptors(
) -> &'static [ActionFormRequestProjectionDescriptor] {
    PROJECTIONS
}

pub fn action_form_request_projection(
    contract: &WorkflowTransitionSubmissionContract,
) -> Option<&'static ActionFormRequestProjectionDescriptor> {
    let submission_variant = match contract {
        WorkflowTransitionSubmissionContract::RecordShapingCheckpoint {
            contract: WorkflowRecordShapingCheckpointSubmissionContract::CreateInitial { .. },
        } => ActionFormSubmissionVariant::CheckpointCreate,
        WorkflowTransitionSubmissionContract::RecordShapingCheckpoint {
            contract: WorkflowRecordShapingCheckpointSubmissionContract::ReplaceCurrent { .. },
        } => ActionFormSubmissionVariant::CheckpointReplace,
        WorkflowTransitionSubmissionContract::UpdateScope {
            contract: WorkflowUpdateScopeSubmissionContract::KeepCurrentChangeUnit { .. },
        } => ActionFormSubmissionVariant::UpdateKeep,
        WorkflowTransitionSubmissionContract::UpdateScope {
            contract: WorkflowUpdateScopeSubmissionContract::GeneralCreateCurrentChangeUnit { .. },
        } => ActionFormSubmissionVariant::UpdateGeneralCreate,
        WorkflowTransitionSubmissionContract::UpdateScope {
            contract: WorkflowUpdateScopeSubmissionContract::GeneralReplaceCurrentChangeUnit { .. },
        } => ActionFormSubmissionVariant::UpdateGeneralReplace,
        WorkflowTransitionSubmissionContract::UpdateScope {
            contract: WorkflowUpdateScopeSubmissionContract::AdvisorCreateCurrentChangeUnit { .. },
        } => ActionFormSubmissionVariant::UpdateAdvisorCreate,
        WorkflowTransitionSubmissionContract::UpdateScope {
            contract: WorkflowUpdateScopeSubmissionContract::AdvisorReplaceCurrentChangeUnit { .. },
        } => ActionFormSubmissionVariant::UpdateAdvisorReplace,
        WorkflowTransitionSubmissionContract::FinalizeAdvice { .. } => {
            ActionFormSubmissionVariant::FinalizeAdvice
        }
        WorkflowTransitionSubmissionContract::AdvanceTask { .. } => {
            ActionFormSubmissionVariant::AdvanceTask
        }
        WorkflowTransitionSubmissionContract::PrepareEvidenceCapture { .. } => {
            ActionFormSubmissionVariant::PrepareEvidenceCapture
        }
        WorkflowTransitionSubmissionContract::PrepareWrite { .. } => {
            ActionFormSubmissionVariant::PrepareWrite
        }
        WorkflowTransitionSubmissionContract::StageArtifact { .. } => {
            ActionFormSubmissionVariant::StageArtifact
        }
        WorkflowTransitionSubmissionContract::RecordRun { .. } => {
            ActionFormSubmissionVariant::RecordRun
        }
        WorkflowTransitionSubmissionContract::RequestUserAction { .. } => {
            ActionFormSubmissionVariant::RequestUserAction
        }
        WorkflowTransitionSubmissionContract::ResolveUserAction { .. } => return None,
        WorkflowTransitionSubmissionContract::ReconcileChanges { .. } => {
            ActionFormSubmissionVariant::ReconcileChanges
        }
        WorkflowTransitionSubmissionContract::CheckClose { .. } => {
            ActionFormSubmissionVariant::CheckClose
        }
        WorkflowTransitionSubmissionContract::CloseTask { .. } => {
            ActionFormSubmissionVariant::CloseTask
        }
    };
    PROJECTIONS
        .iter()
        .find(|descriptor| descriptor.submission_variant == submission_variant)
}

/// Selects the submitted method-owned semantic variant through the canonical
/// action-form projection descriptors.
pub fn submitted_action_form_semantic_variant(
    method: MethodName,
    request: &Value,
) -> Option<WorkflowActionSemanticVariant> {
    let mut projections = PROJECTIONS
        .iter()
        .filter(|descriptor| descriptor.method == method);
    let first = projections.next()?;
    if projections.next().is_none() {
        return first
            .semantic_variant_selector
            .is_none()
            .then_some(first.selected_semantic_variant);
    }
    PROJECTIONS
        .iter()
        .filter(|descriptor| descriptor.method == method)
        .find(|descriptor| {
            descriptor
                .semantic_variant_selector
                .is_some_and(|selector| {
                    request.pointer(selector.path).and_then(Value::as_str) == Some(selector.value)
                })
        })
        .map(|descriptor| descriptor.selected_semantic_variant)
}

impl ActionFormRequestProjectionDescriptor {
    /// Expands every descriptor wildcard to the exact current fixed-argument path.
    pub fn concrete_fixed_argument_paths(
        &self,
        fixed_arguments: &JsonObject,
    ) -> Result<Vec<String>, String> {
        let fixed = Value::Object(fixed_arguments.clone());
        let mut paths = Vec::new();
        for binding in self.fixed_arguments {
            if let Some((prefix, suffix)) = binding.path_pattern.split_once("/*") {
                let values = fixed
                    .pointer(prefix)
                    .and_then(Value::as_array)
                    .ok_or_else(|| {
                        format!(
                            "{} fixed wildcard prefix {} is not an array",
                            self.method.as_str(),
                            prefix
                        )
                    })?;
                for index in 0..values.len() {
                    paths.push(format!("{prefix}/{index}{suffix}"));
                }
            } else {
                paths.push(binding.path_pattern.to_owned());
            }
        }
        paths.sort();
        paths.dedup();
        for path in &paths {
            if fixed.pointer(path).is_none() {
                return Err(format!(
                    "{} fixed argument {} is absent from the current form",
                    self.method.as_str(),
                    path
                ));
            }
        }
        Ok(paths)
    }
}

/// Validates every projection against its canonical semantic request descriptor.
pub fn action_form_request_projection_integrity_errors() -> Vec<String> {
    let mut errors = Vec::new();
    let mut variants = BTreeSet::new();
    let mut projected_methods = BTreeSet::new();
    for projection in PROJECTIONS {
        if !variants.insert(projection.submission_variant) {
            errors.push(format!(
                "duplicate action-form submission projection for {} {}",
                projection.method.as_str(),
                projection.selected_semantic_variant.as_str()
            ));
        }
        projected_methods.insert(projection.method.as_str());
        let method_projection_count = PROJECTIONS
            .iter()
            .filter(|candidate| candidate.method == projection.method)
            .map(|candidate| candidate.selected_semantic_variant)
            .collect::<BTreeSet<_>>()
            .len();
        match (
            method_projection_count,
            projection.semantic_variant_selector,
        ) {
            (1, Some(_)) => errors.push(format!(
                "{} has one semantic variant but declares a selector",
                projection.method.as_str()
            )),
            (2.., None) => errors.push(format!(
                "{} has multiple semantic variants but no selector for {}",
                projection.method.as_str(),
                projection.selected_semantic_variant.as_str()
            )),
            (_, Some(selector))
                if !projection
                    .fixed_arguments
                    .iter()
                    .any(|fixed| fixed.path_pattern == selector.path) =>
            {
                errors.push(format!(
                    "{} semantic-variant selector {} is not a fixed argument",
                    projection.method.as_str(),
                    selector.path
                ));
            }
            _ => {}
        }
        let Some(tool) = AgentToolId::from_method(projection.method) else {
            errors.push(format!(
                "{} has no canonical MCP tool",
                projection.method.as_str()
            ));
            continue;
        };
        let Some(contract) = mcp_tool_contract(tool) else {
            errors.push(format!(
                "{} has no semantic request descriptor",
                projection.method.as_str()
            ));
            continue;
        };
        let mut authority_owners = BTreeMap::new();
        for fixed in projection.fixed_arguments {
            if contract
                .input_descriptor()
                .semantic_types_at_pointer_pattern(fixed.path_pattern)
                .is_empty()
            {
                errors.push(format!(
                    "{} fixed pointer {} does not resolve in {}",
                    projection.method.as_str(),
                    fixed.path_pattern,
                    contract.input_descriptor().semantic_type()
                ));
            }
            if let Some(existing) = authority_owners.insert(fixed.authority, fixed.path_pattern) {
                errors.push(format!(
                    "{} authority {} is fixed by both {} and {}",
                    projection.method.as_str(),
                    fixed.authority,
                    existing,
                    fixed.path_pattern
                ));
            }
            for authored in projection
                .required_agent_inputs
                .iter()
                .chain(projection.optional_agent_inputs)
            {
                if paths_overlap(fixed.path_pattern, authored.path_pattern) {
                    errors.push(format!(
                        "{} fixed pointer {} overlaps Agent-authored slot {}",
                        projection.method.as_str(),
                        fixed.path_pattern,
                        authored.path_pattern
                    ));
                }
            }
        }
        for authored in projection
            .required_agent_inputs
            .iter()
            .chain(projection.optional_agent_inputs)
        {
            if contract
                .input_descriptor()
                .semantic_types_at_pointer_pattern(authored.path_pattern)
                .is_empty()
            {
                errors.push(format!(
                    "{} Agent-authored pointer {} does not resolve in {}",
                    projection.method.as_str(),
                    authored.path_pattern,
                    contract.input_descriptor().semantic_type()
                ));
            }
        }
        for injected in projection.injected_authorities {
            if let Some(existing) =
                authority_owners.insert(injected.authority, injected.canonical_request_target)
            {
                errors.push(format!(
                    "{} authority {} is owned by both {} and {}",
                    projection.method.as_str(),
                    injected.authority,
                    existing,
                    injected.canonical_request_target
                ));
            }
        }
        for current in projection.core_current_authorities {
            if let Some(existing) = authority_owners.insert(current, "Core current state") {
                errors.push(format!(
                    "{} authority {} is owned by both {} and Core current state",
                    projection.method.as_str(),
                    current,
                    existing
                ));
            }
        }
    }
    for contract in PUBLIC_METHOD_CONTRACTS {
        if contract.workflow_action_admission() == WorkflowActionAdmissionClass::TaskStateBound
            && !projected_methods.contains(contract.method().as_str())
        {
            errors.push(format!(
                "{} is Task-state-bound but has no action-form request projection",
                contract.method().as_str()
            ));
        }
    }
    errors.sort();
    errors
}

fn paths_overlap(left: &str, right: &str) -> bool {
    let left = left.replace("/*/", "/#/");
    let right = right.replace("/*/", "/#/");
    left == right
        || right
            .strip_prefix(&left)
            .is_some_and(|suffix| suffix.starts_with('/'))
        || left
            .strip_prefix(&right)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_state_bound_projection_is_integral() {
        let errors = action_form_request_projection_integrity_errors();
        assert!(errors.is_empty(), "{}", errors.join("\n"));
    }

    #[test]
    fn submitted_variants_are_selected_only_by_canonical_projection_descriptors() {
        assert_eq!(
            submitted_action_form_semantic_variant(
                MethodName::UpdateScope,
                &serde_json::json!({"change_unit": {"operation": "replace_current"}}),
            ),
            Some(WorkflowActionSemanticVariant::ReplaceCurrentChangeUnit)
        );
        assert_eq!(
            submitted_action_form_semantic_variant(
                MethodName::RecordShapingCheckpoint,
                &serde_json::json!({"checkpoint_operation": {"operation": "create_initial"}}),
            ),
            Some(WorkflowActionSemanticVariant::CreateInitial)
        );
        assert_eq!(
            submitted_action_form_semantic_variant(
                MethodName::PrepareWrite,
                &Value::Object(Default::default())
            ),
            Some(WorkflowActionSemanticVariant::PrepareWrite)
        );
        assert_eq!(
            submitted_action_form_semantic_variant(
                MethodName::UpdateScope,
                &serde_json::json!({"change_unit": {"operation": "removed_variant"}}),
            ),
            None
        );
    }
}
