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

    use serde_json::Value;
    use volicord_test_support::TempRuntimeHome;

    const CODEX_SMOKE_ENV: &str = "VOLICORD_RUN_CODEX_SMOKE";
    const CLAUDE_SMOKE_ENV: &str = "VOLICORD_RUN_CLAUDE_SMOKE";
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
            "[mcp_servers.volicord.env]",
        )?;
        assert_file_contains(
            &fixture.repo_root.join(".codex/config.toml"),
            "VOLICORD_MCP_LAUNCH = \"managed_host\"",
        )?;
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
        assert_file_contains(&fixture.repo_root.join(".mcp.json"), "\"volicord\"")?;
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
        let args = value
            .get("args")
            .and_then(Value::as_array)
            .expect("Codex MCP entry args should be an array");
        assert!(
            args.iter().any(|arg| arg == "mcp"),
            "Codex MCP args should include mcp: {value}"
        );
        assert!(
            args.iter().any(|arg| arg == "--stdio"),
            "Codex MCP args should include --stdio: {value}"
        );
        assert!(
            args.iter().any(|arg| arg == "--connection"),
            "Codex MCP args should include --connection: {value}"
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
