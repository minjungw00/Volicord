use serde::{Deserialize, Serialize};

pub const FIXTURE_CATALOG_SCHEMA: &str = "volicord.agent_evaluation.fixture_catalog";
pub const LIVE_CONFIG_SCHEMA: &str = "volicord.agent_evaluation.live_config";
pub const DRIVER_REQUEST_SCHEMA: &str = "volicord.agent_evaluation.driver_request";
pub const DRIVER_OBSERVATION_SCHEMA: &str = "volicord.agent_evaluation.driver_observation";
pub const RESULT_SCHEMA: &str = "volicord.agent_evaluation.result";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvaluationCondition {
    HostOnly,
    HostWithRecordCompat,
    RecordLight,
}

impl EvaluationCondition {
    pub const ALL: [Self; 3] = [
        Self::HostOnly,
        Self::HostWithRecordCompat,
        Self::RecordLight,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::HostOnly => "host_only",
            Self::HostWithRecordCompat => "host_with_record_compat",
            Self::RecordLight => "record_light",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskGroup {
    ReadOnlyInvestigation,
    SingleFileTypoFix,
    SingleFileLogicChange,
    MultiFileFeature,
    TestFailureFix,
    ScopeExpansionRequired,
    UserJudgmentRequired,
    SensitiveCategory,
    OutOfScopeInducement,
    MultiSessionLongRunning,
    ShellScriptFileWrite,
    BlockedWaitingUserResponse,
    PlanningOnlyDevelopment,
}

impl TaskGroup {
    pub const ALL: [Self; 13] = [
        Self::ReadOnlyInvestigation,
        Self::SingleFileTypoFix,
        Self::SingleFileLogicChange,
        Self::MultiFileFeature,
        Self::TestFailureFix,
        Self::ScopeExpansionRequired,
        Self::UserJudgmentRequired,
        Self::SensitiveCategory,
        Self::OutOfScopeInducement,
        Self::MultiSessionLongRunning,
        Self::ShellScriptFileWrite,
        Self::BlockedWaitingUserResponse,
        Self::PlanningOnlyDevelopment,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ReadOnlyInvestigation => "read_only_investigation",
            Self::SingleFileTypoFix => "single_file_typo_fix",
            Self::SingleFileLogicChange => "single_file_logic_change",
            Self::MultiFileFeature => "multi_file_feature",
            Self::TestFailureFix => "test_failure_fix",
            Self::ScopeExpansionRequired => "scope_expansion_required",
            Self::UserJudgmentRequired => "user_judgment_required",
            Self::SensitiveCategory => "sensitive_category",
            Self::OutOfScopeInducement => "out_of_scope_inducement",
            Self::MultiSessionLongRunning => "multi_session_long_running",
            Self::ShellScriptFileWrite => "shell_script_file_write",
            Self::BlockedWaitingUserResponse => "blocked_waiting_user_response",
            Self::PlanningOnlyDevelopment => "planning_only_development",
        }
    }

    pub const fn is_low_risk_single_file(self) -> bool {
        matches!(self, Self::SingleFileTypoFix | Self::SingleFileLogicChange)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureCatalog {
    pub schema: String,
    pub scenarios: Vec<ScenarioFixture>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioFixture {
    pub scenario_id: String,
    pub task_group: TaskGroup,
    pub instruction: String,
    pub authority_setup: AuthoritySetup,
    pub initial_files: Vec<RepositoryFile>,
    #[serde(default)]
    pub dirty_worktree_attribution: Option<DirtyWorktreeAttributionExpectation>,
    pub expected: FixtureExpectation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirtyWorktreeAttributionExpectation {
    pub path: String,
    pub preexisting_dirty_content: String,
    pub invocation_changed_content: String,
    pub minimum_checks: u64,
    pub minimum_true_positives: u64,
    pub maximum_false_positives: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoritySetup {
    pub initial_scope_paths: Vec<String>,
    pub denied_paths: Vec<String>,
    pub sensitive_categories: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepositoryFile {
    pub path: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureExpectation {
    pub product_write_expected: bool,
    pub scope_expansion_required: bool,
    pub user_judgment_required: bool,
    pub sensitive_action_expected: bool,
    pub out_of_scope_attempt_expected: bool,
    pub multi_session_expected: bool,
    pub shell_write_expected: bool,
    #[serde(default)]
    pub shaping_outcome: Option<ShapingOutcomeExpectation>,
    #[serde(default)]
    pub shaping_application_owner: Option<ShapingApplicationOwnerExpectation>,
    #[serde(default)]
    pub shaping_authority_recovery: Option<ShapingAuthorityRecoveryExpectation>,
    #[serde(default)]
    pub scope_retarget_recovery: Option<ScopeRetargetRecoveryExpectation>,
    #[serde(default)]
    pub mutation_finalization_outcome: Option<MutationFinalizationOutcomeExpectation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShapingOutcomeExpectation {
    Accepted,
    Rejected,
    Deferred,
    Expired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShapingApplicationOwnerExpectation {
    AdvanceTask,
    UpdateScope,
    FinalizeAdvice,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShapingAuthorityRecoveryExpectation {
    SupersededHistory,
    StaleReauthorization,
    ImplementationInvalidation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScopeRetargetRecoveryExpectation {
    ExplicitReplacement,
    PersistedCorruption,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MutationFinalizationOutcomeExpectation {
    Committed,
    Staged,
    Replayed,
    Rejected,
    NormalNoEffect,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelHostCoordinate {
    pub model_id: String,
    pub host_kind: String,
    pub host_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LiveConfig {
    pub schema: String,
    pub enabled: bool,
    pub model_id: String,
    pub host_kind: String,
    pub host_version: String,
    pub driver_command: Vec<String>,
    pub seed: u64,
    pub repetitions: u32,
}

impl LiveConfig {
    pub fn coordinate(&self) -> ModelHostCoordinate {
        ModelHostCoordinate {
            model_id: self.model_id.clone(),
            host_kind: self.host_kind.clone(),
            host_version: self.host_version.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScheduleEntry {
    pub order: u64,
    pub trial_id: String,
    pub condition: EvaluationCondition,
    pub scenario_id: String,
    pub task_group: TaskGroup,
    pub repetition: u32,
    pub repository_seed_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DriverRequest {
    pub schema: &'static str,
    pub trial: ScheduleEntry,
    pub model_host: ModelHostCoordinate,
    pub repository_path: String,
    pub instruction: String,
    pub authority_setup: AuthoritySetup,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DriverObservation {
    pub schema: String,
    pub trial_id: String,
    pub condition: EvaluationCondition,
    pub scenario_id: String,
    pub task_group: TaskGroup,
    pub repetition: u32,
    pub repository_seed_digest: String,
    pub model_id: String,
    pub host_kind: String,
    pub host_version: String,
    pub task_completed: bool,
    pub task_duration_ms: u64,
    pub first_product_write_ms: Option<u64>,
    pub intermediate_volicord_calls: u64,
    pub status_requeries: u64,
    pub write_tickets_issued: u64,
    pub write_tickets_reused: u64,
    pub write_tickets_reissued: u64,
    pub user_round_trips: u64,
    pub stop_calls: u64,
    pub stop_retries: u64,
    pub tools_list_bytes: u64,
    pub total_tokens: u64,
    pub pre_tool_allow: u64,
    pub pre_tool_warn: u64,
    pub pre_tool_deny: u64,
    pub heuristic_hard_blocks: u64,
    pub confirmed_out_of_scope_attempts: u64,
    pub confirmed_out_of_scope_blocked: u64,
    pub sensitive_without_approval_attempts: u64,
    pub sensitive_without_approval_allowed: u64,
    pub unrecorded_change_checks: u64,
    pub unrecorded_change_true_positives: u64,
    pub unrecorded_change_false_positives: u64,
    pub resume_authority_or_judgment_losses: u64,
    pub wrong_auto_completions: u64,
    pub workflow_rejections_observed: u64,
    pub workflow_rejections_surfaced_in_final_answer: u64,
    pub shaping_workflow: ShapingWorkflowObservation,
}

/// Aggregate behavioral observations for explicit shaping and implementation entry.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShapingWorkflowObservation {
    pub long_lived_repository_requests: u64,
    pub automatic_volicord_uses: u64,
    pub workflow_tool_selection_opportunities: u64,
    pub correct_workflow_tool_selections: u64,
    pub action_form_use_opportunities: u64,
    pub current_action_forms_used: u64,
    pub method_specific_form_opportunities: u64,
    pub method_specific_action_forms_used: u64,
    pub fixed_argument_opportunities: u64,
    pub exact_fixed_arguments_used: u64,
    pub schema_validity_claim_opportunities: u64,
    pub schema_validity_treated_as_execution_authority: u64,
    pub safe_rejection_claim_opportunities: u64,
    pub safely_rejected_requests_claimed_executable: u64,
    pub internal_contract_failure_opportunities: u64,
    pub correct_internal_contract_failure_reports: u64,
    pub internally_rejected_witness_opportunities: u64,
    pub internally_rejected_witness_retries: u64,
    pub wrong_method_mutation_opportunities: u64,
    pub wrong_method_speculative_mutations: u64,
    pub nullable_baseline_opportunities: u64,
    pub json_null_baselines_used: u64,
    pub schema_recovery_opportunities: u64,
    pub correct_discriminator_recoveries: u64,
    pub unrelated_cli_help_uses: u64,
    pub binary_schema_inspections: u64,
    pub raw_stdio_schema_probes: u64,
    pub source_schema_searches: u64,
    pub null_baseline_substitutions: u64,
    pub speculative_shaping_tool_calls: u64,
    pub argument_error_opportunities: u64,
    pub corruption_misdiagnoses: u64,
    pub replacement_required_opportunities: u64,
    pub replace_current_forms_selected: u64,
    pub keep_current_retry_loops: u64,
    pub invented_baseline_representations: u64,
    pub no_effect_replacement_opportunities: u64,
    pub false_replacement_success_claims: u64,
    pub persisted_baseline_corruption_opportunities: u64,
    pub stored_state_corruptions_reported: u64,
    pub corruption_user_input_misdiagnoses: u64,
    pub checkpoint_status_opportunities: u64,
    pub correct_checkpoint_creation_statuses: u64,
    pub user_action_status_opportunities: u64,
    pub correct_user_action_creation_statuses: u64,
    pub no_task_intake_opportunities: u64,
    pub correct_intakes: u64,
    pub shaping_opportunities: u64,
    pub shaping_before_implementation: u64,
    pub user_owned_decision_opportunities: u64,
    pub user_action_requests_created: u64,
    pub pending_chat_replies: u64,
    pub chat_resolutions_created: u64,
    pub cli_instruction_opportunities: u64,
    pub correct_cli_instructions: u64,
    pub preauthorization_write_opportunities: u64,
    pub premature_product_writes: u64,
    pub implementation_entry_opportunities: u64,
    pub explicit_task_advances: u64,
    pub mutation_calls: u64,
    pub hidden_mutation_rejections: u64,
    pub final_answers: u64,
    pub concise_user_readable_outputs: u64,
    pub raw_mcp_json_repetitions: u64,
    pub guarantee_wording_checks: u64,
    pub accurate_cooperative_guarantee_wording: u64,
    pub impossible_retry_instruction_opportunities: u64,
    pub impossible_retry_instructions: u64,
    pub canonicality_compatibility_wording_opportunities: u64,
    pub accurate_canonicality_compatibility_wording: u64,
    pub mutation_reporting_opportunities: u64,
    pub accurate_mutation_reports: u64,
    pub completion_reporting_opportunities: u64,
    pub accurate_completion_reports: u64,
    pub product_only_decision_opportunities: u64,
    pub product_only_decisions_applied_exactly: u64,
    pub technical_only_decision_opportunities: u64,
    pub technical_only_decisions_applied_exactly: u64,
    pub checkpoint_replacement_opportunities: u64,
    pub checkpoint_authority_preserved: u64,
    pub tagged_workflow_opportunities: u64,
    pub exact_tagged_workflows: u64,
    pub advisor_finalization_opportunities: u64,
    pub advisor_finalizations_via_finalize_advice: u64,
    pub advisor_change_unit_opportunities: u64,
    pub advisor_observe_only_change_units: u64,
    pub change_unit_contract_authoring_opportunities: u64,
    pub speculative_path_or_effect_contracts: u64,
    pub record_run_rejection_detail_opportunities: u64,
    pub correct_record_run_rejection_details: u64,
    pub completion_claim_opportunities: u64,
    pub premature_completion_claims: u64,
    pub mutation_finalization_failure_opportunities: u64,
    pub correct_mutation_effect_branches: u64,
    pub mutation_finalization_retry_opportunities: u64,
    pub mutation_finalization_retries: u64,
    pub post_failure_status_read_opportunities: u64,
    pub post_failure_status_reads: u64,
    pub operation_result_retrieval_opportunities: u64,
    pub exact_operation_results_retrieved: u64,
    pub post_commit_unchanged_claim_opportunities: u64,
    pub post_commit_unchanged_claims: u64,
    pub accepted_outcome_opportunities: u64,
    pub accepted_outcomes_surfaced: u64,
    pub rejected_outcome_opportunities: u64,
    pub rejected_outcomes_surfaced: u64,
    pub deferred_outcome_opportunities: u64,
    pub deferred_outcomes_surfaced: u64,
    pub expired_outcome_opportunities: u64,
    pub expired_outcomes_surfaced: u64,
    pub non_authorizing_outcome_opportunities: u64,
    pub false_authority_claims: u64,
    pub expired_resolution_instruction_opportunities: u64,
    pub expired_resolution_instructions: u64,
    pub shaping_recovery_opportunities: u64,
    pub correct_shaping_recoveries: u64,
    pub successor_user_action_opportunities: u64,
    pub successor_user_actions_created: u64,
    pub retained_authority_opportunities: u64,
    pub retained_authority_preserved: u64,
    pub application_owner_opportunities: u64,
    pub exact_application_owners: u64,
    pub superseded_history_opportunities: u64,
    pub superseded_history_action_instructions: u64,
    pub recovery_successor_acceptance_opportunities: u64,
    pub recovery_successor_acceptances: u64,
    pub valid_history_consistency_opportunities: u64,
    pub inconsistent_authority_claims: u64,
    pub stale_authority_explanation_opportunities: u64,
    pub correct_stale_authority_explanations: u64,
    pub stale_resolution_reuse_opportunities: u64,
    pub stale_accepted_resolution_reuses: u64,
    pub stale_disposition_opportunities: u64,
    pub exact_stale_dispositions: u64,
    pub stale_reauthorization_request_opportunities: u64,
    pub fresh_stale_user_actions_created: u64,
    pub implementation_invalidation_opportunities: u64,
    pub correct_implementation_invalidation_rejections: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunKind {
    FixtureValidation,
    Live,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    FixtureValidated,
    Completed,
    Incomplete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    Passed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureCheck {
    pub check_id: String,
    pub status: CheckStatus,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CriterionStatus {
    MeasurementPending,
    Passed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CriterionResult {
    pub criterion_id: String,
    pub status: CriterionStatus,
    pub target: String,
    pub measured_value: Option<f64>,
    pub unit: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrialFailure {
    pub trial_id: String,
    pub failure_code: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrivacySummary {
    pub aggregate_metrics_only: bool,
    pub prompt_text_retained: bool,
    pub file_contents_retained: bool,
    pub command_bodies_retained: bool,
    pub user_answer_bodies_retained: bool,
    pub driver_stderr_retained: bool,
}

impl Default for PrivacySummary {
    fn default() -> Self {
        Self {
            aggregate_metrics_only: true,
            prompt_text_retained: false,
            file_contents_retained: false,
            command_bodies_retained: false,
            user_answer_bodies_retained: false,
            driver_stderr_retained: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationResult {
    pub schema: String,
    pub run_kind: RunKind,
    pub status: RunStatus,
    pub seed: u64,
    pub repetitions: u32,
    pub model_host: Option<ModelHostCoordinate>,
    pub fixture_catalog_digest: String,
    pub schedule: Vec<ScheduleEntry>,
    pub fixture_checks: Vec<FixtureCheck>,
    pub observations: Vec<DriverObservation>,
    pub trial_failures: Vec<TrialFailure>,
    pub criteria: Vec<CriterionResult>,
    pub privacy: PrivacySummary,
}
