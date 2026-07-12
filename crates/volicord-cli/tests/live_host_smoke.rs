#![forbid(unsafe_code)]

#[cfg(unix)]
mod unix {
    use std::{
        env,
        error::Error,
        ffi::OsString,
        fs, io,
        os::unix::fs::PermissionsExt,
        path::{Path, PathBuf},
        process::{Command, ExitStatus, Output, Stdio},
        thread,
        time::{Duration, Instant},
    };

    use rusqlite::OptionalExtension;
    use serde_json::Value;
    use volicord_store::{
        agent_connections::list_agent_connections, bootstrap::list_projects,
        diagnostics::diagnostics_db_path, sqlite::open_project_state_database_read_only,
    };
    use volicord_test_support::TempRuntimeHome;
    use volicord_types::VERIFICATION_BASIS_MCP_ELICITATION_USER_CHANNEL;

    const CODEX_SMOKE_ENV: &str = "VOLICORD_RUN_CODEX_SMOKE";
    const CLAUDE_SMOKE_ENV: &str = "VOLICORD_RUN_CLAUDE_SMOKE";
    const CODEX_JUDGMENT_SMOKE_ENV: &str = "VOLICORD_RUN_CODEX_JUDGMENT_SMOKE";
    const CLAUDE_JUDGMENT_SMOKE_ENV: &str = "VOLICORD_RUN_CLAUDE_JUDGMENT_SMOKE";
    const JUDGMENT_ROUTE_ALPHA_OPTION_ID: &str = "route_alpha";
    const JUDGMENT_ROUTE_BETA_OPTION_ID: &str = "route_beta";
    const JUDGMENT_ROUTE_ALPHA_RUN_MARKER: &str =
        "VOLICORD_LIVE_HOST_JUDGMENT_CONSUMED_ROUTE_ALPHA";
    const JUDGMENT_ROUTE_BETA_RUN_MARKER: &str = "VOLICORD_LIVE_HOST_JUDGMENT_CONSUMED_ROUTE_BETA";
    const COMMAND_TIMEOUT: Duration = Duration::from_secs(20);

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
        let executable = find_executable(executable_name).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("`{executable_name}` was not found on PATH"),
            )
        })?;
        let fixture = LiveSmokeFixture::new(&format!("{host}-judgment"))?;
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
            assert_actionable_inbox_fallback(&fixture, &observation)?;
            return Err(io::Error::other(format!(
                "host-native MCP elicitation was unavailable, so Judgment `{}` remains pending; the actionable CLI inbox fallback above was verified",
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
        let selected_option_id = observation
            .selected_option_id
            .as_deref()
            .expect("a resolved live Judgment must store selected_option_id");
        let expected_run_marker = run_marker_for_selected_option(selected_option_id)
            .unwrap_or_else(|| panic!("unexpected live Judgment option {selected_option_id:?}"));
        let latest_run = observation
            .latest_run
            .as_ref()
            .expect("the agent must record a choice-dependent Run after resolving the Judgment");
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
            observation.state_version >= 4,
            "intake, Judgment creation, User Channel recording, and the choice-consumption Run must advance Task state"
        );
        assert_ne!(
            observation.lifecycle_phase, "waiting_user",
            "a resolved sole Judgment must leave the Task out of waiting_user"
        );
        assert!(observation
            .authority_event_kinds
            .iter()
            .any(|kind| kind == "user_judgment_requested"));
        assert!(observation
            .authority_event_kinds
            .iter()
            .any(|kind| kind == "user_judgment_recorded"));
        assert!(observation
            .authority_event_kinds
            .iter()
            .any(|kind| kind == "run_recorded"));
        assert_native_channel_diagnostic(&fixture)?;

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
        assert_eq!(
            status_json["active_task"]["task_ref"]["record_id"], observation.task_id,
            "CLI status must expose the updated Task"
        );
        smoke_note(
            host,
            format!(
                "verified Judgment {}, selected option {}, consumed marker {}, User Channel basis {}, Task phase {}, state_version {}",
                observation.judgment_id.as_deref().unwrap_or("unknown"),
                selected_option_id,
                expected_run_marker,
                VERIFICATION_BASIS_MCP_ELICITATION_USER_CHANNEL,
                observation.lifecycle_phase,
                observation.state_version
            ),
        );
        Ok(())
    }

    fn live_judgment_prompt(marker: &str) -> String {
        format!(
            concat!(
                "Run a human-in-the-loop Volicord connection smoke using the MCP server named `volicord`. ",
                "Do not edit files, run shell commands, prepare a write, or answer on the user's behalf.\n\n",
                "1. Call `volicord.intake` with `detail=full`, in work mode, with create-new resume behavior. The plain-language request must be exactly `{task_marker}`. Use a narrow no-write initial scope and one `not_required` acceptance criterion. Retain the returned Task ID, current Change Unit ID, and `state.baseline_ref` for step 4.\n",
                "2. For the returned Task, call `volicord.request_user_judgment` for a `product_decision` and omit `detail` so the default compact projection is exercised. Ask which live-smoke route the agent must consume, make it required for `close_complete`, and provide exactly these two caller-authored options in this order:\n",
                "   - `option_id={alpha_option_id}`, label `Route alpha`, description `Select the alpha live-smoke route.`, consequence `The agent records the alpha choice-consumption Run marker.`, `is_default=false`.\n",
                "   - `option_id={beta_option_id}`, label `Route beta`, description `Select the beta live-smoke route.`, consequence `The agent records the beta choice-consumption Run marker.`, `is_default=false`.\n",
                "3. Wait for the host's native MCP elicitation/User Channel UI. The human running this smoke will choose the answer. Never infer, fabricate, or submit that answer yourself.\n",
                "4. After Volicord reports the Judgment resolved, consume `structuredContent.method_result.selected_option_id` from that default result. If it is `{alpha_option_id}`, call `volicord.record_run` with summary exactly `{alpha_run_marker}`. If it is `{beta_option_id}`, call `volicord.record_run` with summary exactly `{beta_run_marker}`. Use the retained Task ID, Change Unit ID, and baseline ref; set `kind=shaping_update`; report `changed_paths=[]`, `product_file_write_observed=false`, `sensitive_categories=[]`, and the same baseline ref in `observed_changes`; do not use a write ticket, artifacts, evidence updates, or a close assessment. Do not record a Run if the selected option is absent or unrecognized.\n",
                "5. After that Run is recorded, call `volicord.status` for the Task and report the selected option ID, exact Run marker, lifecycle phase, and state version. Then stop.\n\n",
                "If a native prompt does not appear and Volicord returns a pending inbox item, do not simulate an answer. Report the exact fallback and stop so the harness can verify `volicord inbox` and `volicord inbox answer`."
            ),
            task_marker = marker,
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
        kind: String,
        summary: String,
        product_file_write_observed: bool,
        changed_paths: Vec<String>,
    }

    #[derive(Debug)]
    struct LiveJudgmentObservation {
        task_id: String,
        lifecycle_phase: String,
        state_version: u64,
        judgment_id: Option<String>,
        judgment_status: Option<String>,
        resolved_by_actor_source: Option<String>,
        resolved_verification_basis: Option<String>,
        selected_option_id: Option<String>,
        option_ids: Vec<String>,
        latest_run: Option<LiveRunObservation>,
        authority_event_kinds: Vec<String>,
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
        let latest_run = conn
            .query_row(
                "SELECT kind, summary_json, observed_changes_json
                   FROM runs
                  WHERE project_id = ?1
                    AND task_id = ?2
                    AND status = 'recorded'
                  ORDER BY created_at DESC, run_id DESC
                  LIMIT 1",
                rusqlite::params![project.project_id, task_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()?
            .map(|(kind, summary_json, observed_changes_json)| {
                live_run_observation(&kind, &summary_json, &observed_changes_json)
            })
            .transpose()?;
        let mut statement = conn.prepare(
            "SELECT event_type
               FROM authority_events
              WHERE project_id = ?1 AND task_id = ?2
              ORDER BY event_seq",
        )?;
        let authority_event_kinds = statement
            .query_map(rusqlite::params![project.project_id, task_id], |row| {
                row.get::<_, String>(0)
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Some(LiveJudgmentObservation {
            task_id,
            lifecycle_phase,
            state_version,
            judgment_id,
            judgment_status,
            resolved_by_actor_source,
            resolved_verification_basis,
            selected_option_id,
            option_ids,
            latest_run,
            authority_event_kinds,
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
            kind: kind.to_owned(),
            summary,
            product_file_write_observed,
            changed_paths,
        })
    }

    fn assert_actionable_inbox_fallback(
        fixture: &LiveSmokeFixture,
        observation: &LiveJudgmentObservation,
    ) -> Result<(), Box<dyn Error>> {
        let judgment_id = observation
            .judgment_id
            .as_deref()
            .ok_or_else(|| io::Error::other("pending Judgment id is missing"))?;
        let choice = observation
            .option_ids
            .first()
            .map(String::as_str)
            .unwrap_or("<choice>");
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
        println!(
            concat!(
                "\nVerified actionable CLI inbox fallback:\n",
                "  volicord inbox --repo {} --task {} --json\n",
                "  volicord inbox answer {} --choice {} --repo {} --json\n"
            ),
            shell_quote(&fixture.repo_root),
            observation.task_id,
            judgment_id,
            choice,
            shell_quote(&fixture.repo_root),
        );
        Ok(())
    }

    fn assert_native_channel_diagnostic(fixture: &LiveSmokeFixture) -> Result<(), Box<dyn Error>> {
        let connections = list_agent_connections(&fixture.runtime_home_path)?;
        let connection_ids = connections
            .iter()
            .map(|connection| connection.connection_internal_id.as_str())
            .collect::<Vec<_>>();
        let conn = rusqlite::Connection::open_with_flags(
            diagnostics_db_path(&fixture.runtime_home_path),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )?;
        let observed = connection_ids
            .iter()
            .try_fold(0_u64, |total, connection_id| {
                conn.query_row(
                    "SELECT COUNT(*)
                   FROM diagnostic_events e
                   JOIN diagnostic_sessions s ON s.session_id = e.session_id
                  WHERE s.connection_id = ?1
                    AND e.tool_name = 'volicord.request_user_judgment'
                    AND e.user_channel_kind = 'mcp_elicitation'",
                    [connection_id],
                    |row| row.get::<_, u64>(0),
                )
                .map(|count| total + count)
            })?;
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
