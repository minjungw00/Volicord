#![forbid(unsafe_code)]

mod support;

#[cfg(unix)]
mod unix {
    use std::{
        env,
        error::Error,
        ffi::OsString,
        fs::{self, OpenOptions},
        io::{self, IsTerminal, Write},
        os::unix::fs::PermissionsExt,
        path::{Path, PathBuf},
        process::{Command, ExitStatus, Output, Stdio},
        thread,
        time::{Duration, Instant, SystemTime, UNIX_EPOCH},
    };

    use rusqlite::OptionalExtension;
    use serde_json::Value;
    use volicord_mcp::{McpAdapter, McpConnectionContext};
    use volicord_store::{
        agent_connections::{agent_connection_record_read_only, VERIFIED_STATUS_COMPLETE},
        bootstrap::list_projects,
        diagnostics::{
            diagnostics_db_path, record_diagnostic_event, start_diagnostic_session,
            DiagnosticEvent, DiagnosticEventKind, DiagnosticFallbackKind, DiagnosticHostKind,
            DiagnosticOutcome, DiagnosticSessionStart, DiagnosticTransport,
        },
        sqlite::open_project_state_database_read_only,
    };
    use volicord_test_support::{core_fixtures::CoreFixture, TempRuntimeHome};
    use volicord_types::{
        canonical_json_string, ArtifactRef, AuthorityReceipt, EvidenceCoverageItem,
        EvidenceCoverageState, EvidenceProducerKind, EvidenceRelevanceStatus, EvidenceTarget,
        PersistedEvidenceMetadata, PersistedEvidenceObservationAuthority,
        PersistedUserActionRequest, StateRecordKind, StateRecordRef, StatusCloseState,
        StatusResult, UserActionBasis, UserActionInboxForm, UserActionPresentationPlan,
        UserActionPresentationSafety, UserActionRequestBody, UserActionResolutionBody,
        USER_ACTION_OBSERVATION_SUMMARY_MAX_CHARS, VERIFICATION_BASIS_CLI_DIRECT_USER_CHANNEL,
        VERIFICATION_BASIS_LOCAL_USER_LOCAL_WEB, VERIFICATION_BASIS_MCP_ELICITATION_USER_CHANNEL,
        VERIFICATION_BASIS_MCP_STDIO_CONNECTION_BINDING, VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
    };

    use crate::support::fake_hosts::{write_fake_claude_code, write_fake_codex};

    const CODEX_SMOKE_ENV: &str = "VOLICORD_RUN_CODEX_SMOKE";
    const CLAUDE_SMOKE_ENV: &str = "VOLICORD_RUN_CLAUDE_SMOKE";
    const CODEX_RECORD_FINAL_OUTPUT_SMOKE_ENV: &str =
        "VOLICORD_RUN_CODEX_RECORD_FINAL_OUTPUT_SMOKE";
    const CODEX_DETECTIVE_FINAL_OUTPUT_SMOKE_ENV: &str =
        "VOLICORD_RUN_CODEX_DETECTIVE_FINAL_OUTPUT_SMOKE";
    const CLAUDE_RECORD_FINAL_OUTPUT_SMOKE_ENV: &str =
        "VOLICORD_RUN_CLAUDE_RECORD_FINAL_OUTPUT_SMOKE";
    const CLAUDE_DETECTIVE_FINAL_OUTPUT_SMOKE_ENV: &str =
        "VOLICORD_RUN_CLAUDE_DETECTIVE_FINAL_OUTPUT_SMOKE";
    const CODEX_USER_ACTION_SMOKE_ENV: &str = "VOLICORD_RUN_CODEX_USER_ACTION_SMOKE";
    const CLAUDE_USER_ACTION_SMOKE_ENV: &str = "VOLICORD_RUN_CLAUDE_USER_ACTION_SMOKE";
    const CODEX_EVIDENCE_OBSERVATION_SMOKE_ENV: &str =
        "VOLICORD_RUN_CODEX_EVIDENCE_OBSERVATION_SMOKE";
    const CLAUDE_EVIDENCE_OBSERVATION_SMOKE_ENV: &str =
        "VOLICORD_RUN_CLAUDE_EVIDENCE_OBSERVATION_SMOKE";
    const CODEX_CLI_FALLBACK_SMOKE_ENV: &str = "VOLICORD_RUN_CODEX_CLI_FALLBACK_SMOKE";
    const CLAUDE_CLI_FALLBACK_SMOKE_ENV: &str = "VOLICORD_RUN_CLAUDE_CLI_FALLBACK_SMOKE";
    const LIVE_HOST_RESULT_PATH_ENV: &str = "VOLICORD_LIVE_HOST_RESULT_PATH";
    const LIVE_USER_ACTION_RESULT_KIND: &str = "live_host_user_action_release_validation";
    const LIVE_EVIDENCE_OBSERVATION_RESULT_KIND: &str =
        "live_host_evidence_observation_release_validation";
    const LIVE_CLI_FALLBACK_RESULT_KIND: &str = "live_host_cli_fallback_release_validation";
    const LIVE_FINAL_OUTPUT_RESULT_KIND: &str = "live_host_final_output_release_validation";
    const USER_ACTION_ROUTE_ALPHA_OPTION_ID: &str = "route_alpha";
    const USER_ACTION_ROUTE_BETA_OPTION_ID: &str = "route_beta";
    const USER_ACTION_ROUTE_ALPHA_RUN_MARKER: &str =
        "VOLICORD_LIVE_HOST_USER_ACTION_CONSUMED_ROUTE_ALPHA";
    const USER_ACTION_ROUTE_BETA_RUN_MARKER: &str =
        "VOLICORD_LIVE_HOST_USER_ACTION_CONSUMED_ROUTE_BETA";
    const LIVE_HOST_BASELINE_REF: &str = "baseline_live_host_user_action";
    const LIVE_EVIDENCE_OBSERVATION_BASELINE_REF: &str = "baseline_live_host_evidence_observation";
    const LIVE_EVIDENCE_OBSERVATION_RUN_MARKER: &str =
        "VOLICORD_LIVE_HOST_EVIDENCE_OBSERVATION_CONSUMED_SUPPORTED";
    const LIVE_EVIDENCE_CALLER_OBSERVED_AT: &str = "2000-01-01T00:00:00Z";
    const LIVE_EVIDENCE_REQUEST_QUESTION: &str =
        "Do the exact registered fixture bytes support the required criterion?";
    const LIVE_EVIDENCE_REQUEST_CONTEXT: &str =
        "Review the exact benign fixture bytes through the user-only local consent path.";
    const LIVE_EVIDENCE_ARTIFACT_DISPLAY_NAME: &str = "credential-routing-fixture.txt";
    const LIVE_EVIDENCE_ARTIFACT_BYTES: &str = "Deterministic benign local-consent fixture bytes.";
    const LIVE_EVIDENCE_RELATION_HINT: &str = "local-consent-routing-fixture";
    const MODEL_INVISIBLE_USER_CHANNEL_CAPABILITY_NAMESPACE: &str = "io.volicord/user-channel";
    const MODEL_INVISIBLE_USER_CHANNEL_CAPABILITY_FIELD: &str = "model_invisible_user_surface";
    const MODEL_INVISIBLE_SURFACE_CONFIRMATION: &str = "surface:host-owned-model-invisible";
    const MODEL_VISIBLE_ABSENCE_CONFIRMATION: &str =
        "model-visible:none-url-token-form-question-request-ref";
    const LIVE_CLI_FALLBACK_BASELINE_REF: &str = "baseline_live_host_cli_fallback";
    const LIVE_INBOX_COMMAND_TEMPLATE: &str =
        "VOLICORD_HOME=<runtime-home> volicord inbox --repo <repo> --task <task-id> --json";
    const LIVE_INBOX_RESOLVE_COMMAND_TEMPLATE: &str = "VOLICORD_HOME=<runtime-home> volicord inbox resolve <user-action-request-id> --choice <option-id> --repo <repo> --json";
    const LIVE_INBOX_RESOLVE_USAGE: &str = "volicord inbox resolve <user-action-request-id> --choice <choice> [--repo PATH] [--note TEXT] [--json]";
    const MAX_HOST_VERSION_CHARS: usize = 256;
    const MAX_CONNECTION_ID_CHARS: usize = 256;
    const MAX_BUILD_ID_CHARS: usize = 1_024;
    const MAX_VALIDATION_RUN_ID_CHARS: usize = 192;
    const MAX_RECORDED_AT_CHARS: usize = 64;
    const MAX_LIVE_HOST_RESULT_BYTES: usize = 16 * 1_024;
    const COMMAND_TIMEOUT: Duration = Duration::from_secs(20);

    #[test]
    fn live_result_helpers_reject_stale_paths_and_keep_bounded_atomic_results(
    ) -> Result<(), Box<dyn Error>> {
        let temp = TempRuntimeHome::new("live-result-recorder")?;
        let result_dir = temp.path().join("release-results");
        fs::create_dir_all(&result_dir)?;
        let result_path = result_dir.join("codex.json");

        let mut recorder = LiveResultRecorder::new("codex", Some(result_path.clone()))?;
        let running: Value = serde_json::from_slice(&fs::read(&result_path)?)?;
        assert_eq!(running["result"], "running");
        let run_id = running["validation_run"]["run_id"]
            .as_str()
            .expect("running result should identify the validation run")
            .to_owned();
        recorder.record_final(&serde_json::json!({
            "kind": "live_host_user_action_release_validation",
            "result": "passed",
            "host": { "kind": "codex" }
        }))?;
        let completed: Value = serde_json::from_slice(&fs::read(&result_path)?)?;
        assert_eq!(completed["result"], "passed");
        assert_eq!(completed["validation_run"]["run_id"], run_id);
        assert!(completed["validation_run"]["started_at"].is_string());
        assert!(completed["validation_run"]["recorded_at"].is_string());
        assert!(atomic_write_live_host_result(&result_path, "{}", "different-run").is_err());

        assert!(LiveResultRecorder::new("codex", Some(result_path.clone())).is_err());
        assert!(required_live_result_path(None).is_err());
        assert_eq!(
            required_live_result_path(Some(result_dir.join("required.json").into_os_string()))?,
            result_dir.join("required.json")
        );
        assert!(validate_external_result_path(Path::new("relative-result.json"), true).is_err());
        assert!(validate_external_result_path(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("live-result.json"),
            true,
        )
        .is_err());
        assert!(serialize_live_host_result(&serde_json::json!({
            "payload": "x".repeat(MAX_LIVE_HOST_RESULT_BYTES)
        }))
        .is_err());
        assert!(bounded_identity("bounded", "line\nbreak", 64).is_err());
        assert!(bounded_identity("bounded", &"x".repeat(65), 64).is_err());

        let early_failure_path = result_dir.join("claude-code.json");
        {
            let _recorder =
                LiveResultRecorder::new("claude-code", Some(early_failure_path.clone()))?;
        }
        let early_failure: Value = serde_json::from_slice(&fs::read(&early_failure_path)?)?;
        assert_eq!(early_failure["result"], "failed_before_completion");
        assert!(fs::read_dir(&result_dir)?.all(|entry| {
            entry
                .ok()
                .is_some_and(|entry| !entry.file_name().to_string_lossy().ends_with(".tmp"))
        }));
        Ok(())
    }

    #[test]
    fn operator_choice_confirmation_accepts_only_fixed_native_options() -> Result<(), Box<dyn Error>>
    {
        assert_eq!(
            parse_native_user_action_choice("choice:route_alpha")?,
            USER_ACTION_ROUTE_ALPHA_OPTION_ID
        );
        assert_eq!(
            parse_native_user_action_choice("choice:route_beta")?,
            USER_ACTION_ROUTE_BETA_OPTION_ID
        );
        assert!(parse_native_user_action_choice("route_alpha").is_err());
        assert!(parse_native_user_action_choice("choice:unrecognized").is_err());
        Ok(())
    }

    #[test]
    fn operator_evidence_summary_confirmation_is_bounded_and_explicit() -> Result<(), Box<dyn Error>>
    {
        assert_eq!(
            parse_live_evidence_summary_confirmation("summary:reviewed exact fixture bytes")?,
            "reviewed exact fixture bytes"
        );
        assert!(parse_live_evidence_summary_confirmation("reviewed exact fixture bytes").is_err());
        assert!(parse_live_evidence_summary_confirmation("summary:").is_err());
        assert!(parse_live_evidence_summary_confirmation("summary:line\nbreak").is_err());
        assert!(parse_live_evidence_summary_confirmation(&format!(
            "summary:{}",
            "x".repeat(USER_ACTION_OBSERVATION_SUMMARY_MAX_CHARS + 1)
        ))
        .is_err());
        Ok(())
    }

    #[test]
    fn local_web_delivery_boundary_confirmation_requires_both_exact_observations(
    ) -> Result<(), Box<dyn Error>> {
        let confirmed = parse_local_web_delivery_boundary_confirmation(
            MODEL_INVISIBLE_SURFACE_CONFIRMATION,
            MODEL_VISIBLE_ABSENCE_CONFIRMATION,
        )?;
        assert!(confirmed.host_owned_model_invisible_surface_confirmed);
        assert!(confirmed.model_visible_forbidden_payloads_absent_confirmed);
        assert!(parse_local_web_delivery_boundary_confirmation(
            "surface:chat",
            MODEL_VISIBLE_ABSENCE_CONFIRMATION,
        )
        .is_err());
        assert!(parse_local_web_delivery_boundary_confirmation(
            MODEL_INVISIBLE_SURFACE_CONFIRMATION,
            "model-visible:unobserved",
        )
        .is_err());
        Ok(())
    }

    #[test]
    fn authenticated_host_launch_removes_inherited_volicord_control_environment() {
        let control_names = [
            "VOLICORD_MCP_VERIFICATION",
            "VOLICORD_MCP_LAUNCH",
            "VOLICORD_MCP_HOST",
            "VOLICORD_MCP_CONNECTION_ID",
            "VOLICORD_MCP_PROJECT_ID",
            "VOLICORD_LOCAL_WEB_CONSENT",
        ];
        let mut command = Command::new("host-fixture");
        for name in control_names {
            command.env(name, "inherited-value");
        }
        LiveSmokeFixture::remove_inherited_host_control_env(&mut command);
        for name in control_names {
            assert!(command
                .get_envs()
                .any(|(key, value)| { key.to_string_lossy() == name && value.is_none() }));
        }
    }

    #[test]
    fn evidence_observation_result_shape_rejects_false_pass_mutations() -> Result<(), Box<dyn Error>>
    {
        let result = evidence_observation_result_shape_fixture();
        validate_live_evidence_observation_result_shape(&result)?;
        assert!(serialize_live_host_result(&result)?.len() < MAX_LIVE_HOST_RESULT_BYTES);

        for (path, replacement) in [
            (
                &["local_web_user_channel", "resolution", "channel_kind"][..],
                Value::String("mcp_elicitation".to_owned()),
            ),
            (
                &["local_web_user_channel", "resolution", "verification_basis"][..],
                Value::String(VERIFICATION_BASIS_MCP_ELICITATION_USER_CHANNEL.to_owned()),
            ),
            (
                &["local_web_user_channel", "resolution", "actor_source"][..],
                Value::String("agent_connection:CONN-live".to_owned()),
            ),
            (
                &[
                    "local_web_user_channel",
                    "handoff_delivery",
                    "effective_exact_capability_observed",
                ][..],
                Value::Bool(false),
            ),
            (
                &[
                    "local_web_user_channel",
                    "handoff_delivery",
                    "handoff_transport",
                ][..],
                Value::String("model_visible_content".to_owned()),
            ),
            (
                &[
                    "local_web_user_channel",
                    "handoff_delivery",
                    "host_owned_model_invisible_surface_operator_confirmed",
                ][..],
                Value::Bool(false),
            ),
            (
                &[
                    "local_web_user_channel",
                    "handoff_delivery",
                    "negative_model_visible_observation",
                    "operator_confirmed_absent",
                ][..],
                Value::Bool(false),
            ),
            (
                &[
                    "local_web_user_channel",
                    "handoff_delivery",
                    "negative_model_visible_observation",
                    "diagnostic_store_scan_passed",
                ][..],
                Value::Bool(false),
            ),
            (
                &["evidence_consumption", "producer_anchor", "producer_kind"][..],
                Value::String("unverified_caller".to_owned()),
            ),
            (
                &["evidence_consumption", "relevance_assessment", "status"][..],
                Value::String("contradicted".to_owned()),
            ),
            (
                &["evidence_consumption", "observed_at_matches_resolution"][..],
                Value::Bool(false),
            ),
            (
                &[
                    "local_web_user_channel",
                    "resolution",
                    "summary_character_count",
                ][..],
                Value::from(0),
            ),
            (
                &["local_web_user_channel", "resolution", "target"][..],
                serde_json::json!({
                    "target_kind": "acceptance_criterion",
                    "acceptance_criterion_id": "AC-other"
                }),
            ),
            (
                &[
                    "local_web_user_channel",
                    "host_resume",
                    "committed_record_run_calls",
                ][..],
                Value::from(0),
            ),
            (
                &["local_web_user_channel", "host_resume", "status_calls"][..],
                Value::from(2),
            ),
            (
                &[
                    "local_web_user_channel",
                    "host_resume",
                    "diagnostic_event_ordered",
                ][..],
                Value::Bool(false),
            ),
            (
                &["evidence_consumption", "input_resolution_ref_id"][..],
                Value::String("URES-other".to_owned()),
            ),
            (
                &["evidence_consumption", "product_file_write_observed"][..],
                Value::Bool(true),
            ),
            (
                &["evidence_consumption", "source_kind"][..],
                Value::String("agent_report".to_owned()),
            ),
            (
                &["authority_events", "user_action_resolved_event_seq"][..],
                Value::from(9),
            ),
            (
                &["stop_hook", "decision"][..],
                Value::String("block".to_owned()),
            ),
            (
                &["authority_receipt", "latest_run_id"][..],
                Value::String("RUN-other".to_owned()),
            ),
            (
                &["evidence_scope", "native_judgment_cell"][..],
                Value::Bool(true),
            ),
            (
                &["sensitive_payloads", "raw_summary_recorded"][..],
                Value::Bool(true),
            ),
        ] {
            let mut mutated = result.clone();
            set_nested_value(&mut mutated, path, replacement)?;
            assert!(
                validate_live_evidence_observation_result_shape(&mutated).is_err(),
                "false-pass mutation at {path:?} was accepted"
            );
        }

        for forbidden in [
            "raw_url",
            "bearer_token",
            "token",
            "raw_summary",
            "user_summary",
            "observation_summary",
            "operator_text",
            "prompt",
            "transcript",
        ] {
            let mut leaked = result.clone();
            leaked[forbidden] = Value::String("must-not-be-recorded".to_owned());
            assert!(validate_live_evidence_observation_result_shape(&leaked).is_err());
        }
        let mut nested_alias = result.clone();
        nested_alias["local_web_user_channel"]["resolution"]["operator_text"] =
            Value::String("must-not-be-recorded".to_owned());
        assert!(validate_live_evidence_observation_result_shape(&nested_alias).is_err());
        let mut uppercase_url = result.clone();
        uppercase_url["host"]["version"] =
            Value::String("HTTPS://LOCALHOST/consent?TOKEN=private".to_owned());
        assert!(validate_live_evidence_observation_result_shape(&uppercase_url).is_err());

        for (stage, expected_result) in [
            ("host_executable", "unavailable"),
            ("interactive_terminal", "unavailable"),
            ("host_delivery_boundary", "unavailable"),
            ("fixture_setup", "failed"),
            ("host_process", "failed"),
            ("stored_resolution", "failed"),
            ("authority_receipt", "failed"),
            ("stop_and_diagnostics", "failed"),
            ("managed_receipt_ui", "failed"),
            ("result_validation", "failed"),
        ] {
            let incomplete = live_evidence_observation_incomplete_summary("codex", stage);
            assert_eq!(incomplete["result"], expected_result);
            validate_live_evidence_observation_incomplete_result_shape(&incomplete)?;
            assert!(serialize_live_host_result(&incomplete)?.len() < MAX_LIVE_HOST_RESULT_BYTES);
        }
        let unknown_stage = live_evidence_observation_incomplete_summary("codex", "raw-error-text");
        assert!(
            validate_live_evidence_observation_incomplete_result_shape(&unknown_stage).is_err()
        );
        Ok(())
    }

    #[test]
    fn evidence_observation_fixture_prepares_no_request_before_the_live_host(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = LiveSmokeFixture::new("evidence-observation-setup")?;
        let init = fixture.run_volicord([
            "init",
            "--shared",
            "--host",
            "codex",
            "--repo",
            fixture.repo_arg(),
            "--profile",
            "detective",
            "--home",
            fixture.runtime_home_arg(),
            "--json",
        ])?;
        assert_success("volicord init for evidence setup fixture", &init);
        let init_json = json_stdout(&init)?;
        let connection_id = init_json["connection"]["connection_id"]
            .as_str()
            .ok_or_else(|| io::Error::other("evidence setup init returned no connection id"))?;
        let prepared = prepare_live_evidence_observation_authority(
            &fixture,
            connection_id,
            "VOLICORD_LIVE_EVIDENCE_SETUP_FIXTURE",
        )?;
        let project = list_projects(&fixture.runtime_home_path)?
            .into_iter()
            .find(|project| project.project_id == prepared.project_id)
            .ok_or_else(|| io::Error::other("evidence setup project is missing"))?;
        let conn = open_project_state_database_read_only(&project.state_db_path)?;
        let counts: (u64, u64, u64, u64) = conn.query_row(
            "SELECT
                 (SELECT COUNT(*) FROM user_action_requests WHERE task_id = ?1),
                 (SELECT COUNT(*) FROM runs WHERE task_id = ?1),
                 (SELECT COUNT(*) FROM artifacts WHERE task_id = ?1),
                 (SELECT COUNT(*) FROM evidence_observations WHERE task_id = ?1)",
            [&prepared.task_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )?;
        assert_eq!(counts, (0, 1, 1, 0));
        assert_eq!(
            prepared.artifact_ref.display_name,
            LIVE_EVIDENCE_ARTIFACT_DISPLAY_NAME
        );
        let presentation =
            UserActionPresentationPlan::from_form(&UserActionInboxForm::EvidenceObservation {
                target_candidates: vec![prepared.target.clone()],
                artifact_candidates: vec![prepared.artifact_ref.clone()],
                relevance_options: vec![
                    EvidenceRelevanceStatus::Supported,
                    EvidenceRelevanceStatus::Contradicted,
                ],
                summary_max_chars: USER_ACTION_OBSERVATION_SUMMARY_MAX_CHARS as u64,
            })?;
        assert_eq!(
            presentation.agent_facing_input_safety(
                LIVE_EVIDENCE_REQUEST_QUESTION,
                LIVE_EVIDENCE_REQUEST_CONTEXT,
            )?,
            UserActionPresentationSafety::UserOnlyInputRequired
        );
        let mut marker_free_artifact = prepared.artifact_ref.clone();
        marker_free_artifact.display_name = "local-consent-routing-fixture.txt".to_owned();
        let marker_free_presentation =
            UserActionPresentationPlan::from_form(&UserActionInboxForm::EvidenceObservation {
                target_candidates: vec![prepared.target.clone()],
                artifact_candidates: vec![marker_free_artifact],
                relevance_options: vec![
                    EvidenceRelevanceStatus::Supported,
                    EvidenceRelevanceStatus::Contradicted,
                ],
                summary_max_chars: USER_ACTION_OBSERVATION_SUMMARY_MAX_CHARS as u64,
            })?;
        assert_eq!(
            marker_free_presentation.agent_facing_input_safety(
                LIVE_EVIDENCE_REQUEST_QUESTION,
                LIVE_EVIDENCE_REQUEST_CONTEXT,
            )?,
            UserActionPresentationSafety::AgentFacingInputAllowed
        );
        assert!(live_evidence_observation_prompt(&prepared)
            .contains(LIVE_EVIDENCE_OBSERVATION_RUN_MARKER));
        Ok(())
    }

    #[test]
    fn local_web_evidence_diagnostic_query_requires_one_ordered_status(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = LiveSmokeFixture::new("evidence-diagnostic-query")?;
        let connection_id = "CONN-evidence-diagnostic-query";
        let project_id = "PRJ-evidence-diagnostic-query";
        let session_id = "SESSION-evidence-diagnostic-query";
        start_diagnostic_session(
            &fixture.runtime_home_path,
            DiagnosticSessionStart {
                session_id,
                connection_id: Some(connection_id),
                project_id: Some(project_id),
                transport: DiagnosticTransport::McpStdio,
                host_kind: Some(DiagnosticHostKind::Codex),
                package_version: env!("CARGO_PKG_VERSION"),
                build_id: "live-evidence-diagnostic-query-fixture",
            },
        )?;
        let record =
            |tool_name, core_committed, replayed, fallback_kind| -> Result<(), Box<dyn Error>> {
                record_diagnostic_event(
                    &fixture.runtime_home_path,
                    DiagnosticEvent {
                        session_id,
                        event_kind: DiagnosticEventKind::McpToolCall,
                        tool_name: Some(tool_name),
                        latency_micros: 1,
                        request_bytes: 1,
                        response_bytes: 1,
                        validation_failure: false,
                        core_reached: true,
                        core_committed,
                        replayed,
                        user_channel_kind: None,
                        fallback_kind,
                        product_file_write_count: 0,
                        authoritative_refresh_failure: false,
                        outcome: DiagnosticOutcome::Success,
                    },
                )?;
                Ok(())
            };
        record(
            "volicord.request_user_action",
            true,
            false,
            Some(DiagnosticFallbackKind::LocalWebConsent),
        )?;
        record("volicord.request_user_action", false, true, None)?;
        record("volicord.record_run", true, false, None)?;
        record("volicord.status", false, false, None)?;

        let observed = assert_local_web_evidence_diagnostic(&fixture, connection_id, project_id)?;
        assert_eq!(observed.create_calls, 1);
        assert_eq!(observed.resume_calls, 1);
        assert_eq!(observed.record_run_calls, 1);
        assert_eq!(observed.committed_record_run_calls, 1);
        assert_eq!(observed.status_calls, 1);
        assert_eq!(observed.successful_status_calls, 1);
        assert!(observed.ordered);

        record("volicord.status", false, false, None)?;
        assert!(assert_local_web_evidence_diagnostic(&fixture, connection_id, project_id).is_err());
        Ok(())
    }

    #[test]
    fn cli_fallback_result_shape_keeps_release_cells_separate() -> Result<(), Box<dyn Error>> {
        let result = cli_fallback_result_shape_fixture();
        validate_live_cli_fallback_result_shape(&result)?;

        let mut native_substitution = result.clone();
        native_substitution["evidence_scope"]["native_judgment_cell"] = Value::Bool(true);
        assert!(validate_live_cli_fallback_result_shape(&native_substitution).is_err());

        let mut missing_retry = result.clone();
        missing_retry["cli_user_channel"]["exact_retry"]["stdout_byte_identical"] =
            Value::Bool(false);
        assert!(validate_live_cli_fallback_result_shape(&missing_retry).is_err());

        let mut skipped_state_version = result.clone();
        skipped_state_version["cli_user_channel"]["resolution"]["committed_state_version"] =
            Value::from(5);
        skipped_state_version["cli_user_channel"]["exact_retry"]["state_version"] = Value::from(5);
        assert!(validate_live_cli_fallback_result_shape(&skipped_state_version).is_err());

        let mut receipt_run_mismatch = result.clone();
        receipt_run_mismatch["authority_receipt"]["latest_run_id"] =
            Value::String("RUN-other".to_owned());
        assert!(validate_live_cli_fallback_result_shape(&receipt_run_mismatch).is_err());

        let mut stop_run_mismatch = result.clone();
        stop_run_mismatch["stop_hook"]["latest_run_id"] = Value::String("RUN-other".to_owned());
        assert!(validate_live_cli_fallback_result_shape(&stop_run_mismatch).is_err());

        let mut stop_version_mismatch = result.clone();
        stop_version_mismatch["stop_hook"]["receipt_state_version"] = Value::from(6);
        assert!(validate_live_cli_fallback_result_shape(&stop_version_mismatch).is_err());

        let mut empty_project = result.clone();
        empty_project["task"]["project_id"] = Value::String(String::new());
        assert!(validate_live_cli_fallback_result_shape(&empty_project).is_err());

        let mut empty_task = result.clone();
        empty_task["task"]["task_id"] = Value::String(String::new());
        assert!(validate_live_cli_fallback_result_shape(&empty_task).is_err());

        let mut empty_connection = result.clone();
        empty_connection["connection"]["connection_id"] = Value::String(String::new());
        assert!(validate_live_cli_fallback_result_shape(&empty_connection).is_err());

        let mut receipt_project_mismatch = result.clone();
        receipt_project_mismatch["authority_receipt"]["project_id"] =
            Value::String("PRJ-other".to_owned());
        assert!(validate_live_cli_fallback_result_shape(&receipt_project_mismatch).is_err());

        let mut receipt_task_mismatch = result.clone();
        receipt_task_mismatch["authority_receipt"]["task_id"] =
            Value::String("TASK-other".to_owned());
        assert!(validate_live_cli_fallback_result_shape(&receipt_task_mismatch).is_err());

        let mut receipt_version_mismatch = result.clone();
        receipt_version_mismatch["authority_receipt"]["state_version"] = Value::from(6);
        assert!(validate_live_cli_fallback_result_shape(&receipt_version_mismatch).is_err());

        let mut zero_event_sequence = result.clone();
        zero_event_sequence["authority_events"]["user_action_requested_event_seq"] = Value::from(0);
        assert!(validate_live_cli_fallback_result_shape(&zero_event_sequence).is_err());

        let mut misleading_event_order = result;
        misleading_event_order["authority_events"]["user_action_resolved_event_seq"] =
            Value::from(13);
        assert!(validate_live_cli_fallback_result_shape(&misleading_event_order).is_err());
        Ok(())
    }

    #[test]
    fn final_output_confirmation_and_result_shape_are_strict() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            parse_final_output_ui_confirmation(
                "surface:managed-final-output",
                &FinalOutputUiExpectation::ManagedSurface,
            )?,
            "surface:managed-final-output"
        );
        let complete_status_fallback = "No active Task is available. Run `volicord status --json`.";
        assert_eq!(
            parse_final_output_ui_confirmation(
                &format!("status-ui:{complete_status_fallback}"),
                &FinalOutputUiExpectation::NoActiveTaskStatus {
                    complete_message: complete_status_fallback.to_owned(),
                },
            )?,
            format!("status-ui:{complete_status_fallback}")
        );
        let complete_receipt = r#"{"project_id":"PRJ-live","state_version":42,"task_ref":{"record_id":"TASK-live","version":1}}"#;
        assert_eq!(
            parse_final_output_ui_confirmation(
                &format!("receipt-json:{complete_receipt}"),
                &FinalOutputUiExpectation::CompleteAuthorityReceipt {
                    canonical_json: complete_receipt.to_owned(),
                },
            )?,
            format!("receipt-json:{complete_receipt}")
        );
        assert!(parse_final_output_ui_confirmation(
            "receipt-json:{\"state_version\":42}",
            &FinalOutputUiExpectation::CompleteAuthorityReceipt {
                canonical_json: complete_receipt.to_owned(),
            },
        )
        .is_err());
        assert!(parse_final_output_ui_confirmation(
            "status-ui:Run `volicord status --json --task TASK-hidden`.",
            &FinalOutputUiExpectation::NoActiveTaskStatus {
                complete_message: complete_status_fallback.to_owned(),
            },
        )
        .is_err());

        let record = final_output_result_shape_fixture(IntegrationProfile::Record);
        validate_final_output_result_shape(&record, IntegrationProfile::Record)?;
        let detective = final_output_result_shape_fixture(IntegrationProfile::Detective);
        validate_final_output_result_shape(&detective, IntegrationProfile::Detective)?;
        for profile in [IntegrationProfile::Record, IntegrationProfile::Detective] {
            let unavailable = final_output_unavailable_summary(
                "fixture",
                profile,
                "fixture prerequisite missing",
            );
            validate_final_output_result_shape(&unavailable, profile)?;
            assert_eq!(
                unavailable["evidence"]["detective_decision"]["status"],
                "unavailable"
            );
        }

        let mut false_pass = detective.clone();
        false_pass["result"] = Value::String("passed".to_owned());
        assert!(
            validate_final_output_result_shape(&false_pass, IntegrationProfile::Detective).is_err()
        );
        let mut block_false_pass = detective.clone();
        block_false_pass["result"] = Value::String("passed".to_owned());
        block_false_pass["evidence"]["exact_replay"]["status"] =
            Value::String("verified".to_owned());
        block_false_pass["evidence"]["exact_replay"]["actual_host_replay"]["status"] =
            Value::String("verified".to_owned());
        assert!(validate_final_output_result_shape(
            &block_false_pass,
            IntegrationProfile::Detective
        )
        .is_err());
        let mut incomplete_fallback_confirmation = record.clone();
        incomplete_fallback_confirmation["evidence"]["actual_host_fixed_ui"]["status_fallback"]
            ["complete_taskless_message_operator_confirmed"] = Value::Bool(false);
        assert!(validate_final_output_result_shape(
            &incomplete_fallback_confirmation,
            IntegrationProfile::Record
        )
        .is_err());
        let mut collapsed = detective;
        collapsed["evidence"]
            .as_object_mut()
            .expect("fixture evidence should be an object")
            .remove("actual_host_event");
        assert!(
            validate_final_output_result_shape(&collapsed, IntegrationProfile::Detective).is_err()
        );
        let mut substituted = record;
        substituted["evidence"]["actual_host_fixed_ui"]["authority_receipt"]["status"] =
            Value::String("unavailable".to_owned());
        assert!(
            validate_final_output_result_shape(&substituted, IntegrationProfile::Record).is_err()
        );
        let unavailable = final_output_unavailable_summary(
            "claude-code",
            IntegrationProfile::Detective,
            "host executable unavailable",
        );
        assert_eq!(unavailable["result"], "incomplete");
        assert_eq!(
            unavailable["evidence"]["actual_host_event"]["status"],
            "unavailable"
        );
        Ok(())
    }

    #[test]
    fn generated_final_output_matrix_layers_are_exercised_without_live_hosts(
    ) -> Result<(), Box<dyn Error>> {
        for (host, profile, expected_host_action) in [
            ("codex", IntegrationProfile::Record, "host_trust_required"),
            (
                "codex",
                IntegrationProfile::Detective,
                "host_trust_required",
            ),
            (
                "claude-code",
                IntegrationProfile::Record,
                "project_approval_required",
            ),
            (
                "claude-code",
                IntegrationProfile::Detective,
                "project_approval_required",
            ),
        ] {
            let fixture = LiveSmokeFixture::new(&format!(
                "direct-final-output-{}-{}",
                host.replace('-', "_"),
                profile.as_str()
            ))?;
            let live_bin = fixture.runtime_home_path.join("live-bin");
            match host {
                "codex" => {
                    write_fake_codex(&live_bin)?;
                }
                "claude-code" => {
                    write_fake_claude_code(&live_bin)?;
                }
                _ => unreachable!("the direct matrix has only maintained hosts"),
            }
            let init = fixture.run_volicord([
                "init",
                "--shared",
                "--host",
                host,
                "--repo",
                fixture.repo_arg(),
                "--profile",
                profile.as_str(),
                "--home",
                fixture.runtime_home_arg(),
                "--json",
            ])?;
            assert_success("volicord init for direct final-output matrix", &init);
            let init_json = json_stdout(&init)?;
            assert_direct_matrix_init_report(&init_json, host, profile, expected_host_action);
            let connection_id = init_json["connection"]["connection_id"]
                .as_str()
                .ok_or_else(|| io::Error::other("matrix init returned no connection id"))?;
            let config_fixture = verify_final_output_config_fixture(
                &fixture, host, profile, &init_json,
            )
            .map_err(|error| {
                io::Error::other(format!(
                    "{host}/{} generated config verification failed: {error}",
                    profile.as_str()
                ))
            })?;
            assert_eq!(config_fixture["status"], "verified");
            let project_id = live_fixture_project_id(&fixture)?;

            let no_active_private_prose = "private matrix no-active prose";
            let no_active_event = live_final_output_event(
                host,
                &fixture.repo_root,
                &format!("direct_no_active_{}_{}", host, profile.as_str()),
                no_active_private_prose,
            )?;
            let before_no_active = guard_observation_counts(&fixture, &project_id)?;
            let first_no_active = run_generated_final_output_handler(
                &fixture.runtime_home_path,
                &fixture.repo_root,
                &fixture.env_path,
                host,
                &no_active_event,
            )?;
            verify_no_active_status_wire(&first_no_active, no_active_private_prose)?;
            let after_first_no_active = guard_observation_counts(&fixture, &project_id)?;
            let second_no_active = run_generated_final_output_handler(
                &fixture.runtime_home_path,
                &fixture.repo_root,
                &fixture.env_path,
                host,
                &no_active_event,
            )?;
            verify_no_active_status_wire(&second_no_active, no_active_private_prose)?;
            let after_second_no_active = guard_observation_counts(&fixture, &project_id)?;
            assert_eq!(first_no_active.stdout, second_no_active.stdout);
            match profile {
                IntegrationProfile::Record => {
                    assert_eq!(before_no_active, after_first_no_active);
                    assert_eq!(after_first_no_active, after_second_no_active);
                }
                IntegrationProfile::Detective => {
                    assert_eq!(
                        after_first_no_active.guard_events,
                        before_no_active.guard_events + 1
                    );
                    assert_eq!(
                        after_second_no_active.guard_events,
                        after_first_no_active.guard_events
                    );
                }
            }

            let prepared = prepare_live_final_authority(
                &fixture,
                connection_id,
                &format!("DIRECT_FINAL_AUTHORITY_{}_{}", host, profile.as_str()),
            )
            .map_err(|error| {
                io::Error::other(format!(
                    "{host}/{} final authority preparation failed: {error}",
                    profile.as_str()
                ))
            })?;
            let canonical_receipt = canonical_json_string(&prepared.receipt.canonical_receipt)?;
            parse_final_output_ui_confirmation(
                &format!("receipt-json:{canonical_receipt}"),
                &FinalOutputUiExpectation::CompleteAuthorityReceipt {
                    canonical_json: canonical_receipt.clone(),
                },
            )?;
            let active_session_id = format!("direct_active_{}_{}", host, profile.as_str());
            let active_private_prose = "private matrix active prose";
            let active_event = live_final_output_event(
                host,
                &fixture.repo_root,
                &active_session_id,
                active_private_prose,
            )?;
            let before_active = guard_observation_counts(&fixture, &project_id)?;
            let first_active = run_generated_final_output_handler(
                &fixture.runtime_home_path,
                &fixture.repo_root,
                &fixture.env_path,
                host,
                &active_event,
            )?;
            let expected_continue = profile == IntegrationProfile::Record
                || prepared.receipt.close_state == StatusCloseState::Ready;
            verify_authority_receipt_wire(
                &first_active,
                &prepared.receipt,
                expected_continue,
                active_private_prose,
            )
            .map_err(|error| {
                io::Error::other(format!(
                    "{host}/{} first active direct receipt failed: {error}",
                    profile.as_str()
                ))
            })?;
            let first_active_wire: Value = serde_json::from_slice(&first_active.stdout)?;
            assert_eq!(
                first_active_wire["systemMessage"],
                format!("Volicord authority receipt: {canonical_receipt}")
            );
            let after_first_active = guard_observation_counts(&fixture, &project_id)?;
            let first_historical = if profile == IntegrationProfile::Detective {
                Some(stored_stop_snapshot_for_session(
                    &fixture,
                    &project_id,
                    &active_session_id,
                )?)
            } else {
                None
            };
            let replayed_authority = advance_live_final_authority(
                &fixture,
                connection_id,
                &prepared,
                &format!(
                    "DIRECT_FINAL_AUTHORITY_ADVANCED_{}_{}",
                    host,
                    profile.as_str()
                ),
            )?;
            assert!(replayed_authority.receipt.state_version > prepared.receipt.state_version);
            let replayed_canonical_receipt =
                canonical_json_string(&replayed_authority.receipt.canonical_receipt)?;
            let second_active = run_generated_final_output_handler(
                &fixture.runtime_home_path,
                &fixture.repo_root,
                &fixture.env_path,
                host,
                &active_event,
            )?;
            let replayed_continue = profile == IntegrationProfile::Record
                || replayed_authority.receipt.close_state == StatusCloseState::Ready;
            verify_authority_receipt_wire(
                &second_active,
                &replayed_authority.receipt,
                replayed_continue,
                active_private_prose,
            )
            .map_err(|error| {
                io::Error::other(format!(
                    "{host}/{} replayed active direct receipt failed: {error}",
                    profile.as_str()
                ))
            })?;
            let second_active_wire: Value = serde_json::from_slice(&second_active.stdout)?;
            assert_eq!(
                second_active_wire["systemMessage"],
                format!("Volicord authority receipt: {replayed_canonical_receipt}")
            );
            let after_second_active = guard_observation_counts(&fixture, &project_id)?;
            assert_ne!(first_active.stdout, second_active.stdout);
            match profile {
                IntegrationProfile::Record => {
                    assert_eq!(before_active, after_first_active);
                    assert_eq!(after_first_active, after_second_active);
                }
                IntegrationProfile::Detective => {
                    assert_eq!(
                        after_first_active.guard_events,
                        before_active.guard_events + 1
                    );
                    assert_eq!(
                        after_second_active.guard_events,
                        after_first_active.guard_events
                    );
                    assert_eq!(
                        stored_stop_snapshot_for_session(
                            &fixture,
                            &project_id,
                            &active_session_id,
                        )?,
                        first_historical
                            .expect("Detective replay should capture a historical Stop")
                    );
                }
            }
        }
        Ok(())
    }

    #[test]
    fn advisor_shaping_close_basis_projects_record_final_output_without_guard_observation(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("live-advisor-stop-ready")?;
        initialize_git_repository(&fixture.product_repo_path())?;
        let context =
            McpConnectionContext::resolve(fixture.runtime_home_path(), fixture.connection_id())?
                .with_invocation_binding_basis(VERIFICATION_BASIS_TEST_FIXTURE_BINDING);
        let adapter = McpAdapter::new(fixture.runtime_home_path(), context);
        let intake = adapter.call_tool(
            "volicord.intake",
            serde_json::json!({
                "detail": "full",
                "plain_language_request": "Validate an advisor no-write Stop receipt.",
                "requested_mode": "advisor",
                "resume_policy": "create_new",
                "acceptance_policy": null,
                "lineage": null,
                "initial_scope": {
                    "boundary": "Validate one advisor no-write Run.",
                    "non_goals": [],
                    "acceptance_criteria": [{
                        "statement": "The advisor Run establishes a close basis.",
                        "evidence_requirement": "not_required"
                    }]
                }
            }),
        )?;
        assert_eq!(intake.response_value["base"]["response_kind"], "result");
        assert_eq!(intake.response_value["state"]["mode"], "advisor");
        assert_eq!(
            intake.response_value["state"]["acceptance_policy"],
            "not_required"
        );
        assert!(intake.response_value["state"]["active_change_unit_ref"].is_null());
        let task_id = intake.response_value["task_ref"]["record_id"]
            .as_str()
            .ok_or("advisor intake should return a Task ref")?
            .to_owned();

        let scope = adapter.call_tool(
            "volicord.update_scope",
            serde_json::json!({
                "detail": "full",
                "task_id": task_id,
                "goal_summary": null,
                "scope_update": null,
                "scope_boundary": null,
                "non_goals": null,
                "acceptance_criteria": null,
                "autonomy_boundary": null,
                "baseline_ref": LIVE_HOST_BASELINE_REF,
                "change_unit": {
                    "operation": "create_current",
                    "scope_summary": "No-write live-host user-action validation.",
                    "affected_paths": []
                },
                "related_scope_decision_refs": []
            }),
        )?;
        assert_eq!(scope.response_value["base"]["response_kind"], "result");
        assert_eq!(
            scope.response_value["state"]["baseline_ref"],
            LIVE_HOST_BASELINE_REF
        );
        let change_unit_id = scope.response_value["state"]["active_change_unit_ref"]["record_id"]
            .as_str()
            .ok_or("update_scope should create the current Change Unit")?
            .to_owned();

        let marker = USER_ACTION_ROUTE_ALPHA_RUN_MARKER;
        let recorded = adapter.call_tool(
            "volicord.record_run",
            serde_json::json!({
                "detail": "full",
                "task_id": task_id,
                "change_unit_id": change_unit_id,
                "kind": "shaping_update",
                "run_id": null,
                "baseline_ref": LIVE_HOST_BASELINE_REF,
                "write_ticket_id": null,
                "summary": marker,
                "observed_changes": {
                    "changed_paths": [],
                    "product_file_write_observed": false,
                    "sensitive_categories": [],
                    "baseline_ref": LIVE_HOST_BASELINE_REF
                },
                "artifact_inputs": [],
                "evidence_updates": [],
                "evidence_observations": [],
                "close_assessment": {
                    "result_summary": marker,
                    "result_refs": [],
                    "residual_risks": [],
                    "sensitive_categories": [],
                    "recovery_constraints": []
                }
            }),
        )?;
        assert_eq!(recorded.response_value["base"]["response_kind"], "result");
        assert_eq!(
            recorded.response_value["run_summary"]["kind"],
            "shaping_update"
        );
        assert!(recorded.response_value["current_close_basis"].is_object());

        let status = adapter.call_tool(
            "volicord.status",
            serde_json::json!({
                "task_id": task_id,
                "detail": "full"
            }),
        )?;
        let state_version = status.response_value["base"]["state_version"]
            .as_u64()
            .ok_or("advisor status should return a state version")?;
        let observation = LiveUserActionObservation {
            project_id: fixture.project_id().to_owned(),
            task_id: task_id.clone(),
            lifecycle_phase: status.response_value["active_task"]["lifecycle"]["lifecycle_phase"]
                .as_str()
                .unwrap_or("unknown")
                .to_owned(),
            state_version,
            user_action_request_id: None,
            user_action_status: None,
            requested_by_actor_source: None,
            user_action_resolution_id: None,
            resolved_by_actor_source: None,
            resolved_verification_basis: None,
            resolved_channel_kind: None,
            selected_option_id: None,
            option_ids: Vec::new(),
        };
        let receipt =
            verify_fresh_authority_receipt(status.response_value.clone(), &observation, marker)?;
        assert_eq!(receipt.close_state, StatusCloseState::Ready);
        assert_eq!(receipt.close_blocker_count, 0);

        let mut mismatched = status.response_value.clone();
        mismatched["authority_receipt"]["task_ref"]["record_id"] =
            Value::String("task_mismatch".to_owned());
        assert!(verify_fresh_authority_receipt(mismatched, &observation, marker).is_err());
        let mut mismatched_close_basis = status.response_value.clone();
        mismatched_close_basis["current_close_basis"]["result_summary"] =
            Value::String("wrong choice marker".to_owned());
        assert!(
            verify_fresh_authority_receipt(mismatched_close_basis, &observation, marker).is_err()
        );

        let bin_dir = fixture.runtime_home_path().join("record-final-output-bin");
        write_fake_codex(&bin_dir)?;
        write_volicord_shim(&bin_dir, Path::new(volicord_bin()))?;
        let path = path_with_prefix(&bin_dir)?;
        let init = Command::new(volicord_bin())
            .args([
                "init",
                "--shared",
                "--host",
                "codex",
                "--repo",
                &path_text(&fixture.product_repo_path()),
                "--profile",
                "record",
                "--json",
            ])
            .env("VOLICORD_HOME", fixture.runtime_home_path())
            .env("PATH", &path)
            .current_dir(fixture.product_repo_path())
            .output()?;
        assert!(
            init.status.success(),
            "Record init failed: {}",
            stderr_output(&init)
        );
        let init_json: Value = serde_json::from_slice(&init.stdout)?;
        assert_eq!(
            init_json["states"]["final_output_authority_disclosure"],
            serde_json::json!({
                "supported": true,
                "configured": true,
                "verified": true
            })
        );
        let event = serde_json::json!({
            "event_id": "live_advisor_stop_ready",
            "session_id": "live_advisor_stop_ready_session",
            "host_kind": "codex",
            "last_assistant_message": "private final model prose must not become authority"
        });
        let before_final_output = fixture.counts()?;
        let final_output = run_record_final_output_handler(
            fixture.runtime_home_path(),
            &fixture.product_repo_path(),
            &path,
            &event,
        )?;
        assert!(
            final_output.status.success(),
            "Record final-output handler failed: {}",
            stderr_output(&final_output)
        );
        assert_eq!(fixture.counts()?, before_final_output);
        let stop_json: Value = serde_json::from_slice(&final_output.stdout)?;
        assert_eq!(
            stop_json["continue"], true,
            "unexpected final-output response: {stop_json:#}"
        );
        assert!(!String::from_utf8_lossy(&final_output.stdout)
            .contains("private final model prose must not become authority"));
        let message = stop_json["systemMessage"]
            .as_str()
            .ok_or("active ready final output should render the fresh AuthorityReceipt")?;
        let stop_receipt: AuthorityReceipt = serde_json::from_str(
            message
                .strip_prefix("Volicord authority receipt: ")
                .ok_or("final-output systemMessage should use the receipt prefix")?,
        )?;
        assert_eq!(stop_receipt.project_id.as_str(), fixture.project_id());
        assert_eq!(stop_receipt.task_ref.record_id.as_str(), task_id);
        assert_eq!(stop_receipt.state_version, receipt.state_version);
        assert_eq!(
            stop_receipt
                .latest_run_ref
                .as_ref()
                .map(|run| run.record_id.as_str()),
            Some(receipt.latest_run_id.as_str())
        );
        assert!(stop_receipt.close_blockers.is_empty());
        let project = list_projects(fixture.runtime_home_path())?
            .into_iter()
            .find(|project| project.project_id == fixture.project_id())
            .ok_or("Record final-output fixture project should remain registered")?;
        let state = open_project_state_database_read_only(&project.state_db_path)?;
        let observation_count: u64 = state.query_row(
            "SELECT (SELECT COUNT(*) FROM guard_events) + (SELECT COUNT(*) FROM agent_sessions)",
            [],
            |row| row.get(0),
        )?;
        assert_eq!(observation_count, 0);
        Ok(())
    }

    #[test]
    #[ignore = "requires an installed live Codex host and VOLICORD_RUN_CODEX_SMOKE=1"]
    fn codex_live_smoke_is_opt_in() -> Result<(), Box<dyn Error>> {
        if !smoke_enabled(CODEX_SMOKE_ENV) {
            return Err(io::Error::other(format!(
                "set {CODEX_SMOKE_ENV}=1 before running the ignored Codex smoke test"
            ))
            .into());
        }
        let codex = find_executable("codex").ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "`codex` was not found on PATH")
        })?;

        let fixture = LiveSmokeFixture::new("codex")?;
        let version = fixture.run_host_command(&codex, ["--version"])?;
        assert_success("codex --version", &version);

        let init = fixture.run_volicord([
            "init",
            "--shared",
            "--host",
            "codex",
            "--repo",
            fixture.repo_arg(),
            "--profile",
            "detective",
            "--home",
            fixture.runtime_home_arg(),
            "--json",
        ])?;
        assert_success("volicord init --host codex --profile detective", &init);
        let init_json = json_stdout(&init)?;
        assert_live_init_reported_action_required(
            &init_json,
            "codex",
            IntegrationProfile::Detective,
            "host_trust_required",
        );
        assert_eq!(init_json["states"]["hook_config"], "created");
        assert_eq!(init_json["states"]["required_hook_phases"], "configured");
        assert_file_contains(
            &fixture.repo_root.join(".codex/config.toml"),
            "[mcp_servers.volicord]",
        )?;
        assert_file_contains(
            &fixture.repo_root.join(".codex/config.toml"),
            "args = [\"mcp\", \"--stdio\", \"--discover-repository\", \"--host\", \"codex\"]",
        )?;
        let codex_mcp = fs::read_to_string(fixture.repo_root.join(".codex/config.toml"))?;
        assert!(!codex_mcp.contains("[mcp_servers.volicord.env]"));
        assert!(!codex_mcp.contains("--connection"));
        assert!(!codex_mcp.contains(fixture.runtime_home_arg()));
        assert_file_contains(&fixture.repo_root.join(".codex/hooks.json"), "PreToolUse")?;
        assert!(fixture
            .repo_root
            .join(".codex/hooks/volicord-dispatch.sh")
            .exists());
        assert!(fixture
            .repo_root
            .join(".codex/hooks/volicord-pre-tool.sh")
            .exists());
        assert!(fixture
            .repo_root
            .join(".codex/rules/volicord.rules")
            .exists());

        let inspect_help = fixture.run_host_command(&codex, ["mcp", "get", "--help"])?;
        if inspect_help.output.status.success() && !inspect_help.timed_out {
            let inspect = fixture.run_host_command(&codex, ["mcp", "get", "--json", "volicord"])?;
            if inspect.output.status.success() {
                let value = json_stdout(&inspect)?;
                assert_codex_mcp_entry(&value);
                smoke_note(
                    "codex",
                    "safe `codex mcp get --json volicord` discovered the generated MCP entry",
                );
            } else if output_text(&inspect).contains("No MCP server named") {
                smoke_note(
                    "codex",
                    "safe `codex mcp get` did not discover project-local `.codex/config.toml`; treating live host discovery as limited because project trust has no non-interactive confirmation path in this smoke test",
                );
            } else {
                panic!(
                    "codex mcp get failed unexpectedly\nstdout:\n{}\nstderr:\n{}",
                    stdout(&inspect),
                    stderr(&inspect)
                );
            }
        } else {
            smoke_note(
                "codex",
                "safe `codex mcp get` inspect command was unavailable; config generation was checked only",
            );
        }
        smoke_note(
            "codex",
            "live deny/block interpretation was not run because Codex non-interactive agent execution can require credentials, model access, or network",
        );
        Ok(())
    }

    #[test]
    #[ignore = "requires an installed live Claude Code host and VOLICORD_RUN_CLAUDE_SMOKE=1"]
    fn claude_code_live_smoke_is_opt_in() -> Result<(), Box<dyn Error>> {
        if !smoke_enabled(CLAUDE_SMOKE_ENV) {
            return Err(io::Error::other(format!(
                "set {CLAUDE_SMOKE_ENV}=1 before running the ignored Claude Code smoke test"
            ))
            .into());
        }
        let claude = find_executable("claude").ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "`claude` was not found on PATH")
        })?;

        let fixture = LiveSmokeFixture::new("claude-code")?;
        let version = fixture.run_host_command(&claude, ["--version"])?;
        assert_success("claude --version", &version);

        let init = fixture.run_volicord([
            "init",
            "--shared",
            "--host",
            "claude-code",
            "--repo",
            fixture.repo_arg(),
            "--profile",
            "detective",
            "--home",
            fixture.runtime_home_arg(),
            "--json",
        ])?;
        assert_success(
            "volicord init --host claude-code --profile detective",
            &init,
        );
        let init_json = json_stdout(&init)?;
        assert_live_init_reported_action_required(
            &init_json,
            "claude-code",
            IntegrationProfile::Detective,
            "project_approval_required",
        );
        assert_eq!(init_json["states"]["hook_config"], "created");
        let claude_mcp = fs::read_to_string(fixture.repo_root.join(".mcp.json"))?;
        assert!(claude_mcp.contains("\"volicord\""));
        assert!(claude_mcp.contains("\"--discover-repository\""));
        assert!(claude_mcp.contains("\"claude-code\""));
        assert!(!claude_mcp.contains("\"--connection\""));
        assert!(!claude_mcp.contains(fixture.runtime_home_arg()));
        assert_file_contains(
            &fixture.repo_root.join(".claude/settings.json"),
            "PreToolUse",
        )?;
        assert!(fixture
            .repo_root
            .join(".claude/hooks/volicord-pre-tool.sh")
            .exists());
        assert!(fixture.repo_root.join(".claude/rules/volicord.md").exists());

        let inspect_help = fixture.run_host_command(&claude, ["mcp", "get", "--help"])?;
        if inspect_help.output.status.success() && !inspect_help.timed_out {
            let inspect = fixture.run_host_command(&claude, ["mcp", "get", "volicord"])?;
            assert_claude_mcp_inspect_output(&inspect);
            smoke_note(
                "claude-code",
                "safe `claude mcp get volicord` returned inspect output for the generated MCP entry",
            );
        } else {
            smoke_note(
                "claude-code",
                "safe `claude mcp get` inspect command was unavailable; config generation was checked only",
            );
        }
        smoke_note(
            "claude-code",
            "live deny/block interpretation was not run because no hook-only non-interactive host runner was detected",
        );
        Ok(())
    }

    #[test]
    #[ignore = "requires an authenticated interactive Codex host and VOLICORD_RUN_CODEX_RECORD_FINAL_OUTPUT_SMOKE=1"]
    fn codex_record_live_final_output_is_opt_in() -> Result<(), Box<dyn Error>> {
        live_final_output_matrix_cell(
            "codex",
            "codex",
            IntegrationProfile::Record,
            CODEX_RECORD_FINAL_OUTPUT_SMOKE_ENV,
            "host_trust_required",
        )
    }

    #[test]
    #[ignore = "requires an authenticated interactive Codex host and VOLICORD_RUN_CODEX_DETECTIVE_FINAL_OUTPUT_SMOKE=1"]
    fn codex_detective_live_final_output_is_opt_in() -> Result<(), Box<dyn Error>> {
        live_final_output_matrix_cell(
            "codex",
            "codex",
            IntegrationProfile::Detective,
            CODEX_DETECTIVE_FINAL_OUTPUT_SMOKE_ENV,
            "host_trust_required",
        )
    }

    #[test]
    #[ignore = "requires an authenticated interactive Claude Code host and VOLICORD_RUN_CLAUDE_RECORD_FINAL_OUTPUT_SMOKE=1"]
    fn claude_code_record_live_final_output_is_opt_in() -> Result<(), Box<dyn Error>> {
        live_final_output_matrix_cell(
            "claude-code",
            "claude",
            IntegrationProfile::Record,
            CLAUDE_RECORD_FINAL_OUTPUT_SMOKE_ENV,
            "project_approval_required",
        )
    }

    #[test]
    #[ignore = "requires an authenticated interactive Claude Code host and VOLICORD_RUN_CLAUDE_DETECTIVE_FINAL_OUTPUT_SMOKE=1"]
    fn claude_code_detective_live_final_output_is_opt_in() -> Result<(), Box<dyn Error>> {
        live_final_output_matrix_cell(
            "claude-code",
            "claude",
            IntegrationProfile::Detective,
            CLAUDE_DETECTIVE_FINAL_OUTPUT_SMOKE_ENV,
            "project_approval_required",
        )
    }

    #[test]
    #[ignore = "requires an authenticated interactive Codex host and VOLICORD_RUN_CODEX_USER_ACTION_SMOKE=1"]
    fn codex_live_user_action_round_trip_is_opt_in() -> Result<(), Box<dyn Error>> {
        live_user_action_round_trip(
            "codex",
            "codex",
            CODEX_USER_ACTION_SMOKE_ENV,
            "host_trust_required",
        )
    }

    #[test]
    #[ignore = "requires an authenticated interactive Claude Code host and VOLICORD_RUN_CLAUDE_USER_ACTION_SMOKE=1"]
    fn claude_code_live_user_action_round_trip_is_opt_in() -> Result<(), Box<dyn Error>> {
        live_user_action_round_trip(
            "claude-code",
            "claude",
            CLAUDE_USER_ACTION_SMOKE_ENV,
            "project_approval_required",
        )
    }

    #[test]
    #[ignore = "requires an authenticated interactive Codex host with an observable host-owned model-invisible user surface and VOLICORD_RUN_CODEX_EVIDENCE_OBSERVATION_SMOKE=1"]
    fn codex_live_evidence_observation_round_trip_is_opt_in() -> Result<(), Box<dyn Error>> {
        live_evidence_observation_round_trip(
            "codex",
            "codex",
            CODEX_EVIDENCE_OBSERVATION_SMOKE_ENV,
            "host_trust_required",
        )
    }

    #[test]
    #[ignore = "requires an authenticated interactive Claude Code host with an observable host-owned model-invisible user surface and VOLICORD_RUN_CLAUDE_EVIDENCE_OBSERVATION_SMOKE=1"]
    fn claude_code_live_evidence_observation_round_trip_is_opt_in() -> Result<(), Box<dyn Error>> {
        live_evidence_observation_round_trip(
            "claude-code",
            "claude",
            CLAUDE_EVIDENCE_OBSERVATION_SMOKE_ENV,
            "project_approval_required",
        )
    }

    #[test]
    #[ignore = "requires an authenticated interactive Codex host and VOLICORD_RUN_CODEX_CLI_FALLBACK_SMOKE=1"]
    fn codex_live_cli_fallback_round_trip_is_opt_in() -> Result<(), Box<dyn Error>> {
        live_cli_fallback_round_trip(
            "codex",
            "codex",
            CODEX_CLI_FALLBACK_SMOKE_ENV,
            "host_trust_required",
        )
    }

    #[test]
    #[ignore = "requires an authenticated interactive Claude Code host and VOLICORD_RUN_CLAUDE_CLI_FALLBACK_SMOKE=1"]
    fn claude_code_live_cli_fallback_round_trip_is_opt_in() -> Result<(), Box<dyn Error>> {
        live_cli_fallback_round_trip(
            "claude-code",
            "claude",
            CLAUDE_CLI_FALLBACK_SMOKE_ENV,
            "project_approval_required",
        )
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum IntegrationProfile {
        Record,
        Detective,
    }

    impl IntegrationProfile {
        fn as_str(self) -> &'static str {
            match self {
                Self::Record => "record",
                Self::Detective => "detective",
            }
        }
    }

    fn live_final_output_matrix_cell(
        host: &str,
        executable_name: &str,
        profile: IntegrationProfile,
        selector_env: &str,
        expected_host_action: &str,
    ) -> Result<(), Box<dyn Error>> {
        if !smoke_enabled(selector_env) {
            return Err(io::Error::other(format!(
                "set {selector_env}=1 before running the ignored {host}/{} final-output smoke test",
                profile.as_str()
            ))
            .into());
        }
        let recorder_host = format!("{host}-{}-final-output", profile.as_str());
        let mut result_recorder =
            LiveResultRecorder::from_env_for_kind(&recorder_host, LIVE_FINAL_OUTPUT_RESULT_KIND)?;
        let executable = match find_executable(executable_name) {
            Some(executable) => executable,
            None => {
                let summary = final_output_unavailable_summary(
                    host,
                    profile,
                    &format!("`{executable_name}` was not found on PATH"),
                );
                validate_final_output_result_shape(&summary, profile)?;
                result_recorder.record_final(&summary)?;
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("`{executable_name}` was not found on PATH"),
                )
                .into());
            }
        };
        if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
            let reason = "authenticated live final-output validation requires interactive terminal stdin and stdout";
            let summary = final_output_unavailable_summary(host, profile, reason);
            validate_final_output_result_shape(&summary, profile)?;
            result_recorder.record_final(&summary)?;
            return Err(io::Error::other(reason).into());
        }
        let fixture = LiveSmokeFixture::new(&recorder_host)?;
        let host_version_output = fixture.run_host_command(&executable, ["--version"])?;
        assert_success(
            &format!("{executable_name} --version"),
            &host_version_output,
        );
        let host_version = host_version_summary(&host_version_output)?;
        let volicord_build_id = bounded_identity(
            "Volicord build_id",
            &volicord_mcp::build_id(),
            MAX_BUILD_ID_CHARS,
        )?;

        let init = fixture.run_volicord([
            "init",
            "--shared",
            "--host",
            host,
            "--repo",
            fixture.repo_arg(),
            "--profile",
            profile.as_str(),
            "--home",
            fixture.runtime_home_arg(),
            "--json",
        ])?;
        assert_success("volicord init for live final-output smoke", &init);
        let init_json = json_stdout(&init)?;
        assert_live_init_reported_action_required(&init_json, host, profile, expected_host_action);
        let connection_id = bounded_identity(
            "Agent Connection id",
            init_json["connection"]["connection_id"]
                .as_str()
                .ok_or_else(|| io::Error::other("init result has no Agent Connection id"))?,
            MAX_CONNECTION_ID_CHARS,
        )?;
        let identity = LiveHostIdentity {
            host: host.to_owned(),
            host_version,
            volicord_build_id,
            connection_id,
        };
        let project_id = live_fixture_project_id(&fixture)?;
        let config_fixture =
            verify_final_output_config_fixture(&fixture, host, profile, &init_json)?;

        let no_active_private_prose =
            "private direct-wrapper model prose must not become authority";
        let direct_event = live_final_output_event(
            host,
            &fixture.repo_root,
            &format!(
                "live_final_output_direct_session_{}_{}",
                host.replace('-', "_"),
                profile.as_str()
            ),
            no_active_private_prose,
        )?;
        let before_direct = guard_observation_counts(&fixture, &project_id)?;
        let first_direct = run_generated_final_output_handler(
            &fixture.runtime_home_path,
            &fixture.repo_root,
            &fixture.env_path,
            host,
            &direct_event,
        )?;
        let first_wire = verify_no_active_status_wire(&first_direct, no_active_private_prose)?;
        let after_first_direct = guard_observation_counts(&fixture, &project_id)?;
        let second_direct = run_generated_final_output_handler(
            &fixture.runtime_home_path,
            &fixture.repo_root,
            &fixture.env_path,
            host,
            &direct_event,
        )?;
        let second_wire = verify_no_active_status_wire(&second_direct, no_active_private_prose)?;
        let after_second_direct = guard_observation_counts(&fixture, &project_id)?;
        if first_direct.stdout != second_direct.stdout {
            return Err(io::Error::other(
                "identical generated-wrapper payloads did not produce the same no-active-Task wire response",
            )
            .into());
        }
        match profile {
            IntegrationProfile::Record => {
                if before_direct != after_first_direct || after_first_direct != after_second_direct
                {
                    return Err(io::Error::other(
                        "Record generated final-output handling must not persist GuardEvent or AgentSession observations",
                    )
                    .into());
                }
            }
            IntegrationProfile::Detective => {
                if after_first_direct.guard_events != before_direct.guard_events + 1
                    || after_second_direct.guard_events != after_first_direct.guard_events
                {
                    return Err(io::Error::other(
                        "Detective identical-payload replay must preserve one immutable historical GuardEvent",
                    )
                    .into());
                }
            }
        }

        let before_actual_host = after_second_direct;
        let no_active_stop_cursor = stop_event_cursor(&fixture, &project_id)?;
        let prompt = concat!(
            "Use only the MCP server named `volicord` to call `volicord.status` once with `detail=full` and no task_id. ",
            "Confirm that it reports no active Task, then reply with exactly VOLICORD_LIVE_FINAL_OUTPUT_NO_ACTIVE_TASK and stop. ",
            "Do not call any other tool, shell command, or edit files."
        );
        println!(
            "\n=== Volicord live {host}/{} final-output smoke ===\nThis first authenticated host turn intentionally has no active Volicord Task. After the host answer, inspect the host-native final-output surface. Do not enter credentials into this test process.\n=== end instruction ===\n",
            profile.as_str()
        );
        let host_status = fixture.run_authenticated_interactive_host(&executable, prompt)?;
        if !host_status.success() {
            return Err(io::Error::other(format!(
                "the interactive {host} process exited unsuccessfully with {}",
                status_text(host_status)
            ))
            .into());
        }
        assert_live_connection_verified(&fixture, &identity.connection_id)?;
        confirm_final_output_ui(host, profile, FinalOutputUiExpectation::ManagedSurface)?;
        confirm_final_output_ui(
            host,
            profile,
            FinalOutputUiExpectation::NoActiveTaskStatus {
                complete_message: first_wire.system_message.clone(),
            },
        )?;
        let after_actual_host = guard_observation_counts(&fixture, &project_id)?;

        let no_active_actual_event = match profile {
            IntegrationProfile::Record => {
                if after_actual_host != before_actual_host {
                    return Err(io::Error::other(
                        "Record actual-host final-output handling persisted a GuardEvent or AgentSession observation",
                    )
                    .into());
                }
                serde_json::json!({
                    "status": "verified",
                    "source": "authenticated_host_owned_surface_delivery",
                    "delivery_evidence": "managed_final_output_ui",
                    "persistent_guard_event": false,
                    "non_observing": true
                })
            }
            IntegrationProfile::Detective => {
                if after_actual_host.guard_events <= before_actual_host.guard_events {
                    return Err(io::Error::other(
                        "Detective actual-host final-output handling did not persist a new Stop GuardEvent",
                    )
                    .into());
                }
                let historical = live_stop_decision_after(
                    &fixture,
                    &project_id,
                    &identity.connection_id,
                    no_active_stop_cursor,
                )?;
                if historical.decision != "allow" {
                    return Err(io::Error::other(format!(
                        "Detective no-active-Task Stop decision was {:?}, expected allow",
                        historical.decision
                    ))
                    .into());
                }
                serde_json::json!({
                    "status": "verified",
                    "source": "persisted_guard_event",
                    "guard_event_id": historical.guard_event_id,
                    "session_id": historical.session_id,
                    "decision": historical.decision
                })
            }
        };

        let authority_marker = format!(
            "VOLICORD_LIVE_FINAL_OUTPUT_AUTHORITY_{}_{}",
            host.replace('-', "_").to_ascii_uppercase(),
            profile.as_str().to_ascii_uppercase()
        );
        let prepared =
            prepare_live_final_authority(&fixture, &identity.connection_id, &authority_marker)?;
        let before_receipt_host = guard_observation_counts(&fixture, &project_id)?;
        let receipt_stop_cursor = stop_event_cursor(&fixture, &project_id)?;
        println!(
            "\n=== Volicord live {host}/{} AuthorityReceipt UI turn ===\nA disposable no-write advisor Task is active at initial state_version {}. Reply with exactly VOLICORD_LIVE_FINAL_OUTPUT_RECEIPT and stop without calling tools. Host lifecycle events may advance authority before Stop, so inspect and copy the complete current receipt from the separate managed final-output UI.\n=== end instruction ===\n",
            profile.as_str(),
            prepared.receipt.state_version
        );
        let receipt_host_status = fixture.run_authenticated_interactive_host(
            &executable,
            concat!(
                "Reply with exactly VOLICORD_LIVE_FINAL_OUTPUT_RECEIPT and then stop. ",
                "Do not call tools, MCP servers, shell commands, or edit files."
            ),
        )?;
        if !receipt_host_status.success() {
            return Err(io::Error::other(format!(
                "the AuthorityReceipt interactive {host} process exited unsuccessfully with {}",
                status_text(receipt_host_status)
            ))
            .into());
        }
        let prepared = read_live_final_authority(
            &fixture,
            &identity.connection_id,
            &prepared.observation.task_id,
            &prepared.change_unit_id,
            &authority_marker,
        )?;
        confirm_final_output_ui(host, profile, FinalOutputUiExpectation::ManagedSurface)?;
        confirm_final_output_ui(
            host,
            profile,
            FinalOutputUiExpectation::CompleteAuthorityReceipt {
                canonical_json: canonical_json_string(&prepared.receipt.canonical_receipt)?,
            },
        )?;
        let after_receipt_host = guard_observation_counts(&fixture, &project_id)?;
        let (receipt_actual_event, detective_decision) = match profile {
            IntegrationProfile::Record => {
                if after_receipt_host != before_receipt_host {
                    return Err(io::Error::other(
                        "Record actual-host AuthorityReceipt handling persisted a GuardEvent or AgentSession observation",
                    )
                    .into());
                }
                (
                    serde_json::json!({
                        "status": "verified",
                        "source": "authenticated_host_owned_surface_delivery",
                        "delivery_evidence": "managed_final_output_ui",
                        "persistent_guard_event": false,
                        "non_observing": true
                    }),
                    serde_json::json!({
                        "status": "not_applicable",
                        "non_observing": true,
                        "non_gating": true
                    }),
                )
            }
            IntegrationProfile::Detective => {
                if after_receipt_host.guard_events <= before_receipt_host.guard_events {
                    return Err(io::Error::other(
                        "Detective actual-host AuthorityReceipt handling did not persist a new Stop GuardEvent",
                    )
                    .into());
                }
                let stop = verify_live_stop_guard_event(
                    &fixture.runtime_home_path,
                    &identity.connection_id,
                    &prepared.observation,
                    &prepared.receipt,
                    receipt_stop_cursor,
                )?;
                (
                    serde_json::json!({
                        "status": "verified",
                        "source": "persisted_guard_event",
                        "guard_event_id": stop.guard_event_id,
                        "session_id": stop.session_id,
                        "decision": stop.decision
                    }),
                    serde_json::json!({
                        "status": "verified",
                        "historical_decision": {
                            "source": "persisted_guard_event",
                            "decision": stop.decision,
                            "receipt_state_version": stop.state_version,
                            "status": "verified"
                        },
                        "fresh_display": {
                            "source": "host_native_system_message",
                            "receipt_state_version": prepared.receipt.state_version,
                            "status": "verified"
                        },
                        "allow": { "status": "verified" },
                        "block": {
                            "status": "unavailable",
                            "reason": "the installed host exposes no safe authenticated block-only finalization entry point"
                        }
                    }),
                )
            }
        };

        let active_direct_session_id = format!(
            "live_final_output_active_direct_session_{}_{}",
            host.replace('-', "_"),
            profile.as_str()
        );
        let active_private_prose =
            "private active direct-wrapper model prose must not become authority";
        let active_direct_event = live_final_output_event(
            host,
            &fixture.repo_root,
            &active_direct_session_id,
            active_private_prose,
        )?;
        let before_active_direct = after_receipt_host;
        let first_active_direct = run_generated_final_output_handler(
            &fixture.runtime_home_path,
            &fixture.repo_root,
            &fixture.env_path,
            host,
            &active_direct_event,
        )?;
        let first_active_wire = verify_authority_receipt_wire(
            &first_active_direct,
            &prepared.receipt,
            true,
            active_private_prose,
        )?;
        let after_first_active_direct = guard_observation_counts(&fixture, &project_id)?;
        let first_direct_historical = if profile == IntegrationProfile::Detective {
            Some(stored_stop_snapshot_for_session(
                &fixture,
                &project_id,
                &active_direct_session_id,
            )?)
        } else {
            None
        };
        let replayed_authority = advance_live_final_authority(
            &fixture,
            &identity.connection_id,
            &prepared,
            &format!("{authority_marker}_REPLAY_REFRESH"),
        )?;
        if replayed_authority.receipt.state_version <= prepared.receipt.state_version {
            return Err(io::Error::other(
                "generated-wrapper replay fixture did not advance current authority state",
            )
            .into());
        }
        let second_active_direct = run_generated_final_output_handler(
            &fixture.runtime_home_path,
            &fixture.repo_root,
            &fixture.env_path,
            host,
            &active_direct_event,
        )?;
        let second_active_wire = verify_authority_receipt_wire(
            &second_active_direct,
            &replayed_authority.receipt,
            true,
            active_private_prose,
        )?;
        let after_second_active_direct = guard_observation_counts(&fixture, &project_id)?;
        if first_active_direct.stdout == second_active_direct.stdout {
            return Err(io::Error::other(
                "identical generated-wrapper payload replay did not refresh the current AuthorityReceipt after state advanced",
            )
            .into());
        }
        match profile {
            IntegrationProfile::Record => {
                if before_active_direct != after_first_active_direct
                    || after_first_active_direct != after_second_active_direct
                {
                    return Err(io::Error::other(
                        "Record active-Task direct-wire handling persisted GuardEvent or AgentSession observations",
                    )
                    .into());
                }
            }
            IntegrationProfile::Detective => {
                if after_first_active_direct.guard_events != before_active_direct.guard_events + 1
                    || after_second_active_direct.guard_events
                        != after_first_active_direct.guard_events
                {
                    return Err(io::Error::other(
                        "Detective active-Task identical replay did not preserve one immutable historical GuardEvent",
                    )
                    .into());
                }
                if stored_stop_snapshot_for_session(
                    &fixture,
                    &project_id,
                    &active_direct_session_id,
                )? != first_direct_historical
                    .expect("Detective replay should have a historical direct Stop snapshot")
                {
                    return Err(io::Error::other(
                        "Detective replay changed the immutable historical Stop decision payload",
                    )
                    .into());
                }
            }
        }
        let actual_host_event = serde_json::json!({
            "status": "verified",
            "status_fallback_event": no_active_actual_event,
            "authority_receipt_event": receipt_actual_event
        });

        let summary = serde_json::json!({
            "kind": LIVE_FINAL_OUTPUT_RESULT_KIND,
            "result": "incomplete",
            "host": {
                "kind": identity.host,
                "version": identity.host_version
            },
            "profile": profile.as_str(),
            "volicord": { "build_id": identity.volicord_build_id },
            "connection": { "connection_id": identity.connection_id },
            "evidence": {
                "config_fixture": config_fixture,
                "generated_wrapper_direct_wire": {
                    "status": "verified",
                    "status_fallback": {
                        "status": "verified",
                        "first_response_bytes": first_wire.response_bytes,
                        "second_response_bytes": second_wire.response_bytes,
                        "private_model_prose_absent": first_wire.private_model_prose_absent
                            && second_wire.private_model_prose_absent
                    },
                    "authority_receipt": {
                        "status": "verified",
                        "first_state_version": prepared.receipt.state_version,
                        "refreshed_state_version": replayed_authority.receipt.state_version,
                        "first_response_bytes": first_active_wire.response_bytes,
                        "second_response_bytes": second_active_wire.response_bytes,
                        "private_model_prose_absent": first_active_wire.private_model_prose_absent
                            && second_active_wire.private_model_prose_absent
                    },
                    "identical_payload_reinvoked": true,
                    "wire_responses_equal": false,
                    "current_receipt_refreshed_after_state_advance": true
                },
                "actual_host_event": actual_host_event,
                "actual_host_fixed_ui": {
                    "status": "verified",
                    "status_fallback": {
                        "status": "verified",
                        "operator_confirmed": true,
                        "complete_taskless_message_operator_confirmed": true
                    },
                    "authority_receipt": {
                        "status": "verified",
                        "operator_confirmed": true,
                        "complete_canonical_receipt_operator_confirmed": true,
                        "project_id": prepared.receipt.project_id,
                        "task_id": prepared.receipt.task_id,
                        "state_version": prepared.receipt.state_version,
                        "latest_run_id": prepared.receipt.latest_run_id,
                        "close_state": prepared.receipt.close_state,
                        "close_blocker_count": prepared.receipt.close_blocker_count
                    }
                },
                "detective_decision": detective_decision,
                "status_fallback": {
                    "status": "verified",
                    "no_active_task": true,
                    "generated_wire_command": "volicord status --json",
                    "operator_confirmed_actual_host_ui": true,
                    "complete_taskless_message_operator_confirmed": true,
                    "task_bound_command_absent": true
                },
                "exact_replay": {
                    "status": "unavailable",
                    "generated_wrapper_identical_payload": {
                        "status": "verified",
                        "state_advanced_between_deliveries": true,
                        "first_receipt_state_version": prepared.receipt.state_version,
                        "refreshed_receipt_state_version":
                            replayed_authority.receipt.state_version,
                        "fresh_current_receipt_displayed": true,
                        "record_non_observing_preserved":
                            profile == IntegrationProfile::Record,
                        "detective_historical_guard_event_preserved":
                            profile == IntegrationProfile::Detective
                    },
                    "actual_host_replay": {
                        "status": "unavailable",
                        "reason": "the installed host exposes no authenticated exact-replay entry point"
                    }
                }
            }
        });
        validate_final_output_result_shape(&summary, profile)?;
        result_recorder.record_final(&summary)?;
        Err(io::Error::other(format!(
            "{host}/{} final-output evidence is incomplete: the actual host exposes no authenticated exact-replay entry point{}",
            profile.as_str(),
            if profile == IntegrationProfile::Detective {
                " and no safe block-only finalization entry point"
            } else {
                ""
            }
        ))
        .into())
    }

    fn live_cli_fallback_round_trip(
        host: &str,
        executable_name: &str,
        selector_env: &str,
        expected_host_action: &str,
    ) -> Result<(), Box<dyn Error>> {
        if !smoke_enabled(selector_env) {
            return Err(io::Error::other(format!(
                "set {selector_env}=1 before running the ignored {host} CLI-fallback smoke test"
            ))
            .into());
        }
        let recorder_host = format!("{host}-cli-fallback");
        let mut result_recorder =
            LiveResultRecorder::from_env_for_kind(&recorder_host, LIVE_CLI_FALLBACK_RESULT_KIND)?;
        let executable = find_executable(executable_name).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("`{executable_name}` was not found on PATH"),
            )
        })?;
        if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
            return Err(io::Error::other(
                "authenticated live CLI-fallback validation requires interactive terminal stdin and stdout",
            )
            .into());
        }

        let fixture = LiveSmokeFixture::new(&recorder_host)?;
        let host_version_output = fixture.run_host_command(&executable, ["--version"])?;
        assert_success(
            &format!("{executable_name} --version"),
            &host_version_output,
        );
        let host_version = host_version_summary(&host_version_output)?;
        let volicord_build_id = bounded_identity(
            "Volicord build_id",
            &volicord_mcp::build_id(),
            MAX_BUILD_ID_CHARS,
        )?;
        let init = fixture.run_volicord([
            "init",
            "--shared",
            "--host",
            host,
            "--repo",
            fixture.repo_arg(),
            "--profile",
            "detective",
            "--home",
            fixture.runtime_home_arg(),
            "--json",
        ])?;
        assert_success("volicord init for live CLI-fallback smoke", &init);
        let init_json = json_stdout(&init)?;
        assert_live_init_reported_action_required(
            &init_json,
            host,
            IntegrationProfile::Detective,
            expected_host_action,
        );
        let connection_id = bounded_identity(
            "Agent Connection id",
            init_json["connection"]["connection_id"]
                .as_str()
                .ok_or_else(|| io::Error::other("init result has no Agent Connection id"))?,
            MAX_CONNECTION_ID_CHARS,
        )?;
        let identity = LiveHostIdentity {
            host: host.to_owned(),
            host_version,
            volicord_build_id,
            connection_id,
        };
        let marker = format!(
            "VOLICORD_LIVE_HOST_CLI_FALLBACK_ROUND_TRIP_{}",
            host.replace('-', "_").to_ascii_uppercase()
        );
        let prepared =
            prepare_live_cli_fallback_action(&fixture, &identity.connection_id, &marker)?;
        if prepared.observation.user_action_status.as_deref() != Some("pending") {
            return Err(io::Error::other(
                "the prepared CLI-fallback user action was not current and pending",
            )
            .into());
        }
        let user_action_request_id = prepared
            .observation
            .user_action_request_id
            .as_deref()
            .ok_or_else(|| io::Error::other("prepared CLI-fallback request id is missing"))?;
        let operator_choice_id = confirm_cli_fallback_choice(host, user_action_request_id)?;
        let cli_resolution =
            resolve_live_user_action_via_cli(&fixture, &prepared.observation, &operator_choice_id)?;
        let expected_run_marker = run_marker_for_selected_option(&operator_choice_id)
            .ok_or_else(|| io::Error::other("operator selected an unsupported fallback option"))?;
        let stop_cursor = stop_event_cursor(&fixture, &prepared.observation.project_id)?;
        let prompt = live_cli_fallback_resume_prompt(
            user_action_request_id,
            &prepared.observation.task_id,
            &prepared.change_unit_id,
        );
        println!(
            "\n=== Volicord live {host} CLI-fallback smoke ===\nThe pending choice was resolved by the human operator through the actual `volicord inbox resolve --json` User Channel. The installed host must now resume that exact request through the same Agent Connection, consume the selected option, record its mapped no-write Run, read fresh status, and stop. Approve the repository or MCP entry if the host asks. Do not type credentials or secrets.\n\n{prompt}\n=== end instruction ===\n"
        );
        let status = fixture.run_authenticated_interactive_host(&executable, &prompt)?;
        smoke_note(
            host,
            format!("interactive host exited with {}", status_text(status)),
        );
        if !status.success() {
            return Err(io::Error::other(format!(
                "the interactive {host} process exited unsuccessfully with {}",
                status_text(status)
            ))
            .into());
        }

        assert_live_connection_verified(&fixture, &identity.connection_id)?;
        let observation = inspect_live_user_action(&fixture, &marker)?
            .ok_or_else(|| io::Error::other("the prepared CLI-fallback Task disappeared"))?;
        if observation.user_action_request_id.as_deref() != Some(user_action_request_id)
            || observation.user_action_resolution_id.as_deref()
                != Some(cli_resolution.user_action_resolution_id.as_str())
            || observation.user_action_status.as_deref() != Some("resolved")
            || observation.selected_option_id.as_deref() != Some(operator_choice_id.as_str())
            || observation.resolved_by_actor_source.as_deref() != Some("local_user")
            || observation.resolved_verification_basis.as_deref()
                != Some(VERIFICATION_BASIS_CLI_DIRECT_USER_CHANNEL)
            || observation.resolved_channel_kind.as_deref() != Some("cli")
        {
            return Err(io::Error::other(
                "the stored CLI User Channel resolution does not match the prepared request and operator choice",
            )
            .into());
        }
        assert_single_live_product_decision_request(&fixture, &observation)?;

        let status_output = fixture.run_volicord([
            "status",
            "--repo",
            fixture.repo_arg(),
            "--task",
            &observation.task_id,
            "--json",
        ])?;
        assert_success("volicord status after live CLI fallback", &status_output);
        let receipt = verify_fresh_authority_receipt(
            json_stdout(&status_output)?,
            &observation,
            expected_run_marker,
        )?;
        let (latest_run, authority_event_order) =
            inspect_live_choice_consumption(&fixture, &observation, &receipt.latest_run_id)?;
        let expected_actor_source = format!("agent_connection:{}", identity.connection_id);
        if latest_run.kind != "shaping_update"
            || latest_run.summary != expected_run_marker
            || latest_run.product_file_write_observed
            || !latest_run.changed_paths.is_empty()
            || latest_run.created_by_actor_source != expected_actor_source
        {
            return Err(io::Error::other(
                "the resumed host did not record the option-mapped no-write shaping Run through the expected Agent Connection",
            )
            .into());
        }
        let stop_observation = verify_live_stop_guard_event(
            &fixture.runtime_home_path,
            &identity.connection_id,
            &observation,
            &receipt,
            stop_cursor,
        )?;
        assert_cli_fallback_resume_diagnostic(
            &fixture,
            &identity.connection_id,
            &observation.project_id,
        )?;
        if let Err(error) = confirm_final_output_ui(
            host,
            IntegrationProfile::Detective,
            FinalOutputUiExpectation::CompleteAuthorityReceipt {
                canonical_json: canonical_json_string(&receipt.canonical_receipt)?,
            },
        ) {
            result_recorder.record_final(&live_cli_fallback_completed_summary(
                LiveCliFallbackSummaryInput {
                    result: "failed_receipt_ui_confirmation",
                    identity: &identity,
                    observation: &observation,
                    operator_choice_id: &operator_choice_id,
                    cli_resolution: &cli_resolution,
                    latest_run: &latest_run,
                    authority_event_order: &authority_event_order,
                    stop_observation: &stop_observation,
                    receipt: &receipt,
                    stop_receipt_ui_confirmed: false,
                },
            ))?;
            return Err(error);
        }
        let summary = live_cli_fallback_completed_summary(LiveCliFallbackSummaryInput {
            result: "passed",
            identity: &identity,
            observation: &observation,
            operator_choice_id: &operator_choice_id,
            cli_resolution: &cli_resolution,
            latest_run: &latest_run,
            authority_event_order: &authority_event_order,
            stop_observation: &stop_observation,
            receipt: &receipt,
            stop_receipt_ui_confirmed: true,
        });
        validate_live_cli_fallback_result_shape(&summary)?;
        result_recorder.record_final(&summary)?;
        smoke_note(
            host,
            format!(
                "verified CLI fallback request {}, choice {}, exact CLI retry, same-connection resume, mapped Run {}, Task-bound Stop, and complete receipt UI",
                user_action_request_id, operator_choice_id, expected_run_marker
            ),
        );
        Ok(())
    }

    fn live_user_action_round_trip(
        host: &str,
        executable_name: &str,
        selector_env: &str,
        expected_host_action: &str,
    ) -> Result<(), Box<dyn Error>> {
        if !smoke_enabled(selector_env) {
            return Err(io::Error::other(format!(
                "set {selector_env}=1 before running the ignored {host} user-action smoke test"
            ))
            .into());
        }
        let mut result_recorder = LiveResultRecorder::from_env(host)?;
        let executable = find_executable(executable_name).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("`{executable_name}` was not found on PATH"),
            )
        })?;
        if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
            return Err(io::Error::other(
                "authenticated live Judgment validation requires interactive terminal stdin and stdout",
            )
            .into());
        }
        let fixture = LiveSmokeFixture::new(&format!("{host}-user-action"))?;
        let host_version_output = fixture.run_host_command(&executable, ["--version"])?;
        assert_success(
            &format!("{executable_name} --version"),
            &host_version_output,
        );
        let host_version = host_version_summary(&host_version_output)?;
        let volicord_build_id = bounded_identity(
            "Volicord build_id",
            &volicord_mcp::build_id(),
            MAX_BUILD_ID_CHARS,
        )?;
        let init = fixture.run_volicord([
            "init",
            "--shared",
            "--host",
            host,
            "--repo",
            fixture.repo_arg(),
            "--profile",
            "detective",
            "--home",
            fixture.runtime_home_arg(),
            "--json",
        ])?;
        assert_success("volicord init for live user-action smoke", &init);
        let init_json = json_stdout(&init)?;
        assert_live_init_reported_action_required(
            &init_json,
            host,
            IntegrationProfile::Detective,
            expected_host_action,
        );
        let connection_id = bounded_identity(
            "Agent Connection id",
            init_json["connection"]["connection_id"]
                .as_str()
                .ok_or_else(|| io::Error::other("init result has no Agent Connection id"))?,
            MAX_CONNECTION_ID_CHARS,
        )?;
        let identity = LiveHostIdentity {
            host: host.to_owned(),
            host_version,
            volicord_build_id,
            connection_id,
        };
        smoke_note(
            host,
            format!(
                "release identity host_version={:?}, volicord_build_id={:?}, connection_id={:?}",
                identity.host_version, identity.volicord_build_id, identity.connection_id
            ),
        );

        let marker = format!(
            "VOLICORD_LIVE_HOST_USER_ACTION_ROUND_TRIP_{}",
            host.replace('-', "_").to_ascii_uppercase()
        );
        let prompt = live_user_action_prompt(&marker);
        let project_id = live_fixture_project_id(&fixture)?;
        let judgment_stop_cursor = stop_event_cursor(&fixture, &project_id)?;
        println!(
            "\n=== Volicord live {host} user-action smoke ===\nThe host will receive this initial instruction and may ask you to trust the repository or approve its MCP server. When the host-native user-action selector appears, choose one option yourself. Do not type credentials or secrets. Exit the host after it reports the final Volicord status.\n\n{prompt}\n=== end instruction ===\n"
        );
        let status = fixture.run_authenticated_interactive_host(&executable, &prompt)?;
        smoke_note(
            host,
            format!("interactive host exited with {}", status_text(status)),
        );
        if !status.success() {
            return Err(io::Error::other(format!(
                "the interactive {host} process exited unsuccessfully with {}",
                status_text(status)
            ))
            .into());
        }

        let observation = inspect_live_user_action(&fixture, &marker)?;
        let Some(observation) = observation else {
            return Err(io::Error::other(format!(
                "the live host did not create the marker Task `{marker}`; rerun the smoke, approve the generated Volicord MCP connection, and let the host complete the instructed intake call"
            ))
            .into());
        };
        if observation.user_action_request_id.is_none() {
            return Err(io::Error::other(format!(
                "Task `{}` was created but no product-decision user action was created; rerun the smoke and let the host complete `volicord.request_user_action`",
                observation.task_id
            ))
            .into());
        }
        if observation.user_action_status.as_deref() != Some("resolved") {
            let fallback = verify_ephemeral_inbox_fallback_shape(&fixture, &observation)?;
            result_recorder.record_final(&live_host_fallback_summary(
                &identity,
                &observation,
                &fallback,
            ))?;
            return Err(io::Error::other(format!(
                "host-native MCP elicitation was unavailable, so user action `{}` remains pending; CLI fallback command shape was verified only inside the disposable fixture",
                observation.user_action_request_id.as_deref().unwrap_or("unknown")
            ))
            .into());
        }

        assert_eq!(
            observation.resolved_by_actor_source.as_deref(),
            Some("local_user"),
            "resolved user action must be owned by the local user"
        );
        assert_eq!(
            observation.resolved_verification_basis.as_deref(),
            Some(VERIFICATION_BASIS_MCP_ELICITATION_USER_CHANNEL),
            "the live round trip must use the host-native MCP User Channel"
        );
        assert_eq!(
            observation.option_ids.len(),
            2,
            "the live user action must preserve exactly the two requested route options"
        );
        assert!(
            observation
                .option_ids
                .iter()
                .any(|option_id| option_id == USER_ACTION_ROUTE_ALPHA_OPTION_ID),
            "the live user action is missing the alpha route option"
        );
        assert!(
            observation
                .option_ids
                .iter()
                .any(|option_id| option_id == USER_ACTION_ROUTE_BETA_OPTION_ID),
            "the live user action is missing the beta route option"
        );
        let operator_choice_id = confirm_native_user_action_choice(host)?;
        let selected_option_id = observation
            .selected_option_id
            .as_deref()
            .expect("a resolved live user action must store selected_option_id");
        if operator_choice_id != selected_option_id {
            result_recorder.record_final(&live_host_choice_mismatch_summary(
                &identity,
                &observation,
                &operator_choice_id,
                selected_option_id,
            ))?;
            return Err(io::Error::other(format!(
                "the operator confirmed choice {operator_choice_id:?}, but Volicord stored {selected_option_id:?}"
            ))
            .into());
        }
        let expected_run_marker = run_marker_for_selected_option(selected_option_id)
            .unwrap_or_else(|| panic!("unexpected live user-action option {selected_option_id:?}"));

        let status_output = fixture.run_volicord([
            "status",
            "--repo",
            fixture.repo_arg(),
            "--task",
            &observation.task_id,
            "--json",
        ])?;
        assert_success("volicord status after live user action", &status_output);
        let status_json = json_stdout(&status_output)?;
        let receipt =
            verify_fresh_authority_receipt(status_json, &observation, expected_run_marker)?;
        let (latest_run, authority_event_order) =
            inspect_live_choice_consumption(&fixture, &observation, &receipt.latest_run_id)?;
        assert_eq!(
            latest_run.kind, "shaping_update",
            "the choice-consumption marker must use the no-write shaping Run branch"
        );
        assert_eq!(
            latest_run.summary, expected_run_marker,
            "the recorded Run marker must match the user's selected option"
        );
        assert!(
            !latest_run.product_file_write_observed,
            "the choice-consumption Run must not report a Product Repository write"
        );
        assert!(
            latest_run.changed_paths.is_empty(),
            "the choice-consumption Run must not report changed Product Repository paths"
        );
        assert!(
            observation.state_version >= 5,
            "intake, Change Unit creation, user-action creation, User Channel resolution, and the choice-consumption Run must advance Task state"
        );
        assert_ne!(
            observation.lifecycle_phase, "waiting_user",
            "a resolved sole user action must leave the Task out of waiting_user"
        );
        let stop_observation = verify_live_stop_guard_event(
            &fixture.runtime_home_path,
            &identity.connection_id,
            &observation,
            &receipt,
            judgment_stop_cursor,
        )?;
        assert_native_channel_diagnostic(
            &fixture,
            &identity.connection_id,
            &observation.project_id,
        )?;
        if let Err(error) = confirm_final_output_ui(
            host,
            IntegrationProfile::Detective,
            FinalOutputUiExpectation::CompleteAuthorityReceipt {
                canonical_json: canonical_json_string(&receipt.canonical_receipt)?,
            },
        ) {
            result_recorder.record_final(&live_host_completed_summary(
                LiveCompletedSummaryInput {
                    result: "failed_receipt_ui_confirmation",
                    identity: &identity,
                    observation: &observation,
                    operator_choice_id: &operator_choice_id,
                    selected_option_id,
                    latest_run: &latest_run,
                    authority_event_order: &authority_event_order,
                    stop_observation: &stop_observation,
                    receipt: &receipt,
                    stop_receipt_ui_confirmed: false,
                },
            ))?;
            return Err(error);
        }
        result_recorder.record_final(&live_host_completed_summary(LiveCompletedSummaryInput {
            result: "passed",
            identity: &identity,
            observation: &observation,
            operator_choice_id: &operator_choice_id,
            selected_option_id,
            latest_run: &latest_run,
            authority_event_order: &authority_event_order,
            stop_observation: &stop_observation,
            receipt: &receipt,
            stop_receipt_ui_confirmed: true,
        }))?;
        smoke_note(
            host,
            format!(
                "verified user action {}, selected option {}, consumed marker {}, User Channel basis {}, Task phase {}, state_version {}, Stop systemMessage receipt UI confirmed",
                observation.user_action_request_id.as_deref().unwrap_or("unknown"),
                selected_option_id,
                expected_run_marker,
                VERIFICATION_BASIS_MCP_ELICITATION_USER_CHANNEL,
                observation.lifecycle_phase,
                receipt.state_version
            ),
        );
        Ok(())
    }

    fn live_user_action_prompt(marker: &str) -> String {
        format!(
            concat!(
                "Run a human-in-the-loop Volicord connection smoke using the MCP server named `volicord`. ",
                "Do not edit files, run shell commands, prepare a write, or answer on the user's behalf.\n\n",
                "1. Call `volicord.intake` with `detail=full`, `requested_mode=advisor`, `acceptance_policy=null`, and create-new resume behavior. The plain-language request must be exactly `{task_marker}`. Use a narrow no-write initial scope and exactly one acceptance criterion whose `evidence_requirement=not_required`. Retain the returned Task ID.\n",
                "2. For that Task, call `volicord.update_scope` with `detail=full`, `baseline_ref={baseline_ref}`, and a `change_unit` whose `operation=create_current`, `scope_summary` describes this no-write live-host user-action validation, and `affected_paths=[]`. Retain `state.active_change_unit_ref.record_id` and `state.baseline_ref`. Do not continue unless both are present.\n",
                "3. Call `volicord.request_user_action` with `request.operation=create`, `request.action.action_type=choice`, `request.action.judgment_kind=product_decision`, and omit `detail` so the default compact projection is exercised. Ask which live-smoke route the agent must consume, make it required for `close_complete`, and provide exactly these two caller-authored options in this order:\n",
                "   - `option_id={alpha_option_id}`, label `Route alpha`, description `Select the alpha live-smoke route.`, consequence `The agent records the alpha choice-consumption Run marker.`, `is_default=false`.\n",
                "   - `option_id={beta_option_id}`, label `Route beta`, description `Select the beta live-smoke route.`, consequence `The agent records the beta choice-consumption Run marker.`, `is_default=false`.\n",
                "4. Wait for the host's native MCP elicitation/User Channel UI. The human running this smoke will choose the answer. Never infer, fabricate, or submit that answer yourself.\n",
                "5. After Volicord reports the user action resolved, consume `structuredContent.method_result.resolution_summary.selected_option_id` from that default compact result. If it is `{alpha_option_id}`, call `volicord.record_run` with summary exactly `{alpha_run_marker}`. If it is `{beta_option_id}`, call `volicord.record_run` with summary exactly `{beta_run_marker}`. Use the retained Task ID, Change Unit ID, and baseline ref; set `kind=shaping_update`, `run_id=null`, `write_ticket_id=null`, `artifact_inputs=[]`, `evidence_updates=[]`, and `evidence_observations=[]`; report `changed_paths=[]`, `product_file_write_observed=false`, `sensitive_categories=[]`, and the same baseline ref in `observed_changes`. Supply a non-null `close_assessment` whose `result_summary` is exactly the selected Run marker and whose `result_refs`, `residual_risks`, `sensitive_categories`, and `recovery_constraints` are all empty arrays. Do not record a Run if the selected option is absent or unrecognized.\n",
                "6. After that Run is recorded, call `volicord.status` for the Task and report the selected option ID, exact Run marker, lifecycle phase, close state, close-blocker count, and state version. Then stop.\n\n",
                "If a native prompt does not appear and Volicord returns only a pending `user_action_request_summary`, do not simulate a resolution or execute a fallback command. Report that the CLI User Channel is required and stop so the disposable harness can verify the trusted CLI inbox and resolve-command shape."
            ),
            task_marker = marker,
            baseline_ref = LIVE_HOST_BASELINE_REF,
            alpha_option_id = USER_ACTION_ROUTE_ALPHA_OPTION_ID,
            beta_option_id = USER_ACTION_ROUTE_BETA_OPTION_ID,
            alpha_run_marker = USER_ACTION_ROUTE_ALPHA_RUN_MARKER,
            beta_run_marker = USER_ACTION_ROUTE_BETA_RUN_MARKER,
        )
    }

    fn live_evidence_observation_round_trip(
        host: &str,
        executable_name: &str,
        selector_env: &str,
        expected_host_action: &str,
    ) -> Result<(), Box<dyn Error>> {
        if !smoke_enabled(selector_env) {
            return Err(io::Error::other(format!(
                "set {selector_env}=1 before running the ignored {host} evidence-observation smoke test"
            ))
            .into());
        }

        // The external result path is deliberately acquired before executable and
        // terminal checks so every selected live cell has one bounded run record.
        let mut result_recorder =
            LiveResultRecorder::from_env_for_kind(host, LIVE_EVIDENCE_OBSERVATION_RESULT_KIND)?;
        let mut stage = "preflight";
        let outcome = live_evidence_observation_round_trip_inner(
            host,
            executable_name,
            expected_host_action,
            &mut stage,
        );
        match outcome {
            Ok(summary) => {
                stage = "result_validation";
                if let Err(error) = validate_live_evidence_observation_result_shape(&summary) {
                    let incomplete = live_evidence_observation_incomplete_summary(host, stage);
                    validate_live_evidence_observation_incomplete_result_shape(&incomplete)?;
                    result_recorder.record_final(&incomplete)?;
                    return Err(error);
                }
                result_recorder.record_final(&summary)
            }
            Err(error) => {
                let incomplete = live_evidence_observation_incomplete_summary(host, stage);
                validate_live_evidence_observation_incomplete_result_shape(&incomplete)?;
                result_recorder.record_final(&incomplete)?;
                Err(error)
            }
        }
    }

    fn live_evidence_observation_round_trip_inner(
        host: &str,
        executable_name: &str,
        expected_host_action: &str,
        stage: &mut &'static str,
    ) -> Result<Value, Box<dyn Error>> {
        *stage = "host_executable";
        let executable = find_executable(executable_name).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("`{executable_name}` was not found on PATH"),
            )
        })?;
        *stage = "interactive_terminal";
        if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
            return Err(io::Error::other(
                "authenticated live evidence-observation validation requires interactive terminal stdin and stdout",
            )
            .into());
        }

        *stage = "fixture_setup";
        let fixture = LiveSmokeFixture::new(&format!("{host}-evidence-observation"))?;
        let host_version_output = fixture.run_host_command(&executable, ["--version"])?;
        require_success(
            &format!("{executable_name} --version"),
            &host_version_output,
        )?;
        let host_version = host_version_summary(&host_version_output)?;
        let volicord_build_id = bounded_identity(
            "Volicord build_id",
            &volicord_mcp::build_id(),
            MAX_BUILD_ID_CHARS,
        )?;
        let init = fixture.run_volicord([
            "init",
            "--shared",
            "--host",
            host,
            "--repo",
            fixture.repo_arg(),
            "--profile",
            "detective",
            "--home",
            fixture.runtime_home_arg(),
            "--json",
        ])?;
        require_success("volicord init for live evidence-observation smoke", &init)?;
        let init_json = json_stdout(&init)?;
        require_live_init_reported_action_required(
            &init_json,
            host,
            IntegrationProfile::Detective,
            expected_host_action,
        )?;
        let connection_id = bounded_identity(
            "Agent Connection id",
            init_json["connection"]["connection_id"]
                .as_str()
                .ok_or_else(|| io::Error::other("init result has no Agent Connection id"))?,
            MAX_CONNECTION_ID_CHARS,
        )?;
        let identity = LiveHostIdentity {
            host: host.to_owned(),
            host_version,
            volicord_build_id,
            connection_id,
        };
        let marker = format!(
            "VOLICORD_LIVE_HOST_EVIDENCE_OBSERVATION_{}",
            host.replace('-', "_").to_ascii_uppercase()
        );
        let prepared = prepare_live_evidence_observation_authority(
            &fixture,
            &identity.connection_id,
            &marker,
        )?;
        let stop_cursor = stop_event_cursor(&fixture, &prepared.project_id)?;
        let prompt = live_evidence_observation_prompt(&prepared);
        println!(
            "\n=== Volicord live {host} evidence-observation smoke ===\nThis cell is valid only if the MCP client negotiated `capabilities.experimental[\"{MODEL_INVISIBLE_USER_CHANNEL_CAPABILITY_NAMESPACE}\"].{MODEL_INVISIBLE_USER_CHANNEL_CAPABILITY_FIELD}=true` and the host consumes `CallToolResult._meta[\"{MODEL_INVISIBLE_USER_CHANNEL_CAPABILITY_NAMESPACE}\"]` in a separate host-owned model-invisible surface. That `_meta` handoff must never enter chat or model output. Never ask the agent to relay, quote, print, or open the URL. If that separate surface appears, use it yourself to select the sole target and artifact, choose `supported`, enter a non-secret one-line summary, and submit. If it does not appear—or if any URL, token, form, question, or request ref appears in returned tool-result content, returned structured output, or host diagnostic text—tell the host to stop without fallback; the harness will record this cell unavailable. Do not enter credentials, secrets, tokens, or private keys.\n\n{prompt}\n=== end instruction ===\n"
        );

        *stage = "host_process";
        let status =
            fixture.run_authenticated_interactive_host_with_local_web(&executable, &prompt)?;
        smoke_note(
            host,
            format!("interactive host exited with {}", status_text(status)),
        );
        if !status.success() {
            return Err(io::Error::other(format!(
                "the interactive {host} process exited unsuccessfully with {}",
                status_text(status)
            ))
            .into());
        }

        *stage = "host_delivery_boundary";
        let delivery_boundary = confirm_local_web_delivery_boundary(host)?;

        *stage = "stored_resolution";
        assert_live_connection_verified(&fixture, &identity.connection_id)?;
        let observation =
            inspect_live_evidence_observation(&fixture, &prepared, &identity.connection_id)?;
        let operator_summary = confirm_live_evidence_summary(host)?;
        if operator_summary != observation.summary {
            return Err(io::Error::other(
                "the operator-confirmed evidence summary does not equal the stored local-web resolution summary",
            )
            .into());
        }
        let operator_summary_character_count = operator_summary.chars().count();
        drop(operator_summary);

        *stage = "authority_receipt";
        let task_observation = observation.task_observation();
        let status_output = fixture.run_volicord([
            "status",
            "--repo",
            fixture.repo_arg(),
            "--task",
            &observation.task_id,
            "--json",
        ])?;
        require_success(
            "volicord status after live evidence observation",
            &status_output,
        )?;
        let receipt = verify_fresh_authority_receipt(
            json_stdout(&status_output)?,
            &task_observation,
            LIVE_EVIDENCE_OBSERVATION_RUN_MARKER,
        )?;
        let (consumption, authority_event_order) = inspect_live_evidence_consumption(
            &fixture,
            &observation,
            &receipt.latest_run_id,
            &identity.connection_id,
        )?;

        *stage = "stop_and_diagnostics";
        let stop_observation = verify_live_stop_guard_event(
            &fixture.runtime_home_path,
            &identity.connection_id,
            &task_observation,
            &receipt,
            stop_cursor,
        )?;
        let diagnostic = assert_local_web_evidence_diagnostic(
            &fixture,
            &identity.connection_id,
            &observation.project_id,
        )?;

        *stage = "managed_receipt_ui";
        confirm_final_output_ui(
            host,
            IntegrationProfile::Detective,
            FinalOutputUiExpectation::CompleteAuthorityReceipt {
                canonical_json: canonical_json_string(&receipt.canonical_receipt)?,
            },
        )?;

        *stage = "completed";
        Ok(live_evidence_observation_completed_summary(
            LiveEvidenceCompletedSummaryInput {
                identity: &identity,
                observation: &observation,
                delivery_boundary: &delivery_boundary,
                operator_summary_character_count,
                consumption: &consumption,
                diagnostic: &diagnostic,
                diagnostic_payload_scan_passed: assert_live_evidence_diagnostic_payload_absence(
                    &fixture,
                    &observation,
                )?,
                authority_event_order: &authority_event_order,
                stop_observation: &stop_observation,
                receipt: &receipt,
            },
        ))
    }

    struct PreparedEvidenceObservation {
        project_id: String,
        task_id: String,
        change_unit_id: String,
        target: EvidenceTarget,
        artifact_ref: ArtifactRef,
    }

    fn prepare_live_evidence_observation_authority(
        fixture: &LiveSmokeFixture,
        connection_id: &str,
        marker: &str,
    ) -> Result<PreparedEvidenceObservation, Box<dyn Error>> {
        let context = McpConnectionContext::resolve(&fixture.runtime_home_path, connection_id)?
            .with_invocation_binding_basis(VERIFICATION_BASIS_TEST_FIXTURE_BINDING);
        let adapter = McpAdapter::new(&fixture.runtime_home_path, context);
        let intake = adapter.call_tool(
            "volicord.intake",
            serde_json::json!({
                "detail": "full",
                "plain_language_request": marker,
                "requested_mode": "advisor",
                "resume_policy": "create_new",
                "acceptance_policy": null,
                "lineage": null,
                "initial_scope": {
                    "boundary": "Validate one user-owned evidence observation without Product Repository writes.",
                    "non_goals": [],
                    "acceptance_criteria": [{
                        "statement": "The exact registered fixture bytes are supported by a user-owned observation.",
                        "evidence_requirement": "required"
                    }]
                }
            }),
        )?;
        if intake.response_value["base"]["response_kind"] != "result" {
            return Err(
                io::Error::other("evidence-observation setup intake was not committed").into(),
            );
        }
        let task_id = intake.response_value["task_ref"]["record_id"]
            .as_str()
            .ok_or_else(|| io::Error::other("evidence setup intake returned no Task id"))?
            .to_owned();
        let scope = adapter.call_tool(
            "volicord.update_scope",
            serde_json::json!({
                "detail": "full",
                "task_id": task_id,
                "goal_summary": null,
                "scope_update": null,
                "scope_boundary": null,
                "non_goals": null,
                "acceptance_criteria": null,
                "autonomy_boundary": null,
                "baseline_ref": LIVE_EVIDENCE_OBSERVATION_BASELINE_REF,
                "change_unit": {
                    "operation": "create_current",
                    "scope_summary": "No-write live-host local consent evidence-observation validation.",
                    "affected_paths": []
                },
                "related_scope_decision_refs": []
            }),
        )?;
        let change_unit_id = scope.response_value["state"]["active_change_unit_ref"]["record_id"]
            .as_str()
            .ok_or_else(|| io::Error::other("evidence setup returned no Change Unit id"))?
            .to_owned();
        let criterion_id = scope.response_value["state"]["acceptance_criteria"]
            .as_array()
            .filter(|criteria| criteria.len() == 1)
            .and_then(|criteria| criteria[0]["acceptance_criterion_id"].as_str())
            .ok_or_else(|| {
                io::Error::other("evidence setup did not preserve exactly one criterion")
            })?
            .to_owned();
        let target: EvidenceTarget = serde_json::from_value(serde_json::json!({
            "target_kind": "acceptance_criterion",
            "acceptance_criterion_id": criterion_id
        }))?;
        let staged = adapter.call_tool(
            "volicord.stage_artifact",
            serde_json::json!({
                "detail": "full",
                "task_id": task_id,
                "display_name": LIVE_EVIDENCE_ARTIFACT_DISPLAY_NAME,
                "content_type": "text/plain",
                "redaction_state": "none",
                "safe_bytes_or_notice": LIVE_EVIDENCE_ARTIFACT_BYTES
            }),
        )?;
        if staged.response_value["base"]["response_kind"] != "result" {
            return Err(io::Error::other(
                "evidence setup did not stage the deterministic safe artifact",
            )
            .into());
        }
        let staged_handle = staged.response_value["staged_artifact_handle"].clone();
        let recorded = adapter.call_tool(
            "volicord.record_run",
            serde_json::json!({
                "detail": "full",
                "task_id": task_id,
                "change_unit_id": change_unit_id,
                "kind": "shaping_update",
                "run_id": null,
                "baseline_ref": LIVE_EVIDENCE_OBSERVATION_BASELINE_REF,
                "write_ticket_id": null,
                "summary": "Register exact bytes for the live evidence-observation User Channel.",
                "observed_changes": {
                    "changed_paths": [],
                    "product_file_write_observed": false,
                    "sensitive_categories": [],
                    "baseline_ref": LIVE_EVIDENCE_OBSERVATION_BASELINE_REF
                },
                "artifact_inputs": [{
                    "artifact_input_id": "artifact_input_live_evidence_observation",
                    "source_kind": "staged_artifact",
                    "staged_artifact_handle": staged_handle,
                    "existing_artifact_ref": null,
                    "relation_hint": LIVE_EVIDENCE_RELATION_HINT,
                    "evidence_target": target,
                    "expected_sha256": null,
                    "expected_size_bytes": null,
                    "redaction_state": "none"
                }],
                "evidence_updates": [],
                "evidence_observations": [],
                "close_assessment": null
            }),
        )?;
        if recorded.response_value["base"]["response_kind"] != "result"
            || recorded.response_value["run_summary"]["kind"] != "shaping_update"
            || recorded.response_value["run_summary"]["observed_changes"]
                ["product_file_write_observed"]
                != false
            || recorded.response_value["run_summary"]["observed_changes"]["changed_paths"]
                != serde_json::json!([])
        {
            return Err(io::Error::other(
                "evidence setup did not record the exact no-write shaping Run",
            )
            .into());
        }
        let artifact_ref: ArtifactRef = recorded.response_value["registered_artifacts"]
            .as_array()
            .filter(|artifacts| artifacts.len() == 1)
            .and_then(|artifacts| artifacts.first())
            .cloned()
            .ok_or_else(|| io::Error::other("evidence setup did not register exactly one artifact"))
            .and_then(|value| serde_json::from_value(value).map_err(io::Error::other))?;
        let project_id = live_fixture_project_id(fixture)?;
        let created_by_run_ref = artifact_ref.created_by_run_ref.as_ref().ok_or_else(|| {
            io::Error::other("registered evidence artifact has no creating Run ref")
        })?;
        if artifact_ref.project_id.as_str() != project_id
            || artifact_ref.task_id.as_str() != task_id
            || serde_json::to_value(artifact_ref.integrity_status)? != "verified"
            || serde_json::to_value(artifact_ref.availability)? != "available"
            || serde_json::to_value(artifact_ref.redaction_state)? != "none"
            || created_by_run_ref.record_kind != StateRecordKind::Run
            || artifact_ref
                .created_by_actor_source
                .as_ref()
                .map(|actor| actor.to_canonical_string())
                != Some(format!("agent_connection:{connection_id}"))
        {
            return Err(io::Error::other(
                "registered evidence artifact is not the exact verified available setup artifact",
            )
            .into());
        }
        Ok(PreparedEvidenceObservation {
            project_id,
            task_id,
            change_unit_id,
            target,
            artifact_ref,
        })
    }

    fn live_evidence_observation_prompt(prepared: &PreparedEvidenceObservation) -> String {
        let target = serde_json::to_string(&prepared.target)
            .expect("prepared evidence target must serialize");
        format!(
            concat!(
                "Continue the prepared Volicord evidence-observation validation using the MCP server named `volicord`. ",
                "Do not edit files, run shell commands, prepare a write, inspect or relay host-only metadata, open any consent URL, or answer on the user's behalf. The fixture contains no real credential or secret; its display metadata contains a routing marker solely so Volicord requires a user-only local consent page.\n\n",
                "1. Call `volicord.request_user_action` exactly once with `detail=full` and nested `request.operation=create`. Use `task_id={task_id}`, `change_unit_id={change_unit_id}`, `request.action.action_type=evidence_observation`, question `{question}`, context summary `{context_summary}`, `target_candidates=[{target}]`, `artifact_candidate_ids=[\"{artifact_id}\"]`, `required_for=[\"record_run\"]`, and `expires_at=null`. Do not create any other user action.\n",
                "2. The returned public result must expose only `user_action_request_summary` with the exact keys `user_action_request_id`, `status=pending`, and `next_actor=user`; it must not expose a full request, request ref, inbox item, form, question, URL, token, command, or channel availability. The local-web handoff exists only in host-consumed top-level `CallToolResult._meta[\"{capability_namespace}\"]` and is owned entirely by a separate host model-invisible surface. Do not inspect, print, quote, relay, or open it. If the host does not render that separate surface, report only that the required User Channel is unavailable and stop without elicitation, prompt capture, or CLI fallback.\n",
                "3. Wait until the operator confirms completion without pasting any URL, token, form value, or observation summary into chat. Then call `volicord.request_user_action` exactly once with the same request ID and nested `request.operation=resume`; never use create again. Require `agent_workflow_result_replayed=true`, `current_status=resolved`, a non-null `user_action_resolution_ref`, and an evidence-observation resolution summary whose target equals `{target}`, whose sole artifact has ID `{artifact_id}`, and whose relevance is `supported`. Do not record a Run if any fact differs.\n",
                "4. Consume that exact resolution in one `volicord.record_run` call. Use `task_id={task_id}`, `change_unit_id={change_unit_id}`, `kind=shaping_update`, `run_id=null`, `baseline_ref={baseline_ref}`, `write_ticket_id=null`, summary exactly `{run_marker}`, no product-file changes, `artifact_inputs=[]`, and one supported evidence update for the resolved target with the exact resolved ArtifactRef. Add exactly one evidence observation for that target with `source_kind=user_observation`, `assurance_level=user_observed`, null observer/tool fields, empty tool metadata/source refs/limitations, `input_refs` containing only the exact resolution ref, `output_artifact_refs` containing only the exact resolved ArtifactRef, and `observed_at={caller_observed_at}`. Supply a close assessment with result summary exactly `{run_marker}` and empty result refs, risks, sensitive categories, and recovery constraints.\n",
                "5. Call `volicord.status` for Task `{task_id}`. Report only the request ID from the safe summary, resolution ID, Run ID, evidence observation ID, lifecycle phase, close state, blocker count, and state version. Do not repeat a request ref, form, question, URL, token, user summary, this prompt, or a transcript. Then stop."
            ),
            task_id = prepared.task_id,
            change_unit_id = prepared.change_unit_id,
            question = LIVE_EVIDENCE_REQUEST_QUESTION,
            context_summary = LIVE_EVIDENCE_REQUEST_CONTEXT,
            target = target,
            artifact_id = prepared.artifact_ref.artifact_id.as_str(),
            baseline_ref = LIVE_EVIDENCE_OBSERVATION_BASELINE_REF,
            run_marker = LIVE_EVIDENCE_OBSERVATION_RUN_MARKER,
            caller_observed_at = LIVE_EVIDENCE_CALLER_OBSERVED_AT,
            capability_namespace = MODEL_INVISIBLE_USER_CHANNEL_CAPABILITY_NAMESPACE,
        )
    }

    struct PreparedCliFallbackAction {
        observation: LiveUserActionObservation,
        change_unit_id: String,
    }

    fn prepare_live_cli_fallback_action(
        fixture: &LiveSmokeFixture,
        connection_id: &str,
        marker: &str,
    ) -> Result<PreparedCliFallbackAction, Box<dyn Error>> {
        let context = McpConnectionContext::resolve(&fixture.runtime_home_path, connection_id)?
            .with_invocation_binding_basis(VERIFICATION_BASIS_TEST_FIXTURE_BINDING);
        let adapter = McpAdapter::new(&fixture.runtime_home_path, context);
        let intake = adapter.call_tool(
            "volicord.intake",
            serde_json::json!({
                "detail": "full",
                "plain_language_request": marker,
                "requested_mode": "advisor",
                "resume_policy": "create_new",
                "acceptance_policy": null,
                "lineage": null,
                "initial_scope": {
                    "boundary": "Validate one no-write live-host CLI User Channel fallback.",
                    "non_goals": [],
                    "acceptance_criteria": [{
                        "statement": "The resumed host records the selected no-write route.",
                        "evidence_requirement": "not_required"
                    }]
                }
            }),
        )?;
        if intake.response_value["base"]["response_kind"] != "result" {
            return Err(io::Error::other("CLI-fallback setup intake was not committed").into());
        }
        let task_id = intake.response_value["task_ref"]["record_id"]
            .as_str()
            .ok_or_else(|| io::Error::other("CLI-fallback setup intake returned no Task id"))?
            .to_owned();
        let scope = adapter.call_tool(
            "volicord.update_scope",
            serde_json::json!({
                "detail": "full",
                "task_id": task_id,
                "goal_summary": null,
                "scope_update": null,
                "scope_boundary": null,
                "non_goals": null,
                "acceptance_criteria": null,
                "autonomy_boundary": null,
                "baseline_ref": LIVE_CLI_FALLBACK_BASELINE_REF,
                "change_unit": {
                    "operation": "create_current",
                    "scope_summary": "No-write live-host CLI User Channel fallback validation.",
                    "affected_paths": []
                },
                "related_scope_decision_refs": []
            }),
        )?;
        let change_unit_id = scope.response_value["state"]["active_change_unit_ref"]["record_id"]
            .as_str()
            .ok_or_else(|| {
                io::Error::other("CLI-fallback setup update_scope returned no Change Unit id")
            })?
            .to_owned();
        let scope_state_version = scope.response_value["base"]["state_version"]
            .as_u64()
            .ok_or_else(|| {
                io::Error::other("CLI-fallback setup update_scope returned no state_version")
            })?;
        let project_id = live_fixture_project_id(fixture)?;
        let requested = adapter.call_tool(
            "volicord.request_user_action",
            serde_json::json!({
                "detail": "full",
                "request": {
                    "operation": "create",
                    "task_id": task_id,
                    "change_unit_id": change_unit_id,
                    "action": {
                        "action_type": "choice",
                        "judgment_kind": "product_decision",
                        "presentation": "short",
                        "question": "Which live CLI-fallback route must the host consume?",
                        "options": [
                            {
                                "option_id": USER_ACTION_ROUTE_ALPHA_OPTION_ID,
                                "label": "Route alpha",
                                "description": "Select the alpha live CLI-fallback route.",
                                "consequence": "The host records the alpha choice-consumption Run marker.",
                                "is_default": false
                            },
                            {
                                "option_id": USER_ACTION_ROUTE_BETA_OPTION_ID,
                                "label": "Route beta",
                                "description": "Select the beta live CLI-fallback route.",
                                "consequence": "The host records the beta choice-consumption Run marker.",
                                "is_default": false
                            }
                        ],
                        "context": {
                            "summary": "A human operator must resolve this prepared request through the CLI User Channel.",
                            "related_refs": [],
                            "artifact_refs": [],
                            "visible_risks": [],
                            "constraints": ["The answer covers only this prepared fallback request."]
                        },
                        "affected_refs": [{
                            "record_kind": "task",
                            "record_id": task_id,
                            "project_id": project_id,
                            "task_id": task_id,
                            "produced_at_state_version": scope_state_version
                        }],
                        "sensitive_action_scope": null
                    },
                    "required_for": ["close_complete"],
                    "expires_at": null
                }
            }),
        )?;
        if requested.response_value["base"]["response_kind"] != "result"
            || requested.response_value["user_action_request_summary"]["status"] != "pending"
            || requested.response_value["user_action_request_summary"]["next_actor"] != "user"
        {
            return Err(
                io::Error::other("CLI-fallback setup did not create a pending request").into(),
            );
        }
        let requested_id = requested.response_value["user_action_request_summary"]
            ["user_action_request_id"]
            .as_str()
            .ok_or_else(|| io::Error::other("CLI-fallback setup returned no request id"))?;
        let observation = inspect_live_user_action(fixture, marker)?
            .ok_or_else(|| io::Error::other("CLI-fallback setup Task was not stored"))?;
        let expected_actor_source = format!("agent_connection:{connection_id}");
        if observation.user_action_request_id.as_deref() != Some(requested_id)
            || observation.user_action_status.as_deref() != Some("pending")
            || observation.option_ids
                != [
                    USER_ACTION_ROUTE_ALPHA_OPTION_ID.to_owned(),
                    USER_ACTION_ROUTE_BETA_OPTION_ID.to_owned(),
                ]
            || observation.requested_by_actor_source.as_deref()
                != Some(expected_actor_source.as_str())
        {
            return Err(io::Error::other(
                "CLI-fallback setup did not preserve the current request, options, and originating Agent Connection",
            )
            .into());
        }
        Ok(PreparedCliFallbackAction {
            observation,
            change_unit_id,
        })
    }

    fn live_cli_fallback_resume_prompt(
        user_action_request_id: &str,
        task_id: &str,
        change_unit_id: &str,
    ) -> String {
        format!(
            concat!(
                "Continue the prepared Volicord CLI User Channel fallback using the MCP server named `volicord`. ",
                "Do not edit files, run shell commands, create or resolve another user action, prepare a write, or answer on the user's behalf.\n\n",
                "1. Call `volicord.request_user_action` exactly once with nested `request.operation=resume` and `request.user_action_request_id={user_action_request_id}`. Do not use `request.operation=create`. The request was created by this same Agent Connection and has already been resolved by the human operator through `volicord inbox resolve --json`.\n",
                "2. Require the resumed result to report `current_status=resolved`, `agent_workflow_result_replayed=true`, and a non-null `user_channel_resolution.resolution_summary.selected_option_id`. Consume that selected option; do not infer it from this instruction.\n",
                "3. If the selected option is `{alpha_option_id}`, call `volicord.record_run` with summary exactly `{alpha_run_marker}`. If it is `{beta_option_id}`, use summary exactly `{beta_run_marker}`. Use `task_id={task_id}`, `change_unit_id={change_unit_id}`, `baseline_ref={baseline_ref}`, `kind=shaping_update`, `run_id=null`, `write_ticket_id=null`, `artifact_inputs=[]`, `evidence_updates=[]`, and `evidence_observations=[]`. Report `changed_paths=[]`, `product_file_write_observed=false`, `sensitive_categories=[]`, and the same baseline ref in `observed_changes`. Supply a non-null `close_assessment` whose `result_summary` is exactly the chosen Run marker and whose `result_refs`, `residual_risks`, `sensitive_categories`, and `recovery_constraints` are empty arrays. Do not record a Run if resume is pending or the option is absent or unrecognized.\n",
                "4. Call `volicord.status` for Task `{task_id}` and report the selected option ID, exact Run marker, lifecycle phase, close state, close-blocker count, and state version. Then stop."
            ),
            user_action_request_id = user_action_request_id,
            alpha_option_id = USER_ACTION_ROUTE_ALPHA_OPTION_ID,
            beta_option_id = USER_ACTION_ROUTE_BETA_OPTION_ID,
            alpha_run_marker = USER_ACTION_ROUTE_ALPHA_RUN_MARKER,
            beta_run_marker = USER_ACTION_ROUTE_BETA_RUN_MARKER,
            task_id = task_id,
            change_unit_id = change_unit_id,
            baseline_ref = LIVE_CLI_FALLBACK_BASELINE_REF,
        )
    }

    struct PreparedFinalAuthority {
        observation: LiveUserActionObservation,
        receipt: VerifiedLiveReceipt,
        change_unit_id: String,
    }

    fn prepare_live_final_authority(
        fixture: &LiveSmokeFixture,
        connection_id: &str,
        marker: &str,
    ) -> Result<PreparedFinalAuthority, Box<dyn Error>> {
        let context = McpConnectionContext::resolve(&fixture.runtime_home_path, connection_id)?
            .with_invocation_binding_basis(VERIFICATION_BASIS_TEST_FIXTURE_BINDING);
        let adapter = McpAdapter::new(&fixture.runtime_home_path, context);
        let intake = adapter.call_tool(
            "volicord.intake",
            serde_json::json!({
                "detail": "full",
                "plain_language_request": marker,
                "requested_mode": "advisor",
                "resume_policy": "create_new",
                "acceptance_policy": null,
                "lineage": null,
                "initial_scope": {
                    "boundary": "Validate one no-write live-host final-output receipt.",
                    "non_goals": [],
                    "acceptance_criteria": [{
                        "statement": "The no-write Run establishes a final-output close basis.",
                        "evidence_requirement": "not_required"
                    }]
                }
            }),
        )?;
        let task_id = intake.response_value["task_ref"]["record_id"]
            .as_str()
            .ok_or_else(|| io::Error::other("final-output setup intake returned no Task id"))?
            .to_owned();
        let scope = adapter.call_tool(
            "volicord.update_scope",
            serde_json::json!({
                "detail": "full",
                "task_id": task_id,
                "goal_summary": null,
                "scope_update": null,
                "scope_boundary": null,
                "non_goals": null,
                "acceptance_criteria": null,
                "autonomy_boundary": null,
                "baseline_ref": "baseline_live_final_output_matrix",
                "change_unit": {
                    "operation": "create_current",
                    "scope_summary": "No-write live-host final-output validation.",
                    "affected_paths": []
                },
                "related_scope_decision_refs": []
            }),
        )?;
        let change_unit_id = scope.response_value["state"]["active_change_unit_ref"]["record_id"]
            .as_str()
            .ok_or_else(|| {
                io::Error::other("final-output setup update_scope returned no Change Unit id")
            })?
            .to_owned();
        let run = adapter.call_tool(
            "volicord.record_run",
            serde_json::json!({
                "detail": "full",
                "task_id": task_id,
                "change_unit_id": change_unit_id,
                "kind": "shaping_update",
                "run_id": null,
                "baseline_ref": "baseline_live_final_output_matrix",
                "write_ticket_id": null,
                "summary": marker,
                "observed_changes": {
                    "changed_paths": [],
                    "product_file_write_observed": false,
                    "sensitive_categories": [],
                    "baseline_ref": "baseline_live_final_output_matrix"
                },
                "artifact_inputs": [],
                "evidence_updates": [],
                "evidence_observations": [],
                "close_assessment": {
                    "result_summary": marker,
                    "result_refs": [],
                    "residual_risks": [],
                    "sensitive_categories": [],
                    "recovery_constraints": []
                }
            }),
        )?;
        if run.response_value["base"]["response_kind"] != "result" {
            return Err(io::Error::other("final-output setup Run was not recorded").into());
        }
        let status = adapter.call_tool(
            "volicord.status",
            serde_json::json!({ "task_id": task_id, "detail": "full" }),
        )?;
        let state_version = status.response_value["base"]["state_version"]
            .as_u64()
            .ok_or_else(|| io::Error::other("final-output setup status has no state_version"))?;
        let observation = LiveUserActionObservation {
            project_id: live_fixture_project_id(fixture)?,
            task_id,
            lifecycle_phase: status.response_value["active_task"]["lifecycle"]["lifecycle_phase"]
                .as_str()
                .unwrap_or("unknown")
                .to_owned(),
            state_version,
            user_action_request_id: None,
            user_action_status: None,
            requested_by_actor_source: None,
            user_action_resolution_id: None,
            resolved_by_actor_source: None,
            resolved_verification_basis: None,
            resolved_channel_kind: None,
            selected_option_id: None,
            option_ids: Vec::new(),
        };
        let receipt = verify_current_final_authority_receipt(
            status.response_value.clone(),
            &observation,
            marker,
        )?;
        Ok(PreparedFinalAuthority {
            observation,
            receipt,
            change_unit_id,
        })
    }

    fn read_live_final_authority(
        fixture: &LiveSmokeFixture,
        connection_id: &str,
        task_id: &str,
        change_unit_id: &str,
        marker: &str,
    ) -> Result<PreparedFinalAuthority, Box<dyn Error>> {
        let context = McpConnectionContext::resolve(&fixture.runtime_home_path, connection_id)?
            .with_invocation_binding_basis(VERIFICATION_BASIS_TEST_FIXTURE_BINDING);
        let adapter = McpAdapter::new(&fixture.runtime_home_path, context);
        let status = adapter.call_tool(
            "volicord.status",
            serde_json::json!({ "task_id": task_id, "detail": "full" }),
        )?;
        let state_version = status.response_value["base"]["state_version"]
            .as_u64()
            .ok_or_else(|| {
                io::Error::other("refreshed final-output status has no state_version")
            })?;
        let observation = LiveUserActionObservation {
            project_id: live_fixture_project_id(fixture)?,
            task_id: task_id.to_owned(),
            lifecycle_phase: status.response_value["active_task"]["lifecycle"]["lifecycle_phase"]
                .as_str()
                .unwrap_or("unknown")
                .to_owned(),
            state_version,
            user_action_request_id: None,
            user_action_status: None,
            requested_by_actor_source: None,
            user_action_resolution_id: None,
            resolved_by_actor_source: None,
            resolved_verification_basis: None,
            resolved_channel_kind: None,
            selected_option_id: None,
            option_ids: Vec::new(),
        };
        let receipt =
            verify_current_final_authority_receipt(status.response_value, &observation, marker)?;
        Ok(PreparedFinalAuthority {
            observation,
            receipt,
            change_unit_id: change_unit_id.to_owned(),
        })
    }

    fn advance_live_final_authority(
        fixture: &LiveSmokeFixture,
        connection_id: &str,
        prepared: &PreparedFinalAuthority,
        marker: &str,
    ) -> Result<PreparedFinalAuthority, Box<dyn Error>> {
        let context = McpConnectionContext::resolve(&fixture.runtime_home_path, connection_id)?
            .with_invocation_binding_basis(VERIFICATION_BASIS_TEST_FIXTURE_BINDING);
        let adapter = McpAdapter::new(&fixture.runtime_home_path, context);
        let run = adapter.call_tool(
            "volicord.record_run",
            serde_json::json!({
                "detail": "full",
                "task_id": prepared.observation.task_id,
                "change_unit_id": prepared.change_unit_id,
                "kind": "shaping_update",
                "run_id": null,
                "baseline_ref": "baseline_live_final_output_matrix",
                "write_ticket_id": null,
                "summary": marker,
                "observed_changes": {
                    "changed_paths": [],
                    "product_file_write_observed": false,
                    "sensitive_categories": [],
                    "baseline_ref": "baseline_live_final_output_matrix"
                },
                "artifact_inputs": [],
                "evidence_updates": [],
                "evidence_observations": [],
                "close_assessment": {
                    "result_summary": marker,
                    "result_refs": [],
                    "residual_risks": [],
                    "sensitive_categories": [],
                    "recovery_constraints": []
                }
            }),
        )?;
        if run.response_value["base"]["response_kind"] != "result" {
            return Err(
                io::Error::other("final-output replay state advance was not recorded").into(),
            );
        }
        read_live_final_authority(
            fixture,
            connection_id,
            &prepared.observation.task_id,
            &prepared.change_unit_id,
            marker,
        )
    }

    fn run_marker_for_selected_option(selected_option_id: &str) -> Option<&'static str> {
        match selected_option_id {
            USER_ACTION_ROUTE_ALPHA_OPTION_ID => Some(USER_ACTION_ROUTE_ALPHA_RUN_MARKER),
            USER_ACTION_ROUTE_BETA_OPTION_ID => Some(USER_ACTION_ROUTE_BETA_RUN_MARKER),
            _ => None,
        }
    }

    #[derive(Debug)]
    struct LiveRunObservation {
        run_id: String,
        kind: String,
        summary: String,
        product_file_write_observed: bool,
        changed_paths: Vec<String>,
        created_by_actor_source: String,
    }

    #[derive(Debug)]
    struct LiveUserActionObservation {
        project_id: String,
        task_id: String,
        lifecycle_phase: String,
        state_version: u64,
        user_action_request_id: Option<String>,
        user_action_status: Option<String>,
        requested_by_actor_source: Option<String>,
        user_action_resolution_id: Option<String>,
        resolved_by_actor_source: Option<String>,
        resolved_verification_basis: Option<String>,
        resolved_channel_kind: Option<String>,
        selected_option_id: Option<String>,
        option_ids: Vec<String>,
    }

    #[derive(Debug)]
    struct LiveEvidenceObservation {
        project_id: String,
        task_id: String,
        lifecycle_phase: String,
        state_version: u64,
        user_action_request_id: String,
        requested_by_actor_source: String,
        user_action_resolution_id: String,
        resolved_by_actor_source: String,
        resolved_verification_basis: String,
        resolved_channel_kind: String,
        resolved_at: String,
        target: EvidenceTarget,
        artifact_ref: ArtifactRef,
        summary: String,
    }

    impl LiveEvidenceObservation {
        fn task_observation(&self) -> LiveUserActionObservation {
            LiveUserActionObservation {
                project_id: self.project_id.clone(),
                task_id: self.task_id.clone(),
                lifecycle_phase: self.lifecycle_phase.clone(),
                state_version: self.state_version,
                user_action_request_id: Some(self.user_action_request_id.clone()),
                user_action_status: Some("resolved".to_owned()),
                requested_by_actor_source: Some(self.requested_by_actor_source.clone()),
                user_action_resolution_id: Some(self.user_action_resolution_id.clone()),
                resolved_by_actor_source: Some(self.resolved_by_actor_source.clone()),
                resolved_verification_basis: Some(self.resolved_verification_basis.clone()),
                resolved_channel_kind: Some(self.resolved_channel_kind.clone()),
                selected_option_id: None,
                option_ids: Vec::new(),
            }
        }
    }

    fn inspect_live_evidence_observation(
        fixture: &LiveSmokeFixture,
        prepared: &PreparedEvidenceObservation,
        connection_id: &str,
    ) -> Result<LiveEvidenceObservation, Box<dyn Error>> {
        let project = list_projects(&fixture.runtime_home_path)?
            .into_iter()
            .find(|project| project.project_id == prepared.project_id)
            .ok_or_else(|| io::Error::other("live evidence project registration is missing"))?;
        let conn = open_project_state_database_read_only(&project.state_db_path)?;
        let (lifecycle_phase, state_version): (String, u64) = conn.query_row(
            "SELECT t.lifecycle_phase, ps.state_version
               FROM tasks t
               JOIN project_state ps ON ps.project_id = t.project_id
              WHERE t.project_id = ?1 AND t.task_id = ?2",
            rusqlite::params![prepared.project_id, prepared.task_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        let mut request_statement = conn.prepare(
            "SELECT user_action_request_id, change_unit_id, request_json, basis_json,
                    basis_status, required_for_json, requested_by_actor_source, source_method
               FROM user_action_requests
              WHERE project_id = ?1
                AND task_id = ?2
                AND action_kind = 'evidence_observation'",
        )?;
        let request_rows = request_statement
            .query_map(
                rusqlite::params![prepared.project_id, prepared.task_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, String>(6)?,
                        row.get::<_, String>(7)?,
                    ))
                },
            )?
            .collect::<Result<Vec<_>, _>>()?;
        let [request_row] = request_rows.as_slice() else {
            return Err(io::Error::other(format!(
                "the authenticated host must create exactly one evidence-observation request; found {}",
                request_rows.len()
            ))
            .into());
        };
        let (
            user_action_request_id,
            request_change_unit_id,
            request_json,
            basis_json,
            basis_status,
            required_for_json,
            requested_by_actor_source,
            source_method,
        ) = request_row;
        let expected_actor_source = format!("agent_connection:{connection_id}");
        let connection_request_count: u64 = conn.query_row(
            "SELECT COUNT(*)
               FROM user_action_requests
              WHERE project_id = ?1
                AND requested_by_actor_source = ?2
                AND source_method = 'volicord.request_user_action'",
            rusqlite::params![prepared.project_id, expected_actor_source],
            |row| row.get(0),
        )?;
        if request_change_unit_id.as_deref() != Some(prepared.change_unit_id.as_str())
            || basis_status != "current"
            || requested_by_actor_source != &expected_actor_source
            || source_method != "volicord.request_user_action"
            || connection_request_count != 1
        {
            return Err(io::Error::other(
                "the evidence-observation request is not bound to the prepared Change Unit and exact Agent Connection",
            )
            .into());
        }

        let stored_request: PersistedUserActionRequest = serde_json::from_str(request_json)?;
        let UserActionRequestBody::EvidenceObservation(request_body) = &stored_request.body else {
            return Err(io::Error::other(
                "the stored evidence-observation request has a non-observation body",
            )
            .into());
        };
        let required_for: Value = serde_json::from_str(required_for_json)?;
        if request_body.question != LIVE_EVIDENCE_REQUEST_QUESTION
            || request_body.context_summary != LIVE_EVIDENCE_REQUEST_CONTEXT
            || request_body.target_candidates.as_slice() != [prepared.target.clone()]
            || request_body.artifact_candidates.as_slice() != [prepared.artifact_ref.clone()]
            || serde_json::to_value(&stored_request.required_for)?
                != serde_json::json!(["record_run"])
            || required_for != serde_json::json!(["record_run"])
        {
            return Err(io::Error::other(
                "the evidence-observation request does not preserve the exact marker-free prose, sole target, artifact, and record_run requirement",
            )
            .into());
        }

        let basis: UserActionBasis = serde_json::from_str(basis_json)?;
        let UserActionBasis::EvidenceObservation(evidence_basis) = &basis else {
            return Err(io::Error::other(
                "the stored evidence-observation request has a non-observation basis",
            )
            .into());
        };
        let coordinates = &evidence_basis.coordinates;
        if coordinates.task_id.as_str() != prepared.task_id
            || coordinates
                .change_unit_id
                .as_ref()
                .map(|value| value.as_str())
                != Some(prepared.change_unit_id.as_str())
            || coordinates
                .baseline_ref
                .as_ref()
                .map(|value| value.as_str())
                != Some(LIVE_EVIDENCE_OBSERVATION_BASELINE_REF)
            || serde_json::to_value(coordinates.compatibility_status)? != "current"
            || evidence_basis.target_candidates.as_slice() != [prepared.target.clone()]
            || evidence_basis.artifact_candidates.as_slice() != [prepared.artifact_ref.clone()]
        {
            return Err(io::Error::other(
                "the evidence-observation basis is not the exact current prepared authority basis",
            )
            .into());
        }

        let resolution_row = conn
            .query_row(
                "SELECT user_action_resolution_id, channel_kind, resolution_json,
                        resolved_by_actor_source, resolved_verification_basis, resolved_at
                   FROM user_action_resolutions
                  WHERE project_id = ?1
                    AND user_action_request_id = ?2
                    AND action_kind = 'evidence_observation'",
                rusqlite::params![prepared.project_id, user_action_request_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                    ))
                },
            )
            .optional()?
            .ok_or_else(|| {
                io::Error::other(
                    "the sole live evidence-observation request has no stored resolution",
                )
            })?;
        let (
            user_action_resolution_id,
            resolved_channel_kind,
            resolution_json,
            resolved_by_actor_source,
            resolved_verification_basis,
            resolved_at,
        ) = resolution_row;
        let resolution: UserActionResolutionBody = serde_json::from_str(&resolution_json)?;
        let UserActionResolutionBody::EvidenceObservation { observation } = resolution else {
            return Err(io::Error::other(
                "the stored live evidence resolution has a non-observation body",
            )
            .into());
        };
        if resolved_channel_kind != "local_web_consent"
            || resolved_by_actor_source != "local_user"
            || resolved_verification_basis != VERIFICATION_BASIS_LOCAL_USER_LOCAL_WEB
            || observation.target != prepared.target
            || observation.relevance_status != EvidenceRelevanceStatus::Supported
            || observation.output_artifact_refs.as_slice() != [prepared.artifact_ref.clone()]
            || observation.summary.trim().is_empty()
            || observation.summary.chars().count() > USER_ACTION_OBSERVATION_SUMMARY_MAX_CHARS
        {
            return Err(io::Error::other(
                "the local-web resolution does not preserve the exact user-owned supported observation",
            )
            .into());
        }
        bounded_identity("evidence resolution id", &user_action_resolution_id, 256)?;
        bounded_identity(
            "evidence resolution timestamp",
            &resolved_at,
            MAX_RECORDED_AT_CHARS,
        )?;
        Ok(LiveEvidenceObservation {
            project_id: prepared.project_id.clone(),
            task_id: prepared.task_id.clone(),
            lifecycle_phase,
            state_version,
            user_action_request_id: user_action_request_id.clone(),
            requested_by_actor_source: requested_by_actor_source.clone(),
            user_action_resolution_id,
            resolved_by_actor_source,
            resolved_verification_basis,
            resolved_channel_kind,
            resolved_at,
            target: observation.target,
            artifact_ref: observation
                .output_artifact_refs
                .into_iter()
                .next()
                .ok_or_else(|| io::Error::other("resolved evidence artifact disappeared"))?,
            summary: observation.summary,
        })
    }

    fn inspect_live_user_action(
        fixture: &LiveSmokeFixture,
        marker: &str,
    ) -> Result<Option<LiveUserActionObservation>, Box<dyn Error>> {
        let projects = list_projects(&fixture.runtime_home_path)?;
        let project = projects
            .iter()
            .find(|project| project.repo_root == fixture.repo_root)
            .ok_or_else(|| io::Error::other("live smoke project registration is missing"))?;
        let conn = open_project_state_database_read_only(&project.state_db_path)?;
        let row = conn
            .query_row(
                "SELECT t.task_id, t.lifecycle_phase, ps.state_version,
                        r.user_action_request_id,
                        CASE
                          WHEN s.user_action_resolution_id IS NOT NULL THEN 'resolved'
                          WHEN r.basis_status = 'stale' THEN 'stale'
                          WHEN r.basis_status = 'superseded' THEN 'superseded'
                          ELSE 'pending'
                        END,
                        r.requested_by_actor_source,
                        s.user_action_resolution_id,
                        s.resolved_by_actor_source,
                        s.resolved_verification_basis, s.channel_kind,
                        r.request_json,
                        s.resolution_json
                   FROM tasks t
                   JOIN project_state ps ON ps.project_id = t.project_id
              LEFT JOIN user_action_requests r
                     ON r.project_id = t.project_id
                    AND r.task_id = t.task_id
                    AND r.action_kind = 'product_decision'
              LEFT JOIN user_action_resolutions s
                     ON s.project_id = r.project_id
                    AND s.user_action_request_id = r.user_action_request_id
                  WHERE t.project_id = ?1 AND t.summary = ?2
                  ORDER BY r.requested_at DESC
                  LIMIT 1",
                rusqlite::params![project.project_id, marker],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, u64>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, Option<String>>(4)?,
                        row.get::<_, Option<String>>(5)?,
                        row.get::<_, Option<String>>(6)?,
                        row.get::<_, Option<String>>(7)?,
                        row.get::<_, Option<String>>(8)?,
                        row.get::<_, Option<String>>(9)?,
                        row.get::<_, Option<String>>(10)?,
                        row.get::<_, Option<String>>(11)?,
                    ))
                },
            )
            .optional()?;
        let Some((
            task_id,
            lifecycle_phase,
            state_version,
            user_action_request_id,
            user_action_status,
            requested_by_actor_source,
            user_action_resolution_id,
            resolved_by_actor_source,
            resolved_verification_basis,
            resolved_channel_kind,
            options_json,
            resolution_json,
        )) = row
        else {
            return Ok(None);
        };
        let selected_option_id =
            selected_option_id_from_resolution_json(resolution_json.as_deref())?;
        let option_ids = options_json
            .as_deref()
            .and_then(|text| serde_json::from_str::<Value>(text).ok())
            .and_then(|value| {
                value
                    .pointer("/body/options")
                    .and_then(Value::as_array)
                    .cloned()
            })
            .unwrap_or_default()
            .iter()
            .filter_map(|option| option.get("option_id").and_then(Value::as_str))
            .map(str::to_owned)
            .collect::<Vec<_>>();
        Ok(Some(LiveUserActionObservation {
            project_id: project.project_id.clone(),
            task_id,
            lifecycle_phase,
            state_version,
            user_action_request_id,
            user_action_status,
            requested_by_actor_source,
            user_action_resolution_id,
            resolved_by_actor_source,
            resolved_verification_basis,
            resolved_channel_kind,
            selected_option_id,
            option_ids,
        }))
    }

    fn selected_option_id_from_resolution_json(
        resolution_json: Option<&str>,
    ) -> Result<Option<String>, Box<dyn Error>> {
        let Some(resolution_json) = resolution_json else {
            return Ok(None);
        };
        let value: Value = serde_json::from_str(resolution_json)?;
        let selected_option_id = value
            .get("selected_option_id")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                io::Error::other("resolved live user action has no selected_option_id")
            })?;
        Ok(Some(selected_option_id.to_owned()))
    }

    fn live_run_observation(
        run_id: &str,
        kind: &str,
        summary_json: &str,
        observed_changes_json: &str,
        created_by_actor_source: &str,
    ) -> Result<LiveRunObservation, Box<dyn Error>> {
        let summary: Value = serde_json::from_str(summary_json)?;
        let observed_changes: Value = serde_json::from_str(observed_changes_json)?;
        let summary = summary
            .get("summary")
            .and_then(Value::as_str)
            .ok_or_else(|| io::Error::other("live smoke Run has no summary"))?
            .to_owned();
        let product_file_write_observed = observed_changes
            .get("product_file_write_observed")
            .and_then(Value::as_bool)
            .ok_or_else(|| {
                io::Error::other("live smoke Run has no product-file write observation")
            })?;
        let changed_paths = observed_changes
            .get("changed_paths")
            .and_then(Value::as_array)
            .ok_or_else(|| io::Error::other("live smoke Run has no changed_paths array"))?
            .iter()
            .map(|path| {
                path.as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| io::Error::other("live smoke Run has a non-string changed path"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(LiveRunObservation {
            run_id: run_id.to_owned(),
            kind: kind.to_owned(),
            summary,
            product_file_write_observed,
            changed_paths,
            created_by_actor_source: created_by_actor_source.to_owned(),
        })
    }

    #[derive(Debug)]
    struct AuthorityEventOrder {
        user_action_requested_event_seq: u64,
        user_action_resolved_event_seq: u64,
        run_recorded_event_seq: u64,
    }

    fn inspect_live_choice_consumption(
        fixture: &LiveSmokeFixture,
        observation: &LiveUserActionObservation,
        run_id: &str,
    ) -> Result<(LiveRunObservation, AuthorityEventOrder), Box<dyn Error>> {
        let projects = list_projects(&fixture.runtime_home_path)?;
        let project = projects
            .iter()
            .find(|project| project.project_id == observation.project_id)
            .ok_or_else(|| io::Error::other("live smoke project registration is missing"))?;
        let conn = open_project_state_database_read_only(&project.state_db_path)?;
        let (
            stored_run_id,
            kind,
            status,
            summary_json,
            observed_changes_json,
            created_by_actor_source,
        ) = conn
            .query_row(
                "SELECT run_id, kind, status, summary_json, observed_changes_json,
                        created_by_actor_source
                   FROM runs
                  WHERE project_id = ?1
                    AND task_id = ?2
                    AND run_id = ?3",
                rusqlite::params![observation.project_id, observation.task_id, run_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                    ))
                },
            )
            .optional()?
            .ok_or_else(|| {
                io::Error::other(format!(
                    "fresh AuthorityReceipt latest_run_ref names missing Run {run_id}"
                ))
            })?;
        if status != "recorded" {
            return Err(io::Error::other(format!(
                "fresh AuthorityReceipt latest_run_ref names Run {run_id} with status {status:?}"
            ))
            .into());
        }
        let run = live_run_observation(
            &stored_run_id,
            &kind,
            &summary_json,
            &observed_changes_json,
            &created_by_actor_source,
        )?;

        let user_action_request_id = observation
            .user_action_request_id
            .as_deref()
            .ok_or_else(|| io::Error::other("resolved live user-action request id is missing"))?;
        let user_action_resolution_id = observation
            .user_action_resolution_id
            .as_deref()
            .ok_or_else(|| {
                io::Error::other("resolved live user-action resolution id is missing")
            })?;
        let resolved_channel_kind = observation
            .resolved_channel_kind
            .as_deref()
            .ok_or_else(|| io::Error::other("resolved live user-action channel kind is missing"))?;
        let mut statement = conn.prepare(
            "SELECT event_seq, event_type, payload_json
               FROM authority_events
              WHERE project_id = ?1
                AND task_id = ?2
                AND event_type IN (
                    'user_action_requested',
                    'user_action_resolved',
                    'run_recorded'
                )
              ORDER BY event_seq",
        )?;
        let rows = statement.query_map(
            rusqlite::params![observation.project_id, observation.task_id],
            |row| {
                Ok((
                    row.get::<_, u64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )?;
        let mut requested = Vec::new();
        let mut recorded = Vec::new();
        let mut runs = Vec::new();
        for row in rows {
            let (event_seq, event_type, payload_json) = row?;
            let payload: Value = serde_json::from_str(&payload_json)?;
            if payload.get("task_id").and_then(Value::as_str) != Some(observation.task_id.as_str())
            {
                continue;
            }
            match event_type.as_str() {
                "user_action_requested"
                    if payload
                        .get("user_action_request_id")
                        .and_then(Value::as_str)
                        == Some(user_action_request_id) =>
                {
                    requested.push(event_seq);
                }
                "user_action_resolved"
                    if payload
                        .get("user_action_request_id")
                        .and_then(Value::as_str)
                        == Some(user_action_request_id) =>
                {
                    if payload
                        .get("user_action_resolution_id")
                        .and_then(Value::as_str)
                        != Some(user_action_resolution_id)
                        || payload.get("action_kind").and_then(Value::as_str)
                            != Some("product_decision")
                        || payload.get("channel_kind").and_then(Value::as_str)
                            != Some(resolved_channel_kind)
                    {
                        return Err(io::Error::other(
                            "matching user_action_resolved event does not preserve the stored resolution and User Channel",
                        )
                        .into());
                    }
                    recorded.push(event_seq);
                }
                "run_recorded" if payload.get("run_id").and_then(Value::as_str) == Some(run_id) => {
                    if payload.get("kind").and_then(Value::as_str) != Some(run.kind.as_str())
                        || payload
                            .get("product_file_write_observed")
                            .and_then(Value::as_bool)
                            != Some(run.product_file_write_observed)
                    {
                        return Err(io::Error::other(
                            "matching run_recorded event does not preserve the inspected Run facts",
                        )
                        .into());
                    }
                    runs.push(event_seq);
                }
                _ => {}
            }
        }
        let requested_event_seq =
            exactly_one_event_seq("matching user_action_requested", requested.as_slice())?;
        let recorded_event_seq =
            exactly_one_event_seq("matching user_action_resolved", recorded.as_slice())?;
        let run_event_seq = exactly_one_event_seq("matching run_recorded", runs.as_slice())?;
        if !(requested_event_seq < recorded_event_seq && recorded_event_seq < run_event_seq) {
            return Err(io::Error::other(format!(
                "matching authority events are out of order: request={requested_event_seq}, record={recorded_event_seq}, run={run_event_seq}"
            ))
            .into());
        }
        Ok((
            run,
            AuthorityEventOrder {
                user_action_requested_event_seq: requested_event_seq,
                user_action_resolved_event_seq: recorded_event_seq,
                run_recorded_event_seq: run_event_seq,
            },
        ))
    }

    #[derive(Debug)]
    struct LiveEvidenceConsumption {
        run: LiveRunObservation,
        evidence_observation_id: String,
        resolution_ref: StateRecordRef,
        observation_ref: StateRecordRef,
        observed_by_actor_source: String,
        producer_kind: String,
        producer_verification_basis: String,
        relevance_status: String,
        assessed_by_actor_source: String,
        coverage_state: String,
        observed_at_matches_resolution: bool,
    }

    fn inspect_live_evidence_consumption(
        fixture: &LiveSmokeFixture,
        observation: &LiveEvidenceObservation,
        run_id: &str,
        connection_id: &str,
    ) -> Result<(LiveEvidenceConsumption, AuthorityEventOrder), Box<dyn Error>> {
        let project = list_projects(&fixture.runtime_home_path)?
            .into_iter()
            .find(|project| project.project_id == observation.project_id)
            .ok_or_else(|| io::Error::other("live evidence project registration is missing"))?;
        let conn = open_project_state_database_read_only(&project.state_db_path)?;
        let run_row = conn
            .query_row(
                "SELECT run_id, kind, status, summary_json, observed_changes_json,
                        created_by_actor_source, write_ticket_id, evidence_updates_json
                   FROM runs
                  WHERE project_id = ?1 AND task_id = ?2 AND run_id = ?3",
                rusqlite::params![observation.project_id, observation.task_id, run_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, Option<String>>(6)?,
                        row.get::<_, String>(7)?,
                    ))
                },
            )
            .optional()?
            .ok_or_else(|| {
                io::Error::other(format!(
                    "fresh evidence AuthorityReceipt names missing Run {run_id}"
                ))
            })?;
        let (
            stored_run_id,
            kind,
            status,
            summary_json,
            observed_changes_json,
            created_by_actor_source,
            write_ticket_id,
            evidence_updates_json,
        ) = run_row;
        let run = live_run_observation(
            &stored_run_id,
            &kind,
            &summary_json,
            &observed_changes_json,
            &created_by_actor_source,
        )?;
        let expected_actor_source = format!("agent_connection:{connection_id}");
        let evidence_updates: Value = serde_json::from_str(&evidence_updates_json)?;
        let expected_update = serde_json::json!([{
            "target": observation.target,
            "coverage_state": "supported",
            "supporting_run_refs": [],
            "observation_refs": [],
            "supporting_artifact_refs": [observation.artifact_ref],
            "gap_refs": []
        }]);
        if status != "recorded"
            || run.kind != "shaping_update"
            || run.summary != LIVE_EVIDENCE_OBSERVATION_RUN_MARKER
            || run.created_by_actor_source != expected_actor_source
            || run.product_file_write_observed
            || !run.changed_paths.is_empty()
            || write_ticket_id.is_some()
            || evidence_updates != expected_update
        {
            return Err(io::Error::other(
                "the consuming Run does not preserve the exact no-write supported evidence update",
            )
            .into());
        }

        let mut observation_statement = conn.prepare(
            "SELECT evidence_observation_id, acceptance_criterion_id, evidence_claim_id,
                    source_kind, assurance_level, observed_by_actor_source, tool_name,
                    tool_invocation_id, tool_metadata_json, input_refs_json,
                    source_refs_json, output_artifact_refs_json, limitations_json,
                    observed_at, metadata_json
               FROM evidence_observations
              WHERE project_id = ?1 AND task_id = ?2 AND run_id = ?3",
        )?;
        let stored_observations = observation_statement
            .query_map(
                rusqlite::params![observation.project_id, observation.task_id, run_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, Option<String>>(5)?,
                        row.get::<_, Option<String>>(6)?,
                        row.get::<_, Option<String>>(7)?,
                        row.get::<_, String>(8)?,
                        row.get::<_, String>(9)?,
                        row.get::<_, String>(10)?,
                        row.get::<_, String>(11)?,
                        row.get::<_, String>(12)?,
                        row.get::<_, String>(13)?,
                        row.get::<_, String>(14)?,
                    ))
                },
            )?
            .collect::<Result<Vec<_>, _>>()?;
        let [stored_observation] = stored_observations.as_slice() else {
            return Err(io::Error::other(format!(
                "the consuming Run must own exactly one evidence observation; found {}",
                stored_observations.len()
            ))
            .into());
        };
        let (
            evidence_observation_id,
            acceptance_criterion_id,
            evidence_claim_id,
            source_kind,
            assurance_level,
            observed_by_actor_source,
            tool_name,
            tool_invocation_id,
            tool_metadata_json,
            input_refs_json,
            source_refs_json,
            output_artifact_refs_json,
            limitations_json,
            observed_at,
            metadata_json,
        ) = stored_observation;
        let EvidenceTarget::AcceptanceCriterion {
            acceptance_criterion_id: expected_criterion_id,
        } = &observation.target
        else {
            return Err(io::Error::other(
                "the prepared live evidence target is not an acceptance criterion",
            )
            .into());
        };
        let input_refs: Vec<StateRecordRef> = serde_json::from_str(input_refs_json)?;
        let output_artifact_refs: Vec<ArtifactRef> =
            serde_json::from_str(output_artifact_refs_json)?;
        let [resolution_ref] = input_refs.as_slice() else {
            return Err(io::Error::other(
                "the user evidence observation must contain only the exact resolution input ref",
            )
            .into());
        };
        if resolution_ref.record_kind != StateRecordKind::UserActionResolution
            || resolution_ref.record_id.as_str() != observation.user_action_resolution_id
            || resolution_ref.project_id.as_str() != observation.project_id
            || resolution_ref.task_id.as_ref().map(|value| value.as_str())
                != Some(observation.task_id.as_str())
            || acceptance_criterion_id.as_deref() != Some(expected_criterion_id.as_str())
            || evidence_claim_id.is_some()
            || source_kind != "user_observation"
            || assurance_level != "user_observed"
            || observed_by_actor_source.as_deref() != Some("local_user")
            || tool_name.is_some()
            || tool_invocation_id.is_some()
            || serde_json::from_str::<Value>(tool_metadata_json)? != serde_json::json!({})
            || serde_json::from_str::<Value>(source_refs_json)? != serde_json::json!([])
            || output_artifact_refs.as_slice() != [observation.artifact_ref.clone()]
            || serde_json::from_str::<Value>(limitations_json)? != serde_json::json!([])
            || observed_at != &observation.resolved_at
            || observed_at == LIVE_EVIDENCE_CALLER_OBSERVED_AT
        {
            return Err(io::Error::other(
                "the stored evidence observation is not the exact Core-derived local-user observation",
            )
            .into());
        }

        let authority: PersistedEvidenceObservationAuthority = serde_json::from_str(metadata_json)?;
        let producer_ref = authority
            .producer_anchor
            .producer_ref
            .as_ref()
            .ok_or_else(|| io::Error::other("user evidence producer has no resolution ref"))?;
        let assessment_ref = authority
            .relevance_assessment
            .assessment_ref
            .as_ref()
            .ok_or_else(|| io::Error::other("user evidence relevance has no resolution ref"))?;
        let assessed_by_actor_source = authority
            .relevance_assessment
            .assessed_by_actor_source
            .as_ref()
            .map(|actor| actor.to_canonical_string())
            .ok_or_else(|| io::Error::other("user evidence relevance has no actor"))?;
        if authority.recorded_by_run_id.as_str() != run_id
            || authority.invocation_verification_basis
                != VERIFICATION_BASIS_MCP_STDIO_CONNECTION_BINDING
            || authority.producer_anchor.producer_kind
                != EvidenceProducerKind::UserChannelObservation
            || producer_ref != resolution_ref
            || authority.producer_anchor.output_artifact_refs.as_slice()
                != [observation.artifact_ref.clone()]
            || authority
                .producer_anchor
                .verification_basis
                .as_ref()
                .map(String::as_str)
                != Some(VERIFICATION_BASIS_LOCAL_USER_LOCAL_WEB)
            || authority.relevance_assessment.status != EvidenceRelevanceStatus::Supported
            || assessment_ref != resolution_ref
            || assessed_by_actor_source != "local_user"
        {
            return Err(io::Error::other(
                "the evidence producer and relevance assessment are not anchored to the same local-web resolution",
            )
            .into());
        }

        let mut coverage_statement = conn.prepare(
            "SELECT status, coverage_json, metadata_json
               FROM evidence_summaries
              WHERE project_id = ?1 AND task_id = ?2",
        )?;
        let coverage_rows = coverage_statement
            .query_map(
                rusqlite::params![observation.project_id, observation.task_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )?
            .collect::<Result<Vec<_>, _>>()?;
        let mut consuming_coverage = Vec::new();
        for (status, coverage_json, metadata_json) in coverage_rows {
            let metadata: PersistedEvidenceMetadata = serde_json::from_str(&metadata_json)?;
            if metadata.updated_by_run_id.as_str() == run_id {
                consuming_coverage.push((status, coverage_json));
            }
        }
        let [(coverage_status, coverage_json)] = consuming_coverage.as_slice() else {
            return Err(io::Error::other(format!(
                "the consuming Run must own exactly one evidence summary; found {}",
                consuming_coverage.len()
            ))
            .into());
        };
        let coverage_items: Vec<EvidenceCoverageItem> = serde_json::from_str(coverage_json)?;
        let [coverage] = coverage_items.as_slice() else {
            return Err(io::Error::other(
                "the consuming Run did not produce exactly one evidence coverage item",
            )
            .into());
        };
        let [observation_ref] = coverage.observation_refs.as_slice() else {
            return Err(io::Error::other(
                "supported coverage does not name exactly one evidence observation",
            )
            .into());
        };
        let [supporting_run_ref] = coverage.supporting_run_refs.as_slice() else {
            return Err(io::Error::other(
                "supported coverage does not name exactly one supporting Run",
            )
            .into());
        };
        if coverage_status != "sufficient"
            || coverage.target != observation.target
            || coverage.coverage_state != EvidenceCoverageState::Supported
            || observation_ref.record_kind != StateRecordKind::EvidenceObservation
            || observation_ref.record_id.as_str() != evidence_observation_id
            || observation_ref.project_id.as_str() != observation.project_id
            || observation_ref.task_id.as_ref().map(|value| value.as_str())
                != Some(observation.task_id.as_str())
            || supporting_run_ref.record_kind != StateRecordKind::Run
            || supporting_run_ref.record_id.as_str() != run_id
            || coverage.supporting_artifact_refs.as_slice() != [observation.artifact_ref.clone()]
            || !coverage.gap_refs.is_empty()
        {
            return Err(io::Error::other(
                "the latest supported coverage is not bound to the consuming Run, observation, target, and artifact",
            )
            .into());
        }

        let mut event_statement = conn.prepare(
            "SELECT event_seq, event_type, payload_json
               FROM authority_events
              WHERE project_id = ?1 AND task_id = ?2
                AND event_type IN ('user_action_requested', 'user_action_resolved', 'run_recorded')
              ORDER BY event_seq",
        )?;
        let event_rows = event_statement.query_map(
            rusqlite::params![observation.project_id, observation.task_id],
            |row| {
                Ok((
                    row.get::<_, u64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )?;
        let mut requested_events = Vec::new();
        let mut resolved_events = Vec::new();
        let mut run_events = Vec::new();
        for row in event_rows {
            let (event_seq, event_type, payload_json) = row?;
            let payload: Value = serde_json::from_str(&payload_json)?;
            match event_type.as_str() {
                "user_action_requested"
                    if payload["user_action_request_id"] == observation.user_action_request_id =>
                {
                    if payload["action_kind"] != "evidence_observation" {
                        return Err(io::Error::other(
                            "matching request authority event has the wrong action kind",
                        )
                        .into());
                    }
                    requested_events.push(event_seq);
                }
                "user_action_resolved"
                    if payload["user_action_request_id"] == observation.user_action_request_id =>
                {
                    if payload["user_action_resolution_id"] != observation.user_action_resolution_id
                        || payload["action_kind"] != "evidence_observation"
                        || payload["channel_kind"] != "local_web_consent"
                    {
                        return Err(io::Error::other(
                            "matching resolution authority event does not preserve the local-web observation",
                        )
                        .into());
                    }
                    resolved_events.push(event_seq);
                }
                "run_recorded" if payload["run_id"] == run_id => {
                    if payload["kind"] != "shaping_update"
                        || payload["product_file_write_observed"] != false
                        || payload["evidence_observation_ids"]
                            != serde_json::json!([evidence_observation_id])
                    {
                        return Err(io::Error::other(
                            "matching Run authority event does not preserve the sole evidence observation",
                        )
                        .into());
                    }
                    run_events.push(event_seq);
                }
                _ => {}
            }
        }
        let requested_event_seq =
            exactly_one_event_seq("evidence user_action_requested", &requested_events)?;
        let resolved_event_seq =
            exactly_one_event_seq("evidence user_action_resolved", &resolved_events)?;
        let run_event_seq = exactly_one_event_seq("evidence run_recorded", &run_events)?;
        if !(requested_event_seq < resolved_event_seq && resolved_event_seq < run_event_seq) {
            return Err(io::Error::other(
                "the evidence request, resolution, and consuming Run authority events are out of order",
            )
            .into());
        }

        Ok((
            LiveEvidenceConsumption {
                run,
                evidence_observation_id: evidence_observation_id.clone(),
                resolution_ref: resolution_ref.clone(),
                observation_ref: observation_ref.clone(),
                observed_by_actor_source: "local_user".to_owned(),
                producer_kind: "user_channel_observation".to_owned(),
                producer_verification_basis: VERIFICATION_BASIS_LOCAL_USER_LOCAL_WEB.to_owned(),
                relevance_status: "supported".to_owned(),
                assessed_by_actor_source,
                coverage_state: "supported".to_owned(),
                observed_at_matches_resolution: true,
            },
            AuthorityEventOrder {
                user_action_requested_event_seq: requested_event_seq,
                user_action_resolved_event_seq: resolved_event_seq,
                run_recorded_event_seq: run_event_seq,
            },
        ))
    }

    fn exactly_one_event_seq(label: &str, event_seqs: &[u64]) -> Result<u64, Box<dyn Error>> {
        match event_seqs {
            [event_seq] => Ok(*event_seq),
            _ => Err(io::Error::other(format!(
                "expected exactly one {label} authority event, found {}",
                event_seqs.len()
            ))
            .into()),
        }
    }

    fn assert_single_live_product_decision_request(
        fixture: &LiveSmokeFixture,
        observation: &LiveUserActionObservation,
    ) -> Result<(), Box<dyn Error>> {
        let project = list_projects(&fixture.runtime_home_path)?
            .into_iter()
            .find(|project| project.project_id == observation.project_id)
            .ok_or_else(|| io::Error::other("live smoke project registration is missing"))?;
        let conn = open_project_state_database_read_only(&project.state_db_path)?;
        let (count, request_id): (u64, Option<String>) = conn.query_row(
            "SELECT COUNT(*), MIN(user_action_request_id)
               FROM user_action_requests
              WHERE project_id = ?1
                AND task_id = ?2
                AND action_kind = 'product_decision'",
            rusqlite::params![observation.project_id, observation.task_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        if count != 1 || request_id.as_deref() != observation.user_action_request_id.as_deref() {
            return Err(io::Error::other(format!(
                "same-request resume must not create another product-decision request; found {count}"
            ))
            .into());
        }
        Ok(())
    }

    struct LiveInboxFallback {
        inbox_command_template: &'static str,
        resolve_command_template: &'static str,
    }

    struct LiveCliResolutionEvidence {
        user_action_resolution_id: String,
        selected_option_id: String,
        state_version_before_resolution: u64,
        committed_state_version: u64,
        exact_retry_state_version: u64,
        inbox_request_visible: bool,
        exact_retry_stdout_identical: bool,
        exact_retry_no_state_change: bool,
    }

    struct LiveHostIdentity {
        host: String,
        host_version: String,
        volicord_build_id: String,
        connection_id: String,
    }

    fn resolve_live_user_action_via_cli(
        fixture: &LiveSmokeFixture,
        observation: &LiveUserActionObservation,
        operator_choice_id: &str,
    ) -> Result<LiveCliResolutionEvidence, Box<dyn Error>> {
        let user_action_request_id = observation
            .user_action_request_id
            .as_deref()
            .ok_or_else(|| io::Error::other("pending CLI-fallback request id is missing"))?;
        if observation.user_action_status.as_deref() != Some("pending") {
            return Err(io::Error::other(
                "CLI fallback can resolve only the prepared pending request",
            )
            .into());
        }
        let inbox = fixture.run_volicord([
            "inbox",
            "--repo",
            fixture.repo_arg(),
            "--task",
            &observation.task_id,
            "--json",
        ])?;
        assert_success("volicord inbox --json for live CLI fallback", &inbox);
        let inbox_json = json_stdout(&inbox)?;
        let inbox_items = inbox_json["pending_user_action_inbox_items"]
            .as_array()
            .ok_or_else(|| io::Error::other("CLI inbox JSON has no pending item array"))?;
        let matching_items = inbox_items
            .iter()
            .filter(|item| item["user_action_request_id"].as_str() == Some(user_action_request_id))
            .collect::<Vec<_>>();
        if matching_items.len() != 1 {
            return Err(io::Error::other(format!(
                "CLI inbox must show the prepared request exactly once; found {}",
                matching_items.len()
            ))
            .into());
        }
        let choice_ids = matching_items[0]["form"]["choices"]
            .as_array()
            .ok_or_else(|| io::Error::other("CLI inbox item has no choice form"))?
            .iter()
            .filter_map(|choice| choice["choice_id"].as_str())
            .collect::<Vec<_>>();
        if choice_ids
            != [
                USER_ACTION_ROUTE_ALPHA_OPTION_ID,
                USER_ACTION_ROUTE_BETA_OPTION_ID,
            ]
        {
            return Err(io::Error::other(
                "CLI inbox did not preserve the two prepared route choices in order",
            )
            .into());
        }

        let resolve_args = [
            "inbox",
            "resolve",
            user_action_request_id,
            "--choice",
            operator_choice_id,
            "--repo",
            fixture.repo_arg(),
            "--json",
        ];
        let resolved = fixture.run_volicord(resolve_args)?;
        assert_success("volicord inbox resolve --json", &resolved);
        let resolved_json = json_stdout(&resolved)?;
        if resolved_json["base"]["response_kind"] != "result"
            || resolved_json["user_action_resolution"]["body"]["selected_option_id"]
                != operator_choice_id
            || resolved_json["user_action_resolution"]["resolved_by_actor_source"] != "local_user"
            || resolved_json["user_action_resolution"]["resolved_verification_basis"]
                != VERIFICATION_BASIS_CLI_DIRECT_USER_CHANNEL
            || resolved_json["user_action_resolution"]["channel_kind"] != "cli"
        {
            return Err(io::Error::other(
                "CLI resolve JSON did not preserve the operator choice and CLI User Channel basis",
            )
            .into());
        }
        let user_action_resolution_id = resolved_json["user_action_resolution_ref"]["record_id"]
            .as_str()
            .ok_or_else(|| io::Error::other("CLI resolve JSON has no resolution id"))?
            .to_owned();
        let committed_state_version = resolved_json["base"]["state_version"]
            .as_u64()
            .ok_or_else(|| io::Error::other("CLI resolve JSON has no state_version"))?;
        let expected_committed_state_version = observation
            .state_version
            .checked_add(1)
            .ok_or_else(|| io::Error::other("pre-resolution state_version cannot advance once"))?;
        if committed_state_version != expected_committed_state_version {
            return Err(io::Error::other(format!(
                "the first CLI resolution must advance project state exactly once: before={}, committed={committed_state_version}",
                observation.state_version
            ))
            .into());
        }

        let exact_retry = fixture.run_volicord(resolve_args)?;
        assert_success("exact retry of volicord inbox resolve --json", &exact_retry);
        let exact_retry_json = json_stdout(&exact_retry)?;
        let exact_retry_state_version = exact_retry_json["base"]["state_version"]
            .as_u64()
            .ok_or_else(|| io::Error::other("CLI exact retry JSON has no state_version"))?;
        let exact_retry_stdout_identical = resolved.output.stdout == exact_retry.output.stdout;
        let exact_retry_no_state_change = exact_retry_state_version == committed_state_version;
        if !exact_retry_stdout_identical || !exact_retry_no_state_change {
            return Err(io::Error::other(
                "the identical CLI resolution retry changed its JSON bytes or state version",
            )
            .into());
        }
        let status = fixture.run_volicord([
            "status",
            "--repo",
            fixture.repo_arg(),
            "--task",
            &observation.task_id,
            "--json",
        ])?;
        assert_success("volicord status after CLI exact retry", &status);
        if json_stdout(&status)?["base"]["state_version"] != committed_state_version {
            return Err(io::Error::other(
                "fresh status changed after the exact CLI resolution retry",
            )
            .into());
        }
        Ok(LiveCliResolutionEvidence {
            user_action_resolution_id,
            selected_option_id: operator_choice_id.to_owned(),
            state_version_before_resolution: observation.state_version,
            committed_state_version,
            exact_retry_state_version,
            inbox_request_visible: true,
            exact_retry_stdout_identical,
            exact_retry_no_state_change,
        })
    }

    fn verify_ephemeral_inbox_fallback_shape(
        fixture: &LiveSmokeFixture,
        observation: &LiveUserActionObservation,
    ) -> Result<LiveInboxFallback, Box<dyn Error>> {
        let user_action_request_id = observation
            .user_action_request_id
            .as_deref()
            .ok_or_else(|| io::Error::other("pending user-action request id is missing"))?;
        if observation.option_ids.is_empty() {
            return Err(io::Error::other("pending user action has no fallback choices").into());
        }
        let inbox = fixture.run_volicord([
            "inbox",
            "--repo",
            fixture.repo_arg(),
            "--task",
            &observation.task_id,
            "--json",
        ])?;
        assert_success("volicord inbox live fallback", &inbox);
        let inbox_text = stdout(&inbox);
        assert!(
            inbox_text.contains(user_action_request_id),
            "CLI inbox did not include pending user action {user_action_request_id}: {inbox_text}"
        );
        let answer_help = fixture.run_volicord(["inbox", "resolve", "--help"])?;
        assert_success("volicord inbox resolve --help", &answer_help);
        assert!(
            stdout(&answer_help)
                .lines()
                .any(|line| line.trim() == LIVE_INBOX_RESOLVE_USAGE),
            "CLI inbox resolve help no longer matches the verified fallback command shape: {}",
            stdout(&answer_help)
        );
        println!(
            concat!(
                "\nVerified CLI fallback shape inside the disposable fixture. ",
                "These templates are not runnable recovery commands because the fixture is deleted after the test:\n",
                "  {}\n",
                "  {}\n"
            ),
            LIVE_INBOX_COMMAND_TEMPLATE, LIVE_INBOX_RESOLVE_COMMAND_TEMPLATE,
        );
        Ok(LiveInboxFallback {
            inbox_command_template: LIVE_INBOX_COMMAND_TEMPLATE,
            resolve_command_template: LIVE_INBOX_RESOLVE_COMMAND_TEMPLATE,
        })
    }

    struct VerifiedLiveReceipt {
        canonical_receipt: AuthorityReceipt,
        project_id: String,
        task_id: String,
        state_version: u64,
        latest_run_id: String,
        close_state: StatusCloseState,
        close_blocker_count: usize,
    }

    fn verify_fresh_authority_receipt(
        status_json: Value,
        observation: &LiveUserActionObservation,
        expected_result_summary: &str,
    ) -> Result<VerifiedLiveReceipt, Box<dyn Error>> {
        verify_authority_receipt_binding(status_json, observation, expected_result_summary, true)
    }

    fn verify_current_final_authority_receipt(
        status_json: Value,
        observation: &LiveUserActionObservation,
        expected_result_summary: &str,
    ) -> Result<VerifiedLiveReceipt, Box<dyn Error>> {
        verify_authority_receipt_binding(status_json, observation, expected_result_summary, false)
    }

    fn verify_authority_receipt_binding(
        status_json: Value,
        observation: &LiveUserActionObservation,
        expected_result_summary: &str,
        require_ready: bool,
    ) -> Result<VerifiedLiveReceipt, Box<dyn Error>> {
        let status: StatusResult = serde_json::from_value(status_json)?;
        let state_version = status
            .base
            .state_version
            .ok_or_else(|| io::Error::other("fresh CLI status has no state_version"))?;
        let active_task = status
            .active_task
            .as_ref()
            .ok_or_else(|| io::Error::other("fresh CLI status has no active Task"))?;
        let active_task_ref = active_task
            .task_ref
            .as_ref()
            .ok_or_else(|| io::Error::other("fresh CLI status active Task has no task_ref"))?;
        let receipt = status
            .authority_receipt
            .as_ref()
            .ok_or_else(|| io::Error::other("fresh CLI status has no authority_receipt"))?;
        let latest_run_ref = receipt
            .latest_run_ref
            .as_ref()
            .ok_or_else(|| io::Error::other("fresh authority_receipt has no latest_run_ref"))?;
        let close_basis = status
            .current_close_basis
            .as_ref()
            .and_then(|basis| basis.as_ref())
            .ok_or_else(|| io::Error::other("fresh CLI status has no current close basis"))?;

        let receipt_matches = receipt.project_id.as_str() == observation.project_id
            && receipt.state_version == state_version
            && receipt.state_version == observation.state_version
            && receipt.task_ref.record_kind == StateRecordKind::Task
            && receipt.task_ref.project_id.as_str() == observation.project_id
            && receipt.task_ref.record_id.as_str() == observation.task_id
            && receipt
                .task_ref
                .task_id
                .as_ref()
                .map(|task_id| task_id.as_str())
                == Some(observation.task_id.as_str())
            && receipt.task_ref.produced_at_state_version.as_ref() == Some(&state_version)
            && active_task.project_id.as_str() == observation.project_id
            && active_task.state_version == state_version
            && active_task_ref == &receipt.task_ref
            && latest_run_ref.record_kind == StateRecordKind::Run
            && latest_run_ref.project_id.as_str() == observation.project_id
            && latest_run_ref
                .task_id
                .as_ref()
                .map(|task_id| task_id.as_str())
                == Some(observation.task_id.as_str())
            && latest_run_ref.produced_at_state_version.as_ref() == Some(&state_version)
            && close_basis.result_summary == expected_result_summary
            && &close_basis.source_run_ref == latest_run_ref;
        if !receipt_matches {
            return Err(io::Error::other(
                "fresh CLI authority_receipt is not bound to the observed project, Task, state_version, and latest Run",
            )
            .into());
        }
        if require_ready
            && (receipt.close_state != StatusCloseState::Ready
                || !receipt.close_blockers.is_empty()
                || status.close_state != Some(StatusCloseState::Ready)
                || status
                    .close_blockers
                    .as_ref()
                    .is_none_or(|blockers| !blockers.is_empty()))
        {
            return Err(io::Error::other(format!(
                "fresh CLI status is not ready to close with an empty close-blocker set: receipt close_state={:?}, receipt blockers={:?}, status close_state={:?}, status blockers={:?}",
                receipt.close_state,
                receipt.close_blockers,
                status.close_state,
                status.close_blockers
            ))
            .into());
        }

        Ok(VerifiedLiveReceipt {
            canonical_receipt: receipt.clone(),
            project_id: receipt.project_id.as_str().to_owned(),
            task_id: receipt.task_ref.record_id.as_str().to_owned(),
            state_version,
            latest_run_id: latest_run_ref.record_id.as_str().to_owned(),
            close_state: receipt.close_state,
            close_blocker_count: receipt.close_blockers.len(),
        })
    }

    fn confirm_native_user_action_choice(host: &str) -> Result<String, Box<dyn Error>> {
        print!(
            "\nConfirm the option you personally selected in the {host} native user-action UI. Type `choice:{USER_ACTION_ROUTE_ALPHA_OPTION_ID}` or `choice:{USER_ACTION_ROUTE_BETA_OPTION_ID}`: "
        );
        io::stdout().flush()?;
        let mut confirmation = String::new();
        if io::stdin().read_line(&mut confirmation)? == 0 {
            return Err(io::Error::other(
                "no operator confirmation was received for the native user-action selection",
            )
            .into());
        }
        parse_native_user_action_choice(confirmation.trim())
    }

    fn confirm_cli_fallback_choice(
        host: &str,
        user_action_request_id: &str,
    ) -> Result<String, Box<dyn Error>> {
        print!(
            "\nChoose the CLI User Channel answer for {host} request `{user_action_request_id}`. Type `choice:{USER_ACTION_ROUTE_ALPHA_OPTION_ID}` or `choice:{USER_ACTION_ROUTE_BETA_OPTION_ID}`; the harness will submit your selection through the actual `volicord inbox resolve --json` command: "
        );
        io::stdout().flush()?;
        let mut confirmation = String::new();
        if io::stdin().read_line(&mut confirmation)? == 0 {
            return Err(io::Error::other(
                "no operator selection was received for the CLI User Channel",
            )
            .into());
        }
        parse_native_user_action_choice(confirmation.trim())
    }

    fn parse_native_user_action_choice(confirmation: &str) -> Result<String, Box<dyn Error>> {
        let selected = confirmation
            .strip_prefix("choice:")
            .and_then(|option_id| run_marker_for_selected_option(option_id).map(|_| option_id))
            .ok_or_else(|| {
                io::Error::other(format!(
                    "operator selection confirmation must be `choice:{USER_ACTION_ROUTE_ALPHA_OPTION_ID}` or `choice:{USER_ACTION_ROUTE_BETA_OPTION_ID}`"
                ))
            })?;
        Ok(selected.to_owned())
    }

    fn confirm_live_evidence_summary(host: &str) -> Result<String, Box<dyn Error>> {
        print!(
            "\nConfirm the exact non-secret one-line evidence summary you personally submitted in the {host} local consent page. Type `summary:` followed by that summary: "
        );
        io::stdout().flush()?;
        let mut confirmation = String::new();
        if io::stdin().read_line(&mut confirmation)? == 0 {
            return Err(io::Error::other(
                "no operator confirmation was received for the local-web evidence summary",
            )
            .into());
        }
        parse_live_evidence_summary_confirmation(confirmation.trim_end_matches(['\r', '\n']))
    }

    fn parse_live_evidence_summary_confirmation(
        confirmation: &str,
    ) -> Result<String, Box<dyn Error>> {
        let summary = confirmation.strip_prefix("summary:").ok_or_else(|| {
            io::Error::other("operator evidence confirmation must start with `summary:`")
        })?;
        if summary.is_empty()
            || summary.chars().any(char::is_control)
            || summary.chars().count() > USER_ACTION_OBSERVATION_SUMMARY_MAX_CHARS
        {
            return Err(io::Error::other(format!(
                "operator evidence summary must be one non-empty printable line of at most {USER_ACTION_OBSERVATION_SUMMARY_MAX_CHARS} characters"
            ))
            .into());
        }
        Ok(summary.to_owned())
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct LiveLocalWebDeliveryBoundaryConfirmation {
        host_owned_model_invisible_surface_confirmed: bool,
        model_visible_forbidden_payloads_absent_confirmed: bool,
    }

    fn confirm_local_web_delivery_boundary(
        host: &str,
    ) -> Result<LiveLocalWebDeliveryBoundaryConfirmation, Box<dyn Error>> {
        print!(
            "\nConfirm the {host} handoff appeared in a separate host-owned model-invisible surface, never in chat or model output. Type `{MODEL_INVISIBLE_SURFACE_CONFIRMATION}` only if observed; type `unavailable` otherwise: "
        );
        io::stdout().flush()?;
        let mut surface_confirmation = String::new();
        if io::stdin().read_line(&mut surface_confirmation)? == 0 {
            return Err(io::Error::other(
                "no operator confirmation was received for the host-owned model-invisible surface",
            )
            .into());
        }

        print!(
            "\nReview the returned tool-result content, returned structured output, and host diagnostic text from this cell. Type `{MODEL_VISIBLE_ABSENCE_CONFIRMATION}` only if all three were observable and none contained a consent URL, bearer token, full form, question, or UserAction request ref; type `unavailable` otherwise: "
        );
        io::stdout().flush()?;
        let mut absence_confirmation = String::new();
        if io::stdin().read_line(&mut absence_confirmation)? == 0 {
            return Err(io::Error::other(
                "no operator confirmation was received for model-visible payload absence",
            )
            .into());
        }

        parse_local_web_delivery_boundary_confirmation(
            surface_confirmation.trim(),
            absence_confirmation.trim(),
        )
    }

    fn parse_local_web_delivery_boundary_confirmation(
        surface_confirmation: &str,
        absence_confirmation: &str,
    ) -> Result<LiveLocalWebDeliveryBoundaryConfirmation, Box<dyn Error>> {
        if surface_confirmation != MODEL_INVISIBLE_SURFACE_CONFIRMATION
            || absence_confirmation != MODEL_VISIBLE_ABSENCE_CONFIRMATION
        {
            return Err(io::Error::other(
                "the live local-web cell is unavailable without both exact delivery-boundary confirmations",
            )
            .into());
        }
        Ok(LiveLocalWebDeliveryBoundaryConfirmation {
            host_owned_model_invisible_surface_confirmed: true,
            model_visible_forbidden_payloads_absent_confirmed: true,
        })
    }

    #[derive(Clone, Debug)]
    enum FinalOutputUiExpectation {
        ManagedSurface,
        NoActiveTaskStatus { complete_message: String },
        CompleteAuthorityReceipt { canonical_json: String },
    }

    fn parse_final_output_ui_confirmation(
        confirmation: &str,
        expectation: &FinalOutputUiExpectation,
    ) -> Result<String, Box<dyn Error>> {
        let expected = match expectation {
            FinalOutputUiExpectation::ManagedSurface => "surface:managed-final-output",
            FinalOutputUiExpectation::NoActiveTaskStatus { complete_message } => {
                let observed = confirmation.strip_prefix("status-ui:").ok_or_else(|| {
                    io::Error::other(
                        "operator status-fallback confirmation must start with `status-ui:`",
                    )
                })?;
                if observed != complete_message {
                    return Err(io::Error::other(
                        "operator status-fallback confirmation does not exactly match the complete taskless managed-UI message",
                    )
                    .into());
                }
                return Ok(confirmation.to_owned());
            }
            FinalOutputUiExpectation::CompleteAuthorityReceipt { canonical_json } => {
                let observed = confirmation.strip_prefix("receipt-json:").ok_or_else(|| {
                    io::Error::other(
                        "operator receipt confirmation must start with `receipt-json:`",
                    )
                })?;
                if observed != canonical_json {
                    return Err(io::Error::other(
                        "operator receipt confirmation does not exactly match the complete canonical AuthorityReceipt",
                    )
                    .into());
                }
                return Ok(confirmation.to_owned());
            }
        };
        if confirmation != expected {
            return Err(io::Error::other(format!(
                "operator final-output confirmation must be exactly {expected:?}"
            ))
            .into());
        }
        Ok(expected.to_owned())
    }

    fn confirm_final_output_ui(
        host: &str,
        profile: IntegrationProfile,
        expectation: FinalOutputUiExpectation,
    ) -> Result<(), Box<dyn Error>> {
        let instruction = match &expectation {
            FinalOutputUiExpectation::ManagedSurface => "`surface:managed-final-output`".to_owned(),
            FinalOutputUiExpectation::NoActiveTaskStatus { .. } =>
                "`status-ui:` followed by the complete taskless fallback message copied from the managed UI".to_owned(),
            FinalOutputUiExpectation::CompleteAuthorityReceipt { .. } =>
                "`receipt-json:` followed by the complete canonical AuthorityReceipt JSON copied from the managed UI".to_owned(),
        };
        print!(
            "\nReview the separate {host}/{} managed final-output UI. Type {instruction} only if that exact evidence was visible; type `missing` otherwise: ",
            profile.as_str()
        );
        io::stdout().flush()?;
        let mut confirmation = String::new();
        if io::stdin().read_line(&mut confirmation)? == 0 {
            return Err(io::Error::other(
                "no operator confirmation was received for the managed final-output UI",
            )
            .into());
        }
        parse_final_output_ui_confirmation(confirmation.trim(), &expectation)?;
        Ok(())
    }

    struct LiveEvidenceCompletedSummaryInput<'a> {
        identity: &'a LiveHostIdentity,
        observation: &'a LiveEvidenceObservation,
        delivery_boundary: &'a LiveLocalWebDeliveryBoundaryConfirmation,
        operator_summary_character_count: usize,
        consumption: &'a LiveEvidenceConsumption,
        diagnostic: &'a LiveEvidenceDiagnosticObservation,
        diagnostic_payload_scan_passed: bool,
        authority_event_order: &'a AuthorityEventOrder,
        stop_observation: &'a VerifiedStopObservation,
        receipt: &'a VerifiedLiveReceipt,
    }

    fn live_evidence_observation_completed_summary(
        input: LiveEvidenceCompletedSummaryInput<'_>,
    ) -> Value {
        let LiveEvidenceCompletedSummaryInput {
            identity,
            observation,
            delivery_boundary,
            operator_summary_character_count,
            consumption,
            diagnostic,
            diagnostic_payload_scan_passed,
            authority_event_order,
            stop_observation,
            receipt,
        } = input;
        serde_json::json!({
            "kind": LIVE_EVIDENCE_OBSERVATION_RESULT_KIND,
            "result": "passed",
            "host": {
                "kind": identity.host,
                "version": identity.host_version
            },
            "volicord": {
                "build_id": identity.volicord_build_id
            },
            "connection": {
                "connection_id": identity.connection_id
            },
            "task": {
                "project_id": observation.project_id,
                "task_id": observation.task_id,
                "lifecycle_phase": observation.lifecycle_phase,
                "state_version": observation.state_version
            },
            "local_web_user_channel": {
                "handoff_delivery": {
                    "capability_namespace": MODEL_INVISIBLE_USER_CHANNEL_CAPABILITY_NAMESPACE,
                    "capability_field": MODEL_INVISIBLE_USER_CHANNEL_CAPABILITY_FIELD,
                    "capability_value": true,
                    "handoff_transport": "top_level_call_tool_result_meta",
                    "handoff_meta_key": MODEL_INVISIBLE_USER_CHANNEL_CAPABILITY_NAMESPACE,
                    "effective_exact_capability_observed": diagnostic.create_calls == 1,
                    "listener_enabled_for_cell": true,
                    "host_owned_model_invisible_surface_operator_confirmed": delivery_boundary.host_owned_model_invisible_surface_confirmed,
                    "agent_was_not_instructed_to_relay_or_open": true,
                    "negative_model_visible_observation": {
                        "surfaces": ["content", "structured_content", "diagnostic_text"],
                        "forbidden_categories": ["url", "token", "form", "question", "request_ref"],
                        "operator_confirmed_absent": delivery_boundary.model_visible_forbidden_payloads_absent_confirmed,
                        "diagnostic_store_scan_passed": diagnostic_payload_scan_passed
                    }
                },
                "request": {
                    "user_action_request_id": observation.user_action_request_id,
                    "action_kind": "evidence_observation",
                    "requested_by_actor_source": observation.requested_by_actor_source,
                    "target": observation.target,
                    "artifact_id": observation.artifact_ref.artifact_id
                },
                "resolution": {
                    "user_action_resolution_id": observation.user_action_resolution_id,
                    "actor_source": observation.resolved_by_actor_source,
                    "channel_kind": observation.resolved_channel_kind,
                    "verification_basis": observation.resolved_verification_basis,
                    "target": observation.target,
                    "artifact_id": observation.artifact_ref.artifact_id,
                    "relevance_status": "supported",
                    "operator_summary_exact_match": true,
                    "summary_character_count": operator_summary_character_count
                },
                "host_resume": {
                    "create_calls": diagnostic.create_calls,
                    "resume_calls": diagnostic.resume_calls,
                    "same_agent_connection": true,
                    "agent_workflow_result_replayed": diagnostic.resume_calls == 1,
                    "additional_evidence_observation_request_created": false,
                    "record_run_calls": diagnostic.record_run_calls,
                    "committed_record_run_calls": diagnostic.committed_record_run_calls,
                    "status_calls": diagnostic.status_calls,
                    "successful_status_calls": diagnostic.successful_status_calls,
                    "diagnostic_event_ordered": diagnostic.ordered
                }
            },
            "evidence_consumption": {
                "run_id": consumption.run.run_id,
                "run_kind": consumption.run.kind,
                "run_marker": consumption.run.summary,
                "created_by_actor_source": consumption.run.created_by_actor_source,
                "product_file_write_observed": consumption.run.product_file_write_observed,
                "changed_path_count": consumption.run.changed_paths.len(),
                "evidence_observation_id": consumption.evidence_observation_id,
                "observation_ref_id": consumption.observation_ref.record_id,
                "target": observation.target,
                "artifact_id": observation.artifact_ref.artifact_id,
                "source_kind": "user_observation",
                "assurance_level": "user_observed",
                "observed_by_actor_source": consumption.observed_by_actor_source,
                "input_resolution_ref_id": consumption.resolution_ref.record_id,
                "producer_anchor": {
                    "producer_kind": consumption.producer_kind,
                    "producer_ref_id": consumption.resolution_ref.record_id,
                    "verification_basis": consumption.producer_verification_basis
                },
                "relevance_assessment": {
                    "status": consumption.relevance_status,
                    "assessment_ref_id": consumption.resolution_ref.record_id,
                    "assessed_by_actor_source": consumption.assessed_by_actor_source
                },
                "coverage_state": consumption.coverage_state,
                "observed_at_matches_resolution": consumption.observed_at_matches_resolution,
                "caller_observed_at_replaced": true
            },
            "authority_events": {
                "user_action_requested_event_seq": authority_event_order.user_action_requested_event_seq,
                "user_action_resolved_event_seq": authority_event_order.user_action_resolved_event_seq,
                "run_recorded_event_seq": authority_event_order.run_recorded_event_seq,
                "ordered": authority_event_order.user_action_requested_event_seq
                    < authority_event_order.user_action_resolved_event_seq
                    && authority_event_order.user_action_resolved_event_seq
                        < authority_event_order.run_recorded_event_seq
            },
            "stop_hook": {
                "guard_event_id": stop_observation.guard_event_id,
                "session_id": stop_observation.session_id,
                "connection_id": stop_observation.connection_id,
                "decision": stop_observation.decision,
                "receipt_state_version": stop_observation.state_version,
                "latest_run_id": stop_observation.latest_run_id
            },
            "authority_receipt": {
                "project_id": receipt.project_id,
                "task_id": receipt.task_id,
                "state_version": receipt.state_version,
                "latest_run_id": receipt.latest_run_id,
                "close_state": receipt.close_state,
                "close_blocker_count": receipt.close_blocker_count,
                "complete_managed_ui_confirmed": true
            },
            "evidence_scope": {
                "live_evidence_observation_cell": true,
                "native_judgment_cell": false,
                "cli_fallback_cell": false,
                "final_output_matrix_cell": false
            },
            "sensitive_payloads": {
                "raw_url_recorded": false,
                "bearer_token_recorded": false,
                "raw_summary_recorded": false,
                "prompt_recorded": false,
                "transcript_recorded": false,
                "secret_material_recorded": false
            }
        })
    }

    fn live_evidence_observation_incomplete_summary(host: &str, stage: &str) -> Value {
        let result = match stage {
            "host_executable" | "interactive_terminal" | "host_delivery_boundary" => "unavailable",
            _ => "failed",
        };
        serde_json::json!({
            "kind": LIVE_EVIDENCE_OBSERVATION_RESULT_KIND,
            "result": result,
            "host": { "kind": host },
            "stage": stage,
            "evidence_scope": {
                "live_evidence_observation_cell": true,
                "native_judgment_cell": false,
                "cli_fallback_cell": false,
                "final_output_matrix_cell": false
            },
            "sensitive_payloads": {
                "raw_url_recorded": false,
                "bearer_token_recorded": false,
                "raw_summary_recorded": false,
                "prompt_recorded": false,
                "transcript_recorded": false,
                "secret_material_recorded": false
            }
        })
    }

    fn evidence_observation_result_shape_fixture() -> Value {
        serde_json::json!({
            "kind": LIVE_EVIDENCE_OBSERVATION_RESULT_KIND,
            "result": "passed",
            "host": { "kind": "codex", "version": "fixture-version" },
            "volicord": { "build_id": "fixture-build" },
            "connection": { "connection_id": "CONN-live" },
            "task": {
                "project_id": "PRJ-live",
                "task_id": "TASK-live",
                "lifecycle_phase": "ready_to_close",
                "state_version": 7
            },
            "local_web_user_channel": {
                "handoff_delivery": {
                    "capability_namespace": MODEL_INVISIBLE_USER_CHANNEL_CAPABILITY_NAMESPACE,
                    "capability_field": MODEL_INVISIBLE_USER_CHANNEL_CAPABILITY_FIELD,
                    "capability_value": true,
                    "handoff_transport": "top_level_call_tool_result_meta",
                    "handoff_meta_key": MODEL_INVISIBLE_USER_CHANNEL_CAPABILITY_NAMESPACE,
                    "effective_exact_capability_observed": true,
                    "listener_enabled_for_cell": true,
                    "host_owned_model_invisible_surface_operator_confirmed": true,
                    "agent_was_not_instructed_to_relay_or_open": true,
                    "negative_model_visible_observation": {
                        "surfaces": ["content", "structured_content", "diagnostic_text"],
                        "forbidden_categories": ["url", "token", "form", "question", "request_ref"],
                        "operator_confirmed_absent": true,
                        "diagnostic_store_scan_passed": true
                    }
                },
                "request": {
                    "user_action_request_id": "UAR-live",
                    "action_kind": "evidence_observation",
                    "requested_by_actor_source": "agent_connection:CONN-live",
                    "target": {
                        "target_kind": "acceptance_criterion",
                        "acceptance_criterion_id": "AC-live"
                    },
                    "artifact_id": "ART-live"
                },
                "resolution": {
                    "user_action_resolution_id": "URES-live",
                    "actor_source": "local_user",
                    "channel_kind": "local_web_consent",
                    "verification_basis": VERIFICATION_BASIS_LOCAL_USER_LOCAL_WEB,
                    "target": {
                        "target_kind": "acceptance_criterion",
                        "acceptance_criterion_id": "AC-live"
                    },
                    "artifact_id": "ART-live",
                    "relevance_status": "supported",
                    "operator_summary_exact_match": true,
                    "summary_character_count": 29
                },
                "host_resume": {
                    "create_calls": 1,
                    "resume_calls": 1,
                    "same_agent_connection": true,
                    "agent_workflow_result_replayed": true,
                    "additional_evidence_observation_request_created": false,
                    "record_run_calls": 1,
                    "committed_record_run_calls": 1,
                    "status_calls": 1,
                    "successful_status_calls": 1,
                    "diagnostic_event_ordered": true
                }
            },
            "evidence_consumption": {
                "run_id": "RUN-live",
                "run_kind": "shaping_update",
                "run_marker": LIVE_EVIDENCE_OBSERVATION_RUN_MARKER,
                "created_by_actor_source": "agent_connection:CONN-live",
                "product_file_write_observed": false,
                "changed_path_count": 0,
                "evidence_observation_id": "EOBS-live",
                "observation_ref_id": "EOBS-live",
                "target": {
                    "target_kind": "acceptance_criterion",
                    "acceptance_criterion_id": "AC-live"
                },
                "artifact_id": "ART-live",
                "source_kind": "user_observation",
                "assurance_level": "user_observed",
                "observed_by_actor_source": "local_user",
                "input_resolution_ref_id": "URES-live",
                "producer_anchor": {
                    "producer_kind": "user_channel_observation",
                    "producer_ref_id": "URES-live",
                    "verification_basis": VERIFICATION_BASIS_LOCAL_USER_LOCAL_WEB
                },
                "relevance_assessment": {
                    "status": "supported",
                    "assessment_ref_id": "URES-live",
                    "assessed_by_actor_source": "local_user"
                },
                "coverage_state": "supported",
                "observed_at_matches_resolution": true,
                "caller_observed_at_replaced": true
            },
            "authority_events": {
                "user_action_requested_event_seq": 2,
                "user_action_resolved_event_seq": 3,
                "run_recorded_event_seq": 4,
                "ordered": true
            },
            "stop_hook": {
                "guard_event_id": "GE-live",
                "session_id": "SESSION-live",
                "connection_id": "CONN-live",
                "decision": "allow",
                "receipt_state_version": 7,
                "latest_run_id": "RUN-live"
            },
            "authority_receipt": {
                "project_id": "PRJ-live",
                "task_id": "TASK-live",
                "state_version": 7,
                "latest_run_id": "RUN-live",
                "close_state": "ready",
                "close_blocker_count": 0,
                "complete_managed_ui_confirmed": true
            },
            "evidence_scope": {
                "live_evidence_observation_cell": true,
                "native_judgment_cell": false,
                "cli_fallback_cell": false,
                "final_output_matrix_cell": false
            },
            "sensitive_payloads": {
                "raw_url_recorded": false,
                "bearer_token_recorded": false,
                "raw_summary_recorded": false,
                "prompt_recorded": false,
                "transcript_recorded": false,
                "secret_material_recorded": false
            }
        })
    }

    fn require_exact_live_evidence_result_keys(
        value: &Value,
        pointer: &str,
        expected: &[&str],
    ) -> Result<(), Box<dyn Error>> {
        let selected = if pointer.is_empty() {
            value
        } else {
            value.pointer(pointer).ok_or_else(|| {
                io::Error::other(format!("live evidence result has no object at {pointer}"))
            })?
        };
        let object = selected.as_object().ok_or_else(|| {
            io::Error::other(format!(
                "live evidence result value at {pointer:?} is not an object"
            ))
        })?;
        if object.len() != expected.len() || !expected.iter().all(|key| object.contains_key(*key)) {
            let mut actual = object.keys().map(String::as_str).collect::<Vec<_>>();
            actual.sort_unstable();
            return Err(io::Error::other(format!(
                "live evidence result object at {pointer:?} has non-canonical keys {actual:?}"
            ))
            .into());
        }
        Ok(())
    }

    fn validate_live_evidence_observation_passed_result_keys(
        value: &Value,
    ) -> Result<(), Box<dyn Error>> {
        require_exact_live_evidence_result_keys(
            value,
            "",
            &[
                "kind",
                "result",
                "host",
                "volicord",
                "connection",
                "task",
                "local_web_user_channel",
                "evidence_consumption",
                "authority_events",
                "stop_hook",
                "authority_receipt",
                "evidence_scope",
                "sensitive_payloads",
            ],
        )?;
        for (pointer, keys) in [
            ("/host", &["kind", "version"][..]),
            ("/volicord", &["build_id"][..]),
            ("/connection", &["connection_id"][..]),
            (
                "/task",
                &["project_id", "task_id", "lifecycle_phase", "state_version"][..],
            ),
            (
                "/local_web_user_channel",
                &["handoff_delivery", "request", "resolution", "host_resume"][..],
            ),
            (
                "/local_web_user_channel/handoff_delivery",
                &[
                    "capability_namespace",
                    "capability_field",
                    "capability_value",
                    "handoff_transport",
                    "handoff_meta_key",
                    "effective_exact_capability_observed",
                    "listener_enabled_for_cell",
                    "host_owned_model_invisible_surface_operator_confirmed",
                    "agent_was_not_instructed_to_relay_or_open",
                    "negative_model_visible_observation",
                ][..],
            ),
            (
                "/local_web_user_channel/handoff_delivery/negative_model_visible_observation",
                &[
                    "surfaces",
                    "forbidden_categories",
                    "operator_confirmed_absent",
                    "diagnostic_store_scan_passed",
                ][..],
            ),
            (
                "/local_web_user_channel/request",
                &[
                    "user_action_request_id",
                    "action_kind",
                    "requested_by_actor_source",
                    "target",
                    "artifact_id",
                ][..],
            ),
            (
                "/local_web_user_channel/request/target",
                &["target_kind", "acceptance_criterion_id"][..],
            ),
            (
                "/local_web_user_channel/resolution",
                &[
                    "user_action_resolution_id",
                    "actor_source",
                    "channel_kind",
                    "verification_basis",
                    "target",
                    "artifact_id",
                    "relevance_status",
                    "operator_summary_exact_match",
                    "summary_character_count",
                ][..],
            ),
            (
                "/local_web_user_channel/resolution/target",
                &["target_kind", "acceptance_criterion_id"][..],
            ),
            (
                "/local_web_user_channel/host_resume",
                &[
                    "create_calls",
                    "resume_calls",
                    "same_agent_connection",
                    "agent_workflow_result_replayed",
                    "additional_evidence_observation_request_created",
                    "record_run_calls",
                    "committed_record_run_calls",
                    "status_calls",
                    "successful_status_calls",
                    "diagnostic_event_ordered",
                ][..],
            ),
            (
                "/evidence_consumption",
                &[
                    "run_id",
                    "run_kind",
                    "run_marker",
                    "created_by_actor_source",
                    "product_file_write_observed",
                    "changed_path_count",
                    "evidence_observation_id",
                    "observation_ref_id",
                    "target",
                    "artifact_id",
                    "source_kind",
                    "assurance_level",
                    "observed_by_actor_source",
                    "input_resolution_ref_id",
                    "producer_anchor",
                    "relevance_assessment",
                    "coverage_state",
                    "observed_at_matches_resolution",
                    "caller_observed_at_replaced",
                ][..],
            ),
            (
                "/evidence_consumption/target",
                &["target_kind", "acceptance_criterion_id"][..],
            ),
            (
                "/evidence_consumption/producer_anchor",
                &["producer_kind", "producer_ref_id", "verification_basis"][..],
            ),
            (
                "/evidence_consumption/relevance_assessment",
                &["status", "assessment_ref_id", "assessed_by_actor_source"][..],
            ),
            (
                "/authority_events",
                &[
                    "user_action_requested_event_seq",
                    "user_action_resolved_event_seq",
                    "run_recorded_event_seq",
                    "ordered",
                ][..],
            ),
            (
                "/stop_hook",
                &[
                    "guard_event_id",
                    "session_id",
                    "connection_id",
                    "decision",
                    "receipt_state_version",
                    "latest_run_id",
                ][..],
            ),
            (
                "/authority_receipt",
                &[
                    "project_id",
                    "task_id",
                    "state_version",
                    "latest_run_id",
                    "close_state",
                    "close_blocker_count",
                    "complete_managed_ui_confirmed",
                ][..],
            ),
        ] {
            require_exact_live_evidence_result_keys(value, pointer, keys)?;
        }
        validate_live_evidence_observation_common_result_keys(value)
    }

    fn validate_live_evidence_observation_incomplete_result_keys(
        value: &Value,
    ) -> Result<(), Box<dyn Error>> {
        require_exact_live_evidence_result_keys(
            value,
            "",
            &[
                "kind",
                "result",
                "host",
                "stage",
                "evidence_scope",
                "sensitive_payloads",
            ],
        )?;
        require_exact_live_evidence_result_keys(value, "/host", &["kind"])?;
        validate_live_evidence_observation_common_result_keys(value)
    }

    fn validate_live_evidence_observation_common_result_keys(
        value: &Value,
    ) -> Result<(), Box<dyn Error>> {
        require_exact_live_evidence_result_keys(
            value,
            "/evidence_scope",
            &[
                "live_evidence_observation_cell",
                "native_judgment_cell",
                "cli_fallback_cell",
                "final_output_matrix_cell",
            ],
        )?;
        require_exact_live_evidence_result_keys(
            value,
            "/sensitive_payloads",
            &[
                "raw_url_recorded",
                "bearer_token_recorded",
                "raw_summary_recorded",
                "prompt_recorded",
                "transcript_recorded",
                "secret_material_recorded",
            ],
        )
    }

    fn validate_live_evidence_observation_result_shape(
        value: &Value,
    ) -> Result<(), Box<dyn Error>> {
        validate_live_evidence_observation_passed_result_keys(value)?;
        reject_forbidden_live_evidence_result_fields(value)?;
        required_result_string(value, "/host/kind")?;
        required_result_string(value, "/host/version")?;
        required_result_string(value, "/volicord/build_id")?;
        let connection_id = required_result_string(value, "/connection/connection_id")?;
        let project_id = required_result_string(value, "/task/project_id")?;
        let task_id = required_result_string(value, "/task/task_id")?;
        required_result_string(value, "/task/lifecycle_phase")?;
        let request_id = required_result_string(
            value,
            "/local_web_user_channel/request/user_action_request_id",
        )?;
        let resolution_id = required_result_string(
            value,
            "/local_web_user_channel/resolution/user_action_resolution_id",
        )?;
        let run_id = required_result_string(value, "/evidence_consumption/run_id")?;
        let evidence_observation_id =
            required_result_string(value, "/evidence_consumption/evidence_observation_id")?;
        let requested_event_seq =
            required_result_u64(value, "/authority_events/user_action_requested_event_seq")?;
        let resolved_event_seq =
            required_result_u64(value, "/authority_events/user_action_resolved_event_seq")?;
        let run_event_seq = required_result_u64(value, "/authority_events/run_recorded_event_seq")?;
        let task_state_version = required_result_u64(value, "/task/state_version")?;
        let summary_character_count = required_result_u64(
            value,
            "/local_web_user_channel/resolution/summary_character_count",
        )?;
        let expected_actor_source = format!("agent_connection:{connection_id}");
        let request_target = value
            .pointer("/local_web_user_channel/request/target")
            .ok_or_else(|| io::Error::other("evidence result has no request target"))?;
        if !matches!(
            serde_json::from_value::<EvidenceTarget>(request_target.clone())?,
            EvidenceTarget::AcceptanceCriterion { .. }
        ) {
            return Err(io::Error::other(
                "live evidence result target is not an acceptance criterion",
            )
            .into());
        }
        let request_artifact_id =
            required_result_string(value, "/local_web_user_channel/request/artifact_id")?;
        let handoff_delivery = &value["local_web_user_channel"]["handoff_delivery"];
        let negative_observation = &handoff_delivery["negative_model_visible_observation"];
        required_result_string(value, "/stop_hook/guard_event_id")?;
        required_result_string(value, "/stop_hook/session_id")?;
        if value["kind"] != LIVE_EVIDENCE_OBSERVATION_RESULT_KIND
            || value["result"] != "passed"
            || request_id.is_empty()
            || resolution_id.is_empty()
            || evidence_observation_id.is_empty()
            || handoff_delivery["capability_namespace"]
                != MODEL_INVISIBLE_USER_CHANNEL_CAPABILITY_NAMESPACE
            || handoff_delivery["capability_field"] != MODEL_INVISIBLE_USER_CHANNEL_CAPABILITY_FIELD
            || handoff_delivery["capability_value"] != true
            || handoff_delivery["handoff_transport"] != "top_level_call_tool_result_meta"
            || handoff_delivery["handoff_meta_key"]
                != MODEL_INVISIBLE_USER_CHANNEL_CAPABILITY_NAMESPACE
            || handoff_delivery["effective_exact_capability_observed"] != true
            || handoff_delivery["listener_enabled_for_cell"] != true
            || handoff_delivery["host_owned_model_invisible_surface_operator_confirmed"] != true
            || handoff_delivery["agent_was_not_instructed_to_relay_or_open"] != true
            || negative_observation["surfaces"]
                != serde_json::json!(["content", "structured_content", "diagnostic_text"])
            || negative_observation["forbidden_categories"]
                != serde_json::json!(["url", "token", "form", "question", "request_ref"])
            || negative_observation["operator_confirmed_absent"] != true
            || negative_observation["diagnostic_store_scan_passed"] != true
            || value["local_web_user_channel"]["request"]["action_kind"] != "evidence_observation"
            || value["local_web_user_channel"]["request"]["requested_by_actor_source"]
                != expected_actor_source
            || value["local_web_user_channel"]["resolution"]["actor_source"] != "local_user"
            || value["local_web_user_channel"]["resolution"]["channel_kind"] != "local_web_consent"
            || value["local_web_user_channel"]["resolution"]["verification_basis"]
                != VERIFICATION_BASIS_LOCAL_USER_LOCAL_WEB
            || value["local_web_user_channel"]["resolution"]["target"] != *request_target
            || value["evidence_consumption"]["target"] != *request_target
            || value["local_web_user_channel"]["resolution"]["artifact_id"] != request_artifact_id
            || value["evidence_consumption"]["artifact_id"] != request_artifact_id
            || value["local_web_user_channel"]["resolution"]["relevance_status"] != "supported"
            || value["local_web_user_channel"]["resolution"]["operator_summary_exact_match"] != true
            || summary_character_count == 0
            || summary_character_count > USER_ACTION_OBSERVATION_SUMMARY_MAX_CHARS as u64
            || value["local_web_user_channel"]["host_resume"]["create_calls"] != 1
            || value["local_web_user_channel"]["host_resume"]["resume_calls"] != 1
            || value["local_web_user_channel"]["host_resume"]["same_agent_connection"] != true
            || value["local_web_user_channel"]["host_resume"]["agent_workflow_result_replayed"]
                != true
            || value["local_web_user_channel"]["host_resume"]
                ["additional_evidence_observation_request_created"]
                != false
            || value["local_web_user_channel"]["host_resume"]["record_run_calls"] != 1
            || value["local_web_user_channel"]["host_resume"]["committed_record_run_calls"] != 1
            || value["local_web_user_channel"]["host_resume"]["status_calls"] != 1
            || value["local_web_user_channel"]["host_resume"]["successful_status_calls"] != 1
            || value["local_web_user_channel"]["host_resume"]["diagnostic_event_ordered"] != true
            || value["evidence_consumption"]["run_kind"] != "shaping_update"
            || value["evidence_consumption"]["run_marker"] != LIVE_EVIDENCE_OBSERVATION_RUN_MARKER
            || value["evidence_consumption"]["created_by_actor_source"] != expected_actor_source
            || value["evidence_consumption"]["product_file_write_observed"] != false
            || value["evidence_consumption"]["changed_path_count"] != 0
            || value["evidence_consumption"]["observation_ref_id"] != evidence_observation_id
            || value["evidence_consumption"]["source_kind"] != "user_observation"
            || value["evidence_consumption"]["assurance_level"] != "user_observed"
            || value["evidence_consumption"]["observed_by_actor_source"] != "local_user"
            || value["evidence_consumption"]["input_resolution_ref_id"] != resolution_id
            || value["evidence_consumption"]["producer_anchor"]["producer_kind"]
                != "user_channel_observation"
            || value["evidence_consumption"]["producer_anchor"]["producer_ref_id"] != resolution_id
            || value["evidence_consumption"]["producer_anchor"]["verification_basis"]
                != VERIFICATION_BASIS_LOCAL_USER_LOCAL_WEB
            || value["evidence_consumption"]["relevance_assessment"]["status"] != "supported"
            || value["evidence_consumption"]["relevance_assessment"]["assessment_ref_id"]
                != resolution_id
            || value["evidence_consumption"]["relevance_assessment"]["assessed_by_actor_source"]
                != "local_user"
            || value["evidence_consumption"]["coverage_state"] != "supported"
            || value["evidence_consumption"]["observed_at_matches_resolution"] != true
            || value["evidence_consumption"]["caller_observed_at_replaced"] != true
            || requested_event_seq == 0
            || !(requested_event_seq < resolved_event_seq && resolved_event_seq < run_event_seq)
            || value["authority_events"]["ordered"] != true
            || value["stop_hook"]["connection_id"] != connection_id
            || value["stop_hook"]["decision"] != "allow"
            || value["stop_hook"]["latest_run_id"] != run_id
            || value["authority_receipt"]["project_id"] != project_id
            || value["authority_receipt"]["task_id"] != task_id
            || value["authority_receipt"]["state_version"] != task_state_version
            || value["stop_hook"]["receipt_state_version"] != task_state_version
            || value["authority_receipt"]["latest_run_id"] != run_id
            || value["authority_receipt"]["close_state"] != "ready"
            || value["authority_receipt"]["close_blocker_count"] != 0
            || value["authority_receipt"]["complete_managed_ui_confirmed"] != true
            || value["evidence_scope"]["live_evidence_observation_cell"] != true
            || value["evidence_scope"]["native_judgment_cell"] != false
            || value["evidence_scope"]["cli_fallback_cell"] != false
            || value["evidence_scope"]["final_output_matrix_cell"] != false
            || !live_evidence_sensitive_payload_flags_are_false(value)
        {
            return Err(io::Error::other(
                "passing evidence-observation result does not preserve the exact separated live evidence",
            )
            .into());
        }
        Ok(())
    }

    fn validate_live_evidence_observation_incomplete_result_shape(
        value: &Value,
    ) -> Result<(), Box<dyn Error>> {
        validate_live_evidence_observation_incomplete_result_keys(value)?;
        reject_forbidden_live_evidence_result_fields(value)?;
        required_result_string(value, "/host/kind")?;
        let stage = required_result_string(value, "/stage")?;
        let expected_result = match stage {
            "host_executable" | "interactive_terminal" | "host_delivery_boundary" => "unavailable",
            "fixture_setup"
            | "host_process"
            | "stored_resolution"
            | "authority_receipt"
            | "stop_and_diagnostics"
            | "managed_receipt_ui"
            | "result_validation" => "failed",
            _ => {
                return Err(io::Error::other(
                    "incomplete evidence-observation result has an unknown safe stage",
                )
                .into());
            }
        };
        if value["kind"] != LIVE_EVIDENCE_OBSERVATION_RESULT_KIND
            || value["result"] != expected_result
            || value["evidence_scope"]["live_evidence_observation_cell"] != true
            || value["evidence_scope"]["native_judgment_cell"] != false
            || value["evidence_scope"]["cli_fallback_cell"] != false
            || value["evidence_scope"]["final_output_matrix_cell"] != false
            || !live_evidence_sensitive_payload_flags_are_false(value)
        {
            return Err(io::Error::other(
                "incomplete evidence-observation result is not a bounded separated cell result",
            )
            .into());
        }
        Ok(())
    }

    fn required_result_string<'a>(
        value: &'a Value,
        pointer: &str,
    ) -> Result<&'a str, Box<dyn Error>> {
        value
            .pointer(pointer)
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty() && !value.chars().any(char::is_control))
            .ok_or_else(|| {
                io::Error::other(format!(
                    "live evidence result has no bounded string at {pointer}"
                ))
                .into()
            })
    }

    fn required_result_u64(value: &Value, pointer: &str) -> Result<u64, Box<dyn Error>> {
        value
            .pointer(pointer)
            .and_then(Value::as_u64)
            .ok_or_else(|| {
                io::Error::other(format!(
                    "live evidence result has no unsigned integer at {pointer}"
                ))
                .into()
            })
    }

    fn live_evidence_sensitive_payload_flags_are_false(value: &Value) -> bool {
        let expected_fields = [
            "raw_url_recorded",
            "bearer_token_recorded",
            "raw_summary_recorded",
            "prompt_recorded",
            "transcript_recorded",
            "secret_material_recorded",
        ];
        value["sensitive_payloads"]
            .as_object()
            .is_some_and(|object| {
                object.len() == expected_fields.len()
                    && expected_fields
                        .into_iter()
                        .all(|field| object.get(field) == Some(&Value::Bool(false)))
            })
    }

    fn reject_forbidden_live_evidence_result_fields(value: &Value) -> Result<(), Box<dyn Error>> {
        fn visit(value: &Value) -> Option<&str> {
            match value {
                Value::Object(object) => object.iter().find_map(|(key, value)| {
                    matches!(
                        key.as_str(),
                        "raw_url"
                            | "bearer_token"
                            | "raw_summary"
                            | "prompt"
                            | "transcript"
                            | "screenshot"
                            | "recording"
                            | "credential"
                            | "credentials"
                            | "secret"
                            | "secrets"
                    )
                    .then_some(key.as_str())
                    .or_else(|| visit(value))
                }),
                Value::Array(values) => values.iter().find_map(visit),
                Value::String(text) => {
                    let normalized = text.to_ascii_lowercase();
                    (normalized.contains("http://")
                        || normalized.contains("https://")
                        || normalized.contains("token="))
                    .then_some("URL-or-token-like string")
                }
                _ => None,
            }
        }
        if let Some(field) = visit(value) {
            return Err(io::Error::other(format!(
                "live evidence result contains forbidden sensitive payload field or value {field:?}"
            ))
            .into());
        }
        Ok(())
    }

    fn set_nested_value(
        value: &mut Value,
        path: &[&str],
        replacement: Value,
    ) -> Result<(), Box<dyn Error>> {
        let (field, parents) = path
            .split_last()
            .ok_or_else(|| io::Error::other("nested result mutation path is empty"))?;
        let mut current = value;
        for parent in parents {
            current = current
                .get_mut(*parent)
                .ok_or_else(|| io::Error::other("nested result mutation parent is missing"))?;
        }
        let object = current
            .as_object_mut()
            .ok_or_else(|| io::Error::other("nested result mutation parent is not an object"))?;
        if !object.contains_key(*field) {
            return Err(io::Error::other("nested result mutation field is missing").into());
        }
        object.insert((*field).to_owned(), replacement);
        Ok(())
    }

    fn cli_fallback_result_shape_fixture() -> Value {
        serde_json::json!({
            "kind": LIVE_CLI_FALLBACK_RESULT_KIND,
            "result": "passed",
            "connection": { "connection_id": "CONN-cli-fallback-fixture" },
            "task": {
                "project_id": "PRJ-cli-fallback-fixture",
                "task_id": "TASK-cli-fallback-fixture",
                "state_version": 5
            },
            "cli_user_channel": {
                "inbox": { "prepared_request_visible": true },
                "resolution": {
                    "user_action_request_id": "UAR-cli-fallback-fixture",
                    "user_action_resolution_id": "UARES-cli-fallback-fixture",
                    "operator_selected_option_id": USER_ACTION_ROUTE_ALPHA_OPTION_ID,
                    "stored_selected_option_id": USER_ACTION_ROUTE_ALPHA_OPTION_ID,
                    "actor_source": "local_user",
                    "channel_kind": "cli",
                    "verification_basis": VERIFICATION_BASIS_CLI_DIRECT_USER_CHANNEL,
                    "state_version_before_resolution": 3,
                    "committed_state_version": 4
                },
                "exact_retry": {
                    "same_command_and_arguments": true,
                    "stdout_byte_identical": true,
                    "state_version": 4,
                    "state_version_unchanged": true
                }
            },
            "host_resume": {
                "request_operation": "resume",
                "same_agent_connection": true,
                "origin_result_replayed_in_host_diagnostics": true,
                "resolved_choice_consumed": true,
                "additional_product_decision_request_created": false
            },
            "choice_consumption": {
                "run_id": "RUN-cli-fallback-fixture",
                "run_kind": "shaping_update",
                "run_marker": USER_ACTION_ROUTE_ALPHA_RUN_MARKER,
                "created_by_actor_source": "agent_connection:CONN-cli-fallback-fixture",
                "product_file_write_observed": false,
                "changed_path_count": 0
            },
            "authority_events": {
                "user_action_requested_event_seq": 10,
                "user_action_resolved_event_seq": 11,
                "run_recorded_event_seq": 12,
                "ordered": true
            },
            "stop_hook": {
                "connection_id": "CONN-cli-fallback-fixture",
                "decision": "allow",
                "receipt_state_version": 5,
                "latest_run_id": "RUN-cli-fallback-fixture"
            },
            "authority_receipt": {
                "project_id": "PRJ-cli-fallback-fixture",
                "task_id": "TASK-cli-fallback-fixture",
                "state_version": 5,
                "latest_run_id": "RUN-cli-fallback-fixture",
                "close_state": "ready",
                "close_blocker_count": 0,
                "complete_managed_ui_confirmed": true
            },
            "evidence_scope": {
                "cli_fallback_release_cell": true,
                "native_judgment_cell": false,
                "final_output_matrix_cell": false
            }
        })
    }

    fn final_output_result_shape_fixture(profile: IntegrationProfile) -> Value {
        let detective_decision = match profile {
            IntegrationProfile::Record => serde_json::json!({
                "status": "not_applicable",
                "non_observing": true,
                "non_gating": true
            }),
            IntegrationProfile::Detective => serde_json::json!({
                "status": "verified",
                "historical_decision": { "status": "verified" },
                "fresh_display": { "status": "verified" },
                "allow": { "status": "verified" },
                "block": { "status": "unavailable" }
            }),
        };
        let actual_event_branch = match profile {
            IntegrationProfile::Record => serde_json::json!({
                "status": "verified",
                "source": "authenticated_host_owned_surface_delivery",
                "delivery_evidence": "managed_final_output_ui",
                "persistent_guard_event": false,
                "non_observing": true
            }),
            IntegrationProfile::Detective => serde_json::json!({
                "status": "verified",
                "source": "persisted_guard_event",
                "persistent_guard_event": true
            }),
        };
        serde_json::json!({
            "kind": LIVE_FINAL_OUTPUT_RESULT_KIND,
            "result": "incomplete",
            "host": { "kind": "fixture" },
            "profile": profile.as_str(),
            "evidence": {
                "config_fixture": { "status": "verified" },
                "generated_wrapper_direct_wire": {
                    "status": "verified",
                    "status_fallback": { "status": "verified" },
                    "authority_receipt": {
                        "status": "verified",
                        "first_state_version": 42,
                        "refreshed_state_version": 43
                    }
                },
                "actual_host_event": {
                    "status": "verified",
                    "status_fallback_event": actual_event_branch.clone(),
                    "authority_receipt_event": actual_event_branch
                },
                "actual_host_fixed_ui": {
                    "status": "verified",
                    "status_fallback": {
                        "status": "verified",
                        "complete_taskless_message_operator_confirmed": true
                    },
                    "authority_receipt": {
                        "status": "verified",
                        "complete_canonical_receipt_operator_confirmed": true,
                        "project_id": "PRJ-fixture",
                        "task_id": "TASK-fixture",
                        "state_version": 42,
                        "latest_run_id": "RUN-fixture",
                        "close_state": "ready",
                        "close_blocker_count": 0
                    }
                },
                "detective_decision": detective_decision,
                "status_fallback": {
                    "status": "verified",
                    "no_active_task": true,
                    "generated_wire_command": "volicord status --json",
                    "operator_confirmed_actual_host_ui": true,
                    "complete_taskless_message_operator_confirmed": true,
                    "task_bound_command_absent": true
                },
                "exact_replay": {
                    "status": "unavailable",
                    "generated_wrapper_identical_payload": {
                        "status": "verified",
                        "state_advanced_between_deliveries": true,
                        "first_receipt_state_version": 42,
                        "refreshed_receipt_state_version": 43,
                        "fresh_current_receipt_displayed": true,
                        "record_non_observing_preserved": profile == IntegrationProfile::Record,
                        "detective_historical_guard_event_preserved": profile == IntegrationProfile::Detective
                    },
                    "actual_host_replay": {
                        "status": "unavailable",
                        "reason": "the installed host exposes no authenticated replay entry point"
                    }
                }
            }
        })
    }

    fn final_output_unavailable_summary(
        host: &str,
        profile: IntegrationProfile,
        reason: &str,
    ) -> Value {
        let unavailable = || serde_json::json!({ "status": "unavailable", "reason": reason });
        let detective_decision = unavailable();
        serde_json::json!({
            "kind": LIVE_FINAL_OUTPUT_RESULT_KIND,
            "result": "incomplete",
            "host": { "kind": host },
            "profile": profile.as_str(),
            "evidence": {
                "config_fixture": unavailable(),
                "generated_wrapper_direct_wire": {
                    "status": "unavailable",
                    "reason": reason,
                    "status_fallback": unavailable(),
                    "authority_receipt": unavailable()
                },
                "actual_host_event": unavailable(),
                "actual_host_fixed_ui": {
                    "status": "unavailable",
                    "reason": reason,
                    "status_fallback": unavailable(),
                    "authority_receipt": unavailable()
                },
                "detective_decision": detective_decision,
                "status_fallback": unavailable(),
                "exact_replay": {
                    "status": "unavailable",
                    "generated_wrapper_identical_payload": unavailable(),
                    "actual_host_replay": unavailable()
                }
            }
        })
    }

    fn validate_final_output_result_shape(
        value: &Value,
        profile: IntegrationProfile,
    ) -> Result<(), Box<dyn Error>> {
        fn status<'a>(value: &'a Value, label: &str) -> Result<&'a str, io::Error> {
            let status = value
                .get("status")
                .and_then(Value::as_str)
                .ok_or_else(|| io::Error::other(format!("evidence {label:?} has no status")))?;
            if !matches!(
                status,
                "verified" | "unavailable" | "not_applicable" | "failed"
            ) {
                return Err(io::Error::other(format!(
                    "evidence {label:?} has unsupported status {status:?}"
                )));
            }
            Ok(status)
        }

        if value["kind"] != LIVE_FINAL_OUTPUT_RESULT_KIND || value["profile"] != profile.as_str() {
            return Err(io::Error::other(
                "live final-output result kind/profile does not match the selected matrix cell",
            )
            .into());
        }
        let result = value["result"]
            .as_str()
            .ok_or_else(|| io::Error::other("live final-output result has no result status"))?;
        if !matches!(result, "passed" | "incomplete") {
            return Err(io::Error::other(format!(
                "live final-output result has unsupported result status {result:?}"
            ))
            .into());
        }
        let evidence = value["evidence"]
            .as_object()
            .ok_or_else(|| io::Error::other("live final-output result has no evidence object"))?;
        for key in [
            "config_fixture",
            "generated_wrapper_direct_wire",
            "actual_host_event",
            "actual_host_fixed_ui",
            "detective_decision",
            "status_fallback",
            "exact_replay",
        ] {
            status(
                evidence
                    .get(key)
                    .ok_or_else(|| io::Error::other(format!("evidence {key:?} is missing")))?,
                key,
            )?;
        }
        let replay = evidence
            .get("exact_replay")
            .ok_or_else(|| io::Error::other("exact_replay evidence is missing"))?;
        let generated_replay_status = status(
            replay
                .get("generated_wrapper_identical_payload")
                .ok_or_else(|| {
                    io::Error::other("generated-wrapper exact replay evidence is missing")
                })?,
            "exact_replay.generated_wrapper_identical_payload",
        )?;
        let actual_replay_status = status(
            replay
                .get("actual_host_replay")
                .ok_or_else(|| io::Error::other("actual-host exact replay evidence is missing"))?,
            "exact_replay.actual_host_replay",
        )?;
        if status(replay, "exact_replay")? == "verified"
            && (generated_replay_status != "verified" || actual_replay_status != "verified")
        {
            return Err(io::Error::other(
                "verified exact replay requires both generated-wrapper and actual-host replay evidence",
            )
            .into());
        }
        if generated_replay_status == "verified" {
            let generated = &replay["generated_wrapper_identical_payload"];
            let first_state_version = generated["first_receipt_state_version"]
                .as_u64()
                .ok_or_else(|| {
                    io::Error::other("generated-wrapper replay has no first receipt state_version")
                })?;
            let refreshed_state_version = generated["refreshed_receipt_state_version"]
                .as_u64()
                .ok_or_else(|| {
                    io::Error::other(
                        "generated-wrapper replay has no refreshed receipt state_version",
                    )
                })?;
            if generated["state_advanced_between_deliveries"] != true
                || generated["fresh_current_receipt_displayed"] != true
                || refreshed_state_version <= first_state_version
                || evidence["generated_wrapper_direct_wire"]["authority_receipt"]
                    ["first_state_version"]
                    != first_state_version
                || evidence["generated_wrapper_direct_wire"]["authority_receipt"]
                    ["refreshed_state_version"]
                    != refreshed_state_version
            {
                return Err(io::Error::other(
                    "verified generated-wrapper replay must advance state and display the fresh current receipt without collapsing its two receipt versions",
                )
                .into());
            }
            let profile_invariant = match profile {
                IntegrationProfile::Record => "record_non_observing_preserved",
                IntegrationProfile::Detective => "detective_historical_guard_event_preserved",
            };
            if generated[profile_invariant] != true {
                return Err(io::Error::other(format!(
                    "verified generated-wrapper replay does not preserve {profile_invariant}"
                ))
                .into());
            }
        }
        for surface in ["generated_wrapper_direct_wire", "actual_host_fixed_ui"] {
            for branch in ["status_fallback", "authority_receipt"] {
                let branch_status = status(
                    evidence[surface].get(branch).ok_or_else(|| {
                        io::Error::other(format!("{surface}.{branch} evidence is missing"))
                    })?,
                    &format!("{surface}.{branch}"),
                )?;
                if evidence[surface]["status"] == "verified" && branch_status != "verified" {
                    return Err(io::Error::other(format!(
                        "verified {surface} evidence requires a verified {branch} branch"
                    ))
                    .into());
                }
            }
        }
        if evidence["actual_host_event"]["status"] == "verified" {
            for branch in ["status_fallback_event", "authority_receipt_event"] {
                if status(
                    evidence["actual_host_event"].get(branch).ok_or_else(|| {
                        io::Error::other(format!(
                            "verified actual-host event evidence has no {branch:?} branch"
                        ))
                    })?,
                    &format!("actual_host_event.{branch}"),
                )? != "verified"
                {
                    return Err(io::Error::other(format!(
                        "verified actual-host event evidence requires verified {branch} evidence"
                    ))
                    .into());
                }
            }
        }
        if evidence["actual_host_fixed_ui"]["status"] == "verified"
            && evidence["actual_host_fixed_ui"]["authority_receipt"]
                ["complete_canonical_receipt_operator_confirmed"]
                != true
        {
            return Err(io::Error::other(
                "verified actual-host receipt UI requires exact operator confirmation of the complete canonical AuthorityReceipt",
            )
            .into());
        }
        if evidence["actual_host_fixed_ui"]["status"] == "verified"
            && evidence["actual_host_fixed_ui"]["status_fallback"]
                ["complete_taskless_message_operator_confirmed"]
                != true
        {
            return Err(io::Error::other(
                "verified actual-host fallback UI requires exact operator confirmation of the complete taskless message",
            )
            .into());
        }
        if evidence["actual_host_fixed_ui"]["status"] == "verified" {
            let receipt = &evidence["actual_host_fixed_ui"]["authority_receipt"];
            for field in ["project_id", "task_id", "latest_run_id", "close_state"] {
                if receipt[field].as_str().is_none_or(str::is_empty) {
                    return Err(io::Error::other(format!(
                        "verified actual-host receipt UI has no {field:?} coordinate"
                    ))
                    .into());
                }
            }
            for field in ["state_version", "close_blocker_count"] {
                if receipt[field].as_u64().is_none() {
                    return Err(io::Error::other(format!(
                        "verified actual-host receipt UI has no numeric {field:?} coordinate"
                    ))
                    .into());
                }
            }
        }
        if evidence["status_fallback"]["status"] == "verified"
            && (evidence["status_fallback"]["no_active_task"] != true
                || evidence["status_fallback"]["generated_wire_command"]
                    != "volicord status --json"
                || evidence["status_fallback"]["operator_confirmed_actual_host_ui"] != true
                || evidence["status_fallback"]["complete_taskless_message_operator_confirmed"]
                    != true
                || evidence["status_fallback"]["task_bound_command_absent"] != true)
        {
            return Err(io::Error::other(
                "verified status fallback must bind the taskless generated command to actual-host UI confirmation",
            )
            .into());
        }
        match profile {
            IntegrationProfile::Record => {
                match evidence["detective_decision"]["status"].as_str() {
                    Some("not_applicable") => {
                        if evidence["detective_decision"]["non_observing"] != true
                            || evidence["detective_decision"]["non_gating"] != true
                        {
                            return Err(io::Error::other(
                                "observed Record evidence must be explicitly non-observing and non-gating",
                            )
                            .into());
                        }
                    }
                    Some("unavailable" | "failed") if result == "incomplete" => {}
                    _ => {
                        return Err(io::Error::other(
                            "Record decision evidence must be observed non-applicable evidence or an explicit incomplete-run limitation",
                        )
                        .into())
                    }
                }
                if evidence["actual_host_event"]["status"] == "verified" {
                    for branch in ["status_fallback_event", "authority_receipt_event"] {
                        let event = &evidence["actual_host_event"][branch];
                        if event["source"] != "authenticated_host_owned_surface_delivery"
                            || event["delivery_evidence"] != "managed_final_output_ui"
                            || event["persistent_guard_event"] != false
                            || event["non_observing"] != true
                        {
                            return Err(io::Error::other(format!(
                                "verified Record {branch} must identify host-owned UI delivery without claiming a persistent observation"
                            ))
                            .into());
                        }
                    }
                }
            }
            IntegrationProfile::Detective => {
                if evidence["detective_decision"]["status"] == "not_applicable" {
                    return Err(io::Error::other(
                        "Detective evidence must report its historical decision status",
                    )
                    .into());
                }
                if evidence["detective_decision"]["status"] == "verified" {
                    for item in ["historical_decision", "fresh_display", "allow", "block"] {
                        status(
                            evidence["detective_decision"].get(item).ok_or_else(|| {
                                io::Error::other(format!(
                                    "verified Detective decision evidence has no {item:?} item"
                                ))
                            })?,
                            &format!("detective_decision.{item}"),
                        )?;
                    }
                }
                if evidence["actual_host_event"]["status"] == "verified" {
                    for branch in ["status_fallback_event", "authority_receipt_event"] {
                        if evidence["actual_host_event"][branch]["source"]
                            != "persisted_guard_event"
                            || evidence["actual_host_event"][branch]["persistent_guard_event"]
                                != true
                        {
                            return Err(io::Error::other(format!(
                                "verified Detective {branch} must be backed by a persisted GuardEvent"
                            ))
                            .into());
                        }
                    }
                }
            }
        }
        if result == "passed" {
            for key in [
                "config_fixture",
                "generated_wrapper_direct_wire",
                "actual_host_event",
                "actual_host_fixed_ui",
                "status_fallback",
                "exact_replay",
            ] {
                if evidence[key]["status"] != "verified" {
                    return Err(io::Error::other(format!(
                        "passing final-output validation requires verified {key} evidence"
                    ))
                    .into());
                }
            }
            if generated_replay_status != "verified" || actual_replay_status != "verified" {
                return Err(io::Error::other(
                    "passing final-output validation requires both replay layers",
                )
                .into());
            }
            if profile == IntegrationProfile::Detective {
                for item in ["historical_decision", "fresh_display", "allow", "block"] {
                    if evidence["detective_decision"][item]["status"] != "verified" {
                        return Err(io::Error::other(format!(
                            "passing Detective validation requires verified {item} evidence"
                        ))
                        .into());
                    }
                }
            } else if evidence["detective_decision"]["status"] != "not_applicable" {
                return Err(io::Error::other(
                    "passing Record validation requires observed non-gating/non-observing evidence",
                )
                .into());
            }
        }
        Ok(())
    }

    struct VerifiedStopObservation {
        guard_event_id: String,
        session_id: String,
        connection_id: String,
        decision: String,
        state_version: u64,
        latest_run_id: String,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct GuardObservationCounts {
        guard_events: u64,
        agent_sessions: u64,
    }

    #[derive(Clone, Copy, Debug)]
    struct StopEventCursor(i64);

    #[derive(Debug, Eq, PartialEq)]
    struct StoredStopSnapshot {
        guard_event_id: String,
        decision: String,
        result_json: String,
    }

    fn guard_observation_counts(
        fixture: &LiveSmokeFixture,
        project_id: &str,
    ) -> Result<GuardObservationCounts, Box<dyn Error>> {
        let project = list_projects(&fixture.runtime_home_path)?
            .into_iter()
            .find(|project| project.project_id == project_id)
            .ok_or_else(|| io::Error::other("live smoke project registration is missing"))?;
        let conn = open_project_state_database_read_only(&project.state_db_path)?;
        Ok(GuardObservationCounts {
            guard_events: conn
                .query_row("SELECT COUNT(*) FROM guard_events", [], |row| row.get(0))?,
            agent_sessions: conn
                .query_row("SELECT COUNT(*) FROM agent_sessions", [], |row| row.get(0))?,
        })
    }

    fn stop_event_cursor(
        fixture: &LiveSmokeFixture,
        project_id: &str,
    ) -> Result<StopEventCursor, Box<dyn Error>> {
        let project = list_projects(&fixture.runtime_home_path)?
            .into_iter()
            .find(|project| project.project_id == project_id)
            .ok_or_else(|| io::Error::other("live smoke project registration is missing"))?;
        let conn = open_project_state_database_read_only(&project.state_db_path)?;
        Ok(StopEventCursor(conn.query_row(
            "SELECT COALESCE(MAX(rowid), 0)
               FROM guard_events
              WHERE project_id = ?1 AND event_kind = 'stop'",
            [project_id],
            |row| row.get(0),
        )?))
    }

    fn assert_live_connection_verified(
        fixture: &LiveSmokeFixture,
        connection_id: &str,
    ) -> Result<(), Box<dyn Error>> {
        let connection =
            agent_connection_record_read_only(&fixture.runtime_home_path, connection_id)?
                .ok_or_else(|| io::Error::other("live Agent Connection record is missing"))?;
        if connection.last_verification_status != VERIFIED_STATUS_COMPLETE {
            return Err(io::Error::other(format!(
                "the authenticated host MCP round trip did not complete Agent Connection verification: observed {:?}",
                connection.last_verification_status
            ))
            .into());
        }
        Ok(())
    }

    fn stored_stop_snapshot_for_session(
        fixture: &LiveSmokeFixture,
        project_id: &str,
        session_id: &str,
    ) -> Result<StoredStopSnapshot, Box<dyn Error>> {
        let project = list_projects(&fixture.runtime_home_path)?
            .into_iter()
            .find(|project| project.project_id == project_id)
            .ok_or_else(|| io::Error::other("live smoke project registration is missing"))?;
        let conn = open_project_state_database_read_only(&project.state_db_path)?;
        conn.query_row(
            "SELECT guard_event_id, decision, result_json
               FROM guard_events
              WHERE project_id = ?1
                AND session_id = ?2
                AND event_kind = 'stop'",
            rusqlite::params![project_id, session_id],
            |row| {
                Ok(StoredStopSnapshot {
                    guard_event_id: row.get(0)?,
                    decision: row.get(1)?,
                    result_json: row.get(2)?,
                })
            },
        )
        .map_err(Into::into)
    }

    fn verify_live_stop_guard_event(
        runtime_home: &Path,
        connection_id: &str,
        observation: &LiveUserActionObservation,
        receipt: &VerifiedLiveReceipt,
        cursor: StopEventCursor,
    ) -> Result<VerifiedStopObservation, Box<dyn Error>> {
        let projects = list_projects(runtime_home)?;
        let project = projects
            .iter()
            .find(|project| project.project_id == observation.project_id)
            .ok_or_else(|| io::Error::other("live smoke project registration is missing"))?;
        let conn = open_project_state_database_read_only(&project.state_db_path)?;
        let mut statement = conn.prepare(
            "SELECT guard_event_id, session_id, connection_internal_id, decision, result_json
               FROM guard_events
              WHERE project_id = ?1
                AND event_kind = 'stop'
                AND connection_internal_id = ?2
                AND rowid > ?3
              ORDER BY rowid DESC",
        )?;
        let rows = statement.query_map(
            rusqlite::params![observation.project_id, connection_id, cursor.0],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            },
        )?;
        let mut matched = None;
        for row in rows {
            let (guard_event_id, session_id, stored_connection_id, decision, result_json) = row?;
            let result: Value = serde_json::from_str(&result_json)?;
            if result
                .pointer("/close_status/active_task")
                .and_then(Value::as_str)
                != Some(observation.task_id.as_str())
            {
                continue;
            }
            if decision != "allow"
                || result.get("decision").and_then(Value::as_str) != Some("allow")
                || result.get("allowed").and_then(Value::as_bool) != Some(true)
                || result
                    .get("reasons")
                    .and_then(Value::as_array)
                    .is_none_or(|reasons| !reasons.is_empty())
                || result
                    .pointer("/close_status/close_blockers")
                    .and_then(Value::as_array)
                    .is_none_or(|blockers| !blockers.is_empty())
            {
                return Err(io::Error::other(
                    "the latest matching live Stop hook did not record an allow decision with no reasons or close blockers",
                )
                .into());
            }
            let stop_receipt: AuthorityReceipt = serde_json::from_value(
                result
                    .pointer("/close_status/authority_receipt")
                    .cloned()
                    .ok_or_else(|| {
                        io::Error::other(
                            "the latest matching live Stop hook has no AuthorityReceipt",
                        )
                    })?,
            )?;
            if stop_receipt != receipt.canonical_receipt {
                return Err(io::Error::other(
                    "the live Stop allow event AuthorityReceipt does not exactly equal the fresh CLI status AuthorityReceipt",
                )
                .into());
            }
            let stop_latest_run = stop_receipt.latest_run_ref.as_ref().ok_or_else(|| {
                io::Error::other("the live Stop AuthorityReceipt has no latest_run_ref")
            })?;
            let session_id = session_id.ok_or_else(|| {
                io::Error::other("the live Stop allow event is not bound to a host session")
            })?;
            if matched.is_some() {
                return Err(io::Error::other(
                    "the authenticated host produced more than one matching Stop event after the validation cursor",
                )
                .into());
            }
            matched = Some(VerifiedStopObservation {
                guard_event_id,
                session_id,
                connection_id: stored_connection_id,
                decision,
                state_version: stop_receipt.state_version,
                latest_run_id: stop_latest_run.record_id.as_str().to_owned(),
            });
        }
        matched.ok_or_else(|| {
            io::Error::other(
                "no new Stop hook event for the live Task was recorded after the validation cursor",
            )
            .into()
        })
    }

    struct LiveCliFallbackSummaryInput<'a> {
        result: &'a str,
        identity: &'a LiveHostIdentity,
        observation: &'a LiveUserActionObservation,
        operator_choice_id: &'a str,
        cli_resolution: &'a LiveCliResolutionEvidence,
        latest_run: &'a LiveRunObservation,
        authority_event_order: &'a AuthorityEventOrder,
        stop_observation: &'a VerifiedStopObservation,
        receipt: &'a VerifiedLiveReceipt,
        stop_receipt_ui_confirmed: bool,
    }

    fn live_cli_fallback_completed_summary(input: LiveCliFallbackSummaryInput<'_>) -> Value {
        let LiveCliFallbackSummaryInput {
            result,
            identity,
            observation,
            operator_choice_id,
            cli_resolution,
            latest_run,
            authority_event_order,
            stop_observation,
            receipt,
            stop_receipt_ui_confirmed,
        } = input;
        serde_json::json!({
            "kind": LIVE_CLI_FALLBACK_RESULT_KIND,
            "result": result,
            "host": {
                "kind": identity.host,
                "version": identity.host_version
            },
            "volicord": {
                "build_id": identity.volicord_build_id
            },
            "connection": {
                "connection_id": identity.connection_id
            },
            "task": {
                "project_id": observation.project_id,
                "task_id": observation.task_id,
                "lifecycle_phase": observation.lifecycle_phase,
                "state_version": observation.state_version
            },
            "cli_user_channel": {
                "inbox": {
                    "command_surface": "volicord inbox --json",
                    "prepared_request_visible": cli_resolution.inbox_request_visible
                },
                "resolution": {
                    "command_surface": "volicord inbox resolve <user-action-request-id> --choice <option-id> --json",
                    "user_action_request_id": observation.user_action_request_id,
                    "user_action_resolution_id": cli_resolution.user_action_resolution_id,
                    "operator_selected_option_id": operator_choice_id,
                    "stored_selected_option_id": cli_resolution.selected_option_id,
                    "actor_source": observation.resolved_by_actor_source,
                    "channel_kind": observation.resolved_channel_kind,
                    "verification_basis": observation.resolved_verification_basis,
                    "state_version_before_resolution": cli_resolution.state_version_before_resolution,
                    "committed_state_version": cli_resolution.committed_state_version
                },
                "exact_retry": {
                    "same_command_and_arguments": true,
                    "stdout_byte_identical": cli_resolution.exact_retry_stdout_identical,
                    "state_version": cli_resolution.exact_retry_state_version,
                    "state_version_unchanged": cli_resolution.exact_retry_no_state_change
                }
            },
            "host_resume": {
                "request_operation": "resume",
                "same_agent_connection": true,
                "origin_result_replayed_in_host_diagnostics": true,
                "resolved_choice_consumed": true,
                "additional_product_decision_request_created": false
            },
            "choice_consumption": {
                "run_id": latest_run.run_id,
                "run_kind": latest_run.kind,
                "run_marker": latest_run.summary,
                "created_by_actor_source": latest_run.created_by_actor_source,
                "product_file_write_observed": latest_run.product_file_write_observed,
                "changed_path_count": latest_run.changed_paths.len()
            },
            "authority_events": {
                "user_action_requested_event_seq": authority_event_order.user_action_requested_event_seq,
                "user_action_resolved_event_seq": authority_event_order.user_action_resolved_event_seq,
                "run_recorded_event_seq": authority_event_order.run_recorded_event_seq,
                "ordered": authority_event_order.user_action_requested_event_seq
                    < authority_event_order.user_action_resolved_event_seq
                    && authority_event_order.user_action_resolved_event_seq
                        < authority_event_order.run_recorded_event_seq
            },
            "stop_hook": {
                "guard_event_id": stop_observation.guard_event_id,
                "session_id": stop_observation.session_id,
                "connection_id": stop_observation.connection_id,
                "decision": stop_observation.decision,
                "receipt_state_version": stop_observation.state_version,
                "latest_run_id": stop_observation.latest_run_id
            },
            "authority_receipt": {
                "project_id": receipt.project_id,
                "task_id": receipt.task_id,
                "state_version": receipt.state_version,
                "latest_run_id": receipt.latest_run_id,
                "close_state": receipt.close_state,
                "close_blocker_count": receipt.close_blocker_count,
                "complete_managed_ui_confirmed": stop_receipt_ui_confirmed
            },
            "evidence_scope": {
                "cli_fallback_release_cell": true,
                "native_judgment_cell": false,
                "final_output_matrix_cell": false
            }
        })
    }

    fn validate_live_cli_fallback_result_shape(value: &Value) -> Result<(), Box<dyn Error>> {
        if value["kind"] != LIVE_CLI_FALLBACK_RESULT_KIND || value["result"] != "passed" {
            return Err(io::Error::other(
                "passing CLI-fallback result has the wrong validation kind or result",
            )
            .into());
        }
        let request_id = value["cli_user_channel"]["resolution"]["user_action_request_id"]
            .as_str()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| io::Error::other("CLI-fallback result has no request id"))?;
        let resolution_id = value["cli_user_channel"]["resolution"]["user_action_resolution_id"]
            .as_str()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| io::Error::other("CLI-fallback result has no resolution id"))?;
        let operator_choice = value["cli_user_channel"]["resolution"]
            ["operator_selected_option_id"]
            .as_str()
            .ok_or_else(|| io::Error::other("CLI-fallback result has no operator choice"))?;
        let stored_choice = value["cli_user_channel"]["resolution"]["stored_selected_option_id"]
            .as_str()
            .ok_or_else(|| io::Error::other("CLI-fallback result has no stored choice"))?;
        let before = value["cli_user_channel"]["resolution"]["state_version_before_resolution"]
            .as_u64()
            .ok_or_else(|| io::Error::other("CLI-fallback result has no pre-resolution version"))?;
        let committed = value["cli_user_channel"]["resolution"]["committed_state_version"]
            .as_u64()
            .ok_or_else(|| io::Error::other("CLI-fallback result has no committed version"))?;
        let retry = value["cli_user_channel"]["exact_retry"]["state_version"]
            .as_u64()
            .ok_or_else(|| io::Error::other("CLI-fallback result has no retry version"))?;
        let expected_committed = before.checked_add(1).ok_or_else(|| {
            io::Error::other("CLI-fallback pre-resolution version cannot advance once")
        })?;
        let project_id = value["task"]["project_id"]
            .as_str()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| io::Error::other("CLI-fallback result has no project id"))?;
        let task_id = value["task"]["task_id"]
            .as_str()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| io::Error::other("CLI-fallback result has no task id"))?;
        let task_state_version = value["task"]["state_version"]
            .as_u64()
            .ok_or_else(|| io::Error::other("CLI-fallback result has no Task state version"))?;
        let run_id = value["choice_consumption"]["run_id"]
            .as_str()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| io::Error::other("CLI-fallback result has no consumed Run id"))?;
        let requested_event_seq = value["authority_events"]["user_action_requested_event_seq"]
            .as_u64()
            .ok_or_else(|| io::Error::other("CLI-fallback result has no request event sequence"))?;
        let resolved_event_seq = value["authority_events"]["user_action_resolved_event_seq"]
            .as_u64()
            .ok_or_else(|| {
                io::Error::other("CLI-fallback result has no resolution event sequence")
            })?;
        let run_event_seq = value["authority_events"]["run_recorded_event_seq"]
            .as_u64()
            .ok_or_else(|| io::Error::other("CLI-fallback result has no Run event sequence"))?;
        let stop_receipt_state_version = value["stop_hook"]["receipt_state_version"]
            .as_u64()
            .ok_or_else(|| io::Error::other("CLI-fallback result has no Stop receipt version"))?;
        let stop_latest_run_id = value["stop_hook"]["latest_run_id"]
            .as_str()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| io::Error::other("CLI-fallback result has no Stop latest Run id"))?;
        let receipt_project_id = value["authority_receipt"]["project_id"]
            .as_str()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| io::Error::other("CLI-fallback result has no receipt project id"))?;
        let receipt_task_id = value["authority_receipt"]["task_id"]
            .as_str()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| io::Error::other("CLI-fallback result has no receipt Task id"))?;
        let receipt_state_version = value["authority_receipt"]["state_version"]
            .as_u64()
            .ok_or_else(|| io::Error::other("CLI-fallback result has no receipt state version"))?;
        let receipt_latest_run_id = value["authority_receipt"]["latest_run_id"]
            .as_str()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| io::Error::other("CLI-fallback result has no receipt latest Run id"))?;
        let expected_run_marker = run_marker_for_selected_option(stored_choice)
            .ok_or_else(|| io::Error::other("CLI-fallback result stores an unknown choice"))?;
        let connection_id = value["connection"]["connection_id"]
            .as_str()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| io::Error::other("CLI-fallback result has no connection id"))?;
        let expected_actor_source = format!("agent_connection:{connection_id}");
        if request_id.is_empty()
            || resolution_id.is_empty()
            || operator_choice != stored_choice
            || value["cli_user_channel"]["inbox"]["prepared_request_visible"] != true
            || value["cli_user_channel"]["resolution"]["actor_source"] != "local_user"
            || value["cli_user_channel"]["resolution"]["channel_kind"] != "cli"
            || value["cli_user_channel"]["resolution"]["verification_basis"]
                != VERIFICATION_BASIS_CLI_DIRECT_USER_CHANNEL
            || committed != expected_committed
            || retry != committed
            || value["cli_user_channel"]["exact_retry"]["same_command_and_arguments"] != true
            || value["cli_user_channel"]["exact_retry"]["stdout_byte_identical"] != true
            || value["cli_user_channel"]["exact_retry"]["state_version_unchanged"] != true
            || value["host_resume"]["request_operation"] != "resume"
            || value["host_resume"]["same_agent_connection"] != true
            || value["host_resume"]["origin_result_replayed_in_host_diagnostics"] != true
            || value["host_resume"]["resolved_choice_consumed"] != true
            || value["host_resume"]["additional_product_decision_request_created"] != false
            || value["choice_consumption"]["run_kind"] != "shaping_update"
            || value["choice_consumption"]["run_marker"] != expected_run_marker
            || value["choice_consumption"]["created_by_actor_source"] != expected_actor_source
            || value["choice_consumption"]["product_file_write_observed"] != false
            || value["choice_consumption"]["changed_path_count"] != 0
            || requested_event_seq == 0
            || resolved_event_seq == 0
            || run_event_seq == 0
            || !(requested_event_seq < resolved_event_seq && resolved_event_seq < run_event_seq)
            || value["authority_events"]["ordered"] != true
            || value["stop_hook"]["connection_id"] != connection_id
            || value["stop_hook"]["decision"] != "allow"
            || receipt_project_id != project_id
            || receipt_task_id != task_id
            || receipt_state_version != task_state_version
            || stop_receipt_state_version != receipt_state_version
            || receipt_latest_run_id != run_id
            || stop_latest_run_id != run_id
            || value["authority_receipt"]["close_state"] != "ready"
            || value["authority_receipt"]["close_blocker_count"] != 0
            || value["authority_receipt"]["complete_managed_ui_confirmed"] != true
            || value["evidence_scope"]["cli_fallback_release_cell"] != true
            || value["evidence_scope"]["native_judgment_cell"] != false
            || value["evidence_scope"]["final_output_matrix_cell"] != false
        {
            return Err(io::Error::other(
                "passing CLI-fallback result does not preserve the required separated evidence",
            )
            .into());
        }
        Ok(())
    }

    struct LiveCompletedSummaryInput<'a> {
        result: &'a str,
        identity: &'a LiveHostIdentity,
        observation: &'a LiveUserActionObservation,
        operator_choice_id: &'a str,
        selected_option_id: &'a str,
        latest_run: &'a LiveRunObservation,
        authority_event_order: &'a AuthorityEventOrder,
        stop_observation: &'a VerifiedStopObservation,
        receipt: &'a VerifiedLiveReceipt,
        stop_receipt_ui_confirmed: bool,
    }

    fn live_host_completed_summary(input: LiveCompletedSummaryInput<'_>) -> Value {
        let LiveCompletedSummaryInput {
            result,
            identity,
            observation,
            operator_choice_id,
            selected_option_id,
            latest_run,
            authority_event_order,
            stop_observation,
            receipt,
            stop_receipt_ui_confirmed,
        } = input;
        serde_json::json!({
            "kind": "live_host_user_action_release_validation",
            "result": result,
            "host": {
                "kind": identity.host,
                "version": identity.host_version
            },
            "volicord": {
                "build_id": identity.volicord_build_id
            },
            "connection": {
                "connection_id": identity.connection_id
            },
            "task": {
                "project_id": observation.project_id,
                "task_id": observation.task_id,
                "lifecycle_phase": observation.lifecycle_phase,
                "state_version": observation.state_version
            },
            "user_action": {
                "user_action_request_id": observation.user_action_request_id,
                "selected_option_id": selected_option_id,
                "operator_confirmed_option_id": operator_choice_id,
                "stored_choice_matches_operator": operator_choice_id == selected_option_id,
                "user_channel_basis": VERIFICATION_BASIS_MCP_ELICITATION_USER_CHANNEL
            },
            "choice_consumption": {
                "run_id": latest_run.run_id,
                "run_kind": latest_run.kind,
                "run_marker": latest_run.summary,
                "product_file_write_observed": latest_run.product_file_write_observed,
                "changed_path_count": latest_run.changed_paths.len()
            },
            "authority_events": {
                "user_action_requested_event_seq": authority_event_order.user_action_requested_event_seq,
                "user_action_resolved_event_seq": authority_event_order.user_action_resolved_event_seq,
                "run_recorded_event_seq": authority_event_order.run_recorded_event_seq,
                "ordered": authority_event_order.user_action_requested_event_seq
                    < authority_event_order.user_action_resolved_event_seq
                    && authority_event_order.user_action_resolved_event_seq
                        < authority_event_order.run_recorded_event_seq
            },
            "native_ui": {
                "user_action_selector_confirmed": true,
                "operator_choice_confirmed": true,
                "stop_system_message_authority_receipt_confirmed": stop_receipt_ui_confirmed
            },
            "stop_hook": {
                "guard_event_id": stop_observation.guard_event_id,
                "session_id": stop_observation.session_id,
                "connection_id": stop_observation.connection_id,
                "decision": stop_observation.decision,
                "decision_observed_from_guard_event": true,
                "receipt_state_version": stop_observation.state_version,
                "latest_run_id": stop_observation.latest_run_id
            },
            "authority_receipt": {
                "project_id": receipt.project_id,
                "task_id": receipt.task_id,
                "state_version": receipt.state_version,
                "latest_run_id": receipt.latest_run_id,
                "close_state": receipt.close_state,
                "close_blocker_count": receipt.close_blocker_count
            },
            "cli_fallback": {
                "verified": false
            }
        })
    }

    fn live_host_choice_mismatch_summary(
        identity: &LiveHostIdentity,
        observation: &LiveUserActionObservation,
        operator_choice_id: &str,
        selected_option_id: &str,
    ) -> Value {
        serde_json::json!({
            "kind": "live_host_user_action_release_validation",
            "result": "failed_choice_mismatch",
            "host": {
                "kind": identity.host,
                "version": identity.host_version
            },
            "volicord": {
                "build_id": identity.volicord_build_id
            },
            "connection": {
                "connection_id": identity.connection_id
            },
            "task": {
                "project_id": observation.project_id,
                "task_id": observation.task_id,
                "lifecycle_phase": observation.lifecycle_phase,
                "state_version": observation.state_version
            },
            "user_action": {
                "user_action_request_id": observation.user_action_request_id,
                "selected_option_id": selected_option_id,
                "operator_confirmed_option_id": operator_choice_id,
                "stored_choice_matches_operator": false,
                "user_channel_basis": observation.resolved_verification_basis
            },
            "native_ui": {
                "user_action_selector_confirmed": true,
                "operator_choice_confirmed": true,
                "stop_system_message_authority_receipt_confirmed": false
            },
            "stop_hook": null,
            "authority_receipt": null,
            "cli_fallback": {
                "verified": false
            }
        })
    }

    fn live_host_fallback_summary(
        identity: &LiveHostIdentity,
        observation: &LiveUserActionObservation,
        fallback: &LiveInboxFallback,
    ) -> Value {
        serde_json::json!({
            "kind": "live_host_user_action_release_validation",
            "result": "failed_native_elicitation",
            "host": {
                "kind": identity.host,
                "version": identity.host_version
            },
            "volicord": {
                "build_id": identity.volicord_build_id
            },
            "connection": {
                "connection_id": identity.connection_id
            },
            "task": {
                "project_id": observation.project_id,
                "task_id": observation.task_id,
                "lifecycle_phase": observation.lifecycle_phase,
                "state_version": observation.state_version
            },
            "user_action": {
                "user_action_request_id": observation.user_action_request_id,
                "status": observation.user_action_status
            },
            "native_ui": {
                "user_action_selector_confirmed": false,
                "operator_choice_confirmed": false,
                "stop_system_message_authority_receipt_confirmed": false
            },
            "stop_hook": null,
            "authority_receipt": null,
            "cli_fallback": {
                "fixture_command_shape_verified": true,
                "fixture_is_ephemeral": true,
                "commands_are_runnable_after_test": false,
                "inbox_command_template": fallback.inbox_command_template,
                "resolve_command_template": fallback.resolve_command_template
            }
        })
    }

    struct LiveResultRecorder {
        host: String,
        result_kind: &'static str,
        result_path: Option<PathBuf>,
        run_id: String,
        started_at: String,
        started: bool,
        finalized: bool,
    }

    impl LiveResultRecorder {
        fn from_env(host: &str) -> Result<Self, Box<dyn Error>> {
            let result_path = required_live_result_path(env::var_os(LIVE_HOST_RESULT_PATH_ENV))?;
            Self::new(host, Some(result_path))
        }

        fn from_env_for_kind(
            host: &str,
            result_kind: &'static str,
        ) -> Result<Self, Box<dyn Error>> {
            let result_path = required_live_result_path(env::var_os(LIVE_HOST_RESULT_PATH_ENV))?;
            Self::new_for_kind(host, result_kind, Some(result_path))
        }

        fn new(host: &str, result_path: Option<PathBuf>) -> Result<Self, Box<dyn Error>> {
            Self::new_for_kind(host, LIVE_USER_ACTION_RESULT_KIND, result_path)
        }

        fn new_for_kind(
            host: &str,
            result_kind: &'static str,
            result_path: Option<PathBuf>,
        ) -> Result<Self, Box<dyn Error>> {
            if let Some(path) = result_path.as_deref() {
                validate_external_result_path(path, true)?;
            }
            let started_at = recorded_at_now()?;
            let run_id = bounded_identity(
                "live validation run_id",
                &format!(
                    "{}-{}-{}",
                    host.replace('-', "_"),
                    std::process::id(),
                    epoch_duration()?.as_nanos()
                ),
                MAX_VALIDATION_RUN_ID_CHARS,
            )?;
            let mut recorder = Self {
                host: host.to_owned(),
                result_kind,
                result_path,
                run_id,
                started_at,
                started: false,
                finalized: false,
            };
            if recorder.result_path.is_some() {
                recorder.write_external_summary(
                    &serde_json::json!({
                        "kind": result_kind,
                        "result": "running",
                        "host": { "kind": host }
                    }),
                    false,
                )?;
            }
            recorder.started = true;
            Ok(recorder)
        }

        fn record_final(&mut self, summary: &Value) -> Result<(), Box<dyn Error>> {
            let summary = self.with_validation_run(summary)?;
            let serialized = serialize_live_host_result(&summary)?;
            if let Some(path) = self.result_path.as_deref() {
                atomic_write_live_host_result(path, &serialized, &self.run_id)?;
            }
            println!("{serialized}");
            self.finalized = true;
            Ok(())
        }

        fn with_validation_run(&self, summary: &Value) -> Result<Value, Box<dyn Error>> {
            let mut summary = summary.clone();
            let object = summary.as_object_mut().ok_or_else(|| {
                io::Error::other("live-host result summary must be a JSON object")
            })?;
            object.insert(
                "validation_run".to_owned(),
                serde_json::json!({
                    "run_id": self.run_id,
                    "started_at": self.started_at,
                    "recorded_at": recorded_at_now()?
                }),
            );
            Ok(summary)
        }

        fn write_external_summary(
            &self,
            summary: &Value,
            replace_existing: bool,
        ) -> Result<(), Box<dyn Error>> {
            let Some(path) = self.result_path.as_deref() else {
                return Ok(());
            };
            let summary = self.with_validation_run(summary)?;
            let serialized = serialize_live_host_result(&summary)?;
            if replace_existing {
                atomic_write_live_host_result(path, &serialized, &self.run_id)
            } else {
                write_new_live_host_result(path, &serialized)
            }
        }
    }

    fn required_live_result_path(value: Option<OsString>) -> Result<PathBuf, Box<dyn Error>> {
        value.map(PathBuf::from).ok_or_else(|| {
            io::Error::other(format!(
                "{LIVE_HOST_RESULT_PATH_ENV} must name a new absolute result path outside the source repository"
            ))
            .into()
        })
    }

    impl Drop for LiveResultRecorder {
        fn drop(&mut self) {
            if !self.started || self.finalized || self.result_path.is_none() {
                return;
            }
            let _ = self.write_external_summary(
                &serde_json::json!({
                    "kind": self.result_kind,
                    "result": "failed_before_completion",
                    "host": { "kind": self.host }
                }),
                true,
            );
        }
    }

    fn serialize_live_host_result(summary: &Value) -> Result<String, Box<dyn Error>> {
        let serialized = serde_json::to_string(summary)?;
        if serialized.len() > MAX_LIVE_HOST_RESULT_BYTES {
            return Err(io::Error::other(format!(
                "live-host result exceeds the {MAX_LIVE_HOST_RESULT_BYTES}-byte limit"
            ))
            .into());
        }
        Ok(serialized)
    }

    fn validate_external_result_path(
        path: &Path,
        reject_existing: bool,
    ) -> Result<(), Box<dyn Error>> {
        if !path.is_absolute() {
            return Err(io::Error::other(format!(
                "{LIVE_HOST_RESULT_PATH_ENV} must name an absolute path outside the source repository"
            ))
            .into());
        }
        let parent = path.parent().ok_or_else(|| {
            io::Error::other(format!(
                "{LIVE_HOST_RESULT_PATH_ENV} has no parent directory"
            ))
        })?;
        let canonical_parent = fs::canonicalize(parent)?;
        let source_repository = fs::canonicalize(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .and_then(Path::parent)
                .ok_or_else(|| io::Error::other("could not resolve the source repository root"))?,
        )?;
        if canonical_parent.starts_with(&source_repository) {
            return Err(io::Error::other(format!(
                "{LIVE_HOST_RESULT_PATH_ENV} must stay outside the source repository"
            ))
            .into());
        }
        if path.file_name().is_none() {
            return Err(io::Error::other(format!(
                "{LIVE_HOST_RESULT_PATH_ENV} must name a result file"
            ))
            .into());
        }
        if let Ok(metadata) = fs::symlink_metadata(path) {
            if reject_existing {
                return Err(io::Error::other(format!(
                    "{LIVE_HOST_RESULT_PATH_ENV} already exists; use a new result path so a prior pass cannot be mistaken for this run"
                ))
                .into());
            }
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                return Err(io::Error::other(format!(
                    "{LIVE_HOST_RESULT_PATH_ENV} must name a regular file, not a symlink or directory"
                ))
                .into());
            }
        }
        Ok(())
    }

    fn atomic_write_live_host_result(
        path: &Path,
        serialized: &str,
        run_id: &str,
    ) -> Result<(), Box<dyn Error>> {
        validate_external_result_path(path, false)?;
        validate_existing_result_run(path, run_id)?;
        let parent = path.parent().ok_or_else(|| {
            io::Error::other(format!(
                "{LIVE_HOST_RESULT_PATH_ENV} has no parent directory"
            ))
        })?;
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| io::Error::other("live-host result filename must be UTF-8"))?;
        let temporary_path = parent.join(format!(
            ".{file_name}.{run_id}.{}.tmp",
            epoch_duration()?.as_nanos()
        ));
        let write_result = (|| -> Result<(), Box<dyn Error>> {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary_path)?;
            file.write_all(serialized.as_bytes())?;
            file.write_all(b"\n")?;
            file.sync_all()?;
            drop(file);
            fs::rename(&temporary_path, path)?;
            Ok(())
        })();
        if write_result.is_err() {
            let _ = fs::remove_file(&temporary_path);
        }
        write_result
    }

    fn write_new_live_host_result(path: &Path, serialized: &str) -> Result<(), Box<dyn Error>> {
        validate_external_result_path(path, true)?;
        let mut created = false;
        let write_result = (|| -> Result<(), Box<dyn Error>> {
            let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
            created = true;
            file.write_all(serialized.as_bytes())?;
            file.write_all(b"\n")?;
            file.sync_all()?;
            Ok(())
        })();
        if write_result.is_err() && created {
            let _ = fs::remove_file(path);
        }
        write_result
    }

    fn validate_existing_result_run(path: &Path, run_id: &str) -> Result<(), Box<dyn Error>> {
        let metadata = fs::metadata(path).map_err(|error| {
            io::Error::other(format!(
                "active live-host result is unavailable before final replacement: {error}"
            ))
        })?;
        if metadata.len() > MAX_LIVE_HOST_RESULT_BYTES as u64 {
            return Err(io::Error::other(
                "active live-host result exceeds the bounded result size before final replacement",
            )
            .into());
        }
        let existing: Value = serde_json::from_slice(&fs::read(path)?)?;
        if existing
            .pointer("/validation_run/run_id")
            .and_then(Value::as_str)
            != Some(run_id)
        {
            return Err(io::Error::other(
                "active live-host result no longer belongs to this validation run",
            )
            .into());
        }
        Ok(())
    }

    fn epoch_duration() -> Result<Duration, Box<dyn Error>> {
        Ok(SystemTime::now().duration_since(UNIX_EPOCH)?)
    }

    fn recorded_at_now() -> Result<String, Box<dyn Error>> {
        let duration = epoch_duration()?;
        bounded_identity(
            "live validation recorded_at",
            &format!(
                "unix_epoch:{}.{:09}",
                duration.as_secs(),
                duration.subsec_nanos()
            ),
            MAX_RECORDED_AT_CHARS,
        )
    }

    fn host_version_summary(output: &TimedOutput) -> Result<String, Box<dyn Error>> {
        let stdout = stdout(output);
        let stderr = stderr(output);
        let version = [stdout.as_str(), stderr.as_str()]
            .into_iter()
            .flat_map(str::lines)
            .map(str::trim)
            .find(|line| !line.is_empty())
            .ok_or_else(|| io::Error::other("host --version returned no version text"))?;
        bounded_identity("host version", version, MAX_HOST_VERSION_CHARS)
    }

    fn bounded_identity(
        label: &str,
        value: &str,
        max_chars: usize,
    ) -> Result<String, Box<dyn Error>> {
        if value.is_empty() || value.chars().any(char::is_control) {
            return Err(
                io::Error::other(format!("{label} must be one non-empty printable line")).into(),
            );
        }
        if value.chars().count() > max_chars {
            return Err(io::Error::other(format!(
                "{label} exceeds the {max_chars}-character result limit"
            ))
            .into());
        }
        Ok(value.to_owned())
    }

    fn assert_native_channel_diagnostic(
        fixture: &LiveSmokeFixture,
        connection_id: &str,
        project_id: &str,
    ) -> Result<(), Box<dyn Error>> {
        let conn = rusqlite::Connection::open_with_flags(
            diagnostics_db_path(&fixture.runtime_home_path),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )?;
        let observed = conn.query_row(
            "SELECT COUNT(*)
                   FROM diagnostic_events e
                   JOIN diagnostic_sessions s ON s.session_id = e.session_id
                  WHERE s.connection_id = ?1
                    AND s.project_id = ?2
                    AND e.tool_name = 'volicord.request_user_action'
                    AND e.user_channel_kind = 'mcp_elicitation'",
            [connection_id, project_id],
            |row| row.get::<_, u64>(0),
        )?;
        assert!(
            observed >= 1,
            "session diagnostics did not observe the verified native User Channel round trip"
        );
        Ok(())
    }

    fn assert_cli_fallback_resume_diagnostic(
        fixture: &LiveSmokeFixture,
        connection_id: &str,
        project_id: &str,
    ) -> Result<(), Box<dyn Error>> {
        let conn = rusqlite::Connection::open_with_flags(
            diagnostics_db_path(&fixture.runtime_home_path),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )?;
        let observed = conn.query_row(
            "SELECT
                 COALESCE(SUM(CASE
                       WHEN e.tool_name = 'volicord.request_user_action'
                        AND e.replayed = 1
                        AND e.outcome = 'success'
                       THEN 1 ELSE 0 END), 0),
                 COALESCE(SUM(CASE
                       WHEN e.tool_name = 'volicord.record_run'
                        AND e.core_committed = 1
                        AND e.outcome = 'success'
                       THEN 1 ELSE 0 END), 0),
                 COALESCE(SUM(CASE
                       WHEN e.tool_name = 'volicord.status'
                        AND e.outcome = 'success'
                       THEN 1 ELSE 0 END), 0)
               FROM diagnostic_sessions s
               JOIN diagnostic_events e ON e.session_id = s.session_id
              WHERE s.connection_id = ?1
                AND s.project_id = ?2",
            [connection_id, project_id],
            |row| {
                Ok((
                    row.get::<_, u64>(0)?,
                    row.get::<_, u64>(1)?,
                    row.get::<_, u64>(2)?,
                ))
            },
        )?;
        if observed.0 < 1 || observed.1 != 1 || observed.2 < 1 {
            return Err(io::Error::other(format!(
                "the authenticated host diagnostics did not show one same-connection resume path: replayed request_user_action={}, committed record_run={}, status={}",
                observed.0, observed.1, observed.2
            ))
            .into());
        }
        Ok(())
    }

    struct LiveEvidenceDiagnosticObservation {
        create_calls: u64,
        resume_calls: u64,
        record_run_calls: u64,
        committed_record_run_calls: u64,
        status_calls: u64,
        successful_status_calls: u64,
        ordered: bool,
    }

    fn assert_local_web_evidence_diagnostic(
        fixture: &LiveSmokeFixture,
        connection_id: &str,
        project_id: &str,
    ) -> Result<LiveEvidenceDiagnosticObservation, Box<dyn Error>> {
        let conn = rusqlite::Connection::open_with_flags(
            diagnostics_db_path(&fixture.runtime_home_path),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )?;
        let observed = conn.query_row(
            "SELECT
                 COALESCE(SUM(CASE
                       WHEN e.tool_name = 'volicord.request_user_action'
                       THEN 1 ELSE 0 END), 0),
                 COALESCE(SUM(CASE
                       WHEN e.tool_name = 'volicord.request_user_action'
                        AND e.core_committed = 1
                        AND e.replayed = 0
                        AND e.user_channel_kind IS NULL
                        AND e.fallback_kind = 'local_web_consent'
                        AND e.outcome = 'success'
                       THEN 1 ELSE 0 END), 0),
                 COALESCE(SUM(CASE
                       WHEN e.tool_name = 'volicord.request_user_action'
                        AND e.core_committed = 0
                        AND e.replayed = 1
                        AND e.user_channel_kind IS NULL
                        AND e.fallback_kind IS NULL
                        AND e.outcome = 'success'
                       THEN 1 ELSE 0 END), 0),
                 COALESCE(SUM(CASE
                       WHEN e.tool_name = 'volicord.request_user_action'
                        AND (e.user_channel_kind IS NOT NULL
                          OR (e.fallback_kind IS NOT NULL
                            AND e.fallback_kind != 'local_web_consent'))
                       THEN 1 ELSE 0 END), 0),
                 COALESCE(SUM(CASE
                       WHEN e.tool_name = 'volicord.record_run'
                       THEN 1 ELSE 0 END), 0),
                 COALESCE(SUM(CASE
                       WHEN e.tool_name = 'volicord.record_run'
                        AND e.core_committed = 1
                        AND e.outcome = 'success'
                       THEN 1 ELSE 0 END), 0),
                 COALESCE(SUM(CASE
                       WHEN e.tool_name = 'volicord.status'
                       THEN 1 ELSE 0 END), 0),
                 COALESCE(SUM(CASE
                       WHEN e.tool_name = 'volicord.status'
                        AND e.outcome = 'success'
                       THEN 1 ELSE 0 END), 0),
                 MIN(CASE
                       WHEN e.tool_name = 'volicord.request_user_action'
                        AND e.core_committed = 1
                        AND e.replayed = 0
                        AND e.fallback_kind = 'local_web_consent'
                       THEN e.event_id END),
                 MIN(CASE
                       WHEN e.tool_name = 'volicord.request_user_action'
                        AND e.core_committed = 0
                        AND e.replayed = 1
                       THEN e.event_id END),
                 MIN(CASE
                       WHEN e.tool_name = 'volicord.record_run'
                        AND e.core_committed = 1
                        AND e.outcome = 'success'
                       THEN e.event_id END),
                 MAX(CASE
                       WHEN e.tool_name = 'volicord.status'
                        AND e.outcome = 'success'
                       THEN e.event_id END)
               FROM diagnostic_sessions s
               JOIN diagnostic_events e ON e.session_id = s.session_id
              WHERE s.connection_id = ?1 AND s.project_id = ?2",
            [connection_id, project_id],
            |row| {
                Ok((
                    row.get::<_, u64>(0)?,
                    row.get::<_, u64>(1)?,
                    row.get::<_, u64>(2)?,
                    row.get::<_, u64>(3)?,
                    row.get::<_, u64>(4)?,
                    row.get::<_, u64>(5)?,
                    row.get::<_, u64>(6)?,
                    row.get::<_, u64>(7)?,
                    row.get::<_, Option<u64>>(8)?,
                    row.get::<_, Option<u64>>(9)?,
                    row.get::<_, Option<u64>>(10)?,
                    row.get::<_, Option<u64>>(11)?,
                ))
            },
        )?;
        let diagnostic_ordered = observed
            .8
            .zip(observed.9)
            .zip(observed.10)
            .zip(observed.11)
            .is_some_and(|(((create, resume), record_run), status)| {
                create < resume && resume < record_run && record_run < status
            });
        if observed.0 != 2
            || observed.1 != 1
            || observed.2 != 1
            || observed.3 != 0
            || observed.4 != 1
            || observed.5 != 1
            || observed.6 != 1
            || observed.7 != 1
            || !diagnostic_ordered
        {
            return Err(io::Error::other(format!(
                "same-connection local-web evidence diagnostics were not exact: request_user_action={}, create_local_web={}, resume_replayed={}, disallowed_channel={}, record_run={}, committed_record_run={}, status={}, successful_status={}, ordered={diagnostic_ordered}",
                observed.0, observed.1, observed.2, observed.3, observed.4, observed.5, observed.6, observed.7
            ))
            .into());
        }
        Ok(LiveEvidenceDiagnosticObservation {
            create_calls: observed.1,
            resume_calls: observed.2,
            record_run_calls: observed.4,
            committed_record_run_calls: observed.5,
            status_calls: observed.6,
            successful_status_calls: observed.7,
            ordered: diagnostic_ordered,
        })
    }

    fn assert_live_evidence_diagnostic_payload_absence(
        fixture: &LiveSmokeFixture,
        observation: &LiveEvidenceObservation,
    ) -> Result<bool, Box<dyn Error>> {
        let diagnostic_bytes = fs::read(diagnostics_db_path(&fixture.runtime_home_path))?;
        let diagnostic_text = String::from_utf8_lossy(&diagnostic_bytes);
        let normalized = diagnostic_text.to_ascii_lowercase();
        for forbidden in [
            LIVE_EVIDENCE_REQUEST_QUESTION,
            LIVE_EVIDENCE_REQUEST_CONTEXT,
            LIVE_EVIDENCE_ARTIFACT_DISPLAY_NAME,
            LIVE_EVIDENCE_ARTIFACT_BYTES,
            observation.user_action_request_id.as_str(),
            observation.summary.as_str(),
            "user_action_request_ref",
        ] {
            if diagnostic_text.contains(forbidden) {
                return Err(io::Error::other(
                    "live diagnostic storage contained a forbidden UserAction payload or request ref",
                )
                .into());
            }
        }
        if [
            "http://",
            "https://",
            "/consent?",
            "token=",
            "\"form\"",
            "\"question\"",
        ]
        .into_iter()
        .any(|forbidden| normalized.contains(forbidden))
        {
            return Err(io::Error::other(
                "live diagnostic storage contained a URL, token, form, or question payload",
            )
            .into());
        }
        Ok(true)
    }

    struct LiveSmokeFixture {
        _runtime_home: TempRuntimeHome,
        runtime_home_path: PathBuf,
        repo_root: PathBuf,
        repo_arg: String,
        runtime_home_arg: String,
        env_path: OsString,
        home: PathBuf,
        codex_home: PathBuf,
        xdg_config_home: PathBuf,
        claude_config_dir: PathBuf,
    }

    impl LiveSmokeFixture {
        fn new(prefix: &str) -> Result<Self, Box<dyn Error>> {
            let runtime_home = TempRuntimeHome::new(&format!("live-host-smoke-{prefix}"))?;
            let runtime_home_path = runtime_home.path().to_path_buf();
            let repo_root = runtime_home.create_product_repo("product-repo")?;
            initialize_git_repository(&repo_root)?;
            fs::write(
                repo_root.join("README.md"),
                "Volicord live smoke repository\n",
            )?;

            let bin_dir = runtime_home_path.join("live-bin");
            fs::create_dir_all(&bin_dir)?;
            write_volicord_shim(&bin_dir, Path::new(volicord_bin()))?;

            let home = runtime_home_path.join("isolated-home");
            let codex_home = runtime_home_path.join("isolated-codex-home");
            let xdg_config_home = runtime_home_path.join("isolated-xdg-config");
            let claude_config_dir = runtime_home_path.join("isolated-claude-config");
            for path in [&home, &codex_home, &xdg_config_home, &claude_config_dir] {
                fs::create_dir_all(path)?;
            }

            let env_path = path_with_prefix(&bin_dir)?;
            let repo_arg = path_text(&repo_root);
            let runtime_home_arg = path_text(&runtime_home_path);
            Ok(Self {
                _runtime_home: runtime_home,
                runtime_home_path,
                repo_root,
                repo_arg,
                runtime_home_arg,
                env_path,
                home,
                codex_home,
                xdg_config_home,
                claude_config_dir,
            })
        }

        fn repo_arg(&self) -> &str {
            &self.repo_arg
        }

        fn runtime_home_arg(&self) -> &str {
            &self.runtime_home_arg
        }

        fn run_volicord<const N: usize>(
            &self,
            args: [&str; N],
        ) -> Result<TimedOutput, Box<dyn Error>> {
            let mut command = Command::new(volicord_bin());
            command.args(args).current_dir(&self.repo_root);
            self.apply_isolated_env(&mut command);
            run_with_timeout(command, COMMAND_TIMEOUT).map_err(Into::into)
        }

        fn run_host_command<const N: usize>(
            &self,
            program: &Path,
            args: [&str; N],
        ) -> Result<TimedOutput, Box<dyn Error>> {
            let mut command = Command::new(program);
            command.args(args).current_dir(&self.repo_root);
            self.apply_isolated_env(&mut command);
            run_with_timeout(command, COMMAND_TIMEOUT).map_err(Into::into)
        }

        fn run_authenticated_interactive_host(
            &self,
            program: &Path,
            prompt: &str,
        ) -> Result<ExitStatus, Box<dyn Error>> {
            let mut command = Command::new(program);
            command
                .arg(prompt)
                .current_dir(&self.repo_root)
                .env("VOLICORD_HOME", &self.runtime_home_path)
                .env("PATH", &self.env_path)
                .env_remove(LIVE_HOST_RESULT_PATH_ENV)
                .stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit());
            Self::remove_inherited_host_control_env(&mut command);
            Ok(command.status()?)
        }

        fn run_authenticated_interactive_host_with_local_web(
            &self,
            program: &Path,
            prompt: &str,
        ) -> Result<ExitStatus, Box<dyn Error>> {
            let mut command = Command::new(program);
            command
                .arg(prompt)
                .current_dir(&self.repo_root)
                .env("VOLICORD_HOME", &self.runtime_home_path)
                .env("PATH", &self.env_path)
                .env_remove(LIVE_HOST_RESULT_PATH_ENV)
                .stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit());
            Self::remove_inherited_host_control_env(&mut command);
            command.env("VOLICORD_LOCAL_WEB_CONSENT", "1");
            Ok(command.status()?)
        }

        fn remove_inherited_host_control_env(command: &mut Command) {
            for name in [
                "VOLICORD_MCP_VERIFICATION",
                "VOLICORD_MCP_LAUNCH",
                "VOLICORD_MCP_HOST",
                "VOLICORD_MCP_CONNECTION_ID",
                "VOLICORD_MCP_PROJECT_ID",
                "VOLICORD_LOCAL_WEB_CONSENT",
            ] {
                command.env_remove(name);
            }
        }

        fn apply_isolated_env(&self, command: &mut Command) {
            command
                .env("VOLICORD_HOME", &self.runtime_home_path)
                .env("HOME", &self.home)
                .env("CODEX_HOME", &self.codex_home)
                .env("XDG_CONFIG_HOME", &self.xdg_config_home)
                .env("CLAUDE_CONFIG_DIR", &self.claude_config_dir)
                .env("PATH", &self.env_path)
                .env("NO_COLOR", "1")
                .env_remove("OPENAI_API_KEY")
                .env_remove("ANTHROPIC_API_KEY")
                .env_remove("CLAUDE_CODE_OAUTH_TOKEN")
                .env_remove("CLAUDE_CODE_API_KEY");
        }
    }

    struct TimedOutput {
        output: Output,
        timed_out: bool,
    }

    fn run_with_timeout(
        mut command: Command,
        timeout: Duration,
    ) -> Result<TimedOutput, std::io::Error> {
        let mut child = command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let deadline = Instant::now() + timeout;
        loop {
            if child.try_wait()?.is_some() {
                return child.wait_with_output().map(|output| TimedOutput {
                    output,
                    timed_out: false,
                });
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                return child.wait_with_output().map(|output| TimedOutput {
                    output,
                    timed_out: true,
                });
            }
            thread::sleep(Duration::from_millis(25));
        }
    }

    fn run_record_final_output_handler(
        runtime_home: &Path,
        repo_root: &Path,
        path: &OsString,
        event: &Value,
    ) -> Result<Output, Box<dyn Error>> {
        run_generated_final_output_handler(runtime_home, repo_root, path, "codex", event)
    }

    fn run_generated_final_output_handler(
        runtime_home: &Path,
        repo_root: &Path,
        path: &OsString,
        host: &str,
        event: &Value,
    ) -> Result<Output, Box<dyn Error>> {
        let mut child = Command::new(generated_stop_wrapper_path(repo_root, host)?)
            .env("VOLICORD_HOME", runtime_home)
            .env("PATH", path)
            .current_dir(repo_root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        child
            .stdin
            .as_mut()
            .ok_or("Stop hook stdin should be piped")?
            .write_all(event.to_string().as_bytes())?;
        drop(child.stdin.take());
        let deadline = Instant::now() + COMMAND_TIMEOUT;
        loop {
            if child.try_wait()?.is_some() {
                return Ok(child.wait_with_output()?);
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                let output = child.wait_with_output()?;
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    format!(
                        "generated final-output wrapper timed out after {} seconds: {}",
                        COMMAND_TIMEOUT.as_secs(),
                        stderr_output(&output)
                    ),
                )
                .into());
            }
            thread::sleep(Duration::from_millis(25));
        }
    }

    fn generated_stop_wrapper_path(
        repo_root: &Path,
        host: &str,
    ) -> Result<PathBuf, Box<dyn Error>> {
        let relative = match host {
            "codex" => ".codex/hooks/volicord-stop.sh",
            "claude-code" => ".claude/hooks/volicord-stop.sh",
            _ => {
                return Err(io::Error::other(format!(
                    "unsupported live final-output host {host:?}"
                ))
                .into())
            }
        };
        Ok(repo_root.join(relative))
    }

    fn live_final_output_event(
        host: &str,
        repo_root: &Path,
        session_id: &str,
        last_assistant_message: &str,
    ) -> Result<Value, Box<dyn Error>> {
        let common = serde_json::json!({
            "session_id": session_id,
            "transcript_path": format!("/tmp/{session_id}.jsonl"),
            "cwd": path_text(repo_root),
            "permission_mode": "default",
            "hook_event_name": "Stop",
            "stop_hook_active": false,
            "last_assistant_message": last_assistant_message
        });
        let mut object = common
            .as_object()
            .cloned()
            .ok_or_else(|| io::Error::other("live final-output event must be an object"))?;
        match host {
            "codex" => {
                object.insert("model".to_owned(), Value::String("gpt-5.5".to_owned()));
                object.insert(
                    "turn_id".to_owned(),
                    Value::String("live-final-output-turn".to_owned()),
                );
            }
            "claude-code" => {
                object.insert("background_tasks".to_owned(), Value::Array(Vec::new()));
                object.insert("session_crons".to_owned(), Value::Array(Vec::new()));
            }
            _ => {
                return Err(io::Error::other(format!(
                    "unsupported live final-output host {host:?}"
                ))
                .into())
            }
        }
        Ok(Value::Object(object))
    }

    struct NoActiveStatusWire {
        response_bytes: usize,
        private_model_prose_absent: bool,
        system_message: String,
    }

    fn verify_no_active_status_wire(
        output: &Output,
        forbidden_private_prose: &str,
    ) -> Result<NoActiveStatusWire, Box<dyn Error>> {
        if !output.status.success() {
            return Err(io::Error::other(format!(
                "generated final-output wrapper failed: {}",
                stderr_output(output)
            ))
            .into());
        }
        if output.stdout.len() > 8_192 {
            return Err(io::Error::other(
                "generated final-output wrapper exceeded the 8192-byte host response budget",
            )
            .into());
        }
        let wire: Value = serde_json::from_slice(&output.stdout)?;
        if wire["continue"] != true {
            return Err(io::Error::other(
                "no-active-Task final-output wire must allow host finalization",
            )
            .into());
        }
        let message = wire["systemMessage"].as_str().ok_or_else(|| {
            io::Error::other("no-active-Task final-output wire has no systemMessage")
        })?;
        if !message.contains("no active Task is available")
            || !message.contains("`volicord status --json`")
            || message.contains("volicord status --task")
        {
            return Err(io::Error::other(
                "no-active-Task final-output wire did not carry only the taskless status fallback",
            )
            .into());
        }
        if String::from_utf8_lossy(&output.stdout).contains(forbidden_private_prose) {
            return Err(io::Error::other(
                "generated no-active-Task final-output wire leaked private model prose",
            )
            .into());
        }
        Ok(NoActiveStatusWire {
            response_bytes: output.stdout.len(),
            private_model_prose_absent: true,
            system_message: message.to_owned(),
        })
    }

    fn verify_authority_receipt_wire(
        output: &Output,
        expected: &VerifiedLiveReceipt,
        expected_continue: bool,
        forbidden_private_prose: &str,
    ) -> Result<NoActiveStatusWire, Box<dyn Error>> {
        if !output.status.success() {
            return Err(io::Error::other(format!(
                "generated active-Task final-output wrapper failed: {}",
                stderr_output(output)
            ))
            .into());
        }
        if output.stdout.len() > 8_192 {
            return Err(io::Error::other(
                "generated AuthorityReceipt wrapper exceeded the 8192-byte host response budget",
            )
            .into());
        }
        let wire: Value = serde_json::from_slice(&output.stdout)?;
        if expected_continue {
            if wire["continue"] != true || wire.get("decision").is_some() {
                return Err(io::Error::other(format!(
                    "active-Task final-output allow wire {:?} did not continue as expected",
                    wire
                ))
                .into());
            }
        } else if wire["decision"] != "block"
            || wire["reason"]
                .as_str()
                .is_none_or(|reason| !reason.contains("close_readiness_blocked"))
        {
            return Err(io::Error::other(format!(
                "active-Task final-output block wire {:?} did not preserve the expected close-readiness decision",
                wire
            ))
            .into());
        }
        let message = wire["systemMessage"].as_str().ok_or_else(|| {
            io::Error::other("active-Task final-output wire has no systemMessage")
        })?;
        let receipt: AuthorityReceipt = serde_json::from_str(
            message
                .strip_prefix("Volicord authority receipt: ")
                .ok_or_else(|| {
                    io::Error::other(
                        "active-Task final-output wire does not contain a complete AuthorityReceipt",
                    )
                })?,
        )?;
        if receipt != expected.canonical_receipt {
            return Err(io::Error::other(
                "generated final-output AuthorityReceipt does not exactly match fresh Core status",
            )
            .into());
        }
        if String::from_utf8_lossy(&output.stdout).contains(forbidden_private_prose) {
            return Err(io::Error::other(
                "generated active-Task final-output wire leaked private model prose",
            )
            .into());
        }
        Ok(NoActiveStatusWire {
            response_bytes: output.stdout.len(),
            private_model_prose_absent: true,
            system_message: message.to_owned(),
        })
    }

    fn live_fixture_project_id(fixture: &LiveSmokeFixture) -> Result<String, Box<dyn Error>> {
        list_projects(&fixture.runtime_home_path)?
            .into_iter()
            .find(|project| project.repo_root == fixture.repo_root)
            .map(|project| project.project_id)
            .ok_or_else(|| io::Error::other("live smoke project registration is missing").into())
    }

    fn verify_final_output_config_fixture(
        fixture: &LiveSmokeFixture,
        host: &str,
        profile: IntegrationProfile,
        init: &Value,
    ) -> Result<Value, Box<dyn Error>> {
        if init["states"]["final_output_authority_disclosure"]
            != serde_json::json!({
                "supported": true,
                "configured": true,
                "verified": true
            })
        {
            return Err(io::Error::other(
                "init did not verify final-output authority disclosure capability",
            )
            .into());
        }
        let wrapper_path = generated_stop_wrapper_path(&fixture.repo_root, host)?;
        let wrapper = fs::read_to_string(&wrapper_path)?;
        let expected_command = match profile {
            IntegrationProfile::Record => "exec volicord _final-output",
            IntegrationProfile::Detective => "exec volicord _hook stop",
        };
        if !wrapper.contains(expected_command)
            || !wrapper.contains(&format!("--integration-profile {}", profile.as_str()))
        {
            return Err(io::Error::other(format!(
                "generated Stop wrapper does not match the {} profile",
                profile.as_str()
            ))
            .into());
        }
        let host_config = match host {
            "codex" => fixture.repo_root.join(".codex/hooks.json"),
            "claude-code" => fixture.repo_root.join(".claude/settings.json"),
            _ => return Err(io::Error::other("unsupported final-output host").into()),
        };
        let config_text = fs::read_to_string(&host_config)?;
        let expected_config_route = match (host, profile) {
            ("codex", IntegrationProfile::Detective) => "volicord-dispatch.sh",
            _ => "volicord-stop.sh",
        };
        if !config_text.contains("Stop") || !config_text.contains(expected_config_route) {
            return Err(io::Error::other(
                "generated host config has no profile-appropriate managed Stop final-output entry",
            )
            .into());
        }
        Ok(serde_json::json!({
            "status": "verified",
            "host_config": host_config.strip_prefix(&fixture.repo_root)?.display().to_string(),
            "generated_wrapper": wrapper_path.strip_prefix(&fixture.repo_root)?.display().to_string(),
            "profile_command_verified": true,
            "capability_supported": true,
            "capability_configured": true,
            "capability_verified": true
        }))
    }

    struct LatestLiveStopDecision {
        guard_event_id: String,
        session_id: String,
        decision: String,
    }

    fn live_stop_decision_after(
        fixture: &LiveSmokeFixture,
        project_id: &str,
        connection_id: &str,
        cursor: StopEventCursor,
    ) -> Result<LatestLiveStopDecision, Box<dyn Error>> {
        let project = list_projects(&fixture.runtime_home_path)?
            .into_iter()
            .find(|project| project.project_id == project_id)
            .ok_or_else(|| io::Error::other("live smoke project registration is missing"))?;
        let conn = open_project_state_database_read_only(&project.state_db_path)?;
        let mut statement = conn.prepare(
            "SELECT guard_event_id, session_id, decision, result_json
               FROM guard_events
              WHERE project_id = ?1
                AND connection_internal_id = ?2
                AND event_kind = 'stop'
                AND rowid > ?3
              ORDER BY rowid ASC",
        )?;
        let mut rows = statement.query(rusqlite::params![project_id, connection_id, cursor.0])?;
        let row = rows.next()?.ok_or_else(|| {
            io::Error::other("actual host produced no new Stop event after the validation cursor")
        })?;
        let guard_event_id = row.get::<_, String>(0)?;
        let session_id = row.get::<_, Option<String>>(1)?;
        let decision = row.get::<_, String>(2)?;
        let result_json = row.get::<_, String>(3)?;
        if rows.next()?.is_some() {
            return Err(io::Error::other(
                "actual host produced more than one Stop event after the validation cursor",
            )
            .into());
        }
        let result: Value = serde_json::from_str(&result_json)?;
        if result["decision"] != decision || result["allowed"] != true {
            return Err(io::Error::other(
                "latest Detective Stop GuardEvent has inconsistent historical decision fields",
            )
            .into());
        }
        Ok(LatestLiveStopDecision {
            guard_event_id,
            session_id: session_id.ok_or_else(|| {
                io::Error::other("latest actual-host Stop GuardEvent has no session binding")
            })?,
            decision,
        })
    }

    fn initialize_git_repository(repo_root: &Path) -> Result<(), Box<dyn Error>> {
        let output = Command::new("git")
            .args(["init", "-q"])
            .current_dir(repo_root)
            .output()?;
        if !output.status.success() {
            return Err(io::Error::other(format!(
                "git init failed for the disposable live-host fixture: {}",
                stderr_output(&output)
            ))
            .into());
        }
        Ok(())
    }

    fn smoke_enabled(name: &str) -> bool {
        env::var(name).is_ok_and(|value| value == "1")
    }

    fn smoke_note(host: &str, note: impl AsRef<str>) {
        println!("live {host} smoke: {}", note.as_ref());
    }

    fn find_executable(program: &str) -> Option<PathBuf> {
        let path = env::var_os("PATH")?;
        env::split_paths(&path)
            .map(|directory| directory.join(program))
            .find(|candidate| candidate.is_file())
    }

    fn write_volicord_shim(dir: &Path, target: &Path) -> Result<PathBuf, Box<dyn Error>> {
        let path = dir.join("volicord");
        let script = format!("#!/bin/sh\nexec {} \"$@\"\n", shell_quote(target));
        fs::write(&path, script)?;
        make_executable(&path)?;
        Ok(path)
    }

    fn make_executable(path: &Path) -> Result<(), Box<dyn Error>> {
        let mut permissions = fs::metadata(path)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions)?;
        Ok(())
    }

    fn shell_quote(path: &Path) -> String {
        format!("'{}'", path_text(path).replace('\'', "'\\''"))
    }

    fn path_with_prefix(prefix: &Path) -> Result<OsString, Box<dyn Error>> {
        let mut paths = vec![prefix.to_path_buf()];
        if let Some(existing) = env::var_os("PATH") {
            paths.extend(env::split_paths(&existing));
        }
        Ok(env::join_paths(paths)?)
    }

    fn assert_success(command: &str, output: &TimedOutput) {
        assert!(
            !output.timed_out,
            "{command} timed out\nstdout:\n{}\nstderr:\n{}",
            stdout(output),
            stderr(output)
        );
        assert!(
            output.output.status.success(),
            "{command} failed with status {}\nstdout:\n{}\nstderr:\n{}",
            status_text(output.output.status),
            stdout(output),
            stderr(output)
        );
    }

    fn require_success(command: &str, output: &TimedOutput) -> Result<(), Box<dyn Error>> {
        if output.timed_out {
            return Err(io::Error::other(format!("{command} timed out")).into());
        }
        if !output.output.status.success() {
            return Err(io::Error::other(format!(
                "{command} failed with status {}",
                status_text(output.output.status)
            ))
            .into());
        }
        Ok(())
    }

    fn require_live_init_reported_action_required(
        value: &Value,
        host: &str,
        profile: IntegrationProfile,
        host_action: &str,
    ) -> Result<(), Box<dyn Error>> {
        let actions = value["actions"]
            .as_array()
            .ok_or_else(|| io::Error::other("live init actions are not an array"))?;
        let has_action = |expected: &str| actions.iter().any(|action| action["id"] == expected);
        let expected_disclosure = serde_json::json!({
            "supported": true,
            "configured": true,
            "verified": true
        });
        let profile_state_matches = match profile {
            IntegrationProfile::Record => {
                value["states"]["hook_config"] == "disabled"
                    && value["states"]["guard_effective"] == "inactive"
                    && value["states"]["prompt_capture"] == "not_configured"
            }
            IntegrationProfile::Detective => {
                value["states"]["guard_installation"] == "reload_required"
                    && value["states"]["prompt_capture"] == "reload_required"
                    && has_action(host_action)
            }
        };
        if value["host"] != host
            || value["selected_profile"] != profile.as_str()
            || value["status"] != "action_required"
            || value["states"]["host_reload_required"] != true
            || value["states"]["final_output_authority_disclosure"] != expected_disclosure
            || !profile_state_matches
            || !has_action("reload_required")
        {
            return Err(io::Error::other(format!(
                "{host}/{} live init did not preserve the required managed-host configuration state",
                profile.as_str()
            ))
            .into());
        }
        Ok(())
    }

    fn assert_live_init_reported_action_required(
        value: &Value,
        host: &str,
        profile: IntegrationProfile,
        host_action: &str,
    ) {
        assert_eq!(value["host"], host, "unexpected live init host: {value}");
        assert_eq!(
            value["selected_profile"],
            profile.as_str(),
            "unexpected live init profile: {value}"
        );
        assert_eq!(
            value["status"],
            "action_required",
            "{host}/{} live init did not reach action_required: {value}",
            profile.as_str()
        );
        assert_eq!(value["states"]["host_reload_required"], true);
        assert_eq!(
            value["states"]["final_output_authority_disclosure"],
            serde_json::json!({
                "supported": true,
                "configured": true,
                "verified": true
            })
        );
        match profile {
            IntegrationProfile::Record => {
                assert_eq!(value["states"]["hook_config"], "disabled");
                assert_eq!(value["states"]["guard_effective"], "inactive");
                assert_eq!(value["states"]["prompt_capture"], "not_configured");
            }
            IntegrationProfile::Detective => {
                assert_eq!(value["states"]["guard_installation"], "reload_required");
                assert_eq!(value["states"]["prompt_capture"], "reload_required");
                assert_action(value, host_action);
            }
        }

        assert_action(value, "reload_required");
    }

    fn assert_direct_matrix_init_report(
        value: &Value,
        host: &str,
        profile: IntegrationProfile,
        host_action: &str,
    ) {
        assert_eq!(value["host"], host, "unexpected matrix init host: {value}");
        assert_eq!(
            value["selected_profile"],
            profile.as_str(),
            "unexpected matrix init profile: {value}"
        );
        assert!(
            matches!(value["status"].as_str(), Some("action_required" | "failed")),
            "{host}/{} direct matrix init has an unsupported status: {value}",
            profile.as_str()
        );
        assert_eq!(value["states"]["host_reload_required"], true);
        assert_eq!(
            value["states"]["final_output_authority_disclosure"],
            serde_json::json!({
                "supported": true,
                "configured": true,
                "verified": true
            })
        );
        let actions = value["actions"]
            .as_array()
            .expect("matrix init actions should be an array");
        assert!(
            actions.iter().any(|action| action["id"] == host_action)
                || (host == "codex"
                    && actions
                        .iter()
                        .any(|action| { action["id"] == "managed_host_startup_not_observed" })),
            "{host}/{} direct matrix init has no expected host action: {actions:?}",
            profile.as_str()
        );
        assert_action(value, "reload_required");
        if value["status"] == "failed" {
            assert_eq!(
                value["states"]["mcp_config"], "missing",
                "the direct matrix permits only the fake host's known MCP-discovery failure"
            );
        }
    }

    fn assert_action(value: &Value, expected: &str) {
        let actions = value["actions"]
            .as_array()
            .expect("actions should be an array");
        assert!(
            actions.iter().any(|action| action["id"] == expected),
            "expected action {expected:?}, got {actions:?}"
        );
    }

    fn assert_codex_mcp_entry(value: &Value) {
        let command = value
            .get("command")
            .and_then(Value::as_str)
            .unwrap_or_default();
        assert_eq!(command, "volicord", "unexpected Codex MCP entry: {value}");
        assert_eq!(
            value.get("args"),
            Some(&serde_json::json!([
                "mcp",
                "--stdio",
                "--discover-repository",
                "--host",
                "codex"
            ])),
            "Codex MCP args should use portable repository discovery: {value}"
        );
        assert!(
            value
                .get("env")
                .is_none_or(|env| env.as_object().is_some_and(serde_json::Map::is_empty)),
            "Codex repository-visible MCP entry must not carry local env: {value}"
        );
    }

    fn assert_claude_mcp_inspect_output(output: &TimedOutput) {
        assert!(
            !output.timed_out,
            "claude mcp get volicord timed out\nstdout:\n{}\nstderr:\n{}",
            stdout(output),
            stderr(output)
        );
        let text = output_text(output);
        let interpretable = text.contains("Status:")
            || text.contains("Connected")
            || text.contains("Pending")
            || text.contains("approval")
            || text.contains("volicord");
        assert!(
            output.output.status.success() || interpretable,
            "claude mcp get volicord returned unsupported output\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
            status_text(output.output.status),
            stdout(output),
            stderr(output)
        );
        assert!(
            interpretable,
            "claude mcp get volicord did not include recognizable MCP state\nstdout:\n{}\nstderr:\n{}",
            stdout(output),
            stderr(output)
        );
    }

    fn assert_file_contains(path: &Path, needle: &str) -> Result<(), Box<dyn Error>> {
        let text = fs::read_to_string(path)?;
        assert!(
            text.contains(needle),
            "{} did not contain {needle:?}\n{text}",
            path.display()
        );
        Ok(())
    }

    fn json_stdout(output: &TimedOutput) -> Result<Value, Box<dyn Error>> {
        Ok(serde_json::from_slice(&output.output.stdout)?)
    }

    fn stdout(output: &TimedOutput) -> String {
        String::from_utf8_lossy(&output.output.stdout).into_owned()
    }

    fn stderr(output: &TimedOutput) -> String {
        String::from_utf8_lossy(&output.output.stderr).into_owned()
    }

    fn output_text(output: &TimedOutput) -> String {
        format!("{}\n{}", stdout(output), stderr(output))
    }

    fn stderr_output(output: &Output) -> String {
        String::from_utf8_lossy(&output.stderr).into_owned()
    }

    fn status_text(status: ExitStatus) -> String {
        status
            .code()
            .map(|code| code.to_string())
            .unwrap_or_else(|| "without exit status".to_owned())
    }

    fn path_text(path: &Path) -> String {
        path.display().to_string()
    }

    fn volicord_bin() -> &'static str {
        env!("CARGO_BIN_EXE_volicord")
    }
}
