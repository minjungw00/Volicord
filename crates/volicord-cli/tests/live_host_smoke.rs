#![forbid(unsafe_code)]

#[cfg(unix)]
mod unix {
    use std::{
        env,
        error::Error,
        ffi::OsString,
        fs::{self, OpenOptions},
        io::{self, Write},
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
        bootstrap::list_projects, diagnostics::diagnostics_db_path,
        sqlite::open_project_state_database_read_only,
    };
    use volicord_test_support::{core_fixtures::CoreFixture, TempRuntimeHome};
    use volicord_types::{
        AuthorityReceipt, StateRecordKind, StatusCloseState, StatusResult,
        VERIFICATION_BASIS_MCP_ELICITATION_USER_CHANNEL, VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
    };

    const CODEX_SMOKE_ENV: &str = "VOLICORD_RUN_CODEX_SMOKE";
    const CLAUDE_SMOKE_ENV: &str = "VOLICORD_RUN_CLAUDE_SMOKE";
    const CODEX_JUDGMENT_SMOKE_ENV: &str = "VOLICORD_RUN_CODEX_JUDGMENT_SMOKE";
    const CLAUDE_JUDGMENT_SMOKE_ENV: &str = "VOLICORD_RUN_CLAUDE_JUDGMENT_SMOKE";
    const LIVE_HOST_RESULT_PATH_ENV: &str = "VOLICORD_LIVE_HOST_RESULT_PATH";
    const JUDGMENT_ROUTE_ALPHA_OPTION_ID: &str = "route_alpha";
    const JUDGMENT_ROUTE_BETA_OPTION_ID: &str = "route_beta";
    const JUDGMENT_ROUTE_ALPHA_RUN_MARKER: &str =
        "VOLICORD_LIVE_HOST_JUDGMENT_CONSUMED_ROUTE_ALPHA";
    const JUDGMENT_ROUTE_BETA_RUN_MARKER: &str = "VOLICORD_LIVE_HOST_JUDGMENT_CONSUMED_ROUTE_BETA";
    const LIVE_HOST_BASELINE_REF: &str = "baseline_live_host_judgment";
    const LIVE_INBOX_COMMAND_TEMPLATE: &str =
        "VOLICORD_HOME=<runtime-home> volicord inbox --repo <repo> --task <task-id> --json";
    const LIVE_INBOX_ANSWER_COMMAND_TEMPLATE: &str = "VOLICORD_HOME=<runtime-home> volicord inbox answer <judgment-id> --choice <option-id> --repo <repo> --json";
    const LIVE_INBOX_ANSWER_USAGE: &str = "volicord inbox answer <judgment-id> --choice <choice> [--repo PATH] [--note TEXT] [--json]";
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
            "kind": "live_host_judgment_release_validation",
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
            parse_native_judgment_choice("choice:route_alpha")?,
            JUDGMENT_ROUTE_ALPHA_OPTION_ID
        );
        assert_eq!(
            parse_native_judgment_choice("choice:route_beta")?,
            JUDGMENT_ROUTE_BETA_OPTION_ID
        );
        assert!(parse_native_judgment_choice("route_alpha").is_err());
        assert!(parse_native_judgment_choice("choice:unrecognized").is_err());
        Ok(())
    }

    #[test]
    fn advisor_shaping_close_basis_allows_active_task_stop_with_fresh_receipt(
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
                    "scope_summary": "No-write live-host Judgment validation.",
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

        let marker = JUDGMENT_ROUTE_ALPHA_RUN_MARKER;
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
        let observation = LiveJudgmentObservation {
            project_id: fixture.project_id().to_owned(),
            task_id: task_id.clone(),
            lifecycle_phase: status.response_value["active_task"]["lifecycle"]["lifecycle_phase"]
                .as_str()
                .unwrap_or("unknown")
                .to_owned(),
            state_version,
            judgment_id: None,
            judgment_status: None,
            resolved_by_actor_source: None,
            resolved_verification_basis: None,
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

        let event = serde_json::json!({
            "event_id": "live_advisor_stop_ready",
            "session_id": "live_advisor_stop_ready_session",
            "connection_id": fixture.connection_id(),
            "host_kind": "codex",
            "message": "Advisor validation complete."
        });
        let stop = run_stop_hook(
            fixture.runtime_home_path(),
            &fixture.product_repo_path(),
            &event,
        )?;
        assert!(
            stop.status.success(),
            "Stop hook failed: {}",
            stderr_output(&stop)
        );
        let stop_json: Value = serde_json::from_slice(&stop.stdout)?;
        assert_eq!(
            stop_json["continue"], true,
            "unexpected Stop output: {stop_json:#}"
        );
        let message = stop_json["systemMessage"]
            .as_str()
            .ok_or("active ready Stop should render the fresh AuthorityReceipt")?;
        let stop_receipt: AuthorityReceipt = serde_json::from_str(
            message
                .strip_prefix("Volicord fresh AuthorityReceipt: ")
                .ok_or("Stop systemMessage should use the fresh receipt prefix")?,
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
        let stored_stop = verify_live_stop_guard_event(
            fixture.runtime_home_path(),
            fixture.connection_id(),
            &observation,
            &receipt,
        )?;
        assert_eq!(stored_stop.decision, "allow");
        assert_eq!(stored_stop.state_version, receipt.state_version);
        assert_eq!(stored_stop.latest_run_id, receipt.latest_run_id);
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
        assert_guarded_init_reported_action_required(&init_json, "codex", "host_trust_required");
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
        assert_guarded_init_reported_action_required(
            &init_json,
            "claude-code",
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
    #[ignore = "requires an authenticated interactive Codex host and VOLICORD_RUN_CODEX_JUDGMENT_SMOKE=1"]
    fn codex_live_judgment_round_trip_is_opt_in() -> Result<(), Box<dyn Error>> {
        live_judgment_round_trip(
            "codex",
            "codex",
            CODEX_JUDGMENT_SMOKE_ENV,
            "host_trust_required",
        )
    }

    #[test]
    #[ignore = "requires an authenticated interactive Claude Code host and VOLICORD_RUN_CLAUDE_JUDGMENT_SMOKE=1"]
    fn claude_code_live_judgment_round_trip_is_opt_in() -> Result<(), Box<dyn Error>> {
        live_judgment_round_trip(
            "claude-code",
            "claude",
            CLAUDE_JUDGMENT_SMOKE_ENV,
            "project_approval_required",
        )
    }

    fn live_judgment_round_trip(
        host: &str,
        executable_name: &str,
        selector_env: &str,
        expected_host_action: &str,
    ) -> Result<(), Box<dyn Error>> {
        if !smoke_enabled(selector_env) {
            return Err(io::Error::other(format!(
                "set {selector_env}=1 before running the ignored {host} Judgment smoke test"
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
        let fixture = LiveSmokeFixture::new(&format!("{host}-judgment"))?;
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
        assert_success("volicord init for live Judgment smoke", &init);
        let init_json = json_stdout(&init)?;
        assert_guarded_init_reported_action_required(&init_json, host, expected_host_action);
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
            "VOLICORD_LIVE_HOST_JUDGMENT_ROUND_TRIP_{}",
            host.replace('-', "_").to_ascii_uppercase()
        );
        let prompt = live_judgment_prompt(&marker);
        println!(
            "\n=== Volicord live {host} Judgment smoke ===\nThe host will receive this initial instruction and may ask you to trust the repository or approve its MCP server. When the host-native Judgment selector appears, choose one option yourself. Do not type credentials or secrets. Exit the host after it reports the final Volicord status.\n\n{prompt}\n=== end instruction ===\n"
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

        let observation = inspect_live_judgment(&fixture, &marker)?;
        let Some(observation) = observation else {
            return Err(io::Error::other(format!(
                "the live host did not create the marker Task `{marker}`; rerun the smoke, approve the generated Volicord MCP connection, and let the host complete the instructed intake call"
            ))
            .into());
        };
        if observation.judgment_id.is_none() {
            return Err(io::Error::other(format!(
                "Task `{}` was created but no product-decision Judgment was created; rerun the smoke and let the host complete `volicord.request_user_judgment`",
                observation.task_id
            ))
            .into());
        }
        if observation.judgment_status.as_deref() != Some("resolved") {
            let fallback = verify_ephemeral_inbox_fallback_shape(&fixture, &observation)?;
            result_recorder.record_final(&live_host_fallback_summary(
                &identity,
                &observation,
                &fallback,
            ))?;
            return Err(io::Error::other(format!(
                "host-native MCP elicitation was unavailable, so Judgment `{}` remains pending; CLI fallback command shape was verified only inside the disposable fixture",
                observation.judgment_id.as_deref().unwrap_or("unknown")
            ))
            .into());
        }

        assert_eq!(
            observation.resolved_by_actor_source.as_deref(),
            Some("local_user"),
            "resolved Judgment must be owned by the local user"
        );
        assert_eq!(
            observation.resolved_verification_basis.as_deref(),
            Some(VERIFICATION_BASIS_MCP_ELICITATION_USER_CHANNEL),
            "the live round trip must use the host-native MCP User Channel"
        );
        assert_eq!(
            observation.option_ids.len(),
            2,
            "the live Judgment must preserve exactly the two requested route options"
        );
        assert!(
            observation
                .option_ids
                .iter()
                .any(|option_id| option_id == JUDGMENT_ROUTE_ALPHA_OPTION_ID),
            "the live Judgment is missing the alpha route option"
        );
        assert!(
            observation
                .option_ids
                .iter()
                .any(|option_id| option_id == JUDGMENT_ROUTE_BETA_OPTION_ID),
            "the live Judgment is missing the beta route option"
        );
        let operator_choice_id = confirm_native_judgment_choice(host)?;
        let selected_option_id = observation
            .selected_option_id
            .as_deref()
            .expect("a resolved live Judgment must store selected_option_id");
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
            .unwrap_or_else(|| panic!("unexpected live Judgment option {selected_option_id:?}"));

        let status_output = fixture.run_volicord([
            "status",
            "--repo",
            fixture.repo_arg(),
            "--task",
            &observation.task_id,
            "--json",
        ])?;
        assert_success("volicord status after live Judgment", &status_output);
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
            "intake, Change Unit creation, Judgment creation, User Channel recording, and the choice-consumption Run must advance Task state"
        );
        assert_ne!(
            observation.lifecycle_phase, "waiting_user",
            "a resolved sole Judgment must leave the Task out of waiting_user"
        );
        let stop_observation = verify_live_stop_guard_event(
            &fixture.runtime_home_path,
            &identity.connection_id,
            &observation,
            &receipt,
        )?;
        assert_native_channel_diagnostic(
            &fixture,
            &identity.connection_id,
            &observation.project_id,
        )?;
        if let Err(error) =
            confirm_stop_system_message_authority_receipt(host, receipt.state_version)
        {
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
                "verified Judgment {}, selected option {}, consumed marker {}, User Channel basis {}, Task phase {}, state_version {}, Stop systemMessage receipt UI confirmed",
                observation.judgment_id.as_deref().unwrap_or("unknown"),
                selected_option_id,
                expected_run_marker,
                VERIFICATION_BASIS_MCP_ELICITATION_USER_CHANNEL,
                observation.lifecycle_phase,
                receipt.state_version
            ),
        );
        Ok(())
    }

    fn live_judgment_prompt(marker: &str) -> String {
        format!(
            concat!(
                "Run a human-in-the-loop Volicord connection smoke using the MCP server named `volicord`. ",
                "Do not edit files, run shell commands, prepare a write, or answer on the user's behalf.\n\n",
                "1. Call `volicord.intake` with `detail=full`, `requested_mode=advisor`, `acceptance_policy=null`, and create-new resume behavior. The plain-language request must be exactly `{task_marker}`. Use a narrow no-write initial scope and exactly one acceptance criterion whose `evidence_requirement=not_required`. Retain the returned Task ID.\n",
                "2. For that Task, call `volicord.update_scope` with `detail=full`, `baseline_ref={baseline_ref}`, and a `change_unit` whose `operation=create_current`, `scope_summary` describes this no-write live-host Judgment validation, and `affected_paths=[]`. Retain `state.active_change_unit_ref.record_id` and `state.baseline_ref`. Do not continue unless both are present.\n",
                "3. Call `volicord.request_user_judgment` for a `product_decision` and omit `detail` so the default compact projection is exercised. Ask which live-smoke route the agent must consume, make it required for `close_complete`, and provide exactly these two caller-authored options in this order:\n",
                "   - `option_id={alpha_option_id}`, label `Route alpha`, description `Select the alpha live-smoke route.`, consequence `The agent records the alpha choice-consumption Run marker.`, `is_default=false`.\n",
                "   - `option_id={beta_option_id}`, label `Route beta`, description `Select the beta live-smoke route.`, consequence `The agent records the beta choice-consumption Run marker.`, `is_default=false`.\n",
                "4. Wait for the host's native MCP elicitation/User Channel UI. The human running this smoke will choose the answer. Never infer, fabricate, or submit that answer yourself.\n",
                "5. After Volicord reports the Judgment resolved, consume `structuredContent.method_result.selected_option_id` from that default result. If it is `{alpha_option_id}`, call `volicord.record_run` with summary exactly `{alpha_run_marker}`. If it is `{beta_option_id}`, call `volicord.record_run` with summary exactly `{beta_run_marker}`. Use the retained Task ID, Change Unit ID, and baseline ref; set `kind=shaping_update`, `run_id=null`, `write_ticket_id=null`, `artifact_inputs=[]`, `evidence_updates=[]`, and `evidence_observations=[]`; report `changed_paths=[]`, `product_file_write_observed=false`, `sensitive_categories=[]`, and the same baseline ref in `observed_changes`. Supply a non-null `close_assessment` whose `result_summary` is exactly the selected Run marker and whose `result_refs`, `residual_risks`, `sensitive_categories`, and `recovery_constraints` are all empty arrays. Do not record a Run if the selected option is absent or unrecognized.\n",
                "6. After that Run is recorded, call `volicord.status` for the Task and report the selected option ID, exact Run marker, lifecycle phase, close state, close-blocker count, and state version. Then stop.\n\n",
                "If a native prompt does not appear and Volicord returns a pending inbox item, do not simulate an answer or execute a fallback command. Report that the pending CLI inbox fallback is required and stop so the disposable harness can verify inbox visibility and the answer-command shape."
            ),
            task_marker = marker,
            baseline_ref = LIVE_HOST_BASELINE_REF,
            alpha_option_id = JUDGMENT_ROUTE_ALPHA_OPTION_ID,
            beta_option_id = JUDGMENT_ROUTE_BETA_OPTION_ID,
            alpha_run_marker = JUDGMENT_ROUTE_ALPHA_RUN_MARKER,
            beta_run_marker = JUDGMENT_ROUTE_BETA_RUN_MARKER,
        )
    }

    fn run_marker_for_selected_option(selected_option_id: &str) -> Option<&'static str> {
        match selected_option_id {
            JUDGMENT_ROUTE_ALPHA_OPTION_ID => Some(JUDGMENT_ROUTE_ALPHA_RUN_MARKER),
            JUDGMENT_ROUTE_BETA_OPTION_ID => Some(JUDGMENT_ROUTE_BETA_RUN_MARKER),
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
    }

    #[derive(Debug)]
    struct LiveJudgmentObservation {
        project_id: String,
        task_id: String,
        lifecycle_phase: String,
        state_version: u64,
        judgment_id: Option<String>,
        judgment_status: Option<String>,
        resolved_by_actor_source: Option<String>,
        resolved_verification_basis: Option<String>,
        selected_option_id: Option<String>,
        option_ids: Vec<String>,
    }

    fn inspect_live_judgment(
        fixture: &LiveSmokeFixture,
        marker: &str,
    ) -> Result<Option<LiveJudgmentObservation>, Box<dyn Error>> {
        let projects = list_projects(&fixture.runtime_home_path)?;
        let project = projects
            .iter()
            .find(|project| project.repo_root == fixture.repo_root)
            .ok_or_else(|| io::Error::other("live smoke project registration is missing"))?;
        let conn = open_project_state_database_read_only(&project.state_db_path)?;
        let row = conn
            .query_row(
                "SELECT t.task_id, t.lifecycle_phase, ps.state_version,
                        j.judgment_id, j.status, j.resolved_by_actor_source,
                        j.resolved_verification_basis, j.options_json,
                        j.resolution_json
                   FROM tasks t
                   JOIN project_state ps ON ps.project_id = t.project_id
              LEFT JOIN user_judgments j
                     ON j.project_id = t.project_id
                    AND j.task_id = t.task_id
                    AND j.judgment_kind = 'product_decision'
                  WHERE t.project_id = ?1 AND t.summary = ?2
                  ORDER BY j.requested_at DESC
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
                    ))
                },
            )
            .optional()?;
        let Some((
            task_id,
            lifecycle_phase,
            state_version,
            judgment_id,
            judgment_status,
            resolved_by_actor_source,
            resolved_verification_basis,
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
            .and_then(|value| value.get("options").and_then(Value::as_array).cloned())
            .unwrap_or_default()
            .iter()
            .filter_map(|option| option.get("option_id").and_then(Value::as_str))
            .map(str::to_owned)
            .collect::<Vec<_>>();
        Ok(Some(LiveJudgmentObservation {
            project_id: project.project_id.clone(),
            task_id,
            lifecycle_phase,
            state_version,
            judgment_id,
            judgment_status,
            resolved_by_actor_source,
            resolved_verification_basis,
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
            .ok_or_else(|| io::Error::other("resolved live Judgment has no selected_option_id"))?;
        Ok(Some(selected_option_id.to_owned()))
    }

    fn live_run_observation(
        run_id: &str,
        kind: &str,
        summary_json: &str,
        observed_changes_json: &str,
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
        })
    }

    #[derive(Debug)]
    struct AuthorityEventOrder {
        user_judgment_requested_event_seq: u64,
        user_judgment_recorded_event_seq: u64,
        run_recorded_event_seq: u64,
    }

    fn inspect_live_choice_consumption(
        fixture: &LiveSmokeFixture,
        observation: &LiveJudgmentObservation,
        run_id: &str,
    ) -> Result<(LiveRunObservation, AuthorityEventOrder), Box<dyn Error>> {
        let projects = list_projects(&fixture.runtime_home_path)?;
        let project = projects
            .iter()
            .find(|project| project.project_id == observation.project_id)
            .ok_or_else(|| io::Error::other("live smoke project registration is missing"))?;
        let conn = open_project_state_database_read_only(&project.state_db_path)?;
        let (stored_run_id, kind, status, summary_json, observed_changes_json) = conn
            .query_row(
                "SELECT run_id, kind, status, summary_json, observed_changes_json
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
        let run =
            live_run_observation(&stored_run_id, &kind, &summary_json, &observed_changes_json)?;

        let judgment_id = observation
            .judgment_id
            .as_deref()
            .ok_or_else(|| io::Error::other("resolved live Judgment id is missing"))?;
        let selected_option_id = observation
            .selected_option_id
            .as_deref()
            .ok_or_else(|| io::Error::other("resolved live Judgment selection is missing"))?;
        let mut statement = conn.prepare(
            "SELECT event_seq, event_type, payload_json
               FROM authority_events
              WHERE project_id = ?1
                AND task_id = ?2
                AND event_type IN (
                    'user_judgment_requested',
                    'user_judgment_recorded',
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
                "user_judgment_requested"
                    if payload.get("judgment_id").and_then(Value::as_str) == Some(judgment_id) =>
                {
                    requested.push(event_seq);
                }
                "user_judgment_recorded"
                    if payload.get("judgment_id").and_then(Value::as_str) == Some(judgment_id) =>
                {
                    if payload.get("selected_option_id").and_then(Value::as_str)
                        != Some(selected_option_id)
                    {
                        return Err(io::Error::other(
                            "matching user_judgment_recorded event does not preserve the stored selected_option_id",
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
            exactly_one_event_seq("matching user_judgment_requested", requested.as_slice())?;
        let recorded_event_seq =
            exactly_one_event_seq("matching user_judgment_recorded", recorded.as_slice())?;
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
                user_judgment_requested_event_seq: requested_event_seq,
                user_judgment_recorded_event_seq: recorded_event_seq,
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

    struct LiveInboxFallback {
        inbox_command_template: &'static str,
        answer_command_template: &'static str,
    }

    struct LiveHostIdentity {
        host: String,
        host_version: String,
        volicord_build_id: String,
        connection_id: String,
    }

    fn verify_ephemeral_inbox_fallback_shape(
        fixture: &LiveSmokeFixture,
        observation: &LiveJudgmentObservation,
    ) -> Result<LiveInboxFallback, Box<dyn Error>> {
        let judgment_id = observation
            .judgment_id
            .as_deref()
            .ok_or_else(|| io::Error::other("pending Judgment id is missing"))?;
        if observation.option_ids.is_empty() {
            return Err(io::Error::other("pending Judgment has no fallback choices").into());
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
            inbox_text.contains(judgment_id),
            "CLI inbox did not include pending Judgment {judgment_id}: {inbox_text}"
        );
        let answer_help = fixture.run_volicord(["inbox", "answer", "--help"])?;
        assert_success("volicord inbox answer --help", &answer_help);
        assert!(
            stdout(&answer_help)
                .lines()
                .any(|line| line.trim() == LIVE_INBOX_ANSWER_USAGE),
            "CLI inbox answer help no longer matches the verified fallback command shape: {}",
            stdout(&answer_help)
        );
        println!(
            concat!(
                "\nVerified CLI fallback shape inside the disposable fixture. ",
                "These templates are not runnable recovery commands because the fixture is deleted after the test:\n",
                "  {}\n",
                "  {}\n"
            ),
            LIVE_INBOX_COMMAND_TEMPLATE, LIVE_INBOX_ANSWER_COMMAND_TEMPLATE,
        );
        Ok(LiveInboxFallback {
            inbox_command_template: LIVE_INBOX_COMMAND_TEMPLATE,
            answer_command_template: LIVE_INBOX_ANSWER_COMMAND_TEMPLATE,
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
        observation: &LiveJudgmentObservation,
        expected_result_summary: &str,
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
        if receipt.close_state != StatusCloseState::Ready
            || !receipt.close_blockers.is_empty()
            || status.close_state != Some(StatusCloseState::Ready)
            || status
                .close_blockers
                .as_ref()
                .is_none_or(|blockers| !blockers.is_empty())
        {
            return Err(io::Error::other(
                "fresh CLI status is not ready to close with an empty close-blocker set",
            )
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

    fn confirm_native_judgment_choice(host: &str) -> Result<String, Box<dyn Error>> {
        print!(
            "\nConfirm the option you personally selected in the {host} native Judgment UI. Type `choice:{JUDGMENT_ROUTE_ALPHA_OPTION_ID}` or `choice:{JUDGMENT_ROUTE_BETA_OPTION_ID}`: "
        );
        io::stdout().flush()?;
        let mut confirmation = String::new();
        if io::stdin().read_line(&mut confirmation)? == 0 {
            return Err(io::Error::other(
                "no operator confirmation was received for the native Judgment selection",
            )
            .into());
        }
        parse_native_judgment_choice(confirmation.trim())
    }

    fn parse_native_judgment_choice(confirmation: &str) -> Result<String, Box<dyn Error>> {
        let selected = confirmation
            .strip_prefix("choice:")
            .and_then(|option_id| run_marker_for_selected_option(option_id).map(|_| option_id))
            .ok_or_else(|| {
                io::Error::other(format!(
                    "operator selection confirmation must be `choice:{JUDGMENT_ROUTE_ALPHA_OPTION_ID}` or `choice:{JUDGMENT_ROUTE_BETA_OPTION_ID}`"
                ))
            })?;
        Ok(selected.to_owned())
    }

    struct VerifiedStopObservation {
        guard_event_id: String,
        session_id: String,
        connection_id: String,
        decision: String,
        state_version: u64,
        latest_run_id: String,
    }

    fn verify_live_stop_guard_event(
        runtime_home: &Path,
        connection_id: &str,
        observation: &LiveJudgmentObservation,
        receipt: &VerifiedLiveReceipt,
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
              ORDER BY rowid DESC",
        )?;
        let rows = statement.query_map(
            rusqlite::params![observation.project_id, connection_id],
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
            return Ok(VerifiedStopObservation {
                guard_event_id,
                session_id,
                connection_id: stored_connection_id,
                decision,
                state_version: stop_receipt.state_version,
                latest_run_id: stop_latest_run.record_id.as_str().to_owned(),
            });
        }
        Err(io::Error::other(
            "no Stop hook event for the live Task was recorded; the host did not provide the required final Stop-hook round trip",
        )
        .into())
    }

    fn confirm_stop_system_message_authority_receipt(
        host: &str,
        state_version: u64,
    ) -> Result<(), Box<dyn Error>> {
        let expected = format!("receipt:{state_version}");
        print!(
            "\nReview the separate Volicord Stop-hook `systemMessage` shown after the final {host} answer. Type `{expected}` only if that supported host UI surface showed the complete fresh AuthorityReceipt with state_version {state_version}. Type `missing` otherwise: "
        );
        io::stdout().flush()?;
        let mut confirmation = String::new();
        if io::stdin().read_line(&mut confirmation)? == 0 {
            return Err(io::Error::other(
                "no operator confirmation was received for the Stop-hook AuthorityReceipt systemMessage",
            )
            .into());
        }
        if confirmation.trim() != expected {
            return Err(io::Error::other(format!(
                "the operator did not confirm the Stop-hook AuthorityReceipt systemMessage bound to state_version {state_version}"
            ))
            .into());
        }
        Ok(())
    }

    struct LiveCompletedSummaryInput<'a> {
        result: &'a str,
        identity: &'a LiveHostIdentity,
        observation: &'a LiveJudgmentObservation,
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
            "kind": "live_host_judgment_release_validation",
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
            "judgment": {
                "judgment_id": observation.judgment_id,
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
                "user_judgment_requested_event_seq": authority_event_order.user_judgment_requested_event_seq,
                "user_judgment_recorded_event_seq": authority_event_order.user_judgment_recorded_event_seq,
                "run_recorded_event_seq": authority_event_order.run_recorded_event_seq,
                "ordered": authority_event_order.user_judgment_requested_event_seq
                    < authority_event_order.user_judgment_recorded_event_seq
                    && authority_event_order.user_judgment_recorded_event_seq
                        < authority_event_order.run_recorded_event_seq
            },
            "native_ui": {
                "judgment_selector_confirmed": true,
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
        observation: &LiveJudgmentObservation,
        operator_choice_id: &str,
        selected_option_id: &str,
    ) -> Value {
        serde_json::json!({
            "kind": "live_host_judgment_release_validation",
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
            "judgment": {
                "judgment_id": observation.judgment_id,
                "selected_option_id": selected_option_id,
                "operator_confirmed_option_id": operator_choice_id,
                "stored_choice_matches_operator": false,
                "user_channel_basis": observation.resolved_verification_basis
            },
            "native_ui": {
                "judgment_selector_confirmed": true,
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
        observation: &LiveJudgmentObservation,
        fallback: &LiveInboxFallback,
    ) -> Value {
        serde_json::json!({
            "kind": "live_host_judgment_release_validation",
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
            "judgment": {
                "judgment_id": observation.judgment_id,
                "status": observation.judgment_status
            },
            "native_ui": {
                "judgment_selector_confirmed": false,
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
                "answer_command_template": fallback.answer_command_template
            }
        })
    }

    struct LiveResultRecorder {
        host: String,
        result_path: Option<PathBuf>,
        run_id: String,
        started_at: String,
        started: bool,
        finalized: bool,
    }

    impl LiveResultRecorder {
        fn from_env(host: &str) -> Result<Self, Box<dyn Error>> {
            let result_path = env::var_os(LIVE_HOST_RESULT_PATH_ENV).map(PathBuf::from);
            Self::new(host, result_path)
        }

        fn new(host: &str, result_path: Option<PathBuf>) -> Result<Self, Box<dyn Error>> {
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
                result_path,
                run_id,
                started_at,
                started: false,
                finalized: false,
            };
            if recorder.result_path.is_some() {
                recorder.write_external_summary(
                    &serde_json::json!({
                        "kind": "live_host_judgment_release_validation",
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

    impl Drop for LiveResultRecorder {
        fn drop(&mut self) {
            if !self.started || self.finalized || self.result_path.is_none() {
                return;
            }
            let _ = self.write_external_summary(
                &serde_json::json!({
                    "kind": "live_host_judgment_release_validation",
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
                    AND e.tool_name = 'volicord.request_user_judgment'
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
            fs::create_dir_all(repo_root.join(".git"))?;
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
            Ok(command.status()?)
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

    fn run_stop_hook(
        runtime_home: &Path,
        repo_root: &Path,
        event: &Value,
    ) -> Result<Output, Box<dyn Error>> {
        let mut child = Command::new(volicord_bin())
            .args([
                "_hook",
                "stop",
                "--repo",
                &path_text(repo_root),
                "--host-output",
                "codex",
                "--integration-profile",
                "record",
            ])
            .env("VOLICORD_HOME", runtime_home)
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
        Ok(child.wait_with_output()?)
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

    fn assert_guarded_init_reported_action_required(value: &Value, host: &str, host_action: &str) {
        assert_eq!(value["host"], host);
        assert_eq!(value["selected_profile"], "detective");
        assert_eq!(value["status"], "action_required");
        assert_eq!(value["states"]["host_reload_required"], true);
        assert_eq!(value["states"]["guard_installation"], "reload_required");
        assert_eq!(value["states"]["prompt_capture"], "reload_required");
        assert_action(value, "reload_required");
        assert_action(value, host_action);
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
