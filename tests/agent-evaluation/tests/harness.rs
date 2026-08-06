use std::{collections::BTreeSet, fs};

use volicord_agent_evaluation::{
    build_schedule, evaluate_live_criteria, fixture_evaluation, live_config_example_path,
    load_embedded_catalog, load_live_config, materialize_repository, result_schema_text,
    run_live_with_driver, validate_catalog, validate_live_config, validate_schedule_matrix,
    write_result_create_new, CriterionResult, CriterionStatus, DriverFailure, DriverObservation,
    DriverRequest, EvaluationCondition, EvaluationResult, LiveConfig, RunKind, RunStatus,
    ShapingWorkflowObservation, TaskGroup, TrialDriver, DRIVER_OBSERVATION_SCHEMA,
    LIVE_CONFIG_SCHEMA, RESULT_SCHEMA,
};

const SEED: u64 = 20_260_716;

type ShapingMetricDefect = (
    &'static str,
    &'static str,
    fn(&mut ShapingWorkflowObservation),
);

type CriterionMetricDefect = (&'static str, fn(&mut ShapingWorkflowObservation));

#[test]
fn catalog_covers_the_three_condition_evaluation_surface_and_shaping_variants() {
    let catalog = load_embedded_catalog().expect("embedded catalog should be valid");
    assert_eq!(EvaluationCondition::ALL.len(), 3);
    assert_eq!(catalog.scenarios.len(), TaskGroup::ALL.len() + 16);

    let actual = catalog
        .scenarios
        .iter()
        .map(|scenario| scenario.task_group)
        .collect::<BTreeSet<_>>();
    let expected = TaskGroup::ALL.into_iter().collect::<BTreeSet<_>>();
    assert_eq!(actual, expected);
    for scenario_id in [
        "planning-product-decision",
        "planning-technical-decision",
        "planning-advisor-recommendation",
        "planning-scope-decision",
        "planning-sensitive-decision",
        "planning-rejected-outcome",
        "planning-deferred-outcome",
        "planning-expired-outcome",
        "planning-superseded-history",
        "planning-stale-reauthorization",
        "planning-implementation-invalidation",
        "planning-explicit-scope-replacement",
        "read-only-persisted-baseline-corruption",
    ] {
        assert!(catalog
            .scenarios
            .iter()
            .any(|scenario| scenario.scenario_id == scenario_id));
    }
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
    assert_eq!(result.criteria.len(), 73);
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
    let status = repository
        .git(&["status", "--porcelain", "--", &attribution.path])
        .expect("Git status should run");
    assert!(status.status.success());
    assert_eq!(
        String::from_utf8(status.stdout).expect("Git status should be UTF-8"),
        format!(" M {}\n", attribution.path)
    );
}

#[test]
fn generic_transformed_content_fixture_uses_the_existing_attribution_metric() {
    let catalog = load_embedded_catalog().expect("embedded catalog should be valid");
    let scenario = catalog
        .scenarios
        .iter()
        .find(|scenario| scenario.scenario_id == "repository-transformation-attribution")
        .expect("generic transformed-content scenario");
    let attribution = scenario
        .dirty_worktree_attribution
        .as_ref()
        .expect("generic repository-attribution expectation");
    for forbidden in ["agents.md", "crlf", "guard_probe"] {
        assert!(!scenario.scenario_id.to_lowercase().contains(forbidden));
    }
    let repository =
        materialize_repository(scenario).expect("transformed repository should materialize");

    assert_eq!(
        repository
            .worktree_bytes(&attribution.path)
            .expect("transformed worktree bytes"),
        attribution.preexisting_dirty_content.as_bytes()
    );
    let status = repository
        .git(&["status", "--porcelain", "--", &attribution.path])
        .expect("transformed Git status");
    assert_eq!(
        String::from_utf8(status.stdout).expect("Git status should be UTF-8"),
        format!(" M {}\n", attribution.path)
    );
    let committed = repository
        .git(&["cat-file", "-p", &format!("HEAD:{}", attribution.path)])
        .expect("committed transformed blob");
    assert_eq!(committed.stdout, b"record=baseline\n");
    assert_eq!(attribution.minimum_true_positives, 1);
    assert_eq!(attribution.maximum_false_positives, 0);
}

#[test]
fn planning_only_fixture_is_neutral_and_contains_no_implementation() {
    let catalog = load_embedded_catalog().expect("embedded catalog should be valid");
    let scenario = catalog
        .scenarios
        .iter()
        .find(|scenario| scenario.task_group == TaskGroup::PlanningOnlyDevelopment)
        .expect("planning-only development scenario");

    assert!(!scenario.instruction.to_lowercase().contains("volicord"));
    assert_eq!(scenario.initial_files.len(), 3);
    assert!(scenario
        .initial_files
        .iter()
        .all(|file| file.path.starts_with("plans/") && file.path.ends_with(".md")));
    assert!(scenario.initial_files.iter().all(|file| {
        !file.path.starts_with("src/") && file.path != "Cargo.toml" && !file.path.ends_with(".rs")
    }));
}

#[test]
fn shaping_evaluation_fixtures_are_generic_plans_without_implementation() {
    let catalog = load_embedded_catalog().expect("embedded catalog should be valid");
    for scenario in catalog.scenarios.iter().filter(|scenario| {
        scenario.expected.shaping_outcome.is_some()
            || scenario.expected.shaping_authority_recovery.is_some()
    }) {
        assert!(scenario
            .initial_files
            .iter()
            .all(|file| file.path.starts_with("plans/") && file.path.ends_with(".md")));
        assert!(scenario.initial_files.iter().all(|file| {
            !file.path.starts_with("src/")
                && file.path != "Cargo.toml"
                && !file.path.ends_with(".rs")
        }));
    }
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
        Some(73)
    );
    assert_eq!(
        schema["properties"]["criteria"]["maxItems"].as_u64(),
        Some(73)
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
        let planning = record && request.trial.task_group == TaskGroup::PlanningOnlyDevelopment;
        let schema_recovery =
            record && request.trial.scenario_id == "planning-only-development-preparation";
        let product_only = record && request.trial.scenario_id == "planning-product-decision";
        let technical_only = record && request.trial.scenario_id == "planning-technical-decision";
        let advisor = record && request.trial.scenario_id == "planning-advisor-recommendation";
        let accepted = record
            && matches!(
                request.trial.scenario_id.as_str(),
                "planning-product-decision"
                    | "planning-technical-decision"
                    | "planning-scope-decision"
                    | "planning-sensitive-decision"
                    | "planning-advisor-recommendation"
            );
        let rejected = record && request.trial.scenario_id == "planning-rejected-outcome";
        let deferred = record && request.trial.scenario_id == "planning-deferred-outcome";
        let expired = record && request.trial.scenario_id == "planning-expired-outcome";
        let non_authorizing = rejected || deferred || expired;
        let superseded_history =
            record && request.trial.scenario_id == "planning-superseded-history";
        let stale_reauthorization =
            record && request.trial.scenario_id == "planning-stale-reauthorization";
        let implementation_invalidation =
            record && request.trial.scenario_id == "planning-implementation-invalidation";
        let record_run_rejection =
            record && request.trial.scenario_id == "workflow-recording-rejection-details";
        let explicit_replacement =
            record && request.trial.scenario_id == "planning-explicit-scope-replacement";
        let persisted_corruption =
            record && request.trial.scenario_id == "read-only-persisted-baseline-corruption";
        let authority_recovery =
            superseded_history || stale_reauthorization || implementation_invalidation;
        let retry_guidance =
            non_authorizing || authority_recovery || explicit_replacement || persisted_corruption;
        let canonicality_wording =
            implementation_invalidation || explicit_replacement || persisted_corruption;
        let shaping_scenario = planning || advisor;
        let planning_decisions =
            if request.trial.scenario_id == "planning-only-development-preparation" {
                3
            } else if accepted || non_authorizing || authority_recovery {
                1
            } else {
                0
            };
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
                record
                    && matches!(
                        request.trial.scenario_id.as_str(),
                        "dirty-worktree-same-path-attribution"
                            | "repository-transformation-attribution"
                    ),
            ),
            unrecorded_change_false_positives: 0,
            resume_authority_or_judgment_losses: 0,
            wrong_auto_completions: 0,
            workflow_rejections_observed: 0,
            workflow_rejections_surfaced_in_final_answer: 0,
            shaping_workflow: ShapingWorkflowObservation {
                long_lived_repository_requests: u64::from(planning),
                automatic_volicord_uses: u64::from(planning),
                workflow_tool_selection_opportunities: u64::from(planning),
                correct_workflow_tool_selections: u64::from(planning),
                action_form_use_opportunities: u64::from(planning),
                current_action_forms_used: u64::from(planning),
                method_specific_form_opportunities: u64::from(planning),
                method_specific_action_forms_used: u64::from(planning),
                fixed_argument_opportunities: u64::from(planning),
                exact_fixed_arguments_used: u64::from(planning),
                schema_validity_claim_opportunities: u64::from(
                    schema_recovery || record_run_rejection,
                ),
                schema_validity_treated_as_execution_authority: 0,
                wrong_method_mutation_opportunities: u64::from(planning),
                wrong_method_speculative_mutations: 0,
                nullable_baseline_opportunities: u64::from(schema_recovery),
                json_null_baselines_used: u64::from(schema_recovery),
                schema_recovery_opportunities: u64::from(schema_recovery),
                correct_discriminator_recoveries: u64::from(schema_recovery),
                unrelated_cli_help_uses: 0,
                binary_schema_inspections: 0,
                raw_stdio_schema_probes: 0,
                source_schema_searches: 0,
                null_baseline_substitutions: 0,
                speculative_shaping_tool_calls: 0,
                argument_error_opportunities: u64::from(schema_recovery),
                corruption_misdiagnoses: 0,
                replacement_required_opportunities: u64::from(explicit_replacement),
                replace_current_forms_selected: u64::from(explicit_replacement),
                keep_current_retry_loops: 0,
                invented_baseline_representations: 0,
                no_effect_replacement_opportunities: u64::from(explicit_replacement),
                false_replacement_success_claims: 0,
                persisted_baseline_corruption_opportunities: u64::from(persisted_corruption),
                stored_state_corruptions_reported: u64::from(persisted_corruption),
                corruption_user_input_misdiagnoses: 0,
                checkpoint_status_opportunities: u64::from(planning),
                correct_checkpoint_creation_statuses: u64::from(planning),
                user_action_status_opportunities: planning_decisions,
                correct_user_action_creation_statuses: planning_decisions,
                no_task_intake_opportunities: u64::from(planning),
                correct_intakes: u64::from(planning),
                shaping_opportunities: u64::from(planning),
                shaping_before_implementation: u64::from(planning),
                user_owned_decision_opportunities: planning_decisions,
                user_action_requests_created: planning_decisions,
                pending_chat_replies: u64::from(planning),
                chat_resolutions_created: 0,
                cli_instruction_opportunities: planning_decisions,
                correct_cli_instructions: planning_decisions,
                preauthorization_write_opportunities: u64::from(planning),
                premature_product_writes: 0,
                implementation_entry_opportunities: u64::from(planning),
                explicit_task_advances: u64::from(planning),
                mutation_calls: if planning { 10 } else { 0 },
                hidden_mutation_rejections: 0,
                final_answers: u64::from(planning || persisted_corruption),
                concise_user_readable_outputs: u64::from(planning || persisted_corruption),
                raw_mcp_json_repetitions: 0,
                guarantee_wording_checks: u64::from(planning),
                accurate_cooperative_guarantee_wording: u64::from(planning),
                impossible_retry_instruction_opportunities: u64::from(retry_guidance),
                impossible_retry_instructions: 0,
                canonicality_compatibility_wording_opportunities: u64::from(canonicality_wording),
                accurate_canonicality_compatibility_wording: u64::from(canonicality_wording),
                mutation_reporting_opportunities: u64::from(planning),
                accurate_mutation_reports: u64::from(planning),
                completion_reporting_opportunities: u64::from(shaping_scenario),
                accurate_completion_reports: u64::from(shaping_scenario),
                product_only_decision_opportunities: u64::from(product_only),
                product_only_decisions_applied_exactly: u64::from(product_only),
                technical_only_decision_opportunities: u64::from(technical_only),
                technical_only_decisions_applied_exactly: u64::from(technical_only),
                checkpoint_replacement_opportunities: u64::from(planning),
                checkpoint_authority_preserved: u64::from(planning),
                tagged_workflow_opportunities: if shaping_scenario { 6 } else { 0 },
                exact_tagged_workflows: if shaping_scenario { 6 } else { 0 },
                advisor_finalization_opportunities: u64::from(advisor),
                advisor_finalizations_via_finalize_advice: u64::from(advisor),
                advisor_change_unit_opportunities: u64::from(advisor),
                advisor_observe_only_change_units: u64::from(advisor),
                change_unit_contract_authoring_opportunities: u64::from(advisor),
                speculative_path_or_effect_contracts: 0,
                record_run_rejection_detail_opportunities: u64::from(record_run_rejection),
                correct_record_run_rejection_details: u64::from(record_run_rejection),
                completion_claim_opportunities: u64::from(shaping_scenario),
                premature_completion_claims: 0,
                accepted_outcome_opportunities: u64::from(accepted),
                accepted_outcomes_surfaced: u64::from(accepted),
                rejected_outcome_opportunities: u64::from(rejected),
                rejected_outcomes_surfaced: u64::from(rejected),
                deferred_outcome_opportunities: u64::from(deferred),
                deferred_outcomes_surfaced: u64::from(deferred),
                expired_outcome_opportunities: u64::from(expired),
                expired_outcomes_surfaced: u64::from(expired),
                non_authorizing_outcome_opportunities: u64::from(non_authorizing),
                false_authority_claims: 0,
                expired_resolution_instruction_opportunities: u64::from(expired),
                expired_resolution_instructions: 0,
                shaping_recovery_opportunities: u64::from(non_authorizing),
                correct_shaping_recoveries: u64::from(non_authorizing),
                successor_user_action_opportunities: u64::from(non_authorizing),
                successor_user_actions_created: u64::from(non_authorizing),
                retained_authority_opportunities: u64::from(shaping_scenario),
                retained_authority_preserved: u64::from(shaping_scenario),
                application_owner_opportunities: u64::from(accepted),
                exact_application_owners: u64::from(accepted),
                superseded_history_opportunities: u64::from(superseded_history),
                superseded_history_action_instructions: 0,
                recovery_successor_acceptance_opportunities: u64::from(superseded_history),
                recovery_successor_acceptances: u64::from(superseded_history),
                valid_history_consistency_opportunities: u64::from(superseded_history),
                inconsistent_authority_claims: 0,
                stale_authority_explanation_opportunities: u64::from(stale_reauthorization),
                correct_stale_authority_explanations: u64::from(stale_reauthorization),
                stale_resolution_reuse_opportunities: u64::from(stale_reauthorization),
                stale_accepted_resolution_reuses: 0,
                stale_disposition_opportunities: if stale_reauthorization { 2 } else { 0 },
                exact_stale_dispositions: if stale_reauthorization { 2 } else { 0 },
                stale_reauthorization_request_opportunities: u64::from(stale_reauthorization),
                fresh_stale_user_actions_created: u64::from(stale_reauthorization),
                implementation_invalidation_opportunities: if implementation_invalidation {
                    3
                } else {
                    0
                },
                correct_implementation_invalidation_rejections: if implementation_invalidation {
                    3
                } else {
                    0
                },
            },
        })
    }
}

struct OmittedWorkflowRejectionDriver;

impl TrialDriver for OmittedWorkflowRejectionDriver {
    fn run_trial(
        &mut self,
        request: &DriverRequest,
        repository_root: &std::path::Path,
    ) -> Result<DriverObservation, DriverFailure> {
        let mut observation = AggregateSyntheticDriver.run_trial(request, repository_root)?;
        observation.workflow_rejections_observed = 1;
        observation.workflow_rejections_surfaced_in_final_answer = 0;
        Ok(observation)
    }
}

#[test]
fn live_evaluation_rejects_an_omitted_workflow_rejection() {
    let result = run_live_with_driver(&enabled_test_config(), &mut OmittedWorkflowRejectionDriver)
        .expect("evaluation should return an incomplete result");
    assert_eq!(result.status, RunStatus::Incomplete);
    assert!(result.trial_failures.iter().any(|failure| {
        failure.failure_code == "observation_coordinate_mismatch"
            && failure.detail.contains("omitted a workflow rejection")
    }));
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
        EvaluationCondition::ALL.len() * load_embedded_catalog().expect("catalog").scenarios.len()
    );
    assert_eq!(result.observations.len(), result.schedule.len());
    assert!(result.trial_failures.is_empty());
    assert!(result
        .criteria
        .iter()
        .all(|criterion| criterion.status == CriterionStatus::Passed));
}

#[test]
fn shaping_behavior_defect_fails_its_quantitative_criterion() {
    let mut result = run_live_with_driver(&enabled_test_config(), &mut AggregateSyntheticDriver)
        .expect("synthetic in-process matrix should run");
    let planning = result
        .observations
        .iter_mut()
        .find(|observation| {
            observation.condition == EvaluationCondition::RecordLight
                && observation.task_group == TaskGroup::PlanningOnlyDevelopment
        })
        .expect("record-light planning observation");
    planning.shaping_workflow.automatic_volicord_uses = 0;

    let criteria = evaluate_live_criteria(&result.observations);
    assert_eq!(
        status_for(&criteria, "automatic_volicord_use"),
        CriterionStatus::Failed
    );
    assert_eq!(
        status_for(&criteria, "shaping_before_implementation"),
        CriterionStatus::Passed
    );
}

#[test]
fn schema_recovery_metric_defects_fail_their_generic_criteria() {
    fn assert_defect(
        baseline: &[DriverObservation],
        criterion_id: &str,
        mutate: impl FnOnce(&mut ShapingWorkflowObservation),
    ) {
        let mut observations = baseline.to_vec();
        let observation = observations
            .iter_mut()
            .find(|observation| {
                observation.condition == EvaluationCondition::RecordLight
                    && observation.scenario_id == "planning-only-development-preparation"
            })
            .expect("generic planning recovery observation");
        mutate(&mut observation.shaping_workflow);
        assert_eq!(
            status_for(&evaluate_live_criteria(&observations), criterion_id),
            CriterionStatus::Failed,
            "{criterion_id}"
        );
    }

    let result = run_live_with_driver(&enabled_test_config(), &mut AggregateSyntheticDriver)
        .expect("synthetic in-process matrix should run");
    let defects: [CriterionMetricDefect; 14] = [
        ("correct_workflow_tool_selection", |workflow| {
            workflow.correct_workflow_tool_selections = 0
        }),
        ("current_action_form_use", |workflow| {
            workflow.current_action_forms_used = 0
        }),
        ("method_specific_action_form_use", |workflow| {
            workflow.method_specific_action_forms_used = 0
        }),
        ("exact_fixed_argument_use", |workflow| {
            workflow.exact_fixed_arguments_used = 0
        }),
        ("schema_validity_is_not_execution_authority", |workflow| {
            workflow.schema_validity_treated_as_execution_authority = 1
        }),
        ("wrong_method_speculative_mutation", |workflow| {
            workflow.wrong_method_speculative_mutations = 1
        }),
        ("nullable_baseline_json_null", |workflow| {
            workflow.json_null_baselines_used = 0
        }),
        ("discriminator_error_recovery", |workflow| {
            workflow.correct_discriminator_recoveries = 0
        }),
        ("unrelated_cli_help_during_schema_recovery", |workflow| {
            workflow.unrelated_cli_help_uses = 1
        }),
        ("binary_schema_inspection_during_recovery", |workflow| {
            workflow.binary_schema_inspections = 1
        }),
        ("schema_recovery_bypass", |workflow| {
            workflow.raw_stdio_schema_probes = 1
        }),
        ("argument_error_corruption_misdiagnosis", |workflow| {
            workflow.corruption_misdiagnoses = 1
        }),
        ("correct_checkpoint_creation_status", |workflow| {
            workflow.correct_checkpoint_creation_statuses = 0
        }),
        ("correct_user_action_creation_status", |workflow| {
            workflow.correct_user_action_creation_statuses = 0
        }),
    ];
    for (criterion_id, mutate) in defects {
        assert_defect(&result.observations, criterion_id, mutate);
    }
}

#[test]
fn shaping_authority_metric_defects_fail_their_focused_criteria() {
    fn assert_defect(
        baseline: &[DriverObservation],
        scenario_id: &str,
        criterion_id: &str,
        mutate: impl FnOnce(&mut ShapingWorkflowObservation),
    ) {
        let mut observations = baseline.to_vec();
        let observation = observations
            .iter_mut()
            .find(|observation| {
                observation.condition == EvaluationCondition::RecordLight
                    && observation.scenario_id == scenario_id
            })
            .expect("focused shaping observation");
        mutate(&mut observation.shaping_workflow);
        assert_eq!(
            status_for(&evaluate_live_criteria(&observations), criterion_id),
            CriterionStatus::Failed,
            "{criterion_id}"
        );
    }

    let result = run_live_with_driver(&enabled_test_config(), &mut AggregateSyntheticDriver)
        .expect("synthetic in-process matrix should run");
    assert_defect(
        &result.observations,
        "planning-product-decision",
        "product_only_decision_application",
        |workflow| workflow.product_only_decisions_applied_exactly = 0,
    );
    assert_defect(
        &result.observations,
        "planning-technical-decision",
        "technical_only_decision_application",
        |workflow| workflow.technical_only_decisions_applied_exactly = 0,
    );
    assert_defect(
        &result.observations,
        "planning-only-development-preparation",
        "checkpoint_authority_preservation",
        |workflow| workflow.checkpoint_authority_preserved = 0,
    );
    assert_defect(
        &result.observations,
        "planning-only-development-preparation",
        "exact_tagged_workflow",
        |workflow| workflow.exact_tagged_workflows = 0,
    );
    assert_defect(
        &result.observations,
        "planning-advisor-recommendation",
        "advisor_finalization_via_finalize_advice",
        |workflow| workflow.advisor_finalizations_via_finalize_advice = 0,
    );
    assert_defect(
        &result.observations,
        "planning-advisor-recommendation",
        "advisor_observe_only_change_unit",
        |workflow| workflow.advisor_observe_only_change_units = 0,
    );
    assert_defect(
        &result.observations,
        "planning-advisor-recommendation",
        "no_speculative_change_unit_contract",
        |workflow| workflow.speculative_path_or_effect_contracts = 1,
    );
    assert_defect(
        &result.observations,
        "workflow-recording-rejection-details",
        "record_run_rejection_detail_preservation",
        |workflow| workflow.correct_record_run_rejection_details = 0,
    );
    assert_defect(
        &result.observations,
        "planning-only-development-preparation",
        "premature_completion_claim",
        |workflow| workflow.premature_completion_claims = 1,
    );
    let outcome_defects: [ShapingMetricDefect; 7] = [
        (
            "planning-product-decision",
            "accepted_outcome_surfacing",
            |workflow: &mut ShapingWorkflowObservation| workflow.accepted_outcomes_surfaced = 0,
        ),
        (
            "planning-rejected-outcome",
            "rejected_outcome_surfacing",
            |workflow: &mut ShapingWorkflowObservation| workflow.rejected_outcomes_surfaced = 0,
        ),
        (
            "planning-deferred-outcome",
            "deferred_outcome_surfacing",
            |workflow: &mut ShapingWorkflowObservation| workflow.deferred_outcomes_surfaced = 0,
        ),
        (
            "planning-expired-outcome",
            "expired_outcome_surfacing",
            |workflow: &mut ShapingWorkflowObservation| workflow.expired_outcomes_surfaced = 0,
        ),
        (
            "planning-rejected-outcome",
            "shaping_recovery_request",
            |workflow: &mut ShapingWorkflowObservation| workflow.correct_shaping_recoveries = 0,
        ),
        (
            "planning-deferred-outcome",
            "successor_user_action_creation",
            |workflow: &mut ShapingWorkflowObservation| workflow.successor_user_actions_created = 0,
        ),
        (
            "planning-scope-decision",
            "exact_decision_application_owner",
            |workflow: &mut ShapingWorkflowObservation| workflow.exact_application_owners = 0,
        ),
    ];
    for (scenario_id, criterion_id, mutate) in outcome_defects {
        assert_defect(&result.observations, scenario_id, criterion_id, mutate);
    }
    assert_defect(
        &result.observations,
        "planning-rejected-outcome",
        "non_authorizing_outcome_authority_claim",
        |workflow| workflow.false_authority_claims = 1,
    );
    assert_defect(
        &result.observations,
        "planning-expired-outcome",
        "expired_request_resolution_instruction",
        |workflow| workflow.expired_resolution_instructions = 1,
    );
    assert_defect(
        &result.observations,
        "planning-only-development-preparation",
        "decision_authority_retention",
        |workflow| workflow.retained_authority_preserved = 0,
    );
    let recovery_defects: [ShapingMetricDefect; 8] = [
        (
            "planning-superseded-history",
            "superseded_history_action_instruction",
            |workflow| workflow.superseded_history_action_instructions = 1,
        ),
        (
            "planning-superseded-history",
            "recovery_successor_request_acceptance",
            |workflow| workflow.recovery_successor_acceptances = 0,
        ),
        (
            "planning-superseded-history",
            "valid_history_inconsistent_authority_claim",
            |workflow| workflow.inconsistent_authority_claims = 1,
        ),
        (
            "planning-stale-reauthorization",
            "correct_stale_authority_explanation",
            |workflow| workflow.correct_stale_authority_explanations = 0,
        ),
        (
            "planning-stale-reauthorization",
            "stale_accepted_resolution_reuse",
            |workflow| workflow.stale_accepted_resolution_reuses = 1,
        ),
        (
            "planning-stale-reauthorization",
            "exact_stale_retirement_or_reissue",
            |workflow| workflow.exact_stale_dispositions = 1,
        ),
        (
            "planning-stale-reauthorization",
            "fresh_user_action_for_stale_reauthorization",
            |workflow| workflow.fresh_stale_user_actions_created = 0,
        ),
        (
            "planning-implementation-invalidation",
            "implementation_phase_invalidation_rejection",
            |workflow| workflow.correct_implementation_invalidation_rejections = 2,
        ),
    ];
    for (scenario_id, criterion_id, mutate) in recovery_defects {
        assert_defect(&result.observations, scenario_id, criterion_id, mutate);
    }
    let retarget_defects: [ShapingMetricDefect; 6] = [
        (
            "planning-explicit-scope-replacement",
            "replace_current_form_selection",
            |workflow| workflow.replace_current_forms_selected = 0,
        ),
        (
            "planning-explicit-scope-replacement",
            "keep_current_retry_loop",
            |workflow| workflow.keep_current_retry_loops = 1,
        ),
        (
            "planning-explicit-scope-replacement",
            "invented_baseline_representation",
            |workflow| workflow.invented_baseline_representations = 1,
        ),
        (
            "planning-explicit-scope-replacement",
            "false_replacement_success_claim",
            |workflow| workflow.false_replacement_success_claims = 1,
        ),
        (
            "read-only-persisted-baseline-corruption",
            "stored_baseline_corruption_reporting",
            |workflow| workflow.stored_state_corruptions_reported = 0,
        ),
        (
            "read-only-persisted-baseline-corruption",
            "corruption_as_user_input_misdiagnosis",
            |workflow| workflow.corruption_user_input_misdiagnoses = 1,
        ),
    ];
    for (scenario_id, criterion_id, mutate) in retarget_defects {
        assert_defect(&result.observations, scenario_id, criterion_id, mutate);
    }
    let reporting_defects: [ShapingMetricDefect; 4] = [
        (
            "planning-implementation-invalidation",
            "impossible_retry_instruction",
            |workflow| workflow.impossible_retry_instructions = 1,
        ),
        (
            "planning-implementation-invalidation",
            "accurate_canonicality_compatibility_wording",
            |workflow| workflow.accurate_canonicality_compatibility_wording = 0,
        ),
        (
            "planning-only-development-preparation",
            "accurate_mutation_reporting",
            |workflow| workflow.accurate_mutation_reports = 0,
        ),
        (
            "planning-only-development-preparation",
            "accurate_completion_reporting",
            |workflow| workflow.accurate_completion_reports = 0,
        ),
    ];
    for (scenario_id, criterion_id, mutate) in reporting_defects {
        assert_defect(&result.observations, scenario_id, criterion_id, mutate);
    }
}

struct MissingDirtyAttributionDriver;

impl TrialDriver for MissingDirtyAttributionDriver {
    fn run_trial(
        &mut self,
        request: &DriverRequest,
        repository_root: &std::path::Path,
    ) -> Result<DriverObservation, DriverFailure> {
        let mut observation = AggregateSyntheticDriver.run_trial(request, repository_root)?;
        if matches!(
            request.trial.scenario_id.as_str(),
            "dirty-worktree-same-path-attribution" | "repository-transformation-attribution"
        ) {
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
    assert_eq!(result.trial_failures.len(), 2);
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
