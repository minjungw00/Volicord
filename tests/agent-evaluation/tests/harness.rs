use std::{collections::BTreeSet, fs};

use volicord_agent_evaluation::{
    build_schedule, evaluate_live_criteria, fixture_evaluation, live_config_example_path,
    load_embedded_catalog, load_live_config, materialize_repository, result_schema_text,
    run_live_with_driver, validate_catalog, validate_live_config, validate_schedule_matrix,
    write_result_create_new, CriterionResult, CriterionStatus, DriverFailure, DriverObservation,
    DriverRequest, EvaluationCondition, EvaluationResult, LiveConfig, RunKind, RunStatus,
    TaskGroup, TrialDriver, DRIVER_OBSERVATION_SCHEMA, LIVE_CONFIG_SCHEMA, RESULT_SCHEMA,
};

const SEED: u64 = 20_260_716;

#[test]
fn catalog_covers_the_three_by_twelve_evaluation_surface() {
    let catalog = load_embedded_catalog().expect("embedded catalog should be valid");
    assert_eq!(EvaluationCondition::ALL.len(), 3);
    assert_eq!(catalog.scenarios.len(), TaskGroup::ALL.len() + 1);

    let actual = catalog
        .scenarios
        .iter()
        .map(|scenario| scenario.task_group)
        .collect::<BTreeSet<_>>();
    let expected = TaskGroup::ALL.into_iter().collect::<BTreeSet<_>>();
    assert_eq!(actual, expected);
}

#[test]
fn schedule_is_seeded_randomized_repeated_and_complete() {
    let catalog = load_embedded_catalog().expect("embedded catalog should be valid");
    let first = build_schedule(&catalog, 3, SEED).expect("schedule should build");
    let repeated = build_schedule(&catalog, 3, SEED).expect("schedule should repeat");
    let different_seed = build_schedule(&catalog, 3, SEED + 1).expect("schedule should build");

    assert_eq!(first, repeated);
    assert_ne!(first, different_seed);
    assert_eq!(
        first.len(),
        EvaluationCondition::ALL.len() * catalog.scenarios.len() * 3
    );
    validate_schedule_matrix(&first, 3).expect("matrix should be complete");

    for repetition in 1..=3 {
        for scenario in &catalog.scenarios {
            let matching = first
                .iter()
                .filter(|trial| {
                    trial.scenario_id == scenario.scenario_id && trial.repetition == repetition
                })
                .collect::<Vec<_>>();
            assert_eq!(matching.len(), EvaluationCondition::ALL.len());
            assert_eq!(
                matching
                    .iter()
                    .map(|trial| trial.repository_seed_digest.as_str())
                    .collect::<BTreeSet<_>>()
                    .len(),
                1
            );
        }
    }
}

#[test]
fn schedule_validation_rejects_coordinate_or_repository_drift() {
    let catalog = load_embedded_catalog().expect("embedded catalog should be valid");
    let schedule = build_schedule(&catalog, 1, SEED).expect("schedule should build");

    let mut coordinate_drift = schedule.clone();
    coordinate_drift[0].repetition = 2;
    assert!(validate_schedule_matrix(&coordinate_drift, 1).is_err());

    let mut repository_drift = schedule;
    repository_drift[0].repository_seed_digest = "fnv1a64:0000000000000000".to_owned();
    assert!(validate_schedule_matrix(&repository_drift, 1).is_err());
}

#[test]
fn fixture_result_leaves_all_quantitative_live_criteria_measurement_pending() {
    let result = fixture_evaluation(SEED, 2).expect("fixture evaluation should succeed");

    assert_eq!(result.schema, RESULT_SCHEMA);
    assert_eq!(result.run_kind, RunKind::FixtureValidation);
    assert_eq!(result.status, RunStatus::FixtureValidated);
    assert!(result.model_host.is_none());
    assert!(result.observations.is_empty());
    assert!(result.trial_failures.is_empty());
    assert_eq!(result.criteria.len(), 10);
    assert!(result.criteria.iter().all(|criterion| {
        criterion.status == CriterionStatus::MeasurementPending
            && criterion.measured_value.is_none()
            && criterion
                .reason
                .contains("no live model or host observations")
    }));
    assert!(result.privacy.aggregate_metrics_only);
    assert!(!result.privacy.prompt_text_retained);
    assert!(!result.privacy.file_contents_retained);
    assert!(!result.privacy.command_bodies_retained);
    assert!(!result.privacy.user_answer_bodies_retained);
    assert!(!result.privacy.driver_stderr_retained);
}

#[test]
fn fixture_repositories_are_fresh_and_content_identical() {
    let catalog = load_embedded_catalog().expect("embedded catalog should be valid");
    let scenario = &catalog.scenarios[0];
    let first = materialize_repository(scenario).expect("first repository should materialize");
    let second = materialize_repository(scenario).expect("second repository should materialize");

    assert_ne!(first.path(), second.path());
    for file in &scenario.initial_files {
        assert_eq!(
            fs::read_to_string(first.path().join(&file.path))
                .expect("first file should be readable"),
            file.content
        );
        assert_eq!(
            fs::read_to_string(second.path().join(&file.path))
                .expect("second file should be readable"),
            file.content
        );
    }

    let first_file = first.path().join(&scenario.initial_files[0].path);
    fs::write(&first_file, "changed only in the first repository\n")
        .expect("temporary fixture should be writable");
    assert_eq!(
        fs::read_to_string(second.path().join(&scenario.initial_files[0].path))
            .expect("second repository should remain readable"),
        scenario.initial_files[0].content
    );
}

#[test]
fn dirty_worktree_fixture_materializes_a_stable_preexisting_change() {
    let catalog = load_embedded_catalog().expect("embedded catalog should be valid");
    let scenario = catalog
        .scenarios
        .iter()
        .find(|scenario| scenario.dirty_worktree_attribution.is_some())
        .expect("dirty-worktree attribution scenario should exist");
    let attribution = scenario
        .dirty_worktree_attribution
        .as_ref()
        .expect("attribution fixture");
    let repository = materialize_repository(scenario).expect("dirty repository should materialize");

    assert_eq!(
        fs::read_to_string(repository.path().join(&attribution.path))
            .expect("dirty fixture path should be readable"),
        attribution.preexisting_dirty_content
    );
    let status = std::process::Command::new("git")
        .arg("-C")
        .arg(repository.path())
        .args(["status", "--porcelain", "--", &attribution.path])
        .output()
        .expect("Git status should run");
    assert!(status.status.success());
    assert_eq!(
        String::from_utf8(status.stdout).expect("Git status should be UTF-8"),
        format!(" M {}\n", attribution.path)
    );
}

#[test]
fn catalog_rejects_repository_path_traversal() {
    let mut catalog = load_embedded_catalog().expect("embedded catalog should be valid");
    catalog.scenarios[0].initial_files[0].path = "../outside".to_owned();
    let error = validate_catalog(&catalog).expect_err("path traversal must be rejected");
    assert!(error.to_string().contains("traversal-free"));
}

#[test]
fn result_schema_and_disabled_live_example_are_parseable() {
    let schema: serde_json::Value =
        serde_json::from_str(result_schema_text()).expect("result schema should be valid JSON");
    assert_eq!(
        schema["properties"]["schema"]["const"],
        serde_json::Value::String(RESULT_SCHEMA.to_owned())
    );
    assert_eq!(
        schema["properties"]["criteria"]["minItems"].as_u64(),
        Some(10)
    );
    assert_eq!(
        schema["properties"]["criteria"]["maxItems"].as_u64(),
        Some(10)
    );

    let config = load_live_config(&live_config_example_path())
        .expect("example live configuration should parse");
    assert_eq!(config.schema, LIVE_CONFIG_SCHEMA);
    assert!(!config.enabled);
    let error = validate_live_config(&config).expect_err("example must not start a live run");
    assert!(error.to_string().contains("disabled"));
}

#[derive(Default)]
struct AggregateSyntheticDriver;

impl TrialDriver for AggregateSyntheticDriver {
    fn run_trial(
        &mut self,
        request: &DriverRequest,
        _repository_root: &std::path::Path,
    ) -> Result<DriverObservation, DriverFailure> {
        let volicord = request.trial.condition != EvaluationCondition::HostOnly;
        let record = request.trial.condition == EvaluationCondition::RecordLight;
        let out_of_scope = record && request.trial.task_group == TaskGroup::OutOfScopeInducement;
        let sensitive = volicord && request.trial.task_group == TaskGroup::SensitiveCategory;
        Ok(DriverObservation {
            schema: DRIVER_OBSERVATION_SCHEMA.to_owned(),
            trial_id: request.trial.trial_id.clone(),
            condition: request.trial.condition,
            scenario_id: request.trial.scenario_id.clone(),
            task_group: request.trial.task_group,
            repetition: request.trial.repetition,
            repository_seed_digest: request.trial.repository_seed_digest.clone(),
            model_id: request.model_host.model_id.clone(),
            host_kind: request.model_host.host_kind.clone(),
            host_version: request.model_host.host_version.clone(),
            task_completed: !matches!(
                request.trial.task_group,
                TaskGroup::UserJudgmentRequired
                    | TaskGroup::SensitiveCategory
                    | TaskGroup::BlockedWaitingUserResponse
            ),
            task_duration_ms: 1_000,
            first_product_write_ms: Some(250),
            intermediate_volicord_calls: if volicord { 4 } else { 0 },
            status_requeries: 0,
            write_tickets_issued: u64::from(volicord),
            write_tickets_reused: 0,
            write_tickets_reissued: 0,
            user_round_trips: 0,
            stop_calls: u64::from(volicord),
            stop_retries: 0,
            tools_list_bytes: if volicord { 32_000 } else { 0 },
            total_tokens: match request.trial.condition {
                EvaluationCondition::HostWithRecordCompat => 100,
                EvaluationCondition::RecordLight => 105,
                _ => 100,
            },
            pre_tool_allow: u64::from(volicord),
            pre_tool_warn: 0,
            pre_tool_deny: u64::from(out_of_scope),
            heuristic_hard_blocks: 0,
            confirmed_out_of_scope_attempts: u64::from(out_of_scope),
            confirmed_out_of_scope_blocked: u64::from(out_of_scope),
            sensitive_without_approval_attempts: u64::from(sensitive),
            sensitive_without_approval_allowed: 0,
            unrecorded_change_checks: if record { 100 } else { 0 },
            unrecorded_change_true_positives: u64::from(
                record && request.trial.scenario_id == "dirty-worktree-same-path-attribution",
            ),
            unrecorded_change_false_positives: 0,
            resume_authority_or_judgment_losses: 0,
            wrong_auto_completions: 0,
        })
    }
}

#[test]
fn complete_synthetic_driver_matrix_exercises_every_live_criterion() {
    let config = enabled_test_config();
    let result = run_live_with_driver(&config, &mut AggregateSyntheticDriver)
        .expect("synthetic in-process matrix should run");

    assert_eq!(result.run_kind, RunKind::Live);
    assert_eq!(result.status, RunStatus::Completed);
    assert_eq!(
        result.schedule.len(),
        EvaluationCondition::ALL.len() * (TaskGroup::ALL.len() + 1)
    );
    assert_eq!(result.observations.len(), result.schedule.len());
    assert!(result.trial_failures.is_empty());
    assert!(result
        .criteria
        .iter()
        .all(|criterion| criterion.status == CriterionStatus::Passed));
}

struct MissingDirtyAttributionDriver;

impl TrialDriver for MissingDirtyAttributionDriver {
    fn run_trial(
        &mut self,
        request: &DriverRequest,
        repository_root: &std::path::Path,
    ) -> Result<DriverObservation, DriverFailure> {
        let mut observation = AggregateSyntheticDriver.run_trial(request, repository_root)?;
        if request.trial.scenario_id == "dirty-worktree-same-path-attribution" {
            observation.unrecorded_change_true_positives = 0;
        }
        Ok(observation)
    }
}

#[test]
fn live_runner_requires_dirty_worktree_true_positive_and_zero_false_positives() {
    let result = run_live_with_driver(&enabled_test_config(), &mut MissingDirtyAttributionDriver)
        .expect("missing attribution should produce an incomplete structured result");

    assert_eq!(result.status, RunStatus::Incomplete);
    assert_eq!(result.trial_failures.len(), 1);
    assert!(result
        .trial_failures
        .iter()
        .all(|failure| failure.detail.contains("dirty-worktree attribution checks")));
}

#[test]
fn compatibility_baseline_defects_do_not_fail_modified_condition_criteria() {
    let mut result = run_live_with_driver(&enabled_test_config(), &mut AggregateSyntheticDriver)
        .expect("synthetic in-process matrix should run");
    for observation in result
        .observations
        .iter_mut()
        .filter(|observation| observation.condition == EvaluationCondition::HostWithRecordCompat)
    {
        observation.stop_retries = observation.stop_calls;
        observation.heuristic_hard_blocks = 1;
        observation.wrong_auto_completions = 1;
        if observation.task_group == TaskGroup::SensitiveCategory {
            observation.sensitive_without_approval_allowed = 1;
        }
        if observation.task_group == TaskGroup::MultiSessionLongRunning {
            observation.resume_authority_or_judgment_losses = 1;
        }
    }

    let baseline_only_defects = evaluate_live_criteria(&result.observations);
    for criterion_id in [
        "stop_retries",
        "shell_heuristic_hard_blocks",
        "sensitive_write_without_approval_allowed",
        "long_task_resume_authority_or_judgment_loss",
        "wrong_automatic_completion",
    ] {
        assert_eq!(
            status_for(&baseline_only_defects, criterion_id),
            CriterionStatus::Passed
        );
    }

    let record_light = result
        .observations
        .iter_mut()
        .find(|observation| observation.condition == EvaluationCondition::RecordLight)
        .expect("record_light observation should exist");
    record_light.stop_retries = record_light.stop_calls;
    let modified_defect = evaluate_live_criteria(&result.observations);
    assert_eq!(
        status_for(&modified_defect, "stop_retries"),
        CriterionStatus::Failed
    );
}

struct CoordinateMismatchDriver;

impl TrialDriver for CoordinateMismatchDriver {
    fn run_trial(
        &mut self,
        request: &DriverRequest,
        repository_root: &std::path::Path,
    ) -> Result<DriverObservation, DriverFailure> {
        let mut observation = AggregateSyntheticDriver.run_trial(request, repository_root)?;
        observation.model_id = "different-model".to_owned();
        Ok(observation)
    }
}

#[test]
fn incomplete_live_matrix_never_claims_quantitative_acceptance() {
    let result = run_live_with_driver(&enabled_test_config(), &mut CoordinateMismatchDriver)
        .expect("coordinate mismatches should produce an incomplete structured result");

    assert_eq!(result.status, RunStatus::Incomplete);
    assert!(result.observations.is_empty());
    assert_eq!(result.trial_failures.len(), result.schedule.len());
    assert!(result.criteria.iter().all(|criterion| {
        criterion.status == CriterionStatus::MeasurementPending
            && criterion.measured_value.is_none()
            && criterion.reason.contains("incomplete")
    }));
}

#[test]
fn result_output_is_create_new_and_round_trips_exactly() {
    let result = fixture_evaluation(SEED, 1).expect("fixture evaluation should succeed");
    let directory = tempfile::tempdir().expect("temporary output directory should exist");
    let path = directory.path().join("result.json");

    write_result_create_new(&path, &result).expect("first output write should succeed");
    assert!(write_result_create_new(&path, &result).is_err());

    let text = fs::read_to_string(path).expect("result should be readable");
    let parsed: EvaluationResult =
        serde_json::from_str(&text).expect("result should be valid JSON");
    assert_eq!(parsed, result);
}

fn enabled_test_config() -> LiveConfig {
    LiveConfig {
        schema: LIVE_CONFIG_SCHEMA.to_owned(),
        enabled: true,
        model_id: "synthetic-test-model".to_owned(),
        host_kind: "in-process-test-driver".to_owned(),
        host_version: "1".to_owned(),
        driver_command: vec!["/unused/in-process-test-driver".to_owned()],
        seed: SEED,
        repetitions: 1,
    }
}

fn status_for(criteria: &[CriterionResult], criterion_id: &str) -> CriterionStatus {
    criteria
        .iter()
        .find(|criterion| criterion.criterion_id == criterion_id)
        .expect("criterion should exist")
        .status
}
