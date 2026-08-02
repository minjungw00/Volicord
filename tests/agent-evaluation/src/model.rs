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
}

impl TaskGroup {
    pub const ALL: [Self; 12] = [
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
