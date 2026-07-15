#![forbid(unsafe_code)]

mod support;

#[cfg(unix)]
mod unix {
    use std::{
        collections::{BTreeMap, BTreeSet},
        env,
        error::Error,
        ffi::{OsStr, OsString},
        fs::{self, OpenOptions},
        io::{self, IsTerminal, Read, Seek, SeekFrom, Write},
        os::unix::fs::{MetadataExt, PermissionsExt},
        panic::{catch_unwind, resume_unwind, AssertUnwindSafe},
        path::{Path, PathBuf},
        process::{Command, ExitStatus, Output, Stdio},
        thread,
        time::{Duration, Instant, SystemTime, UNIX_EPOCH},
    };

    use chrono::{DateTime, SecondsFormat, Utc};
    use rusqlite::OptionalExtension;
    use serde::{Deserialize, Serialize};
    use serde_json::Value;
    use sha2::{Digest, Sha256};
    use volicord_cli::host_integration::{
        capability_status::{
            canonical_codex_host_version_from_probe, default_host_feature_support_json_for_version,
            host_feature_implementation_for_version, HostFeature, HostFeatureDiagnosticProjection,
            HostFeatureImplementation, REVIEWED_CODEX_HOST_VERSION, REVIEWED_CODEX_MCP_CLIENT_NAME,
        },
        HostKind,
    };
    use volicord_mcp::{McpAdapter, McpConnectionContext};
    use volicord_release_validation_tests::io::{
        read_bounded_external_file, sha256_external_file, ResultRootLease, ValidationContext,
    };
    use volicord_store::{
        agent_connections::{
            agent_connection_record_read_only, CONNECTION_MODE_WORKFLOW, VERIFIED_STATUS_COMPLETE,
        },
        bootstrap::{list_projects, write_installation_profile, InstallationProfileRegistration},
        diagnostics::{
            diagnostics_db_path, record_diagnostic_event, start_diagnostic_session,
            DiagnosticEvent, DiagnosticEventKind, DiagnosticFallbackKind, DiagnosticHostKind,
            DiagnosticOutcome, DiagnosticSessionStart, DiagnosticTransport,
        },
        inspection::{inspect_runtime_home, DatabaseInspection},
        session_watch::{
            latest_watch_baseline_for_session, update_watch_status, watch_baseline,
            SessionWatchStatus, WatchBaselineRecord, WatchStatusUpdate,
        },
        sqlite::open_project_state_database_read_only,
    };
    use volicord_test_support::{core_fixtures::CoreFixture, TempRuntimeHome};
    use volicord_types::{
        canonical_json_bare_sha256, canonical_json_string, managed_host_session_id,
        validate_managed_host_session_id, ArtifactRef, AuthorityReceipt, EvidenceCoverageItem,
        EvidenceCoverageState, EvidenceProducer, EvidenceProducerKind, EvidenceRelevanceStatus,
        EvidenceTarget, IntegrationProfile, ManagedMcpClientInfo,
        PersistedEvidenceCaptureReceiptBody, PersistedEvidenceMetadata,
        PersistedEvidenceObservationAuthority, PersistedUserActionRequest, StateRecordKind,
        StateRecordRef, StatusCloseState, StatusResult, UserActionBasis, UserActionInboxForm,
        UserActionPresentationPlan, UserActionPresentationSafety, UserActionRequestBody,
        UserActionResolutionBody, USER_ACTION_OBSERVATION_SUMMARY_MAX_CHARS,
        VERIFICATION_BASIS_CLI_DIRECT_USER_CHANNEL, VERIFICATION_BASIS_LOCAL_USER_LOCAL_WEB,
        VERIFICATION_BASIS_MCP_ELICITATION_USER_CHANNEL,
        VERIFICATION_BASIS_MCP_STDIO_CONNECTION_BINDING, VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
    };

    use crate::support::fake_hosts::{
        write_counting_fake_codex_with_version, write_fake_claude_code, write_fake_codex,
    };

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
    const CODEX_VERIFIED_TOOL_PRODUCER_SMOKE_ENV: &str =
        "VOLICORD_RUN_CODEX_VERIFIED_TOOL_PRODUCER_SMOKE";
    const CLAUDE_VERIFIED_TOOL_PRODUCER_SMOKE_ENV: &str =
        "VOLICORD_RUN_CLAUDE_VERIFIED_TOOL_PRODUCER_SMOKE";
    const CODEX_REGISTERED_CONNECTION_OBSERVATION_SMOKE_ENV: &str =
        "VOLICORD_RUN_CODEX_REGISTERED_CONNECTION_OBSERVATION_SMOKE";
    const CLAUDE_REGISTERED_CONNECTION_OBSERVATION_SMOKE_ENV: &str =
        "VOLICORD_RUN_CLAUDE_REGISTERED_CONNECTION_OBSERVATION_SMOKE";
    const LIVE_HOST_RESULT_PATH_ENV: &str = "VOLICORD_LIVE_HOST_RESULT_PATH";
    const RELEASE_CANDIDATE_PATH_ENV: &str = "VOLICORD_RELEASE_CANDIDATE_PATH";
    const RELEASE_REQUEST_VERIFIED_ENV: &str = "VOLICORD_RELEASE_REQUEST_VERIFIED";
    const RELEASE_CANDIDATE_SCHEMA: &str = "volicord-release-candidate-v1";
    const RELEASE_CELL_SCHEMA: &str = "volicord-host-release-cell-v3";
    const RELEASE_SOURCE_ARCHIVE_ALGORITHM: &str = "git_archive_tar_sha256_v1";
    const LIVE_USER_ACTION_RESULT_KIND: &str = "live_host_user_action_release_validation";
    const LIVE_EVIDENCE_OBSERVATION_RESULT_KIND: &str =
        "live_host_evidence_observation_release_validation";
    const LIVE_CLI_FALLBACK_RESULT_KIND: &str = "live_host_cli_fallback_release_validation";
    const LIVE_FINAL_OUTPUT_RESULT_KIND: &str = "live_host_final_output_release_validation";
    const LIVE_VERIFIED_TOOL_PRODUCER_RESULT_KIND: &str =
        "live_host_verified_tool_producer_release_validation";
    const LIVE_REGISTERED_CONNECTION_OBSERVATION_RESULT_KIND: &str =
        "live_host_registered_connection_observation_release_validation";
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
    const LIVE_VERIFIED_TOOL_PRODUCER_BASELINE_REF: &str =
        "baseline_live_host_verified_tool_producer";
    const LIVE_REGISTERED_CONNECTION_OBSERVATION_BASELINE_REF: &str =
        "baseline_live_host_registered_connection_observation";
    const LIVE_VERIFIED_TOOL_NAME: &str = "Bash";
    const LIVE_VERIFIED_TOOL_COMMAND: &str = "true";
    const LIVE_PRODUCER_CALLER_OBSERVED_AT: &str = "2000-01-01T00:00:00Z";
    const LIVE_INBOX_COMMAND_TEMPLATE: &str =
        "VOLICORD_HOME=<runtime-home> volicord inbox --repo <repo> --task <task-id> --json";
    const LIVE_INBOX_RESOLVE_COMMAND_TEMPLATE: &str = "VOLICORD_HOME=<runtime-home> volicord inbox resolve <user-action-request-id> --choice <option-id> --repo <repo> --json";
    const LIVE_INBOX_RESOLVE_USAGE: &str = "volicord inbox resolve <user-action-request-id> --choice <choice> [--repo PATH] [--note TEXT] [--json]";
    const MAX_HOST_VERSION_BYTES: usize = 256;
    const MAX_CONNECTION_ID_BYTES: usize = 256;
    const MAX_BUILD_ID_BYTES: usize = 1_024;
    const MAX_VALIDATION_RUN_ID_BYTES: usize = 192;
    const MAX_RECORDED_AT_BYTES: usize = 64;
    const MAX_LIVE_HOST_RESULT_BYTES: usize = 16 * 1_024;

    fn create_live_result_root(root: &Path) -> Result<(PathBuf, PathBuf, PathBuf), Box<dyn Error>> {
        let cells = root.join("cells");
        let evidence = root.join("evidence");
        let auxiliary = root.join("auxiliary");
        fs::create_dir_all(&cells)?;
        fs::create_dir_all(&evidence)?;
        fs::create_dir_all(&auxiliary)?;
        Ok((cells, evidence, auxiliary))
    }
    const MAX_RELEASE_CANDIDATE_DESCRIPTOR_BYTES: usize = 64 * 1_024;
    const MAX_RELEASE_CANDIDATE_BINARY_BYTES: u64 = 256 * 1024 * 1024;
    const MAX_MANAGED_MCP_SESSIONS_PER_HOST_TURN: u64 = 8;
    const COMMAND_TIMEOUT: Duration = Duration::from_secs(20);

    #[test]
    fn live_result_helpers_create_one_bounded_immutable_result() -> Result<(), Box<dyn Error>> {
        let temp = TempRuntimeHome::new("live-result-recorder")?;
        let result_root = temp.product_repo_path("release-results");
        let (result_dir, _, auxiliary_dir) = create_live_result_root(&result_root)?;
        let result_path = result_dir.join("codex.json");

        let mut recorder = LiveResultRecorder::new("codex", Some(result_path.clone()))?;
        assert!(!result_path.exists());
        let run_id = recorder.run_id.clone();
        let diagnostics = canonical_release_host_feature_diagnostics(
            "codex",
            IntegrationProfile::Detective,
            true,
            true,
        );
        recorder.record_final(&serde_json::json!({
            "kind": "live_host_user_action_release_validation",
            "result": "passed",
            "host": { "kind": "codex" },
            "host_feature_support": diagnostics.host_feature_support,
            "final_output_authority_disclosure": diagnostics.final_output_authority_disclosure
        }))?;
        let completed: Value = serde_json::from_slice(&fs::read(&result_path)?)?;
        assert_eq!(completed["result"], "passed");
        validate_release_host_feature_diagnostics(
            &completed,
            Some(IntegrationProfile::Detective),
            true,
            true,
        )?;
        assert_eq!(completed["validation_run"]["run_id"], run_id);
        assert!(completed["validation_run"]["started_at"].is_string());
        assert!(completed["validation_run"]["recorded_at"].is_string());
        let mut shape_guard = LiveResultRecorder::new("codex", None)?;
        assert!(shape_guard
            .record_final(&serde_json::json!({
                "kind": LIVE_USER_ACTION_RESULT_KIND,
                "result": "passed",
                "host": { "kind": "codex" }
            }))
            .is_err());
        assert!(LiveResultRecorder::new("codex", Some(result_path.clone())).is_err());
        assert!(required_live_result_path(None).is_err());
        assert!(required_live_result_path(Some(OsString::new())).is_err());
        assert!(required_release_candidate_path(None).is_err());
        assert!(required_release_candidate_path(Some(OsString::new())).is_err());
        assert_eq!(
            required_live_result_path(Some(result_dir.join("required.json").into_os_string()))?,
            result_dir.join("required.json")
        );
        let path_context = release_validation_context()?;
        assert!(path_context
            .validate_new_output(Path::new("relative-result.json"))
            .is_err());
        assert!(path_context
            .validate_new_output(&Path::new(env!("CARGO_MANIFEST_DIR")).join("live-result.json"))
            .is_err());
        assert!(serialize_live_host_result(&serde_json::json!({
            "payload": "x".repeat(MAX_LIVE_HOST_RESULT_BYTES)
        }))
        .is_err());
        assert!(bounded_identity("bounded", "line\nbreak", 64).is_err());
        assert!(bounded_identity("bounded", &"x".repeat(65), 64).is_err());
        let multibyte_exact_limit = format!("{}a", "가".repeat(21));
        assert_eq!(multibyte_exact_limit.len(), 64);
        assert_eq!(
            bounded_identity("bounded", &multibyte_exact_limit, 64)?,
            multibyte_exact_limit
        );
        let multibyte_over_limit = "가".repeat(22);
        assert!(multibyte_over_limit.chars().count() < 64);
        assert!(multibyte_over_limit.len() > 64);
        assert!(bounded_identity("bounded", &multibyte_over_limit, 64).is_err());

        let initialized_metadata = serde_json::json!({
            "source": "volicord_session_watch",
            "launch_origin": "managed_host",
            "host_kind": "codex",
            "connection_id": "CONN-client-observation",
            "project_id": "PRJ-client-observation",
            "client_name": "codex-mcp-client",
            "client_version": "0.144.4",
            "lifecycle_events": [{
                "lifecycle_event": "managed_host_initialize_response",
                "launch_origin": "managed_host",
                "host_kind": "codex",
                "connection_id": "CONN-client-observation",
                "project_id": "PRJ-client-observation"
            }]
        });
        let initialized_client = initialized_client_info_from_watch_metadata(
            &initialized_metadata,
            "codex",
            "CONN-client-observation",
            "PRJ-client-observation",
        )?
        .ok_or_else(|| io::Error::other("exact managed initialize identity was not observed"))?;
        assert_eq!(initialized_client.name(), "codex-mcp-client");
        assert_eq!(initialized_client.version(), "0.144.4");
        assert!(initialized_client_info_from_watch_metadata(
            &initialized_metadata,
            "codex",
            "CONN-other",
            "PRJ-client-observation",
        )?
        .is_none());
        let mut partial_identity = initialized_metadata.clone();
        partial_identity
            .as_object_mut()
            .expect("fixture metadata object")
            .remove("client_version");
        assert!(initialized_client_info_from_watch_metadata(
            &partial_identity,
            "codex",
            "CONN-client-observation",
            "PRJ-client-observation",
        )
        .is_err());
        let mut missing_identity = initialized_metadata.clone();
        missing_identity["client_name"] = Value::Null;
        missing_identity["client_version"] = Value::Null;
        assert!(required_initialized_client_info_from_watch_metadata(
            &missing_identity,
            "codex",
            "CONN-client-observation",
            "PRJ-client-observation",
        )
        .is_err());
        let mut wrong_binding = initialized_metadata.clone();
        wrong_binding["source"] = Value::String("forged_session_watch".to_owned());
        wrong_binding["host_kind"] = Value::String("claude_code".to_owned());
        assert!(required_initialized_client_info_from_watch_metadata(
            &wrong_binding,
            "codex",
            "CONN-client-observation",
            "PRJ-client-observation",
        )
        .is_err());

        let (early_failure_dir, _, _) =
            create_live_result_root(&temp.product_repo_path("early-failure-results"))?;
        let early_failure_path = early_failure_dir.join("claude-code.json");
        {
            let _recorder =
                LiveResultRecorder::new("claude-code", Some(early_failure_path.clone()))?;
        }
        let early_failure: Value = serde_json::from_slice(&fs::read(&early_failure_path)?)?;
        assert_eq!(early_failure["result"], "failed_before_completion");
        validate_terminal_release_host_feature_diagnostics(&early_failure)?;
        assert_eq!(early_failure["host"]["kind"], "claude-code");
        assert_eq!(
            early_failure["host_feature_support"]
                .as_object()
                .map(serde_json::Map::len),
            Some(6)
        );
        assert_eq!(
            early_failure["final_output_authority_disclosure"]["configured"],
            false
        );

        let (observed_failure_dir, _, _) =
            create_live_result_root(&temp.product_repo_path("observed-failure-results"))?;
        let observed_early_failure_path =
            observed_failure_dir.join("codex-observed-early-failure.json");
        {
            let mut recorder =
                LiveResultRecorder::new("codex", Some(observed_early_failure_path.clone()))?;
            recorder.bind_observed_host_identity(ObservedReleaseHostIdentity::new(
                "codex fixture 1.0".to_owned(),
                "e".repeat(64),
                "fixture-build-observed-before-failure".to_owned(),
            )?)?;
        }
        let observed_early_failure: Value =
            serde_json::from_slice(&fs::read(&observed_early_failure_path)?)?;
        assert_eq!(observed_early_failure["result"], "failed_before_completion");
        assert_eq!(
            observed_early_failure["host"]["version"],
            "codex fixture 1.0"
        );
        assert_eq!(
            observed_early_failure["host"]["executable_sha256"],
            "e".repeat(64)
        );
        assert_eq!(
            observed_early_failure["volicord"]["build_id"],
            "fixture-build-observed-before-failure"
        );

        let (final_record_failure_dir, _, _) =
            create_live_result_root(&temp.product_repo_path("final-record-failure-results"))?;
        let final_record_failure_path =
            final_record_failure_dir.join("codex-record-final-output.json");
        {
            let _recorder = LiveResultRecorder::new_for_kind_and_profile(
                "codex-record-final-output",
                "codex",
                LIVE_FINAL_OUTPUT_RESULT_KIND,
                Some(IntegrationProfile::Record),
                Some(final_record_failure_path.clone()),
            )?;
        }
        let final_record_failure: Value =
            serde_json::from_slice(&fs::read(&final_record_failure_path)?)?;
        validate_terminal_release_host_feature_diagnostics(&final_record_failure)?;
        assert_eq!(final_record_failure["result"], "failed_before_completion");
        assert_eq!(final_record_failure["profile"], "record");
        assert_eq!(
            final_record_failure["final_output_authority_disclosure"]["required_subcapabilities"],
            serde_json::json!(["authority_display", "authenticated_exact_replay"])
        );

        let unselected_final_failure_path =
            auxiliary_dir.join("codex-unselected-final-output.json");
        {
            let _recorder = LiveResultRecorder::new_for_kind_and_profile(
                "codex-unselected-final-output",
                "codex",
                LIVE_FINAL_OUTPUT_RESULT_KIND,
                None,
                Some(unselected_final_failure_path.clone()),
            )?;
        }
        let unselected_final_failure: Value =
            serde_json::from_slice(&fs::read(&unselected_final_failure_path)?)?;
        validate_terminal_release_host_feature_diagnostics(&unselected_final_failure)?;
        assert!(unselected_final_failure["profile"].is_null());
        assert!(unselected_final_failure["final_output_authority_disclosure"].is_null());
        assert!(fs::read_dir(&result_dir)?.all(|entry| {
            entry
                .ok()
                .is_some_and(|entry| !entry.file_name().to_string_lossy().ends_with(".tmp"))
        }));
        Ok(())
    }

    #[test]
    fn live_producer_reuses_canonical_external_release_path_policy() -> Result<(), Box<dyn Error>> {
        fn candidate_fixture(
            candidate_path: &Path,
            descriptor_path: &Path,
        ) -> Result<ReleaseCandidate, Box<dyn Error>> {
            Ok(ReleaseCandidate {
                descriptor_path: Some(descriptor_path.to_path_buf()),
                schema: RELEASE_CANDIDATE_SCHEMA.to_owned(),
                candidate_id: "candidate_external_path_policy".to_owned(),
                candidate_path: path_text(candidate_path),
                source_revision: "a".repeat(40),
                source_clean: true,
                source_archive_algorithm: RELEASE_SOURCE_ARCHIVE_ALGORITHM.to_owned(),
                source_archive_sha256: "b".repeat(64),
                target_triple: "fixture-target".to_owned(),
                release_profile: "release".to_owned(),
                binary_sha256: sha256_file(candidate_path, MAX_RELEASE_CANDIDATE_BINARY_BYTES)?,
                build_environment: ReleaseCandidateBuildEnvironment {
                    runner_os: "fixture-os".to_owned(),
                    runner_os_version: "fixture-version".to_owned(),
                    runner_arch: "fixture-arch".to_owned(),
                    git_version: "git fixture".to_owned(),
                    rustc_version: "rustc fixture".to_owned(),
                    cargo_version: "cargo fixture".to_owned(),
                },
                recorded_at: "2026-07-14T00:00:00Z".to_owned(),
            })
        }

        let context = release_validation_context()?;
        for forbidden_parent in [
            context.source_checkout().to_path_buf(),
            context.target_directory().to_path_buf(),
            context.source_checkout().join("docs"),
        ] {
            let result_path = forbidden_parent.join(format!(
                "volicord-forbidden-live-result-{}-{}.json",
                std::process::id(),
                epoch_duration()?.as_nanos()
            ));
            assert!(LiveResultRecorder::new("codex", Some(result_path.clone())).is_err());
            assert!(!result_path.exists());
        }
        assert!(validate_external_regular_input(
            &context,
            Path::new(volicord_bin()),
            MAX_RELEASE_CANDIDATE_BINARY_BYTES,
            "candidate_path",
        )
        .is_err());

        let runtime_home = TempRuntimeHome::new("live-path-observed-runtime")?;
        let release_root = runtime_home.product_repo_path("external-release");
        fs::create_dir_all(&release_root)?;
        let publication_root = release_root.join("publication");
        let (cell_directory, _evidence_directory, _auxiliary_directory) =
            create_live_result_root(&publication_root)?;

        let forbidden_result = cell_directory.join("forbidden-result.json");
        {
            let mut recorder = LiveResultRecorder::new("codex", Some(forbidden_result.clone()))?;
            assert!(recorder
                .bind_observed_runtime_home(&publication_root)
                .is_err());
            assert!(recorder.release_path_validation_failed);
        }
        assert!(!forbidden_result.exists());

        let pre_extension_runtime = TempRuntimeHome::new("live-path-pre-extension-failure")?;
        let registry_directory = pre_extension_runtime.path().join("registry.sqlite");
        fs::create_dir(&registry_directory)?;
        let pre_extension_result = cell_directory.join("pre-extension-result.json");
        {
            let mut recorder =
                LiveResultRecorder::new("codex", Some(pre_extension_result.clone()))?;
            assert!(recorder
                .bind_observed_runtime_home(pre_extension_runtime.path())
                .is_err());
            assert!(recorder.release_path_validation_failed);
            assert!(recorder.observed_runtime_home.is_none());
        }
        assert!(!pre_extension_result.exists());
        assert!(registry_directory.is_dir());

        let clean_runtime = TempRuntimeHome::new("live-path-read-only-bind")?;
        let clean_bind_result = cell_directory.join("clean-bind-result.json");
        {
            let mut recorder = LiveResultRecorder::new("codex", Some(clean_bind_result.clone()))?;
            recorder.bind_observed_runtime_home(clean_runtime.path())?;
            assert!(recorder.observed_runtime_home.is_some());
            assert!(!recorder.release_path_validation_failed);
            recorder.finalized = true;
        }
        for registry_name in [
            "registry.sqlite",
            "registry.sqlite-wal",
            "registry.sqlite-shm",
            "registry.sqlite-journal",
        ] {
            assert!(!clean_runtime.path().join(registry_name).exists());
        }
        assert!(!clean_bind_result.exists());

        let missing_runtime = release_root.join("missing-runtime-home");
        let missing_runtime_result = cell_directory.join("missing-runtime-result.json");
        {
            let mut recorder =
                LiveResultRecorder::new("codex", Some(missing_runtime_result.clone()))?;
            assert!(recorder
                .bind_observed_runtime_home(&missing_runtime)
                .is_err());
            assert!(recorder.release_path_validation_failed);
            assert!(recorder.observed_runtime_home.is_none());
        }
        assert!(!missing_runtime_result.exists());

        let malformed_runtime = TempRuntimeHome::new("live-path-malformed-registry")?;
        let malformed_registry = malformed_runtime.path().join("registry.sqlite");
        let malformed_bytes = b"not-a-sqlite-registry";
        fs::write(&malformed_registry, malformed_bytes)?;
        let malformed_result = cell_directory.join("malformed-runtime-result.json");
        {
            let mut recorder = LiveResultRecorder::new("codex", Some(malformed_result.clone()))?;
            assert!(recorder
                .bind_observed_runtime_home(malformed_runtime.path())
                .is_err());
            assert!(recorder.release_path_validation_failed);
            assert!(recorder.observed_runtime_home.is_none());
        }
        assert_eq!(fs::read(&malformed_registry)?, malformed_bytes);
        for sidecar in [
            "registry.sqlite-wal",
            "registry.sqlite-shm",
            "registry.sqlite-journal",
        ] {
            assert!(!malformed_runtime.path().join(sidecar).exists());
        }
        assert!(!malformed_result.exists());

        let rejected_descriptor_result = cell_directory.join("rejected-descriptor-result.json");
        {
            let mut recorder =
                LiveResultRecorder::new("codex", Some(rejected_descriptor_result.clone()))?;
            assert!(recorder
                .load_release_candidate(Path::new(volicord_bin()))
                .is_err());
            assert!(recorder.release_path_validation_failed);
        }
        assert!(!rejected_descriptor_result.exists());

        let external_candidate = release_root.join("candidate");
        fs::copy(volicord_bin(), &external_candidate)?;
        make_executable(&external_candidate)?;
        let external_descriptor = release_root.join("candidate.json");
        let external_candidate_record =
            candidate_fixture(&external_candidate, &external_descriptor)?;
        fs::write(
            &external_descriptor,
            serde_json::to_vec(&external_candidate_record)?,
        )?;
        let valid_descriptor_result = cell_directory.join("valid-descriptor-result.json");
        {
            let mut recorder =
                LiveResultRecorder::new("claude-code", Some(valid_descriptor_result.clone()))?;
            recorder.load_release_candidate(&external_descriptor)?;
            assert_eq!(
                recorder
                    .release_candidate
                    .as_ref()
                    .map(ReleaseCandidate::executable_path),
                Some(external_candidate.as_path())
            );
            recorder.finalized = true;
        }
        assert!(!valid_descriptor_result.exists());
        assert!(
            !publication_root
                .join(".volicord-live-publication.lock")
                .exists(),
            "pre-publication path and Runtime Home validation must not create lease state"
        );

        let stale_result_root = release_root.join("stale-publication");
        let (stale_cell_directory, _, _) = create_live_result_root(&stale_result_root)?;
        let stale_cell_path = stale_cell_directory.join("stale-cell.json");
        let stale_evidence_path = release_evidence_path(&stale_cell_path)?;
        let stale_cell_bytes = b"concurrent-cell-owner";
        {
            let mut recorder =
                LiveResultRecorder::new("claude-code", Some(stale_cell_path.clone()))?;
            recorder.load_release_candidate(&external_descriptor)?;
            recorder.bind_observed_runtime_home(runtime_home.path())?;
            fs::write(&stale_cell_path, stale_cell_bytes)?;
            let summary = recorder.failed_before_completion_summary();
            assert!(recorder.record_final(&summary).is_err());
            assert!(recorder.release_path_validation_failed);
        }
        assert_eq!(fs::read(&stale_cell_path)?, stale_cell_bytes);
        assert!(!stale_evidence_path.exists());

        let stale_evidence_cell = stale_cell_directory.join("stale-evidence-cell.json");
        let stale_evidence = release_evidence_path(&stale_evidence_cell)?;
        let stale_evidence_bytes = b"concurrent-evidence-owner";
        {
            let mut recorder =
                LiveResultRecorder::new("claude-code", Some(stale_evidence_cell.clone()))?;
            recorder.load_release_candidate(&external_descriptor)?;
            recorder.bind_observed_runtime_home(runtime_home.path())?;
            fs::write(&stale_evidence, stale_evidence_bytes)?;
            let summary = recorder.failed_before_completion_summary();
            assert!(recorder.record_final(&summary).is_err());
            assert!(recorder.release_path_validation_failed);
        }
        assert!(!stale_evidence_cell.exists());
        assert_eq!(fs::read(&stale_evidence)?, stale_evidence_bytes);

        let forbidden_descriptor = runtime_home.path().join("candidate.json");
        fs::write(&forbidden_descriptor, b"{}")?;
        let descriptor_result = cell_directory.join("descriptor-result.json");
        {
            let mut recorder =
                LiveResultRecorder::new("claude-code", Some(descriptor_result.clone()))?;
            recorder.release_candidate = Some(candidate_fixture(
                &external_candidate,
                &forbidden_descriptor,
            )?);
            assert!(recorder
                .bind_observed_runtime_home(runtime_home.path())
                .is_err());
        }
        assert!(!descriptor_result.exists());

        let forbidden_candidate = runtime_home.path().join("candidate");
        fs::copy(volicord_bin(), &forbidden_candidate)?;
        make_executable(&forbidden_candidate)?;
        let candidate_result = cell_directory.join("candidate-result.json");
        {
            let mut recorder =
                LiveResultRecorder::new("claude-code", Some(candidate_result.clone()))?;
            recorder.release_candidate = Some(candidate_fixture(
                &forbidden_candidate,
                &external_descriptor,
            )?);
            assert!(recorder
                .bind_observed_runtime_home(runtime_home.path())
                .is_err());
        }
        assert!(!candidate_result.exists());

        let sidecar_root = release_root.join("sidecar-policy");
        let (sidecar_cells, sidecar_evidence, _) = create_live_result_root(&sidecar_root)?;
        let mut evidence_context = release_validation_context()?;
        evidence_context.add_runtime_home(&sidecar_evidence)?;
        let sidecar_cell = sidecar_cells.join("cell.json");
        assert!(LivePublicationDomain::acquire_for_cell(&evidence_context, &sidecar_cell).is_err());
        assert!(sidecar_evidence.is_dir());

        let competing_root = release_root.join("competing-final");
        let (competing_cells, _, _) = create_live_result_root(&competing_root)?;
        let competing_path = competing_cells.join("cell.json");
        let competing_domain = LivePublicationDomain::acquire_for_cell(&context, &competing_path)?;
        let competing_stage = competing_domain.stage(
            &context,
            &competing_path,
            r#"{"kind":"staged"}"#,
            MAX_LIVE_HOST_RESULT_BYTES,
        )?;
        let competing_stage_path = competing_stage.stage_path();
        let competing_bytes = b"concurrent-final-owner";
        fs::write(&competing_path, competing_bytes)?;
        assert!(competing_stage
            .publish(&context, &competing_domain)
            .is_err());
        assert_eq!(fs::read(&competing_path)?, competing_bytes);
        assert_eq!(fs::read(&competing_stage_path)?, b"{\"kind\":\"staged\"}\n");

        let rejected_stage_root = release_root.join("evidence-stage-rejection");
        let (rejected_stage_cells, rejected_stage_evidence, _) =
            create_live_result_root(&rejected_stage_root)?;
        let rejected_stage_cell = rejected_stage_cells.join("cell.json");
        let rejected_stage_sidecar = release_evidence_path(&rejected_stage_cell)?;
        let rejected_stage_domain =
            LivePublicationDomain::acquire_for_cell(&context, &rejected_stage_cell)?;
        assert!(rejected_stage_domain
            .stage(&context, &rejected_stage_sidecar, &"x".repeat(65), 64,)
            .is_err());
        assert!(!rejected_stage_cell.exists());
        assert!(!rejected_stage_sidecar.exists());
        assert_eq!(fs::read_dir(&rejected_stage_cells)?.count(), 0);
        assert_eq!(fs::read_dir(&rejected_stage_evidence)?.count(), 0);
        drop(rejected_stage_domain);
        assert!(LivePublicationDomain::acquire_for_cell(
            &context,
            &rejected_stage_cells.join("retry.json")
        )
        .is_err());

        let orphan_root = release_root.join("evidence-first-failure");
        let (orphan_cells, _, _) = create_live_result_root(&orphan_root)?;
        let orphan_cell = orphan_cells.join("cell.json");
        let orphan_evidence = release_evidence_path(&orphan_cell)?;
        let orphan_domain = LivePublicationDomain::acquire_for_cell(&context, &orphan_cell)?;
        let evidence_stage = orphan_domain.stage(
            &context,
            &orphan_evidence,
            r#"{"kind":"evidence"}"#,
            MAX_LIVE_HOST_RESULT_BYTES,
        )?;
        let cell_stage = orphan_domain.stage(
            &context,
            &orphan_cell,
            r#"{"kind":"cell"}"#,
            MAX_LIVE_HOST_RESULT_BYTES,
        )?;
        let cell_stage_path = cell_stage.stage_path();
        evidence_stage.publish(&context, &orphan_domain)?;
        let competing_cell_bytes = b"concurrent-cell-commit-marker";
        fs::write(&orphan_cell, competing_cell_bytes)?;
        assert!(cell_stage.publish(&context, &orphan_domain).is_err());
        assert_eq!(fs::read(&orphan_evidence)?, b"{\"kind\":\"evidence\"}\n");
        assert_eq!(fs::read(&orphan_cell)?, competing_cell_bytes);
        assert_eq!(fs::read(&cell_stage_path)?, b"{\"kind\":\"cell\"}\n");
        drop(orphan_domain);
        assert!(LivePublicationDomain::acquire_for_cell(
            &context,
            &orphan_cells.join("retry.json")
        )
        .is_err());

        let replaced_root = release_root.join("replaced-directory");
        let (replaced_cells, _, _) = create_live_result_root(&replaced_root)?;
        let replaced_cell = replaced_cells.join("cell.json");
        let replaced_domain = LivePublicationDomain::acquire_for_cell(&context, &replaced_cell)?;
        let detached_cells = replaced_root.join("detached-cells");
        fs::rename(&replaced_cells, &detached_cells)?;
        fs::create_dir(&replaced_cells)?;
        let replacement_marker = b"concurrent-directory-owner";
        fs::write(replaced_cells.join("owner"), replacement_marker)?;
        assert!(replaced_domain.validate_attached(&context).is_err());
        assert_eq!(fs::read(replaced_cells.join("owner"))?, replacement_marker);

        Ok(())
    }

    #[test]
    fn release_cell_recorder_binds_exact_candidate_and_closed_assertions(
    ) -> Result<(), Box<dyn Error>> {
        let temp = TempRuntimeHome::new("release-cell-recorder")?;
        let release_root = temp.product_repo_path("release-artifacts");
        fs::create_dir_all(&release_root)?;
        let candidate_path = release_root.join("candidate-volicord");
        fs::copy(volicord_bin(), &candidate_path)?;
        let mut permissions = fs::metadata(&candidate_path)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&candidate_path, permissions)?;
        let binary_sha256 = sha256_file(&candidate_path, MAX_RELEASE_CANDIDATE_BINARY_BYTES)?;
        let candidate = ReleaseCandidate {
            descriptor_path: None,
            schema: RELEASE_CANDIDATE_SCHEMA.to_owned(),
            candidate_id: "candidate_release_cell_fixture".to_owned(),
            candidate_path: path_text(&candidate_path),
            source_revision: "a".repeat(40),
            source_clean: true,
            source_archive_algorithm: RELEASE_SOURCE_ARCHIVE_ALGORITHM.to_owned(),
            source_archive_sha256: "b".repeat(64),
            target_triple: "fixture-target".to_owned(),
            release_profile: "release".to_owned(),
            binary_sha256: binary_sha256.clone(),
            build_environment: ReleaseCandidateBuildEnvironment {
                runner_os: "fixture-os".to_owned(),
                runner_os_version: "fixture-version".to_owned(),
                runner_arch: "fixture-arch".to_owned(),
                git_version: "git fixture".to_owned(),
                rustc_version: "rustc fixture".to_owned(),
                cargo_version: "cargo fixture".to_owned(),
            },
            recorded_at: "2026-07-14T00:00:00Z".to_owned(),
        };
        candidate.validate()?;

        let result_root = release_root.join("release-results");
        let (result_dir, _, _) = create_live_result_root(&result_root)?;
        let cell_path = result_dir.join("claude-native-user-action.json");
        let mut recorder = LiveResultRecorder::new("claude-code", Some(cell_path.clone()))?;
        recorder.release_candidate = Some(candidate.clone());
        recorder.release_feature = Some(HostFeature::NativeUserAction);
        recorder.bind_observed_initialized_client_info(ObservedInitializedClientInfo::new(
            "claude-code".to_owned(),
            "fixture-host 1.0".to_owned(),
        )?)?;
        recorder.record_final(&native_user_action_result_shape_fixture(
            "claude-code",
            &binary_sha256,
        ))?;

        let cell: Value = serde_json::from_slice(&fs::read(&cell_path)?)?;
        assert_eq!(cell["schema"], RELEASE_CELL_SCHEMA);
        assert_eq!(cell["candidate_id"], "candidate_release_cell_fixture");
        assert_eq!(cell["binary_sha256"], binary_sha256);
        assert_eq!(cell["host_kind"], "claude_code");
        assert_eq!(cell["client_name"], "claude-code");
        assert_eq!(cell["client_version"], "fixture-host 1.0");
        assert_eq!(cell["environment"]["client_name"], cell["client_name"]);
        assert_eq!(
            cell["environment"]["client_version"],
            cell["client_version"]
        );
        assert_eq!(cell["feature"], "native_user_action");
        assert_eq!(cell["requested_verified"], true);
        assert_eq!(cell["claimed_status"], "verified");
        assert_eq!(cell["run_state"], "completed");
        let measured_runner = LiveRunnerEnvironment::measure()?;
        assert_eq!(cell["environment"]["runner_os"], measured_runner.runner_os);
        assert_eq!(
            cell["environment"]["runner_os_version"],
            measured_runner.runner_os_version
        );
        assert_eq!(
            cell["environment"]["runner_arch"],
            measured_runner.runner_arch
        );
        assert_ne!(cell["environment"]["runner_os"], "fixture-os");
        assert_ne!(cell["environment"]["runner_os_version"], "fixture-version");
        assert_ne!(cell["environment"]["runner_arch"], "fixture-arch");
        let assertions = cell["assertions"]
            .as_array()
            .ok_or_else(|| io::Error::other("release cell assertions must be an array"))?;
        assert_eq!(assertions.len(), 5);
        assert!(assertions
            .iter()
            .all(|assertion| assertion["passed"] == true));
        let evidence_path = PathBuf::from(
            cell["evidence_artifact_path"]
                .as_str()
                .ok_or_else(|| io::Error::other("release cell has no evidence path"))?,
        );
        assert!(evidence_path.exists());
        assert_ne!(evidence_path.parent(), cell_path.parent());
        assert_eq!(
            cell["evidence_artifact_sha256"],
            sha256_file(&evidence_path, MAX_LIVE_HOST_RESULT_BYTES as u64 + 1)?
        );
        let evidence: Value = serde_json::from_slice(&fs::read(&evidence_path)?)?;
        assert_eq!(evidence["validation_run"]["client_name"], "claude-code");
        assert_eq!(
            evidence["validation_run"]["client_version"],
            "fixture-host 1.0"
        );
        assert_eq!(
            fs::read_dir(&result_dir)?
                .filter_map(Result::ok)
                .filter(|entry| entry
                    .path()
                    .extension()
                    .is_some_and(|value| value == "json"))
                .count(),
            1,
            "release-cell directory must contain only the release cell, not its evidence sidecar"
        );

        let forced_false_path = result_dir.join("claude-native-user-action-unrequested.json");
        let mut forced_false =
            LiveResultRecorder::new("claude-code", Some(forced_false_path.clone()))?;
        forced_false.release_candidate = Some(candidate.clone());
        forced_false.release_feature = Some(HostFeature::NativeUserAction);
        forced_false.release_requested_verified = Some(false);
        forced_false.bind_observed_initialized_client_info(ObservedInitializedClientInfo::new(
            "claude-code".to_owned(),
            "fixture-host 1.0".to_owned(),
        )?)?;
        forced_false.record_final(&native_user_action_result_shape_fixture(
            "claude-code",
            &binary_sha256,
        ))?;
        let forced_false_cell: Value = serde_json::from_slice(&fs::read(&forced_false_path)?)?;
        assert_eq!(forced_false_cell["requested_verified"], false);
        assert_eq!(forced_false_cell["claimed_status"], "verified");

        let missing_client_path = result_dir.join("claude-native-user-action-no-client.json");
        let mut missing_client =
            LiveResultRecorder::new("claude-code", Some(missing_client_path.clone()))?;
        missing_client.release_candidate = Some(candidate.clone());
        missing_client.release_feature = Some(HostFeature::NativeUserAction);
        missing_client.release_requested_verified = Some(false);
        missing_client.record_final(&native_user_action_result_shape_fixture(
            "claude-code",
            &binary_sha256,
        ))?;
        let missing_client_cell: Value = serde_json::from_slice(&fs::read(&missing_client_path)?)?;
        assert_eq!(missing_client_cell["client_name"], Value::Null);
        assert_eq!(missing_client_cell["client_version"], Value::Null);
        assert_eq!(
            missing_client_cell["environment"]["client_name"],
            Value::Null
        );
        assert_eq!(
            missing_client_cell["environment"]["client_version"],
            Value::Null
        );
        assert_eq!(
            missing_client_cell["claimed_status"],
            "implemented_unverified"
        );
        assert!(missing_client_cell["assertions"]
            .as_array()
            .is_some_and(|assertions| assertions
                .iter()
                .all(|assertion| assertion["passed"] == true)));

        let mismatched_client_path =
            result_dir.join("claude-native-user-action-client-mismatch.json");
        let mut mismatched_client =
            LiveResultRecorder::new("claude-code", Some(mismatched_client_path.clone()))?;
        mismatched_client.release_candidate = Some(candidate.clone());
        mismatched_client.release_feature = Some(HostFeature::NativeUserAction);
        mismatched_client.release_requested_verified = Some(false);
        mismatched_client.bind_observed_initialized_client_info(
            ObservedInitializedClientInfo::new(
                "claude-code".to_owned(),
                "fixture-host 2.0".to_owned(),
            )?,
        )?;
        mismatched_client.record_final(&native_user_action_result_shape_fixture(
            "claude-code",
            &binary_sha256,
        ))?;
        let mismatched_client_cell: Value =
            serde_json::from_slice(&fs::read(&mismatched_client_path)?)?;
        assert_eq!(
            mismatched_client_cell["claimed_status"],
            "implemented_unverified"
        );

        let unavailable_path = result_dir.join("claude-null-host-default-requested.json");
        let mut unavailable =
            LiveResultRecorder::new("claude-code", Some(unavailable_path.clone()))?;
        unavailable.release_candidate = Some(candidate.clone());
        unavailable.release_feature = Some(HostFeature::NativeUserAction);
        unavailable.record_final(&live_user_action_unavailable_summary(
            "claude-code",
            None,
            "host_executable",
            "fixture host unavailable",
        ))?;
        let unavailable_cell: Value = serde_json::from_slice(&fs::read(&unavailable_path)?)?;
        assert_eq!(unavailable_cell["host_version"], Value::Null);
        assert_eq!(
            unavailable_cell["environment"]["host_executable_sha256"],
            Value::Null
        );
        assert_eq!(unavailable_cell["requested_verified"], true);
        assert_eq!(unavailable_cell["run_state"], "ignored");
        assert_eq!(unavailable_cell["claimed_status"], "implemented_unverified");
        assert!(unavailable_cell["adapter_version"].is_string());
        assert!(unavailable_cell["evidence_artifact_path"].is_string());

        let observed_failure_path = result_dir.join("claude-observed-host-failure.json");
        let mut observed_failure =
            LiveResultRecorder::new("claude-code", Some(observed_failure_path.clone()))?;
        observed_failure.release_candidate = Some(candidate.clone());
        observed_failure.release_feature = Some(HostFeature::NativeUserAction);
        observed_failure.bind_observed_host_identity(ObservedReleaseHostIdentity::new(
            "fixture-observed-host 2.0".to_owned(),
            "f".repeat(64),
            "fixture-observed-build".to_owned(),
        )?)?;
        observed_failure.record_final(&live_user_action_unavailable_summary(
            "claude-code",
            Some("fixture-observed-host 2.0"),
            "connection_observation",
            "fixture post-preflight failure",
        ))?;
        let observed_failure_cell: Value =
            serde_json::from_slice(&fs::read(&observed_failure_path)?)?;
        assert_eq!(
            observed_failure_cell["host_version"],
            "fixture-observed-host 2.0"
        );
        assert_eq!(
            observed_failure_cell["environment"]["host_executable_sha256"],
            "f".repeat(64)
        );
        assert_eq!(
            observed_failure_cell["adapter_version"],
            "fixture-observed-build"
        );
        assert_eq!(observed_failure_cell["run_state"], "completed");

        let excluded_null_path = result_dir.join("claude-null-host-unrequested.json");
        let mut excluded_null =
            LiveResultRecorder::new("claude-code", Some(excluded_null_path.clone()))?;
        excluded_null.release_candidate = Some(candidate.clone());
        excluded_null.release_feature = Some(HostFeature::NativeUserAction);
        excluded_null.release_requested_verified = Some(false);
        excluded_null.record_final(&live_user_action_unavailable_summary(
            "claude-code",
            None,
            "host_executable",
            "fixture host unavailable",
        ))?;
        let excluded_null_cell: Value = serde_json::from_slice(&fs::read(&excluded_null_path)?)?;
        assert_eq!(excluded_null_cell["host_version"], Value::Null);
        assert_eq!(excluded_null_cell["requested_verified"], false);
        assert_eq!(excluded_null_cell["run_state"], "ignored");

        let static_final_path = result_dir.join("codex-record-final-output.json");
        let mut static_final = LiveResultRecorder::new_for_kind_and_profile(
            "codex-record-final-output",
            "codex",
            LIVE_FINAL_OUTPUT_RESULT_KIND,
            Some(IntegrationProfile::Record),
            Some(static_final_path.clone()),
        )?;
        static_final.release_candidate = Some(candidate.clone());
        static_final.release_feature = Some(HostFeature::RecordFinalOutput);
        let static_summary = final_output_unavailable_summary_with_host_identity(
            "codex",
            IntegrationProfile::Record,
            "fixture static unsupported feature",
            "0.144.4",
            &binary_sha256,
            "fixture-build-id",
        );
        validate_final_output_result_shape(&static_summary, IntegrationProfile::Record)?;
        static_final.record_final(&static_summary)?;
        let static_final_cell: Value = serde_json::from_slice(&fs::read(&static_final_path)?)?;
        assert_eq!(static_final_cell["host_version"], "0.144.4");
        assert_eq!(static_final_cell["client_name"], Value::Null);
        assert_eq!(static_final_cell["client_version"], Value::Null);
        assert_eq!(static_final_cell["requested_verified"], false);
        assert_eq!(
            static_final_cell["implementation_disposition"],
            "unsupported_by_host"
        );
        assert_eq!(static_final_cell["claimed_status"], "unsupported_by_host");
        assert_eq!(static_final_cell["run_state"], "not_applicable");
        assert_eq!(static_final_cell["evidence_artifact_path"], Value::Null);
        assert!(!release_evidence_path(&static_final_path)?.exists());

        let reviewed_local_web_path = result_dir.join("codex-reviewed-local-web.json");
        let mut reviewed_local_web =
            LiveResultRecorder::new("codex", Some(reviewed_local_web_path.clone()))?;
        reviewed_local_web.release_candidate = Some(candidate.clone());
        reviewed_local_web.release_feature = Some(HostFeature::LocalWebUserChannel);
        reviewed_local_web.bind_observed_host_identity(ObservedReleaseHostIdentity::new(
            "0.144.4".to_owned(),
            "e".repeat(64),
            "fixture-reviewed-build".to_owned(),
        )?)?;
        reviewed_local_web.record_final(&live_user_action_unavailable_summary(
            "codex",
            Some("0.144.4"),
            "static_unsupported_by_host",
            "reviewed Codex version does not expose the required local-web surface",
        ))?;
        let reviewed_local_web_cell: Value =
            serde_json::from_slice(&fs::read(&reviewed_local_web_path)?)?;
        assert_eq!(reviewed_local_web_cell["schema"], RELEASE_CELL_SCHEMA);
        assert_eq!(reviewed_local_web_cell["host_version"], "0.144.4");
        assert_eq!(reviewed_local_web_cell["client_name"], Value::Null);
        assert_eq!(reviewed_local_web_cell["client_version"], Value::Null);
        assert_eq!(
            reviewed_local_web_cell["implementation_disposition"],
            "unsupported_by_host"
        );
        assert_eq!(reviewed_local_web_cell["requested_verified"], false);
        assert_eq!(reviewed_local_web_cell["run_state"], "not_applicable");
        assert_eq!(
            reviewed_local_web_cell["evidence_artifact_path"],
            Value::Null
        );

        let forbidden_static_root = release_root.join("forbidden-static-results");
        let (forbidden_static_dir, _, _) = create_live_result_root(&forbidden_static_root)?;
        let forbidden_static_path =
            forbidden_static_dir.join("codex-record-final-output-requested.json");
        let mut forbidden_static = LiveResultRecorder::new_for_kind_and_profile(
            "codex-record-final-output-requested",
            "codex",
            LIVE_FINAL_OUTPUT_RESULT_KIND,
            Some(IntegrationProfile::Record),
            Some(forbidden_static_path.clone()),
        )?;
        forbidden_static.release_candidate = Some(candidate.clone());
        forbidden_static.release_feature = Some(HostFeature::RecordFinalOutput);
        forbidden_static.release_requested_verified = Some(true);
        assert!(forbidden_static.record_final(&static_summary).is_err());
        assert!(!forbidden_static_path.exists());
        drop(forbidden_static);

        let rejected_result_root = release_root.join("rejected-cell-results");
        let (rejected_result_dir, _, _) = create_live_result_root(&rejected_result_root)?;
        let rejected_cell_path = rejected_result_dir.join("claude-native-user-action-mutated.json");
        {
            let mut rejected =
                LiveResultRecorder::new("claude-code", Some(rejected_cell_path.clone()))?;
            rejected.release_candidate = Some(candidate);
            rejected.release_feature = Some(HostFeature::NativeUserAction);
            let mut semantically_false =
                native_user_action_result_shape_fixture("claude-code", &binary_sha256);
            semantically_false["native_ui"]["operator_choice_confirmed"] = Value::Bool(false);
            assert!(rejected.record_final(&semantically_false).is_err());
            assert!(!rejected_cell_path.exists());
        }
        let rejected_cell: Value = serde_json::from_slice(&fs::read(&rejected_cell_path)?)?;
        assert_ne!(rejected_cell["claimed_status"], "verified");
        assert!(rejected_cell["assertions"]
            .as_array()
            .is_some_and(|assertions| assertions
                .iter()
                .all(|assertion| assertion["passed"] == false)));
        assert!(LiveResultRecorder::new("claude-code", Some(cell_path)).is_err());
        Ok(())
    }

    #[test]
    fn release_cell_recorder_uses_only_initialized_identity_from_the_captured_host_turn(
    ) -> Result<(), Box<dyn Error>> {
        fn run_managed_claude_initialize_turn(
            fixture: &LiveSmokeFixture,
            connection_id: &str,
            project_id: &str,
            native_session_id: &str,
            client_name: &str,
            client_version: &str,
        ) -> Result<Output, Box<dyn Error>> {
            let input = [
                serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "initialize",
                    "params": {
                        "protocolVersion": "2025-11-25",
                        "capabilities": {},
                        "clientInfo": {
                            "name": client_name,
                            "version": client_version
                        }
                    }
                }),
                serde_json::json!({
                    "jsonrpc": "2.0",
                    "method": "notifications/initialized"
                }),
            ]
            .into_iter()
            .map(|message| serde_json::to_string(&message))
            .collect::<Result<Vec<_>, _>>()?
            .join("\n")
                + "\n";
            let mut command = Command::new(&fixture.volicord_path);
            command
                .args([
                    "mcp",
                    "--stdio",
                    "--connection",
                    connection_id,
                    "--project",
                    project_id,
                ])
                .current_dir(&fixture.repo_root)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            fixture.apply_isolated_env(&mut command);
            command
                .env("VOLICORD_MCP_LAUNCH", "managed_host")
                .env("VOLICORD_MCP_HOST", "claude_code")
                .env("VOLICORD_MCP_CONNECTION_ID", connection_id)
                .env("VOLICORD_MCP_PROJECT_ID", project_id)
                .env("CLAUDECODE", "1")
                .env("CLAUDE_CODE_SESSION_ID", native_session_id);
            fixture.with_private_candidate_digest_guard(|| {
                let mut child = command.spawn()?;
                child
                    .stdin
                    .take()
                    .ok_or_else(|| io::Error::other("managed MCP child has no stdin"))?
                    .write_all(input.as_bytes())?;
                Ok(child.wait_with_output()?)
            })
        }

        let fixture = LiveSmokeFixture::new("release-client-turn-refresh")?;
        let fake_claude = write_fake_claude_code(fixture.live_bin())?;
        let result_root = fixture.release_artifact_root.join("release-results");
        let (cell_directory, _, _) = create_live_result_root(&result_root)?;
        let cell_path = cell_directory.join("claude-native-user-action.json");
        let stale_only_result_root = fixture
            .release_artifact_root
            .join("release-results-stale-only");
        let (stale_only_cell_directory, _, _) = create_live_result_root(&stale_only_result_root)?;
        let stale_only_cell_path =
            stale_only_cell_directory.join("claude-native-user-action-stale-only.json");
        let tampered_result_root = fixture
            .release_artifact_root
            .join("release-results-tampered");
        let (tampered_cell_directory, _, _) = create_live_result_root(&tampered_result_root)?;
        let tampered_cell_path =
            tampered_cell_directory.join("claude-native-user-action-tampered.json");
        let tampered_evidence_path = release_evidence_path(&tampered_cell_path)?;

        let mut recorder = LiveResultRecorder::new("claude-code", Some(cell_path.clone()))?;
        recorder.bind_observed_runtime_home(&fixture.runtime_home_path)?;
        let mut stale_only_recorder =
            LiveResultRecorder::new("claude-code", Some(stale_only_cell_path.clone()))?;
        stale_only_recorder.bind_observed_runtime_home(&fixture.runtime_home_path)?;
        let mut tampered_recorder =
            LiveResultRecorder::new("claude-code", Some(tampered_cell_path.clone()))?;
        tampered_recorder.bind_observed_runtime_home(&fixture.runtime_home_path)?;
        let binary_sha256 =
            sha256_file(&fixture.volicord_path, MAX_RELEASE_CANDIDATE_BINARY_BYTES)?;
        let candidate = ReleaseCandidate {
            descriptor_path: None,
            schema: RELEASE_CANDIDATE_SCHEMA.to_owned(),
            candidate_id: "candidate_host_turn_refresh".to_owned(),
            candidate_path: path_text(&fixture.volicord_path),
            source_revision: "a".repeat(40),
            source_clean: true,
            source_archive_algorithm: RELEASE_SOURCE_ARCHIVE_ALGORITHM.to_owned(),
            source_archive_sha256: "b".repeat(64),
            target_triple: "fixture-target".to_owned(),
            release_profile: "release".to_owned(),
            binary_sha256,
            build_environment: ReleaseCandidateBuildEnvironment {
                runner_os: "fixture-os".to_owned(),
                runner_os_version: "fixture-version".to_owned(),
                runner_arch: "fixture-arch".to_owned(),
                git_version: "git fixture".to_owned(),
                rustc_version: "rustc fixture".to_owned(),
                cargo_version: "cargo fixture".to_owned(),
            },
            recorded_at: "2026-01-01T00:00:00Z".to_owned(),
        };
        candidate.validate()?;
        recorder.release_candidate = Some(candidate.clone());
        recorder.release_feature = Some(HostFeature::NativeUserAction);
        recorder.release_requested_verified = Some(true);
        stale_only_recorder.release_candidate = Some(candidate.clone());
        stale_only_recorder.release_feature = Some(HostFeature::NativeUserAction);
        stale_only_recorder.release_requested_verified = Some(true);
        tampered_recorder.release_candidate = Some(candidate);
        tampered_recorder.release_feature = Some(HostFeature::NativeUserAction);
        tampered_recorder.release_requested_verified = Some(true);

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
        assert_success("volicord init for release client-turn refresh", &init);
        let init_json = json_stdout(&init)?;
        let connection_id = bounded_identity(
            "fixture Agent Connection id",
            init_json["connection"]["connection_id"]
                .as_str()
                .ok_or_else(|| io::Error::other("fixture init returned no connection id"))?,
            MAX_CONNECTION_ID_BYTES,
        )?;
        let project_id = live_fixture_project_id(&fixture)?;

        let historical_name = "historical-client";
        let historical_version = "historical-host 0.9";
        let historical_session = "claude.session.historical";
        let historical_output = run_managed_claude_initialize_turn(
            &fixture,
            &connection_id,
            &project_id,
            historical_session,
            historical_name,
            historical_version,
        )?;
        assert!(
            historical_output.status.success(),
            "historical managed MCP turn failed: {}",
            String::from_utf8_lossy(&historical_output.stderr)
        );
        let historical_session_id =
            managed_host_session_id("claude_code", &connection_id, historical_session)?;
        let historical_baseline = latest_watch_baseline_for_session(
            &fixture.runtime_home_path,
            &project_id,
            &historical_session_id,
        )?
        .ok_or_else(|| io::Error::other("historical managed baseline was not created"))?;
        let historical_metadata_before = historical_baseline.metadata_json.clone();
        let historical_metadata: Value = serde_json::from_str(&historical_metadata_before)?;
        assert_eq!(historical_metadata["client_name"], historical_name);
        assert_eq!(historical_metadata["client_version"], historical_version);

        let before = fixture.managed_baseline_observations()?;
        let historical_key = ObservedHostTurnBaseline {
            project_id: project_id.clone(),
            watch_baseline_id: historical_baseline.watch_baseline_id.clone(),
        };
        assert!(before.contains_key(&historical_key));

        let current_name = "claude-code";
        let current_version = "fixture-host 1.0";
        let host_executable_sha256 = sha256_file(&fake_claude, MAX_RELEASE_CANDIDATE_BINARY_BYTES)?;
        let volicord_build_id = fixture.release_build_id()?;

        stale_only_recorder.bind_observed_host_turn_baselines(&before, &before)?;
        assert!(stale_only_recorder.observed_host_turn_baselines.is_empty());
        stale_only_recorder.bind_observed_host_identity(ObservedReleaseHostIdentity::new(
            current_version.to_owned(),
            host_executable_sha256.clone(),
            volicord_build_id.clone(),
        )?)?;
        let mut stale_only_summary =
            native_user_action_result_shape_fixture("claude-code", &host_executable_sha256);
        stale_only_summary["host"]["version"] = Value::String(current_version.to_owned());
        stale_only_summary["volicord"]["build_id"] = Value::String(volicord_build_id.clone());
        stale_only_summary["connection"]["connection_id"] = Value::String(connection_id.clone());
        stale_only_summary["stop_hook"]["connection_id"] = Value::String(connection_id.clone());
        stale_only_recorder.record_final(&stale_only_summary)?;

        let stale_only_cell: Value = serde_json::from_slice(&fs::read(&stale_only_cell_path)?)?;
        assert_eq!(stale_only_cell["client_name"], Value::Null);
        assert_eq!(stale_only_cell["client_version"], Value::Null);
        assert_eq!(stale_only_cell["environment"]["client_name"], Value::Null);
        assert_eq!(
            stale_only_cell["environment"]["client_version"],
            Value::Null
        );
        assert_eq!(stale_only_cell["claimed_status"], "implemented_unverified");

        let current_session = "claude.session.current";
        let current_output = run_managed_claude_initialize_turn(
            &fixture,
            &connection_id,
            &project_id,
            current_session,
            current_name,
            current_version,
        )?;
        assert!(
            current_output.status.success(),
            "current managed MCP turn failed: {}",
            String::from_utf8_lossy(&current_output.stderr)
        );
        let historical_unrelated_observed_at = recorded_at_now()?;
        let mut historical_metadata_after_unrelated = historical_metadata.clone();
        historical_metadata_after_unrelated["latest_lifecycle_event"] =
            Value::String("managed_host_tools_list".to_owned());
        historical_metadata_after_unrelated["latest_lifecycle_observed_at"] =
            Value::String(historical_unrelated_observed_at.clone());
        historical_metadata_after_unrelated["lifecycle_events"]
            .as_array_mut()
            .ok_or_else(|| io::Error::other("historical lifecycle events are not an array"))?
            .push(serde_json::json!({
                "connection_id": connection_id,
                "project_id": project_id,
                "host_kind": "claude_code",
                "launch_origin": "managed_host",
                "lifecycle_event": "managed_host_tools_list",
                "timestamp": historical_unrelated_observed_at,
                "storage_capability": "read_write",
                "effective_tool_mode": "workflow"
            }));
        let historical_metadata_after_unrelated_text =
            serde_json::to_string(&historical_metadata_after_unrelated)?;
        update_watch_status(
            &fixture.runtime_home_path,
            &project_id,
            &historical_baseline.watch_baseline_id,
            WatchStatusUpdate {
                status: SessionWatchStatus::Active,
                updated_at: recorded_at_now()?,
                metadata_json: historical_metadata_after_unrelated_text.clone(),
            },
        )?;
        let after = fixture.managed_baseline_observations()?;
        let historical_before_observation = before
            .get(&historical_key)
            .ok_or_else(|| io::Error::other("historical baseline was absent before the turn"))?;
        let historical_after_observation = after
            .get(&historical_key)
            .ok_or_else(|| io::Error::other("historical baseline disappeared after the turn"))?;
        assert_ne!(
            historical_before_observation.metadata_fingerprint,
            historical_after_observation.metadata_fingerprint
        );
        assert_eq!(
            historical_before_observation.initialize_event_fingerprints,
            historical_after_observation.initialize_event_fingerprints
        );
        recorder.bind_observed_host_turn_baselines(&before, &after)?;
        tampered_recorder.bind_observed_host_turn_baselines(&before, &after)?;
        recorder.bind_observed_host_turn_baselines(&after, &after)?;
        tampered_recorder.bind_observed_host_turn_baselines(&after, &after)?;

        let current_session_id =
            managed_host_session_id("claude_code", &connection_id, current_session)?;
        let current_baseline = latest_watch_baseline_for_session(
            &fixture.runtime_home_path,
            &project_id,
            &current_session_id,
        )?
        .ok_or_else(|| io::Error::other("current managed baseline was not created"))?;
        assert!(is_exact_managed_host_turn_baseline(
            &current_baseline,
            &project_id,
            &connection_id,
        ));
        let mut forged_generic_baseline = current_baseline.clone();
        forged_generic_baseline.session_id = "session_generic".to_owned();
        forged_generic_baseline.watch_baseline_id =
            format!("watch_base_managed_{}", forged_generic_baseline.session_id);
        assert!(!is_exact_managed_host_turn_baseline(
            &forged_generic_baseline,
            &project_id,
            &connection_id,
        ));
        let mut forged_baseline_id = current_baseline.clone();
        forged_baseline_id.watch_baseline_id = "watch_base_generic".to_owned();
        assert!(!is_exact_managed_host_turn_baseline(
            &forged_baseline_id,
            &project_id,
            &connection_id,
        ));
        let mut wrong_coordinates = current_baseline.clone();
        wrong_coordinates.project_id = "project_other".to_owned();
        assert!(!is_exact_managed_host_turn_baseline(
            &wrong_coordinates,
            &project_id,
            &connection_id,
        ));
        wrong_coordinates.project_id = project_id.clone();
        wrong_coordinates.connection_internal_id = "connection_other".to_owned();
        assert!(!is_exact_managed_host_turn_baseline(
            &wrong_coordinates,
            &project_id,
            &connection_id,
        ));
        let current_key = ObservedHostTurnBaseline {
            project_id: project_id.clone(),
            watch_baseline_id: current_baseline.watch_baseline_id.clone(),
        };
        let current_fingerprint = after
            .get(&current_key)
            .ok_or_else(|| io::Error::other("current managed baseline has no fingerprint"))?
            .metadata_fingerprint
            .clone();
        let expected_current_baseline =
            BTreeMap::from([(current_key.clone(), current_fingerprint)]);
        assert_eq!(
            recorder.observed_host_turn_baselines,
            expected_current_baseline
        );
        assert_eq!(
            tampered_recorder.observed_host_turn_baselines,
            recorder.observed_host_turn_baselines
        );
        let current_metadata: Value = serde_json::from_str(&current_baseline.metadata_json)?;
        assert_eq!(current_metadata["client_name"], current_name);
        assert_eq!(current_metadata["client_version"], current_version);

        let repeated_before = fixture.managed_baseline_observations()?;
        assert_eq!(repeated_before, after);
        let mut repeated_metadata = current_metadata.clone();
        repeated_metadata["captured_repeated_turn"] = Value::Bool(true);
        let repeated_baseline = update_watch_status(
            &fixture.runtime_home_path,
            &project_id,
            &current_baseline.watch_baseline_id,
            WatchStatusUpdate {
                status: SessionWatchStatus::Active,
                updated_at: recorded_at_now()?,
                metadata_json: serde_json::to_string(&repeated_metadata)?,
            },
        )?;
        let repeated_after = fixture.managed_baseline_observations()?;
        assert_ne!(
            repeated_before.get(&current_key),
            repeated_after.get(&current_key)
        );
        recorder.bind_observed_host_turn_baselines(&repeated_before, &repeated_after)?;
        tampered_recorder.bind_observed_host_turn_baselines(&repeated_before, &repeated_after)?;
        let repeated_fingerprint = repeated_after
            .get(&current_key)
            .ok_or_else(|| io::Error::other("repeated managed baseline has no fingerprint"))?
            .metadata_fingerprint
            .clone();
        assert_eq!(
            recorder.observed_host_turn_baselines,
            BTreeMap::from([(current_key.clone(), repeated_fingerprint)])
        );

        recorder.bind_observed_host_identity(ObservedReleaseHostIdentity::new(
            current_version.to_owned(),
            host_executable_sha256.clone(),
            volicord_build_id.clone(),
        )?)?;
        tampered_recorder.bind_observed_host_identity(ObservedReleaseHostIdentity::new(
            current_version.to_owned(),
            host_executable_sha256.clone(),
            volicord_build_id.clone(),
        )?)?;
        let mut summary =
            native_user_action_result_shape_fixture("claude-code", &host_executable_sha256);
        summary["host"]["version"] = Value::String(current_version.to_owned());
        summary["volicord"]["build_id"] = Value::String(volicord_build_id);
        summary["connection"]["connection_id"] = Value::String(connection_id.clone());
        summary["stop_hook"]["connection_id"] = Value::String(connection_id);
        recorder.record_final(&summary)?;

        let cell_text = fs::read_to_string(&cell_path)?;
        let cell: Value = serde_json::from_str(&cell_text)?;
        assert_eq!(cell["client_name"], current_name);
        assert_eq!(cell["client_version"], current_version);
        assert_eq!(cell["environment"]["client_name"], current_name);
        assert_eq!(cell["environment"]["client_version"], current_version);
        assert_eq!(cell["claimed_status"], "verified");
        assert!(!cell_text.contains(historical_name));
        assert!(!cell_text.contains(historical_version));

        let evidence_path = PathBuf::from(
            cell["evidence_artifact_path"]
                .as_str()
                .ok_or_else(|| io::Error::other("verified cell has no evidence path"))?,
        );
        let evidence_text = fs::read_to_string(evidence_path)?;
        let evidence: Value = serde_json::from_str(&evidence_text)?;
        assert_eq!(evidence["validation_run"]["client_name"], current_name);
        assert_eq!(
            evidence["validation_run"]["client_version"],
            current_version
        );
        assert!(!evidence_text.contains(historical_name));
        assert!(!evidence_text.contains(historical_version));

        let historical_after = latest_watch_baseline_for_session(
            &fixture.runtime_home_path,
            &project_id,
            &historical_session_id,
        )?
        .ok_or_else(|| io::Error::other("historical managed baseline disappeared"))?;
        assert_eq!(
            historical_after.metadata_json,
            historical_metadata_after_unrelated_text
        );

        let mut tampered_metadata: Value = serde_json::from_str(&repeated_baseline.metadata_json)?;
        tampered_metadata["post_capture_mutation"] = Value::Bool(true);
        update_watch_status(
            &fixture.runtime_home_path,
            &project_id,
            &current_baseline.watch_baseline_id,
            WatchStatusUpdate {
                status: SessionWatchStatus::Active,
                updated_at: recorded_at_now()?,
                metadata_json: serde_json::to_string(&tampered_metadata)?,
            },
        )?;
        assert!(tampered_recorder.record_final(&summary).is_err());
        drop(tampered_recorder);
        assert!(!tampered_cell_path.exists());
        assert!(!tampered_evidence_path.exists());
        Ok(())
    }

    #[test]
    fn release_cell_recorder_advances_baseline_fingerprint_only_from_exact_repeated_turn_before(
    ) -> Result<(), Box<dyn Error>> {
        fn observation(
            metadata_fingerprint: &str,
            initialize_event_fingerprints: &[&str],
        ) -> ManagedBaselineObservation {
            ManagedBaselineObservation {
                metadata_fingerprint: metadata_fingerprint.to_owned(),
                initialize_event_fingerprints: initialize_event_fingerprints
                    .iter()
                    .map(|fingerprint| (*fingerprint).to_owned())
                    .collect(),
            }
        }

        let runtime_home = TempRuntimeHome::new("release-baseline-repeated-turn")?;
        let mut recorder = LiveResultRecorder::new("claude-code", None)?;
        recorder.bind_observed_runtime_home(runtime_home.path())?;
        let baseline = ObservedHostTurnBaseline {
            project_id: "project_fixture".to_owned(),
            watch_baseline_id: "watch_base_fixture".to_owned(),
        };
        let empty = ManagedBaselineObservations::new();
        let first = BTreeMap::from([(
            baseline.clone(),
            observation(&"a".repeat(64), &["initialize-a"]),
        )]);
        let first_fingerprints = BTreeMap::from([(baseline.clone(), "a".repeat(64))]);
        recorder.bind_observed_host_turn_baselines(&empty, &first)?;
        assert_eq!(recorder.observed_host_turn_baselines, first_fingerprints);

        recorder.bind_observed_host_turn_baselines(&first, &first)?;
        assert_eq!(recorder.observed_host_turn_baselines, first_fingerprints);

        let second = BTreeMap::from([(
            baseline.clone(),
            observation(&"b".repeat(64), &["initialize-a"]),
        )]);
        let second_fingerprints = BTreeMap::from([(baseline.clone(), "b".repeat(64))]);
        recorder.bind_observed_host_turn_baselines(&first, &second)?;
        assert_eq!(recorder.observed_host_turn_baselines, second_fingerprints);

        assert!(recorder
            .bind_observed_host_turn_baselines(&first, &second)
            .is_err());
        assert_eq!(recorder.observed_host_turn_baselines, second_fingerprints);
        assert!(recorder
            .bind_observed_host_turn_baselines(&second, &empty)
            .is_err());
        assert_eq!(recorder.observed_host_turn_baselines, second_fingerprints);

        let historical_baseline = ObservedHostTurnBaseline {
            project_id: "project_fixture".to_owned(),
            watch_baseline_id: "watch_base_managed_historical".to_owned(),
        };
        let historical_before = BTreeMap::from([(
            historical_baseline.clone(),
            observation(&"c".repeat(64), &["initialize-historical"]),
        )]);
        let historical_unrelated_after = BTreeMap::from([(
            historical_baseline.clone(),
            observation(&"d".repeat(64), &["initialize-historical"]),
        )]);
        let mut historical_recorder = LiveResultRecorder::new("claude-code", None)?;
        historical_recorder.bind_observed_runtime_home(runtime_home.path())?;
        historical_recorder
            .bind_observed_host_turn_baselines(&historical_before, &historical_unrelated_after)?;
        assert!(historical_recorder.observed_host_turn_baselines.is_empty());
        let historical_initialized_after = BTreeMap::from([(
            historical_baseline.clone(),
            observation(
                &"e".repeat(64),
                &["initialize-historical", "initialize-current"],
            ),
        )]);
        historical_recorder
            .bind_observed_host_turn_baselines(&historical_before, &historical_initialized_after)?;
        assert_eq!(
            historical_recorder.observed_host_turn_baselines,
            BTreeMap::from([(historical_baseline, "e".repeat(64))])
        );
        Ok(())
    }

    #[test]
    fn candidate_digest_guards_reject_private_mutation_and_descriptor_path_replacement(
    ) -> Result<(), Box<dyn Error>> {
        fn candidate_fixture(path: &Path) -> Result<ReleaseCandidate, Box<dyn Error>> {
            let binary_sha256 = sha256_file(path, MAX_RELEASE_CANDIDATE_BINARY_BYTES)?;
            Ok(ReleaseCandidate {
                descriptor_path: None,
                schema: RELEASE_CANDIDATE_SCHEMA.to_owned(),
                candidate_id: "candidate_integrity_fixture".to_owned(),
                candidate_path: path_text(path),
                source_revision: "1".repeat(40),
                source_clean: true,
                source_archive_algorithm: RELEASE_SOURCE_ARCHIVE_ALGORITHM.to_owned(),
                source_archive_sha256: "2".repeat(64),
                target_triple: "fixture-target".to_owned(),
                release_profile: "release".to_owned(),
                binary_sha256,
                build_environment: ReleaseCandidateBuildEnvironment {
                    runner_os: "descriptor-build-os".to_owned(),
                    runner_os_version: "descriptor-build-version".to_owned(),
                    runner_arch: "descriptor-build-arch".to_owned(),
                    git_version: "git fixture".to_owned(),
                    rustc_version: "rustc fixture".to_owned(),
                    cargo_version: "cargo fixture".to_owned(),
                },
                recorded_at: "2026-07-14T00:00:00Z".to_owned(),
            })
        }

        let temp = TempRuntimeHome::new("candidate-integrity-guards")?;
        let release_root = temp.product_repo_path("release-artifacts");
        fs::create_dir_all(&release_root)?;
        let candidate_path = release_root.join("candidate-volicord");
        fs::copy(volicord_bin(), &candidate_path)?;
        let mut permissions = fs::metadata(&candidate_path)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&candidate_path, permissions)?;
        let candidate = candidate_fixture(&candidate_path)?;
        candidate.validate()?;

        let fixture = LiveSmokeFixture::new_with_release_candidate(
            "candidate-private-integrity",
            &candidate,
        )?;
        fixture.verify_private_candidate_digest()?;
        assert_eq!(
            fs::metadata(&fixture.volicord_path)?.permissions().mode() & 0o222,
            0
        );
        let mut writable = fs::metadata(&fixture.volicord_path)?.permissions();
        writable.set_mode(0o755);
        fs::set_permissions(&fixture.volicord_path, writable)?;
        OpenOptions::new()
            .append(true)
            .open(&fixture.volicord_path)?
            .write_all(b"candidate-mutation")?;
        let mut read_only = fs::metadata(&fixture.volicord_path)?.permissions();
        read_only.set_mode(0o555);
        fs::set_permissions(&fixture.volicord_path, read_only)?;
        assert!(fixture.run_volicord(["--version"]).is_err());

        let replacement_path = release_root.join("replacement-candidate-volicord");
        fs::copy(volicord_bin(), &replacement_path)?;
        let mut replacement_permissions = fs::metadata(&replacement_path)?.permissions();
        replacement_permissions.set_mode(0o755);
        fs::set_permissions(&replacement_path, replacement_permissions)?;
        let replacement_candidate = candidate_fixture(&replacement_path)?;
        replacement_candidate.validate()?;
        let result_root = release_root.join("release-results");
        let (cell_directory, _, _) = create_live_result_root(&result_root)?;
        let result_path = cell_directory.join("replacement-release-cell.json");
        {
            let mut recorder = LiveResultRecorder::new("claude-code", Some(result_path.clone()))?;
            recorder.release_candidate = Some(replacement_candidate.clone());
            recorder.release_feature = Some(HostFeature::NativeUserAction);
            let original_path = release_root.join("original-candidate-volicord");
            fs::rename(&replacement_path, original_path)?;
            fs::write(&replacement_path, b"#!/bin/sh\nexit 1\n")?;
            let mut replacement_permissions = fs::metadata(&replacement_path)?.permissions();
            replacement_permissions.set_mode(0o755);
            fs::set_permissions(&replacement_path, replacement_permissions)?;
            assert!(recorder
                .record_final(&native_user_action_result_shape_fixture(
                    "claude-code",
                    &"3".repeat(64),
                ))
                .is_err());
        }
        assert!(
            !result_path.exists(),
            "candidate path replacement must not produce a passing release cell"
        );
        Ok(())
    }

    #[test]
    fn installed_host_preflight_retains_real_coordinates_and_rejects_false_null_availability(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = LiveSmokeFixture::new("installed-host-preflight")?;
        let fake_host = fixture.runtime_home_path.join("fake-installed-host");
        fs::write(
            &fake_host,
            "#!/bin/sh\nprintf 'fixture-host 1.0\\n'\nexit 7\n",
        )?;
        make_executable(&fake_host)?;

        let mut recorder = LiveResultRecorder::new("codex", None)?;
        assert!(fixture
            .observe_and_bind_installed_host_identity(&mut recorder, "fake-host", &fake_host)
            .is_err());
        let enriched =
            recorder.with_observed_host_identity(&recorder.failed_before_completion_summary())?;
        assert_eq!(enriched["host"]["version"], "fixture-host 1.0");
        assert_eq!(
            enriched["host"]["executable_sha256"],
            sha256_file(&fake_host, MAX_RELEASE_CANDIDATE_BINARY_BYTES)?
        );

        let candidate = ReleaseCandidate {
            descriptor_path: None,
            schema: RELEASE_CANDIDATE_SCHEMA.to_owned(),
            candidate_id: "candidate_unresolved_installed_host".to_owned(),
            candidate_path: path_text(&fixture.volicord_path),
            source_revision: "4".repeat(40),
            source_clean: true,
            source_archive_algorithm: RELEASE_SOURCE_ARCHIVE_ALGORITHM.to_owned(),
            source_archive_sha256: "5".repeat(64),
            target_triple: "fixture-target".to_owned(),
            release_profile: "release".to_owned(),
            binary_sha256: sha256_file(&fixture.volicord_path, MAX_RELEASE_CANDIDATE_BINARY_BYTES)?,
            build_environment: ReleaseCandidateBuildEnvironment {
                runner_os: "fixture-os".to_owned(),
                runner_os_version: "fixture-version".to_owned(),
                runner_arch: "fixture-arch".to_owned(),
                git_version: "git fixture".to_owned(),
                rustc_version: "rustc fixture".to_owned(),
                cargo_version: "cargo fixture".to_owned(),
            },
            recorded_at: "2026-07-14T00:00:00Z".to_owned(),
        };
        candidate.validate()?;

        let successful_host = fixture.runtime_home_path.join("successful-installed-host");
        fs::write(
            &successful_host,
            "#!/bin/sh\nprintf 'fixture-host 1.0\\n'\n",
        )?;
        make_executable(&successful_host)?;
        let result_root = fixture.release_artifact_root.join("release-results");
        let (cell_dir, _, _) = create_live_result_root(&result_root)?;
        let terminal_path = cell_dir.join("installed-host-terminal-failure.json");
        let mut terminal = LiveResultRecorder::new("codex", Some(terminal_path.clone()))?;
        terminal.release_candidate = Some(candidate.clone());
        terminal.release_feature = Some(HostFeature::NativeUserAction);
        fixture.observe_and_bind_installed_host_identity(
            &mut terminal,
            "fake-host",
            &successful_host,
        )?;
        terminal.record_final(&live_user_action_unavailable_summary(
            "codex",
            Some("fixture-host 1.0"),
            "interactive_terminal",
            "fixture terminal unavailable after installed-host preflight",
        ))?;
        let terminal_cell: Value = serde_json::from_slice(&fs::read(&terminal_path)?)?;
        assert_eq!(terminal_cell["host_version"], "fixture-host 1.0");
        assert_eq!(
            terminal_cell["environment"]["host_executable_sha256"],
            sha256_file(&successful_host, MAX_RELEASE_CANDIDATE_BINARY_BYTES)?
        );
        assert_eq!(terminal_cell["run_state"], "completed");

        let unresolved_path = cell_dir.join("unresolved-installed-host-cell.json");
        {
            let mut unresolved = LiveResultRecorder::new("codex", Some(unresolved_path.clone()))?;
            unresolved.release_candidate = Some(candidate);
            unresolved.release_feature = Some(HostFeature::NativeUserAction);
            unresolved.mark_installed_host_detected();
            assert!(unresolved
                .record_final(&live_user_action_unavailable_summary(
                    "codex",
                    None,
                    "fixture_setup",
                    "fixture installed identity unresolved",
                ))
                .is_err());
        }
        assert!(
            !unresolved_path.exists(),
            "an installed host with unresolved coordinates must not become a null-identity cell"
        );
        Ok(())
    }

    #[test]
    fn reviewed_codex_static_unsupported_live_runner_never_launches_host_and_publishes_cell(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = LiveSmokeFixture::new("reviewed-codex-static-unsupported-runner")?;
        let candidate = ReleaseCandidate {
            descriptor_path: None,
            schema: RELEASE_CANDIDATE_SCHEMA.to_owned(),
            candidate_id: "candidate_reviewed_codex_static_unsupported".to_owned(),
            candidate_path: path_text(&fixture.volicord_path),
            source_revision: "a".repeat(40),
            source_clean: true,
            source_archive_algorithm: RELEASE_SOURCE_ARCHIVE_ALGORITHM.to_owned(),
            source_archive_sha256: "b".repeat(64),
            target_triple: "fixture-target".to_owned(),
            release_profile: "release".to_owned(),
            binary_sha256: fixture.expected_volicord_sha256.clone(),
            build_environment: ReleaseCandidateBuildEnvironment {
                runner_os: "fixture-os".to_owned(),
                runner_os_version: "fixture-version".to_owned(),
                runner_arch: "fixture-arch".to_owned(),
                git_version: "git fixture".to_owned(),
                rustc_version: "rustc fixture".to_owned(),
                cargo_version: "cargo fixture".to_owned(),
            },
            recorded_at: "2026-07-15T00:00:00Z".to_owned(),
        };
        candidate.validate()?;

        let host_bin = fixture.release_artifact_root.join("counting-host-bin");
        let authenticated_launch_log = fixture
            .release_artifact_root
            .join("authenticated-host-launches.log");
        let fake_codex = write_counting_fake_codex_with_version(
            &host_bin,
            REVIEWED_CODEX_HOST_VERSION,
            &authenticated_launch_log,
        )?;
        let result_root = fixture
            .release_artifact_root
            .join("static-unsupported-results");
        let (cell_directory, _, _) = create_live_result_root(&result_root)?;
        let cell_path = cell_directory.join("codex-local-web.json");
        let mut recorder = LiveResultRecorder::new("codex", Some(cell_path.clone()))?;
        recorder.release_candidate = Some(candidate.clone());
        recorder.release_feature = Some(HostFeature::LocalWebUserChannel);

        execute_live_evidence_observation_round_trip(
            "codex",
            InstalledHostExecutable::at_path("codex", &fake_codex),
            "host_trust_required",
            recorder,
        )?;
        assert_eq!(
            fs::read_to_string(&authenticated_launch_log)?
                .lines()
                .count(),
            0,
            "reviewed static unsupported Codex must not reach login status or an authenticated host turn"
        );

        let cell: Value = serde_json::from_slice(&fs::read(&cell_path)?)?;
        assert_eq!(cell["schema"], RELEASE_CELL_SCHEMA);
        assert_eq!(cell["host_kind"], "codex");
        assert_eq!(cell["host_version"], REVIEWED_CODEX_HOST_VERSION);
        assert_eq!(cell["feature"], "local_web_user_channel");
        assert_eq!(cell["implementation_disposition"], "unsupported_by_host");
        assert_eq!(cell["requested_verified"], false);
        assert_eq!(cell["claimed_status"], "unsupported_by_host");
        assert_eq!(cell["run_state"], "not_applicable");
        assert_eq!(cell["client_name"], Value::Null);
        assert_eq!(cell["client_version"], Value::Null);
        assert_eq!(cell["evidence_artifact_path"], Value::Null);
        assert_eq!(cell["evidence_artifact_sha256"], Value::Null);
        assert!(!release_evidence_path(&cell_path)?.exists());
        assert_eq!(
            fs::read_to_string(&authenticated_launch_log)?
                .lines()
                .count(),
            0,
            "host-free publication must not launch the reviewed Codex host"
        );

        drop(ResultRootLease::acquire_exclusive_for_cell_path(
            &release_validation_context()?,
            &cell_directory.join("next-cell.json"),
        )?);
        Ok(())
    }

    #[test]
    fn every_fixture_host_invocation_guards_the_private_candidate_digest(
    ) -> Result<(), Box<dyn Error>> {
        fn write_mutating_host(
            fixture: &LiveSmokeFixture,
            name: &str,
            output: &str,
        ) -> Result<PathBuf, Box<dyn Error>> {
            let host = fixture.runtime_home_path.join(name);
            let candidate = shell_quote(&fixture.volicord_path);
            fs::write(
                &host,
                format!(
                    "#!/bin/sh\nchmod u+w {candidate}\nprintf mutation >> {candidate}\nprintf '%s\\n' {}\n",
                    shell_quote(Path::new(output))
                ),
            )?;
            make_executable(&host)?;
            Ok(host)
        }

        let version_fixture = LiveSmokeFixture::new("guarded-host-version")?;
        let version_host = write_mutating_host(
            &version_fixture,
            "mutating-version-host",
            "fixture-host 1.0",
        )?;
        assert!(version_fixture
            .run_installed_host_version_probe(&version_host)
            .is_err());

        let login_fixture = LiveSmokeFixture::new("guarded-login-status")?;
        let login_host = write_mutating_host(
            &login_fixture,
            "mutating-login-host",
            "Logged in using ChatGPT",
        )?;
        assert!(login_fixture
            .require_codex_chatgpt_login_immediately_before_cell("codex", &login_host,)
            .is_err());

        let unwind_fixture = LiveSmokeFixture::new("guarded-host-unwind")?;
        assert!(unwind_fixture
            .with_private_candidate_digest_guard(|| -> Result<(), Box<dyn Error>> {
                let mut permissions = fs::metadata(&unwind_fixture.volicord_path)?.permissions();
                permissions.set_mode(0o755);
                fs::set_permissions(&unwind_fixture.volicord_path, permissions)?;
                OpenOptions::new()
                    .append(true)
                    .open(&unwind_fixture.volicord_path)?
                    .write_all(b"candidate-mutation-before-unwind")?;
                panic!("fixture host invocation unwind");
            })
            .is_err());
        Ok(())
    }

    #[test]
    fn installed_host_version_probe_does_not_use_disposable_codex_home(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = LiveSmokeFixture::new("host-version-home")?;
        let host = fixture.runtime_home_path.join("version-host");
        fs::write(
            &host,
            "#!/bin/sh\nif [ \"${CODEX_HOME+x}\" = x ]; then\n  printf 'unexpected CODEX_HOME\\n' >&2\n  exit 21\nfi\nprintf 'fixture-host 1.0\\n'\n",
        )?;
        make_executable(&host)?;

        let output = fixture.run_installed_host_version_probe(&host)?;
        require_success("fixture installed-host version probe", &output)?;
        assert_eq!(stdout(&output), "fixture-host 1.0\n");
        assert!(stderr(&output).is_empty());
        Ok(())
    }

    #[test]
    fn candidate_host_probe_codex_home_is_disposable_and_outside_system_temp(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = LiveSmokeFixture::new("candidate-host-version-home")?;
        let host_home_root = fixture.host_home_root.clone();
        let system_temp = fs::canonicalize(env::temp_dir())?;
        let codex_home = fs::canonicalize(&fixture.codex_home)?;
        let target_directory = release_validation_context()?
            .target_directory()
            .to_path_buf();
        assert!(!codex_home.starts_with(system_temp));
        assert!(codex_home.starts_with(target_directory));

        drop(fixture);
        assert!(!host_home_root.exists());
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
            LIVE_HOST_RESULT_PATH_ENV,
            RELEASE_CANDIDATE_PATH_ENV,
            RELEASE_REQUEST_VERIFIED_ENV,
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
    fn authenticated_host_launch_removes_inherited_api_key_and_token_environment() {
        let secret_names = [
            "OPENAI_API_KEY",
            "CODEX_API_KEY",
            "ANTHROPIC_API_KEY",
            "ANTHROPIC_AUTH_TOKEN",
            "CLAUDE_CODE_OAUTH_TOKEN",
            "CLAUDE_CODE_API_KEY",
        ];
        let mut command = Command::new("host-fixture");
        for name in secret_names {
            command.env(name, "inherited-secret");
        }
        LiveSmokeFixture::remove_inherited_auth_secret_env(&mut command);
        for name in secret_names {
            assert!(command
                .get_envs()
                .any(|(key, value)| { key.to_string_lossy() == name && value.is_none() }));
        }
    }

    #[test]
    fn codex_login_status_requires_one_unambiguous_chatgpt_line() {
        assert!(validate_codex_chatgpt_login_status("Logged in using ChatGPT\n", "").is_ok());
        assert!(validate_codex_chatgpt_login_status(
            "Logged in using ChatGPT\nambiguous additional identity\n",
            ""
        )
        .is_err());
        assert!(validate_codex_chatgpt_login_status(
            "Logged in using ChatGPT\n",
            "API key authentication is also configured"
        )
        .is_err());
        assert!(validate_codex_chatgpt_login_status("Logged in using API key\n", "").is_err());
    }

    #[test]
    fn release_requested_verified_claim_is_strict_and_availability_bounded(
    ) -> Result<(), Box<dyn Error>> {
        assert_eq!(parse_release_requested_verified_claim(None)?, None);
        assert_eq!(
            parse_release_requested_verified_claim(Some(OsStr::new("0")))?,
            Some(false)
        );
        assert_eq!(
            parse_release_requested_verified_claim(Some(OsStr::new("1")))?,
            Some(true)
        );
        assert!(parse_release_requested_verified_claim(Some(OsStr::new("true"))).is_err());
        assert!(parse_release_requested_verified_claim(Some(OsStr::new(""))).is_err());
        assert!(resolve_release_requested_verified(None, false, true)?);
        assert!(!resolve_release_requested_verified(
            Some(false),
            false,
            true
        )?);
        assert!(resolve_release_requested_verified(None, false, false)?);
        assert!(resolve_release_requested_verified(
            Some(true),
            false,
            false
        )?);
        assert!(!resolve_release_requested_verified(
            Some(false),
            false,
            false
        )?);
        assert!(!resolve_release_requested_verified(None, true, true)?);
        assert!(resolve_release_requested_verified(Some(true), true, true).is_err());
        Ok(())
    }

    #[test]
    fn native_user_action_result_shape_rejects_false_pass_mutations() -> Result<(), Box<dyn Error>>
    {
        let result = native_user_action_result_shape_fixture("codex", &"a".repeat(64));
        validate_live_user_action_result_shape(&result)?;
        validate_release_cell_passed_summary(HostFeature::NativeUserAction, &result)?;
        for path in [
            &["user_action", "stored_choice_matches_operator"][..],
            &["native_ui", "user_action_selector_confirmed"][..],
            &["native_ui", "operator_choice_confirmed"][..],
            &[
                "native_ui",
                "stop_system_message_authority_receipt_confirmed",
            ][..],
            &["stop_hook", "decision_observed_from_guard_event"][..],
            &["authority_events", "ordered"][..],
        ] {
            let mut mutated = result.clone();
            set_nested_value(&mut mutated, path, Value::Bool(false))?;
            assert!(validate_live_user_action_result_shape(&mutated).is_err());
            assert!(
                validate_release_cell_passed_summary(HostFeature::NativeUserAction, &mutated)
                    .is_err()
            );
        }
        for (path, replacement) in [
            (
                &["stop_hook", "session_id"][..],
                Value::String("SESSION-native-fixture".to_owned()),
            ),
            (
                &["stop_hook", "guard_event_id"][..],
                Value::String("GE-native-fixture".to_owned()),
            ),
        ] {
            let mut mutated = result.clone();
            set_nested_value(&mut mutated, path, replacement)?;
            assert!(validate_live_user_action_result_shape(&mutated).is_err());
        }
        Ok(())
    }

    #[test]
    fn producer_result_shape_rejects_false_pass_mutations() -> Result<(), Box<dyn Error>> {
        for feature in [
            HostFeature::VerifiedToolProducer,
            HostFeature::RegisteredConnectionObservation,
        ] {
            let result = producer_result_shape_fixture(feature);
            validate_live_producer_result_shape(&result, feature)?;
            validate_release_cell_passed_summary(feature, &result)?;
            assert!(serialize_live_host_result(&result)?.len() < MAX_LIVE_HOST_RESULT_BYTES);

            for path in [
                &["assertions", "actual_host_event"][..],
                &["assertions", "intent_precedes_source"][..],
                &[
                    "assertions",
                    "exact_session_connection_actor_scope_baseline",
                ][..],
                &["assertions", "capture_receipt_bound"][..],
                &["assertions", "strong_producer_chain"][..],
                &["assertions", "criterion_coverage_projected"][..],
                &["assertions", "negative_rejections_zero_effect"][..],
                &["actual_host_event", "observed"][..],
                &["capture_intent", "intent_precedes_source"][..],
                &["capture_receipt", "bound"][..],
                &["host_resume", "ordered"][..],
                &["producer_chain", "strong"][..],
                &["producer_chain", "criterion_coverage_projected"][..],
                &["close", "ready"][..],
            ] {
                let mut mutated = result.clone();
                set_nested_value(&mut mutated, path, Value::Bool(false))?;
                assert!(validate_live_producer_result_shape(&mutated, feature).is_err());
                assert!(validate_release_cell_passed_summary(feature, &mutated).is_err());
            }

            let mut nonzero_effect = result.clone();
            nonzero_effect["negative_capture"]["receipt_delta"] = serde_json::json!(1);
            assert!(validate_live_producer_result_shape(&nonzero_effect, feature).is_err());

            let mut invalid_digest = result.clone();
            invalid_digest["host"]["executable_sha256"] = Value::String("A".repeat(64));
            assert!(validate_live_producer_result_shape(&invalid_digest, feature).is_err());

            let mut blank_chain_id = result.clone();
            blank_chain_id["producer_chain"]["producer_id"] = Value::String(String::new());
            assert!(validate_live_producer_result_shape(&blank_chain_id, feature).is_err());

            let mut invalid_invocation = result.clone();
            invalid_invocation["actual_host_event"]["host_invocation_id"] =
                if feature == HostFeature::VerifiedToolProducer {
                    Value::Null
                } else {
                    Value::String("unexpected-tool-invocation".to_owned())
                };
            assert!(validate_live_producer_result_shape(&invalid_invocation, feature).is_err());
            if feature == HostFeature::VerifiedToolProducer {
                let mut raw_invocation = result.clone();
                raw_invocation["actual_host_event"]["host_invocation_id"] =
                    Value::String("tool-call-fixture".to_owned());
                assert!(validate_live_producer_result_shape(&raw_invocation, feature).is_err());
            }

            let mut raw_session = result.clone();
            raw_session["actual_host_event"]["opaque_session_id"] =
                Value::String("SESSION-producer-fixture".to_owned());
            assert!(validate_live_producer_result_shape(&raw_session, feature).is_err());

            let mut raw_source_event = result.clone();
            raw_source_event["actual_host_event"]["source_event_ids"][0] =
                Value::String("GE-producer-fixture".to_owned());
            assert!(validate_live_producer_result_shape(&raw_source_event, feature).is_err());

            let mut stale_close = result.clone();
            let intent_state_version =
                stale_close["capture_intent"]["intent_state_version"].clone();
            stale_close["close"]["state_version"] = intent_state_version;
            assert!(validate_live_producer_result_shape(&stale_close, feature).is_err());

            let mut forbidden_payload = result.clone();
            forbidden_payload["host"]["version"] = Value::String("https://secret.invalid".into());
            assert!(validate_live_producer_result_shape(&forbidden_payload, feature).is_err());

            let mut missing_semantic = result.clone();
            missing_semantic["assertions"]
                .as_object_mut()
                .expect("fixture assertions are an object")
                .remove("capture_receipt_bound");
            assert!(validate_release_cell_passed_summary(feature, &missing_semantic).is_err());
        }
        Ok(())
    }

    #[test]
    fn evidence_observation_result_shape_rejects_false_pass_mutations() -> Result<(), Box<dyn Error>>
    {
        let result = evidence_observation_result_shape_fixture();
        validate_live_evidence_observation_result_shape(&result)?;
        assert!(serialize_live_host_result(&result)?.len() < MAX_LIVE_HOST_RESULT_BYTES);

        for (path, replacement) in [
            (
                &["host", "executable_sha256"][..],
                Value::String("A".repeat(64)),
            ),
            (
                &["host_feature_support", "native_user_action"][..],
                Value::String("verified".to_owned()),
            ),
            (
                &[
                    "final_output_authority_disclosure",
                    "configuration_verified",
                ][..],
                Value::Bool(false),
            ),
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
                &["stop_hook", "session_id"][..],
                Value::String("SESSION-live".to_owned()),
            ),
            (
                &["stop_hook", "guard_event_id"][..],
                Value::String("GE-live".to_owned()),
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
            ("static_unsupported_by_host", "unavailable"),
            ("fixture_setup", "failed"),
            ("connection_observation", "failed"),
            ("host_process", "failed"),
            ("stored_resolution", "failed"),
            ("authority_receipt", "failed"),
            ("stop_and_diagnostics", "failed"),
            ("managed_receipt_ui", "failed"),
            ("result_validation", "failed"),
        ] {
            let incomplete =
                live_evidence_observation_incomplete_summary("codex", None, stage, None);
            assert_eq!(incomplete["result"], expected_result);
            assert_eq!(
                incomplete["final_output_authority_disclosure"]["configured"],
                false
            );
            validate_live_evidence_observation_incomplete_result_shape(&incomplete)?;
            assert!(serialize_live_host_result(&incomplete)?.len() < MAX_LIVE_HOST_RESULT_BYTES);
        }
        let unknown_stage =
            live_evidence_observation_incomplete_summary("codex", None, "raw-error-text", None);
        assert!(
            validate_live_evidence_observation_incomplete_result_shape(&unknown_stage).is_err()
        );
        let mut observed_connection_failure =
            LiveResultRecorder::new_for_kind("codex", LIVE_EVIDENCE_OBSERVATION_RESULT_KIND, None)?;
        observed_connection_failure.bind_observed_host_identity(
            ObservedReleaseHostIdentity::new(
                "codex fixture 1.0".to_owned(),
                "d".repeat(64),
                "fixture-observed-build".to_owned(),
            )?,
        )?;
        let observed_connection_failure = observed_connection_failure.with_observed_host_identity(
            &live_evidence_observation_incomplete_summary(
                "codex",
                Some("codex fixture 1.0"),
                "connection_observation",
                None,
            ),
        )?;
        validate_live_evidence_observation_incomplete_result_shape(&observed_connection_failure)?;
        assert_eq!(
            observed_connection_failure["host"]["executable_sha256"],
            "d".repeat(64)
        );
        assert_eq!(
            observed_connection_failure["volicord"]["build_id"],
            "fixture-observed-build"
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
        let prompt = live_evidence_observation_prompt(&prepared);
        assert!(prompt.contains(LIVE_EVIDENCE_OBSERVATION_RUN_MARKER));
        assert!(prompt.contains(&format!("`project_selector={}`", prepared.project_id)));
        for expected in [
            "`detail=full`",
            "`request.operation=create`",
            "`request.task_id=",
            "`request.change_unit_id=",
            "`request.required_for=[\"record_run\"]`",
            "`request.expires_at=null`",
            "`request.action.action_type=evidence_observation`",
            "`request.action.question=",
            "`request.action.context_summary=",
            "`request.action.target_candidates=",
            "`request.action.artifact_candidate_ids=",
            "Do not put `request.task_id`, `request.change_unit_id`, `request.required_for`, or `request.expires_at` inside `request.action`",
            "`request.operation=resume`",
            "`request.user_action_request_id`",
            "Do not include create-only fields in the resume `request`",
        ] {
            assert!(
                prompt.contains(expected),
                "live evidence-observation prompt omitted closed request guidance: {expected}"
            );
        }
        Ok(())
    }

    #[test]
    fn authenticated_live_prompts_bind_the_registered_opaque_project_selector(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = LiveSmokeFixture::new("registered-live-prompt-selector")?;
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
        assert_success("volicord init for live prompt routing fixture", &init);
        let init_json = json_stdout(&init)?;
        let connection_id = init_json["connection"]["connection_id"]
            .as_str()
            .ok_or_else(|| io::Error::other("live prompt init returned no connection id"))?;
        let prepared = prepare_live_cli_fallback_action(
            &fixture,
            connection_id,
            "VOLICORD_LIVE_PROMPT_MARKER",
        )?;
        let project_selector = live_fixture_project_id(&fixture)?;
        assert_eq!(prepared.observation.project_id, project_selector);
        let repository_name = fixture
            .repo_root
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| io::Error::other("live prompt repository name is unavailable"))?;
        assert_ne!(project_selector, repository_name);
        let expected = format!("`project_selector={project_selector}`");
        let guessed = format!("`project_selector={repository_name}`");
        let prompts = [
            live_final_output_no_active_prompt(&project_selector),
            live_user_action_prompt("VOLICORD_LIVE_PROMPT_MARKER", &project_selector),
            live_cli_fallback_resume_prompt(&prepared)?,
        ];

        for prompt in prompts {
            assert!(prompt.contains(&expected));
            assert!(prompt.contains("exact opaque selector"));
            assert!(!prompt.contains(&guessed));
        }
        Ok(())
    }

    #[test]
    fn live_user_action_prompt_names_the_closed_create_shape() {
        let prompt = live_user_action_prompt(
            "VOLICORD_LIVE_PROMPT_CLOSED_CREATE_SHAPE",
            "proj_live_prompt_closed_create_shape",
        );

        for expected in [
            "`request.task_id`",
            "`request.change_unit_id`",
            "`request.required_for=[\"close_complete\"]`",
            "`request.expires_at=null`",
            "`request.action.presentation=short`",
            "`request.action.question=\"Which live-smoke route must the agent consume?\"`",
            "`request.action.context={\"summary\":\"A human operator must choose the live-smoke route.\",\"related_refs\":[],\"artifact_refs\":[],\"visible_risks\":[],\"constraints\":[]}`",
            "`request.action.affected_refs=[]`",
            "`request.action.sensitive_action_scope=null`",
            "Do not add aliases such as `title` or `prompt`",
        ] {
            assert!(
                prompt.contains(expected),
                "live UserAction prompt omitted closed create guidance: {expected}"
            );
        }
    }

    #[test]
    fn cli_fallback_authority_event_inspection_uses_row_task_coordinates(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = LiveSmokeFixture::new("cli-fallback-authority-event-coordinates")?;
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
        assert_success("volicord init for CLI-fallback event fixture", &init);
        let init_json = json_stdout(&init)?;
        let connection_id = init_json["connection"]["connection_id"]
            .as_str()
            .ok_or_else(|| io::Error::other("CLI-fallback event init returned no connection id"))?;
        let marker = "VOLICORD_LIVE_CLI_FALLBACK_EVENT_COORDINATES";
        let prepared = prepare_live_cli_fallback_action(&fixture, connection_id, marker)?;
        resolve_live_user_action_via_cli(
            &fixture,
            &prepared.observation,
            USER_ACTION_ROUTE_ALPHA_OPTION_ID,
        )?;

        let context = McpConnectionContext::resolve(&fixture.runtime_home_path, connection_id)?
            .with_invocation_binding_basis(VERIFICATION_BASIS_TEST_FIXTURE_BINDING);
        let adapter = McpAdapter::new(&fixture.runtime_home_path, context);
        let task_id = prepared.observation.task_id.clone();
        let recorded = adapter.call_tool(
            "volicord.record_run",
            serde_json::json!({
                "detail": "full",
                "task_id": task_id,
                "change_unit_id": prepared.change_unit_id,
                "kind": "shaping_update",
                "run_id": null,
                "baseline_ref": LIVE_CLI_FALLBACK_BASELINE_REF,
                "write_ticket_id": null,
                "summary": USER_ACTION_ROUTE_ALPHA_RUN_MARKER,
                "observed_changes": {
                    "changed_paths": [],
                    "product_file_write_observed": false,
                    "sensitive_categories": [],
                    "baseline_ref": LIVE_CLI_FALLBACK_BASELINE_REF
                },
                "artifact_inputs": [],
                "evidence_updates": [],
                "evidence_observations": [],
                "close_assessment": {
                    "result_summary": USER_ACTION_ROUTE_ALPHA_RUN_MARKER,
                    "result_refs": [],
                    "residual_risks": [],
                    "sensitive_categories": [],
                    "recovery_constraints": []
                }
            }),
        )?;
        let run_id = recorded.response_value["run_summary"]["run_ref"]["record_id"]
            .as_str()
            .ok_or_else(|| io::Error::other("CLI-fallback event Run has no record id"))?;
        let observation = inspect_live_user_action(&fixture, marker)?
            .ok_or_else(|| io::Error::other("CLI-fallback event Task is missing"))?;
        let (run, event_order) = inspect_live_choice_consumption(&fixture, &observation, run_id)?;

        assert_eq!(run.summary, USER_ACTION_ROUTE_ALPHA_RUN_MARKER);
        assert!(
            event_order.user_action_requested_event_seq
                < event_order.user_action_resolved_event_seq
        );
        assert!(event_order.user_action_resolved_event_seq < event_order.run_recorded_event_seq);
        Ok(())
    }

    #[test]
    fn live_diagnostic_queries_require_ordered_post_cursor_status() -> Result<(), Box<dyn Error>> {
        let fixture = LiveSmokeFixture::new("evidence-diagnostic-query")?;
        assert_eq!(diagnostic_event_cursor(&fixture)?.0, 0);
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
        let probe_cursor = diagnostic_event_cursor(&fixture)?;
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
        record("volicord.status", false, false, None)?;
        assert_connection_observation_diagnostic(
            &fixture,
            connection_id,
            project_id,
            probe_cursor,
        )?;
        let diagnostic_cursor = diagnostic_event_cursor(&fixture)?;
        record(
            "volicord.request_user_action",
            true,
            false,
            Some(DiagnosticFallbackKind::LocalWebConsent),
        )?;
        record("volicord.request_user_action", false, true, None)?;
        record("volicord.record_run", true, false, None)?;
        assert!(assert_cli_fallback_resume_diagnostic(
            &fixture,
            connection_id,
            project_id,
            diagnostic_cursor,
        )
        .is_err());
        record("volicord.status", false, false, None)?;
        assert_cli_fallback_resume_diagnostic(
            &fixture,
            connection_id,
            project_id,
            diagnostic_cursor,
        )?;

        let observed = assert_local_web_evidence_diagnostic(
            &fixture,
            connection_id,
            project_id,
            diagnostic_cursor,
        )?;
        assert_eq!(observed.create_calls, 1);
        assert_eq!(observed.resume_calls, 1);
        assert_eq!(observed.record_run_calls, 1);
        assert_eq!(observed.committed_record_run_calls, 1);
        assert_eq!(observed.status_calls, 1);
        assert_eq!(observed.successful_status_calls, 1);
        assert!(observed.ordered);

        record("volicord.status", false, false, None)?;
        assert!(assert_local_web_evidence_diagnostic(
            &fixture,
            connection_id,
            project_id,
            diagnostic_cursor,
        )
        .is_err());
        Ok(())
    }

    #[test]
    fn cli_fallback_result_shape_keeps_release_cells_separate() -> Result<(), Box<dyn Error>> {
        let result = cli_fallback_result_shape_fixture();
        validate_live_cli_fallback_result_shape(&result)?;
        let mut mismatched_support = result.clone();
        mismatched_support["host_feature_support"]["native_user_action"] =
            Value::String("verified".to_owned());
        assert!(validate_live_cli_fallback_result_shape(&mismatched_support).is_err());
        let unavailable = live_cli_fallback_unavailable_summary(
            "codex",
            None,
            "host_executable",
            "fixture executable unavailable",
        );
        validate_release_host_feature_diagnostics(
            &unavailable,
            Some(IntegrationProfile::Detective),
            false,
            false,
        )?;
        assert_eq!(
            unavailable["host_feature_support"]["native_user_action"],
            "implemented_unverified"
        );
        assert_eq!(
            unavailable["final_output_authority_disclosure"]["configured"],
            false
        );

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
    fn user_action_preflight_result_keeps_canonical_host_feature_diagnostics(
    ) -> Result<(), Box<dyn Error>> {
        let unavailable = live_user_action_unavailable_summary(
            "claude-code",
            None,
            "interactive_terminal",
            "fixture terminal unavailable",
        );
        validate_release_host_feature_diagnostics(
            &unavailable,
            Some(IntegrationProfile::Detective),
            false,
            false,
        )?;
        assert_eq!(
            unavailable["host_feature_support"]
                .as_object()
                .map(serde_json::Map::len),
            Some(6)
        );
        assert_eq!(
            unavailable["final_output_authority_disclosure"]["configured"],
            false
        );
        let unselected = canonical_release_host_feature_diagnostics_for_profile(
            "codex", None, None, false, false,
        );
        let unselected_result = serde_json::json!({
            "host": { "kind": "codex" },
            "host_feature_support": unselected.host_feature_support,
            "final_output_authority_disclosure": unselected.final_output_authority_disclosure
        });
        validate_release_host_feature_diagnostics(&unselected_result, None, false, false)?;
        assert!(unselected_result["final_output_authority_disclosure"].is_null());
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
        assert_eq!(
            record["host_feature_support"],
            record["evidence"]["config_fixture"]["host_feature_support"]
        );
        assert_eq!(
            record["final_output_authority_disclosure"],
            record["evidence"]["config_fixture"]["final_output_authority_disclosure"]
        );
        let mut missing_top_level_support = record.clone();
        missing_top_level_support
            .as_object_mut()
            .expect("fixture result should be an object")
            .remove("host_feature_support");
        assert!(validate_final_output_result_shape(
            &missing_top_level_support,
            IntegrationProfile::Record
        )
        .is_err());
        let mut mismatched_top_level_support = record.clone();
        mismatched_top_level_support["host_feature_support"]["record_final_output"] =
            Value::String("verified".to_owned());
        assert!(validate_final_output_result_shape(
            &mismatched_top_level_support,
            IntegrationProfile::Record
        )
        .is_err());
        let mut mismatched_nested_disclosure = detective.clone();
        mismatched_nested_disclosure["evidence"]["config_fixture"]
            ["final_output_authority_disclosure"]["configured"] = Value::Bool(false);
        assert!(validate_final_output_result_shape(
            &mismatched_nested_disclosure,
            IntegrationProfile::Detective
        )
        .is_err());
        for profile in [IntegrationProfile::Record, IntegrationProfile::Detective] {
            let unavailable =
                final_output_unavailable_summary("codex", profile, "fixture prerequisite missing");
            validate_final_output_result_shape(&unavailable, profile)?;
            assert_eq!(
                unavailable["evidence"]["detective_decision"]["status"],
                "unavailable"
            );
            assert_eq!(
                unavailable["host_feature_support"],
                unavailable["evidence"]["config_fixture"]["host_feature_support"]
            );
            assert_eq!(
                unavailable["final_output_authority_disclosure"],
                unavailable["evidence"]["config_fixture"]["final_output_authority_disclosure"]
            );
            assert_eq!(
                unavailable["final_output_authority_disclosure"]["configured"],
                false
            );
            assert_eq!(
                unavailable["final_output_authority_disclosure"]["configuration_verified"],
                false
            );
            assert_eq!(
                unavailable["host_feature_support"]["record_final_output"],
                "unsupported_by_host"
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
        let mut missing_record_mcp_observation = record.clone();
        missing_record_mcp_observation["evidence"]["actual_host_event"]["status_fallback_event"]
            .as_object_mut()
            .expect("Record event should be an object")
            .remove("managed_mcp_observation");
        assert!(validate_final_output_result_shape(
            &missing_record_mcp_observation,
            IntegrationProfile::Record
        )
        .is_err());
        let mut bounded_multi_session_record = record.clone();
        let multi_session = &mut bounded_multi_session_record["evidence"]["actual_host_event"]
            ["authority_receipt_event"]["managed_mcp_observation"];
        multi_session["agent_session_delta"] = Value::from(2);
        multi_session["session_ids"] = serde_json::json!([
            format!("mhs_{}", "d".repeat(64)),
            format!("mhs_{}", "a".repeat(64))
        ]);
        validate_final_output_result_shape(
            &bounded_multi_session_record,
            IntegrationProfile::Record,
        )?;
        let mut zero_session_delta = record.clone();
        zero_session_delta["evidence"]["actual_host_event"]["authority_receipt_event"]
            ["managed_mcp_observation"]["agent_session_delta"] = Value::from(0);
        assert!(validate_final_output_result_shape(
            &zero_session_delta,
            IntegrationProfile::Record
        )
        .is_err());
        let mut mismatched_session_set = record.clone();
        mismatched_session_set["evidence"]["actual_host_event"]["authority_receipt_event"]
            ["managed_mcp_observation"]["agent_session_delta"] = Value::from(2);
        assert!(validate_final_output_result_shape(
            &mismatched_session_set,
            IntegrationProfile::Record
        )
        .is_err());
        let mut duplicate_session_set = record.clone();
        let duplicate_managed = &mut duplicate_session_set["evidence"]["actual_host_event"]
            ["authority_receipt_event"]["managed_mcp_observation"];
        duplicate_managed["agent_session_delta"] = Value::from(2);
        duplicate_managed["session_ids"] = serde_json::json!([
            format!("mhs_{}", "a".repeat(64)),
            format!("mhs_{}", "a".repeat(64))
        ]);
        assert!(validate_final_output_result_shape(
            &duplicate_session_set,
            IntegrationProfile::Record
        )
        .is_err());
        let mut missing_detective_persistence = detective.clone();
        missing_detective_persistence["evidence"]["actual_host_event"]["authority_receipt_event"]
            ["persistent_guard_event"] = Value::Bool(false);
        assert!(validate_final_output_result_shape(
            &missing_detective_persistence,
            IntegrationProfile::Detective
        )
        .is_err());
        let mut raw_record_session = record.clone();
        raw_record_session["evidence"]["actual_host_event"]["status_fallback_event"]
            ["managed_mcp_observation"]["session_ids"][0] =
            Value::String("SESSION-status-fixture".to_owned());
        assert!(validate_final_output_result_shape(
            &raw_record_session,
            IntegrationProfile::Record
        )
        .is_err());
        let mut raw_detective_correlation = detective.clone();
        raw_detective_correlation["evidence"]["actual_host_event"]["status_fallback_event"]
            ["guard_event_id"] = Value::String("GE-status-fixture".to_owned());
        assert!(validate_final_output_result_shape(
            &raw_detective_correlation,
            IntegrationProfile::Detective
        )
        .is_err());
        let mut raw_detective_session = detective.clone();
        raw_detective_session["evidence"]["actual_host_event"]["authority_receipt_event"]
            ["session_id"] = Value::String("SESSION-receipt-fixture".to_owned());
        assert!(validate_final_output_result_shape(
            &raw_detective_session,
            IntegrationProfile::Detective
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
            match host {
                "codex" => {
                    write_fake_codex(fixture.live_bin())?;
                }
                "claude-code" => {
                    write_fake_claude_code(fixture.live_bin())?;
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
            assert_direct_matrix_init_report(&init_json, host, None, profile, expected_host_action);
            let connection_id = init_json["connection"]["connection_id"]
                .as_str()
                .ok_or_else(|| io::Error::other("matrix init returned no connection id"))?;
            let config_fixture =
                verify_final_output_config_fixture(&fixture, host, None, profile, &init_json)
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
            let first_no_active =
                fixture.run_generated_final_output_handler(host, &no_active_event)?;
            verify_no_active_status_wire(&first_no_active, no_active_private_prose)?;
            let after_first_no_active = guard_observation_counts(&fixture, &project_id)?;
            let second_no_active =
                fixture.run_generated_final_output_handler(host, &no_active_event)?;
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
            let first_active = fixture.run_generated_final_output_handler(host, &active_event)?;
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
                Some(stored_stop_snapshot_for_native_session(
                    &fixture,
                    &project_id,
                    host,
                    connection_id,
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
            let second_active = fixture.run_generated_final_output_handler(host, &active_event)?;
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
                        stored_stop_snapshot_for_native_session(
                            &fixture,
                            &project_id,
                            host,
                            connection_id,
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
        let selected_volicord = fs::canonicalize(volicord_bin())?;
        write_installation_profile(
            fixture.runtime_home_path(),
            InstallationProfileRegistration {
                installation_id: "default".to_owned(),
                volicord_command: path_text(&selected_volicord),
                volicord_mcp_command: path_text(&selected_volicord),
                bin_dir: selected_volicord
                    .parent()
                    .ok_or("test Volicord binary should have a parent directory")?
                    .to_path_buf(),
                default_connection_mode: CONNECTION_MODE_WORKFLOW.to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;
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
        assert_init_host_feature_support(&init_json, "codex", None, IntegrationProfile::Record);
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
        let host_version = canonical_codex_version_summary(&version)?;

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
            Some(&host_version),
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
        assert!(codex_mcp.contains("env_vars = [\"VOLICORD_HOME\"]"));
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
        let host_version = host_version_summary(&version)?;

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
            Some(&host_version),
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
        let claude_mcp_json: Value = serde_json::from_str(&claude_mcp)?;
        assert_eq!(
            claude_mcp_json["mcpServers"]["volicord"]["env"],
            serde_json::json!({"VOLICORD_HOME": "${VOLICORD_HOME}"})
        );
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
    #[ignore = "requires an installed Codex host and VOLICORD_RUN_CODEX_RECORD_FINAL_OUTPUT_SMOKE=1"]
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
    #[ignore = "requires an installed Codex host and VOLICORD_RUN_CODEX_DETECTIVE_FINAL_OUTPUT_SMOKE=1"]
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

    #[test]
    #[ignore = "requires an authenticated interactive Codex host and VOLICORD_RUN_CODEX_VERIFIED_TOOL_PRODUCER_SMOKE=1"]
    fn codex_live_verified_tool_producer_is_opt_in() -> Result<(), Box<dyn Error>> {
        live_evidence_producer_matrix_cell(
            "codex",
            "codex",
            HostFeature::VerifiedToolProducer,
            CODEX_VERIFIED_TOOL_PRODUCER_SMOKE_ENV,
            "host_trust_required",
        )
    }

    #[test]
    #[ignore = "requires an authenticated interactive Claude Code host and VOLICORD_RUN_CLAUDE_VERIFIED_TOOL_PRODUCER_SMOKE=1"]
    fn claude_code_live_verified_tool_producer_is_opt_in() -> Result<(), Box<dyn Error>> {
        live_evidence_producer_matrix_cell(
            "claude-code",
            "claude",
            HostFeature::VerifiedToolProducer,
            CLAUDE_VERIFIED_TOOL_PRODUCER_SMOKE_ENV,
            "project_approval_required",
        )
    }

    #[test]
    #[ignore = "requires an authenticated interactive Codex host and VOLICORD_RUN_CODEX_REGISTERED_CONNECTION_OBSERVATION_SMOKE=1"]
    fn codex_live_registered_connection_observation_is_opt_in() -> Result<(), Box<dyn Error>> {
        live_evidence_producer_matrix_cell(
            "codex",
            "codex",
            HostFeature::RegisteredConnectionObservation,
            CODEX_REGISTERED_CONNECTION_OBSERVATION_SMOKE_ENV,
            "host_trust_required",
        )
    }

    #[test]
    #[ignore = "requires an authenticated interactive Claude Code host and VOLICORD_RUN_CLAUDE_REGISTERED_CONNECTION_OBSERVATION_SMOKE=1"]
    fn claude_code_live_registered_connection_observation_is_opt_in() -> Result<(), Box<dyn Error>>
    {
        live_evidence_producer_matrix_cell(
            "claude-code",
            "claude",
            HostFeature::RegisteredConnectionObservation,
            CLAUDE_REGISTERED_CONNECTION_OBSERVATION_SMOKE_ENV,
            "project_approval_required",
        )
    }

    fn live_evidence_producer_matrix_cell(
        host: &str,
        executable_name: &str,
        feature: HostFeature,
        selector_env: &str,
        expected_host_action: &str,
    ) -> Result<(), Box<dyn Error>> {
        if !smoke_enabled(selector_env) {
            return Err(io::Error::other(format!(
                "set {selector_env}=1 before running the ignored {host}/{} live producer cell",
                feature.as_str()
            ))
            .into());
        }
        let (result_kind, baseline_ref) = match feature {
            HostFeature::VerifiedToolProducer => (
                LIVE_VERIFIED_TOOL_PRODUCER_RESULT_KIND,
                LIVE_VERIFIED_TOOL_PRODUCER_BASELINE_REF,
            ),
            HostFeature::RegisteredConnectionObservation => (
                LIVE_REGISTERED_CONNECTION_OBSERVATION_RESULT_KIND,
                LIVE_REGISTERED_CONNECTION_OBSERVATION_BASELINE_REF,
            ),
            _ => {
                return Err(io::Error::other(
                    "live evidence producer cell requires a producer HostFeature",
                )
                .into())
            }
        };
        let recorder_label = format!("{host}-{}", feature.as_str());
        let mut result_recorder = LiveResultRecorder::from_env_for_kind_and_profile(
            &recorder_label,
            host,
            result_kind,
            Some(IntegrationProfile::Detective),
        )?;
        let release_candidate = result_recorder.release_candidate()?.clone();
        let fixture = LiveSmokeFixture::new_with_release_candidate_for_recorder(
            &recorder_label,
            &release_candidate,
            &mut result_recorder,
        )?;
        let executable = find_executable(executable_name).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("`{executable_name}` was not found on PATH"),
            )
        })?;
        result_recorder.mark_installed_host_detected();
        let observed_identity = fixture.observe_and_bind_installed_host_identity(
            &mut result_recorder,
            executable_name,
            &executable,
        )?;
        if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
            return Err(io::Error::other(
                "authenticated live producer validation requires interactive terminal stdin and stdout",
            )
            .into());
        }
        let ObservedReleaseHostIdentity {
            host_version,
            host_executable_sha256,
            volicord_build_id,
        } = observed_identity;
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
        require_success("volicord init for live producer cell", &init)?;
        let init_json = json_stdout(&init)?;
        assert_live_init_reported_action_required(
            &init_json,
            host,
            Some(&host_version),
            IntegrationProfile::Detective,
            expected_host_action,
        );
        let host_feature_diagnostics = release_host_feature_diagnostics_from_init(
            &init_json,
            host,
            Some(&host_version),
            IntegrationProfile::Detective,
        )?;
        let connection_id = bounded_identity(
            "Agent Connection id",
            init_json["connection"]["connection_id"]
                .as_str()
                .ok_or_else(|| io::Error::other("producer init result has no connection id"))?,
            MAX_CONNECTION_ID_BYTES,
        )?;
        let identity = LiveHostIdentity {
            host: host.to_owned(),
            host_version,
            host_executable_sha256,
            volicord_build_id,
            connection_id,
        };
        observe_and_verify_live_connection_before_task(
            &fixture,
            host,
            &executable,
            &identity.connection_id,
            &mut result_recorder,
        )?;

        let marker = format!(
            "VOLICORD_LIVE_{}_{}",
            feature.as_str().to_ascii_uppercase(),
            host.replace('-', "_").to_ascii_uppercase()
        );
        let prepared = prepare_live_producer_authority_basis(
            &fixture,
            &identity.connection_id,
            &marker,
            baseline_ref,
        )?;
        let source_prompt = live_producer_source_prompt(&prepared, feature)?;
        println!(
            "\n=== Volicord live {host}/{} source turn ===\nThe authenticated host must prepare an exact short-lived capture intent before producing the registered source event in the same opaque managed session. Approve the repository or MCP entry if the host asks. Do not type credentials or secrets.\n\n{source_prompt}\n=== end instruction ===\n",
            feature.as_str()
        );
        let source_status = fixture.run_authenticated_interactive_host(
            host,
            &executable,
            &source_prompt,
            &mut result_recorder,
        )?;
        if !source_status.success() {
            return Err(io::Error::other(format!(
                "the live {host}/{} source turn exited unsuccessfully with {}",
                feature.as_str(),
                status_text(source_status)
            ))
            .into());
        }

        let captured_source = inspect_actual_live_producer_source(
            &fixture,
            &prepared,
            &identity.connection_id,
            feature,
        )?;
        let before_negative = live_capture_durable_snapshot(&fixture, &prepared.project_id)?;
        let negative = run_mismatched_live_capture(&fixture, &captured_source, feature, &prepared)?;
        let after_negative = live_capture_durable_snapshot(&fixture, &prepared.project_id)?;
        if before_negative != after_negative {
            return Err(io::Error::other(
                "mismatched live capture changed durable receipt, staging, producer, artifact, authority-event, or state-version counts",
            )
            .into());
        }

        let capture_output =
            run_exact_live_capture(&fixture, &captured_source, feature, &prepared)?;
        let receipt = inspect_live_capture_receipt(
            &fixture,
            &prepared,
            &captured_source,
            feature,
            &capture_output,
        )?;
        let resume_cursor = diagnostic_event_cursor(&fixture)?;
        let resume_prompt = live_producer_resume_prompt(&prepared, &captured_source, feature)?;
        println!(
            "\n=== Volicord live {host}/{} producer finalization turn ===\nThe authenticated host must use the same registered Agent Connection to finalize the captured receipt into one Run, one Strong Evidence observation, criterion coverage, status, and check-close.\n\n{resume_prompt}\n=== end instruction ===\n",
            feature.as_str()
        );
        let resume_status = fixture.run_authenticated_interactive_host(
            host,
            &executable,
            &resume_prompt,
            &mut result_recorder,
        )?;
        if !resume_status.success() {
            return Err(io::Error::other(format!(
                "the live {host}/{} producer finalization turn exited unsuccessfully with {}",
                feature.as_str(),
                status_text(resume_status)
            ))
            .into());
        }
        assert_live_connection_verified(&fixture, &identity.connection_id)?;
        let host_resume = assert_live_producer_resume_diagnostic(
            &fixture,
            &identity.connection_id,
            &prepared.project_id,
            resume_cursor,
        )?;
        let chain = inspect_live_producer_chain(
            &fixture,
            &prepared,
            &captured_source,
            &receipt,
            &identity.connection_id,
            feature,
            &marker,
        )?;
        let status_output = fixture.run_volicord([
            "status",
            "--repo",
            fixture.repo_arg(),
            "--task",
            &prepared.task_id,
            "--json",
        ])?;
        require_success(
            "volicord status after live producer finalization",
            &status_output,
        )?;
        let observation = LiveUserActionObservation {
            project_id: prepared.project_id.clone(),
            task_id: prepared.task_id.clone(),
            lifecycle_phase: chain.lifecycle_phase.clone(),
            state_version: chain.state_version,
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
        let authority_receipt =
            verify_fresh_authority_receipt(json_stdout(&status_output)?, &observation, &marker)?;
        if authority_receipt.latest_run_id != chain.run_id {
            return Err(io::Error::other(
                "fresh producer AuthorityReceipt does not name the producer-linked Run",
            )
            .into());
        }

        let assertions = LiveProducerAssertionFamilies {
            actual_host_event: true,
            intent_precedes_source: captured_source.intent_precedes_source,
            exact_session_connection_actor_scope_baseline: captured_source
                .exact_session_connection_actor_scope_baseline,
            capture_receipt_bound: receipt.capture_receipt_bound,
            strong_producer_chain: chain.strong_producer_chain,
            criterion_coverage_projected: chain.criterion_coverage_projected,
            negative_rejections_zero_effect: negative.rejected && before_negative == after_negative,
        };
        if !assertions.all_passed() {
            return Err(io::Error::other(
                "live producer cell did not close all seven assertion families",
            )
            .into());
        }
        let summary = live_producer_completed_summary(LiveProducerSummaryInput {
            identity: &identity,
            feature,
            prepared: &prepared,
            source: &captured_source,
            negative: &negative,
            receipt: &receipt,
            host_resume: &host_resume,
            chain: &chain,
            authority_receipt: &authority_receipt,
            assertions: &assertions,
            host_feature_diagnostics: &host_feature_diagnostics,
        });
        validate_live_producer_result_shape(&summary, feature)?;
        result_recorder.record_final(&summary)?;
        Ok(())
    }

    struct PreparedLiveProducerBasis {
        project_id: String,
        task_id: String,
        change_unit_id: String,
        target: EvidenceTarget,
        baseline_ref: String,
        run_marker: String,
    }

    fn prepare_live_producer_authority_basis(
        fixture: &LiveSmokeFixture,
        connection_id: &str,
        marker: &str,
        baseline_ref: &str,
    ) -> Result<PreparedLiveProducerBasis, Box<dyn Error>> {
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
                    "boundary": "Validate one registered live-host evidence producer without Product Repository writes.",
                    "non_goals": [],
                    "acceptance_criteria": [{
                        "statement": "The registered live-host producer supplies current Strong Evidence.",
                        "evidence_requirement": "required"
                    }]
                }
            }),
        )?;
        if intake.response_value["base"]["response_kind"] != "result" {
            return Err(io::Error::other("live producer setup intake was not committed").into());
        }
        let task_id = intake.response_value["task_ref"]["record_id"]
            .as_str()
            .ok_or_else(|| io::Error::other("live producer setup returned no Task id"))?
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
                "baseline_ref": baseline_ref,
                "change_unit": {
                    "operation": "create_current",
                    "scope_summary": "No-write registered live-host evidence producer validation.",
                    "affected_paths": []
                },
                "related_scope_decision_refs": []
            }),
        )?;
        if scope.response_value["base"]["response_kind"] != "result" {
            return Err(
                io::Error::other("live producer setup scope update was not committed").into(),
            );
        }
        let change_unit_id = scope.response_value["state"]["active_change_unit_ref"]["record_id"]
            .as_str()
            .ok_or_else(|| io::Error::other("live producer setup returned no Change Unit id"))?
            .to_owned();
        let criterion_id = scope.response_value["state"]["acceptance_criteria"]
            .as_array()
            .filter(|criteria| criteria.len() == 1)
            .and_then(|criteria| criteria[0]["acceptance_criterion_id"].as_str())
            .ok_or_else(|| {
                io::Error::other("live producer setup did not preserve exactly one criterion")
            })?
            .to_owned();
        Ok(PreparedLiveProducerBasis {
            project_id: live_fixture_project_id(fixture)?,
            task_id,
            change_unit_id,
            target: serde_json::from_value(serde_json::json!({
                "target_kind": "acceptance_criterion",
                "acceptance_criterion_id": criterion_id
            }))?,
            baseline_ref: baseline_ref.to_owned(),
            run_marker: marker.to_owned(),
        })
    }

    fn live_producer_source_prompt(
        prepared: &PreparedLiveProducerBasis,
        feature: HostFeature,
    ) -> Result<String, Box<dyn Error>> {
        let target = canonical_json_string(&prepared.target)?;
        let capture = match feature {
            HostFeature::VerifiedToolProducer => {
                let input_sha256 = canonical_json_bare_sha256(&serde_json::json!({
                    "command": LIVE_VERIFIED_TOOL_COMMAND
                }))?;
                serde_json::json!({
                    "capture_kind": "verified_tool_invocation",
                    "tool_name": LIVE_VERIFIED_TOOL_NAME,
                    "tool_input_sha256": input_sha256,
                    "expected_success": true
                })
            }
            HostFeature::RegisteredConnectionObservation => serde_json::json!({
                "capture_kind": "registered_connection_observation",
                "source_selector": {
                    "source_kind": "guard_event",
                    "event_kind": "stop"
                },
                "expected_complete": true
            }),
            _ => {
                return Err(io::Error::other(
                    "live producer source prompt requires a producer feature",
                )
                .into())
            }
        };
        let capture = canonical_json_string(&capture)?;
        let source_step = match feature {
            HostFeature::VerifiedToolProducer => format!(
                "2. After the committed intent result is returned, invoke the host Bash tool exactly once with the exact input object `{{\"command\":\"{LIVE_VERIFIED_TOOL_COMMAND}\"}}`; do not add description, timeout, or any other input field. This benign command must complete successfully and must not write a Product Repository file."
            ),
            HostFeature::RegisteredConnectionObservation => "2. After the committed intent result is returned, call no further tool. Reply with exactly VOLICORD_LIVE_REGISTERED_CONNECTION_SOURCE and stop so the installed host emits its actual Stop event in this same session.".to_owned(),
            _ => unreachable!("producer feature was validated above"),
        };
        Ok(format!(
            concat!(
                "Use only the MCP server named `volicord` for the Volicord call. {routing} ",
                "Do not edit files, inspect authentication material, print a prompt or transcript, or use a fixture event.\n\n",
                "1. Call `volicord.prepare_evidence_capture` exactly once with `detail=full`, `task_id={task_id}`, `change_unit_id={change_unit_id}`, `baseline_ref={baseline_ref}`, target `{target}`, and capture `{capture}`. Require a committed result and retain only its capture-intent ref.\n",
                "{source_step}\n",
                "3. Report only whether the intent and actual source completed, then stop. Do not print tool input, tool output, a prompt, transcript, URL, token, or credential."
            ),
            routing = live_project_routing_instruction(&prepared.project_id),
            task_id = prepared.task_id,
            change_unit_id = prepared.change_unit_id,
            baseline_ref = prepared.baseline_ref,
            target = target,
            capture = capture,
            source_step = source_step,
        ))
    }

    struct ActualLiveProducerSource {
        capture_intent_id: String,
        capture_input_sha256: String,
        intent_state_version: u64,
        session_id: String,
        guard_installation_id: String,
        source_event_ids: Vec<String>,
        mismatched_event_id: String,
        host_invocation_id: Option<String>,
        intent_precedes_source: bool,
        exact_session_connection_actor_scope_baseline: bool,
    }

    fn inspect_actual_live_producer_source(
        fixture: &LiveSmokeFixture,
        prepared: &PreparedLiveProducerBasis,
        connection_id: &str,
        feature: HostFeature,
    ) -> Result<ActualLiveProducerSource, Box<dyn Error>> {
        let project = list_projects(&fixture.runtime_home_path)?
            .into_iter()
            .find(|project| project.project_id == prepared.project_id)
            .ok_or_else(|| io::Error::other("live producer project registration is missing"))?;
        let conn = open_project_state_database_read_only(&project.state_db_path)?;
        let capture_kind = match feature {
            HostFeature::VerifiedToolProducer => "verified_tool_invocation",
            HostFeature::RegisteredConnectionObservation => "registered_connection_observation",
            _ => return Err(io::Error::other("unsupported live producer feature").into()),
        };
        let rows = conn
            .prepare(
                "SELECT evidence_capture_intent_id, scope_revision, baseline_ref,
                        target_json, capture_spec_json, input_sha256,
                        requested_by_actor_source, requesting_connection_internal_id,
                        session_context_json, workspace_context_json, created_at
                   FROM evidence_capture_intents
                  WHERE project_id = ?1 AND task_id = ?2 AND capture_kind = ?3",
            )?
            .query_map(
                rusqlite::params![prepared.project_id, prepared.task_id, capture_kind],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, u64>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, String>(6)?,
                        row.get::<_, String>(7)?,
                        row.get::<_, String>(8)?,
                        row.get::<_, String>(9)?,
                        row.get::<_, String>(10)?,
                    ))
                },
            )?
            .collect::<Result<Vec<_>, _>>()?;
        let [row] = rows.as_slice() else {
            return Err(io::Error::other(format!(
                "actual host must prepare exactly one {capture_kind} intent; found {}",
                rows.len()
            ))
            .into());
        };
        let (
            capture_intent_id,
            scope_revision,
            baseline_ref,
            target_json,
            capture_spec_json,
            input_sha256,
            requested_by_actor_source,
            requesting_connection_internal_id,
            session_context_json,
            workspace_context_json,
            intent_created_at,
        ) = row;
        let session_context: Value = serde_json::from_str(session_context_json)?;
        let session_id = session_context["session_id"]
            .as_str()
            .ok_or_else(|| io::Error::other("actual producer intent has no exact session id"))?
            .to_owned();
        let managed_digest = session_id.strip_prefix("mhs_").ok_or_else(|| {
            io::Error::other("actual producer intent is not opaque-session bound")
        })?;
        validate_lower_hex("managed host session digest", managed_digest, &[64])?;
        let target: EvidenceTarget = serde_json::from_str(target_json)?;
        let capture_spec: Value = serde_json::from_str(capture_spec_json)?;
        let workspace: Value = serde_json::from_str(workspace_context_json)?;
        let expected_input_sha256 = match feature {
            HostFeature::VerifiedToolProducer => canonical_json_bare_sha256(&serde_json::json!({
                "command": LIVE_VERIFIED_TOOL_COMMAND
            }))?,
            HostFeature::RegisteredConnectionObservation => canonical_json_bare_sha256(
                &serde_json::json!({"source_kind": "guard_event", "event_kind": "stop"}),
            )?,
            _ => unreachable!("producer feature was validated above"),
        };
        let expected_capture = match feature {
            HostFeature::VerifiedToolProducer => {
                capture_spec["tool_name"] == LIVE_VERIFIED_TOOL_NAME
                    && capture_spec["tool_input_sha256"] == expected_input_sha256
                    && capture_spec["expected_success"] == true
            }
            HostFeature::RegisteredConnectionObservation => {
                capture_spec["source_selector"]
                    == serde_json::json!({"source_kind": "guard_event", "event_kind": "stop"})
                    && capture_spec["expected_complete"] == true
            }
            _ => unreachable!("producer feature was validated above"),
        };
        let event_rows = conn
            .prepare(
                "SELECT state_version, actor_source, payload_json
                   FROM authority_events
                  WHERE project_id = ?1 AND task_id = ?2
                    AND event_type = 'evidence_capture_prepared'",
            )?
            .query_map(
                rusqlite::params![prepared.project_id, prepared.task_id],
                |row| {
                    Ok((
                        row.get::<_, u64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )?
            .collect::<Result<Vec<_>, _>>()?;
        let matching_events = event_rows
            .into_iter()
            .filter_map(|(state_version, actor, payload_json)| {
                let payload = serde_json::from_str::<Value>(&payload_json).ok()?;
                (payload["capture_intent_ref"]["record_id"] == *capture_intent_id)
                    .then_some((state_version, actor))
            })
            .collect::<Vec<_>>();
        let [(intent_state_version, authority_actor)] = matching_events.as_slice() else {
            return Err(io::Error::other(
                "actual producer intent has no unique matching authority event",
            )
            .into());
        };
        let expected_actor = format!("agent_connection:{connection_id}");
        let watch_baseline = latest_watch_baseline_for_session(
            &fixture.runtime_home_path,
            &prepared.project_id,
            &session_id,
        )?
        .ok_or_else(|| io::Error::other("actual producer session has no managed watch baseline"))?;
        let watch_metadata: Value = serde_json::from_str(&watch_baseline.metadata_json)?;

        let guard_rows = conn
            .prepare(
                "SELECT guard_event_id, event_kind, decision, subject_json,
                        occurred_at, guard_installation_id
                   FROM guard_events
                  WHERE project_id = ?1 AND connection_internal_id = ?2
                    AND session_id = ?3
                  ORDER BY rowid",
            )?
            .query_map(
                rusqlite::params![prepared.project_id, connection_id, session_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, Option<String>>(5)?,
                    ))
                },
            )?
            .collect::<Result<Vec<_>, _>>()?;
        let mismatched_event_id = match feature {
            HostFeature::VerifiedToolProducer => guard_rows
                .iter()
                .find(|(_, event_kind, _, _, _, _)| event_kind != "pre_tool")
                .map(|row| row.0.clone())
                .ok_or_else(|| io::Error::other("actual tool session has no mismatched event"))?,
            HostFeature::RegisteredConnectionObservation => conn
                .query_row(
                    "SELECT guard_event_id
                       FROM guard_events
                      WHERE project_id = ?1
                        AND connection_internal_id = ?2
                        AND event_kind = 'stop'
                        AND (session_id <> ?3 OR occurred_at < ?4)
                      ORDER BY rowid DESC
                      LIMIT 1",
                    rusqlite::params![
                        prepared.project_id,
                        connection_id,
                        session_id,
                        intent_created_at
                    ],
                    |row| row.get::<_, String>(0),
                )
                .optional()?
                .ok_or_else(|| {
                    io::Error::other(
                        "registered-connection negative capture has no prior actual-host Stop event with a stale or different session binding",
                    )
                })?,
            _ => unreachable!("producer feature was validated above"),
        };
        let (source_event_ids, source_times, source_installation, host_invocation_id) =
            match feature {
                HostFeature::VerifiedToolProducer => {
                    let mut matching = Vec::new();
                    for (event_id, event_kind, decision, subject_json, occurred_at, installation) in
                        &guard_rows
                    {
                        if !matches!(event_kind.as_str(), "pre_tool" | "post_tool") {
                            continue;
                        }
                        let subject: Value = serde_json::from_str(subject_json)?;
                        if subject["tool_input_sha256"] != expected_input_sha256
                            || subject["raw_event"]["tool_name"] != LIVE_VERIFIED_TOOL_NAME
                        {
                            continue;
                        }
                        let invocation = host_event_invocation_id(&subject["raw_event"])?;
                        matching.push((
                            event_id.clone(),
                            event_kind.clone(),
                            decision.clone(),
                            occurred_at.clone(),
                            installation.clone(),
                            invocation,
                        ));
                    }
                    if matching.len() != 2
                        || matching[0].1 != "pre_tool"
                        || matching[1].1 != "post_tool"
                        || matching[0].2 == "deny"
                        || matching[0].5 != matching[1].5
                    {
                        return Err(io::Error::other(
                            "actual host did not emit one allowed exact pre/post Bash event pair",
                        )
                        .into());
                    }
                    (
                        matching.iter().map(|event| event.0.clone()).collect(),
                        matching.iter().map(|event| event.3.clone()).collect(),
                        matching[0].4.clone(),
                        Some(matching[0].5.clone()),
                    )
                }
                HostFeature::RegisteredConnectionObservation => {
                    let matching = guard_rows
                        .iter()
                        .filter(|(_, event_kind, _, _, occurred_at, _)| {
                            event_kind == "stop" && occurred_at >= intent_created_at
                        })
                        .collect::<Vec<_>>();
                    let [event] = matching.as_slice() else {
                        return Err(io::Error::other(format!(
                            "actual host must emit exactly one post-intent Stop event; found {}",
                            matching.len()
                        ))
                        .into());
                    };
                    (
                        vec![event.0.clone()],
                        vec![event.4.clone()],
                        event.5.clone(),
                        None,
                    )
                }
                _ => unreachable!("producer feature was validated above"),
            };
        let guard_installation_id = source_installation.ok_or_else(|| {
            io::Error::other("actual producer source has no registered guard installation")
        })?;
        let intent_precedes_source = source_times
            .iter()
            .all(|occurred_at| occurred_at >= intent_created_at);
        let exact_basis = *scope_revision > 0
            && baseline_ref == &prepared.baseline_ref
            && target == prepared.target
            && input_sha256 == &expected_input_sha256
            && expected_capture
            && requested_by_actor_source == &expected_actor
            && authority_actor == &expected_actor
            && requesting_connection_internal_id == connection_id
            && workspace
                .as_object()
                .is_some_and(|object| !object.is_empty())
            && watch_baseline.session_id == session_id
            && watch_baseline.connection_internal_id == connection_id
            && watch_baseline.status == "active"
            && watch_metadata["launch_origin"] == "managed_host";
        if !intent_precedes_source || !exact_basis {
            return Err(io::Error::other(
                "actual producer intent/source does not preserve exact time, session, connection, actor, scope, baseline, target, and workspace binding",
            )
            .into());
        }
        Ok(ActualLiveProducerSource {
            capture_intent_id: capture_intent_id.clone(),
            capture_input_sha256: input_sha256.clone(),
            intent_state_version: *intent_state_version,
            session_id,
            guard_installation_id,
            source_event_ids,
            mismatched_event_id,
            host_invocation_id,
            intent_precedes_source,
            exact_session_connection_actor_scope_baseline: exact_basis,
        })
    }

    fn host_event_invocation_id(event: &Value) -> Result<String, Box<dyn Error>> {
        for pointer in [
            "/tool_use_id",
            "/tool_invocation_id",
            "/tool_call_id",
            "/invocation_id",
            "/call_id",
            "/tool/id",
            "/tool_use/id",
        ] {
            if let Some(value) = event.pointer(pointer).and_then(Value::as_str) {
                if !value.trim().is_empty() {
                    return Ok(value.trim().to_owned());
                }
            }
        }
        Err(io::Error::other("actual host tool event has no invocation id").into())
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct LiveCaptureDurableSnapshot {
        state_version: u64,
        authority_events: u64,
        capture_receipts: u64,
        staged_artifacts: u64,
        artifacts: u64,
        evidence_observations: u64,
        evidence_producers: u64,
    }

    fn live_capture_durable_snapshot(
        fixture: &LiveSmokeFixture,
        project_id: &str,
    ) -> Result<LiveCaptureDurableSnapshot, Box<dyn Error>> {
        let project = list_projects(&fixture.runtime_home_path)?
            .into_iter()
            .find(|project| project.project_id == project_id)
            .ok_or_else(|| io::Error::other("live producer project registration is missing"))?;
        let conn = open_project_state_database_read_only(&project.state_db_path)?;
        conn.query_row(
            "SELECT ps.state_version,
                    (SELECT COUNT(*) FROM authority_events WHERE project_id = ps.project_id),
                    (SELECT COUNT(*) FROM evidence_capture_receipts WHERE project_id = ps.project_id),
                    (SELECT COUNT(*) FROM artifact_staging WHERE project_id = ps.project_id),
                    (SELECT COUNT(*) FROM artifacts WHERE project_id = ps.project_id),
                    (SELECT COUNT(*) FROM evidence_observations WHERE project_id = ps.project_id),
                    (SELECT COUNT(*) FROM evidence_producers WHERE project_id = ps.project_id)
               FROM project_state ps
              WHERE ps.project_id = ?1",
            [project_id],
            |row| {
                Ok(LiveCaptureDurableSnapshot {
                    state_version: row.get(0)?,
                    authority_events: row.get(1)?,
                    capture_receipts: row.get(2)?,
                    staged_artifacts: row.get(3)?,
                    artifacts: row.get(4)?,
                    evidence_observations: row.get(5)?,
                    evidence_producers: row.get(6)?,
                })
            },
        )
        .map_err(Into::into)
    }

    struct NegativeLiveCapture {
        rejected: bool,
        exit_code: i32,
    }

    fn run_mismatched_live_capture(
        fixture: &LiveSmokeFixture,
        source: &ActualLiveProducerSource,
        feature: HostFeature,
        prepared: &PreparedLiveProducerBasis,
    ) -> Result<NegativeLiveCapture, Box<dyn Error>> {
        let output = match feature {
            HostFeature::VerifiedToolProducer => {
                let [pre_event, post_event] = source.source_event_ids.as_slice() else {
                    return Err(io::Error::other(
                        "verified-tool negative capture requires the exact source pair",
                    )
                    .into());
                };
                fixture.run_volicord([
                    "evidence",
                    "capture-tool",
                    "--intent",
                    &source.capture_intent_id,
                    "--pre-event",
                    post_event,
                    "--post-event",
                    pre_event,
                    "--repo",
                    fixture.repo_arg(),
                    "--json",
                ])?
            }
            HostFeature::RegisteredConnectionObservation => fixture.run_volicord([
                "evidence",
                "capture-connection",
                "--intent",
                &source.capture_intent_id,
                "--guard-event",
                &source.mismatched_event_id,
                "--repo",
                fixture.repo_arg(),
                "--json",
            ])?,
            _ => return Err(io::Error::other("unsupported negative producer capture").into()),
        };
        if output.timed_out || output.output.status.success() {
            return Err(io::Error::other(format!(
                "mismatched {}/{} capture did not fail closed",
                prepared.project_id,
                feature.as_str()
            ))
            .into());
        }
        Ok(NegativeLiveCapture {
            rejected: true,
            exit_code: output.output.status.code().unwrap_or(-1),
        })
    }

    struct LiveCaptureCommandOutput {
        capture_intent_id: String,
        capture_receipt_id: String,
        staged_receipt_handle_id: String,
        complete: bool,
    }

    fn run_exact_live_capture(
        fixture: &LiveSmokeFixture,
        source: &ActualLiveProducerSource,
        feature: HostFeature,
        _prepared: &PreparedLiveProducerBasis,
    ) -> Result<LiveCaptureCommandOutput, Box<dyn Error>> {
        let output = match feature {
            HostFeature::VerifiedToolProducer => {
                let [pre_event, post_event] = source.source_event_ids.as_slice() else {
                    return Err(io::Error::other(
                        "verified-tool capture requires the exact source pair",
                    )
                    .into());
                };
                fixture.run_volicord([
                    "evidence",
                    "capture-tool",
                    "--intent",
                    &source.capture_intent_id,
                    "--pre-event",
                    pre_event,
                    "--post-event",
                    post_event,
                    "--repo",
                    fixture.repo_arg(),
                    "--json",
                ])?
            }
            HostFeature::RegisteredConnectionObservation => {
                let [guard_event] = source.source_event_ids.as_slice() else {
                    return Err(io::Error::other(
                        "registered-connection capture requires one exact source event",
                    )
                    .into());
                };
                fixture.run_volicord([
                    "evidence",
                    "capture-connection",
                    "--intent",
                    &source.capture_intent_id,
                    "--guard-event",
                    guard_event,
                    "--repo",
                    fixture.repo_arg(),
                    "--json",
                ])?
            }
            _ => return Err(io::Error::other("unsupported exact producer capture").into()),
        };
        require_success("exact release-candidate evidence capture", &output)?;
        let value = json_stdout(&output)?;
        let capture_intent_id = required_result_string(&value, "/capture_intent_id")?.to_owned();
        let capture_receipt_id = required_result_string(&value, "/capture_receipt_id")?.to_owned();
        let staged_receipt_handle_id =
            required_result_string(&value, "/staged_receipt_handle_id")?.to_owned();
        let complete = value["complete"] == true;
        if capture_intent_id != source.capture_intent_id || !complete {
            return Err(io::Error::other(
                "exact evidence capture output does not name the complete selected intent",
            )
            .into());
        }
        Ok(LiveCaptureCommandOutput {
            capture_intent_id,
            capture_receipt_id,
            staged_receipt_handle_id,
            complete,
        })
    }

    struct InspectedLiveCaptureReceipt {
        capture_receipt_id: String,
        staged_receipt_handle_id: String,
        safe_receipt_sha256: String,
        result_sha256: String,
        source_claim_count: u64,
        capture_receipt_bound: bool,
    }

    fn inspect_live_capture_receipt(
        fixture: &LiveSmokeFixture,
        prepared: &PreparedLiveProducerBasis,
        source: &ActualLiveProducerSource,
        feature: HostFeature,
        output: &LiveCaptureCommandOutput,
    ) -> Result<InspectedLiveCaptureReceipt, Box<dyn Error>> {
        let project = list_projects(&fixture.runtime_home_path)?
            .into_iter()
            .find(|project| project.project_id == prepared.project_id)
            .ok_or_else(|| io::Error::other("live producer project registration is missing"))?;
        let conn = open_project_state_database_read_only(&project.state_db_path)?;
        let row = conn
            .query_row(
                "SELECT evidence_capture_receipt_id, evidence_capture_intent_id,
                        staging_handle_id, capture_kind, input_sha256, result_sha256,
                        safe_receipt_json, safe_receipt_sha256, safe_receipt_size_bytes,
                        completeness
                   FROM evidence_capture_receipts
                  WHERE project_id = ?1 AND evidence_capture_intent_id = ?2",
                rusqlite::params![prepared.project_id, source.capture_intent_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, String>(6)?,
                        row.get::<_, String>(7)?,
                        row.get::<_, u64>(8)?,
                        row.get::<_, String>(9)?,
                    ))
                },
            )
            .optional()?
            .ok_or_else(|| io::Error::other("exact capture created no durable receipt"))?;
        let (
            receipt_id,
            intent_id,
            staging_handle_id,
            capture_kind,
            input_sha256,
            result_sha256,
            safe_receipt_json,
            safe_receipt_sha256,
            safe_receipt_size_bytes,
            completeness,
        ) = row;
        validate_lower_hex("safe receipt sha256", &safe_receipt_sha256, &[64])?;
        validate_lower_hex("capture result sha256", &result_sha256, &[64])?;
        let body: PersistedEvidenceCaptureReceiptBody = serde_json::from_str(&safe_receipt_json)?;
        let guard_event_ids = body
            .source
            .guard_event_ids
            .iter()
            .map(|value| value.as_str())
            .collect::<Vec<_>>();
        let expected_kind = match feature {
            HostFeature::VerifiedToolProducer => "verified_tool_invocation",
            HostFeature::RegisteredConnectionObservation => "registered_connection_observation",
            _ => return Err(io::Error::other("unsupported producer receipt kind").into()),
        };
        let source_claim_count: u64 = conn.query_row(
            "SELECT COUNT(*)
               FROM evidence_capture_source_claims
              WHERE project_id = ?1 AND evidence_capture_receipt_id = ?2",
            rusqlite::params![prepared.project_id, receipt_id],
            |row| row.get(0),
        )?;
        let expected_claim_count = if feature == HostFeature::VerifiedToolProducer {
            3
        } else {
            1
        };
        let host_invocation_matches = match feature {
            HostFeature::VerifiedToolProducer => {
                body.source.host_invocation_id.as_ref().map(String::as_str)
                    == source.host_invocation_id.as_deref()
            }
            HostFeature::RegisteredConnectionObservation => {
                body.source.host_invocation_id.as_ref().is_none()
            }
            _ => false,
        };
        let bound = output.capture_intent_id == source.capture_intent_id
            && output.capture_receipt_id == receipt_id
            && output.staged_receipt_handle_id == staging_handle_id
            && output.complete
            && intent_id == source.capture_intent_id
            && capture_kind == expected_kind
            && input_sha256 == source.capture_input_sha256
            && completeness == "complete"
            && safe_receipt_size_bytes == u64::try_from(safe_receipt_json.len())?
            && format!("{:x}", Sha256::digest(safe_receipt_json.as_bytes())) == safe_receipt_sha256
            && body.capture_intent_id.as_str() == source.capture_intent_id
            && evidence_producer_kind_text(body.capture_kind) == expected_kind
            && body.input_sha256 == source.capture_input_sha256
            && body.result_sha256 == result_sha256
            && body.complete
            && body.source.connection_id.as_str() == prepared_connection_id(fixture, prepared)?
            && body.source.session_id.as_ref().map(|value| value.as_str())
                == Some(source.session_id.as_str())
            && body
                .source
                .guard_installation_id
                .as_ref()
                .map(|value| value.as_str())
                == Some(source.guard_installation_id.as_str())
            && guard_event_ids.as_slice()
                == source
                    .source_event_ids
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>()
                    .as_slice()
            && body.source.watch_observation_refs.is_empty()
            && host_invocation_matches
            && source_claim_count == expected_claim_count;
        if !bound {
            return Err(io::Error::other(
                "durable capture receipt is not exactly bound to the live intent and source",
            )
            .into());
        }
        Ok(InspectedLiveCaptureReceipt {
            capture_receipt_id: receipt_id,
            staged_receipt_handle_id: staging_handle_id,
            safe_receipt_sha256,
            result_sha256,
            source_claim_count,
            capture_receipt_bound: bound,
        })
    }

    fn prepared_connection_id(
        fixture: &LiveSmokeFixture,
        prepared: &PreparedLiveProducerBasis,
    ) -> Result<String, Box<dyn Error>> {
        let project = list_projects(&fixture.runtime_home_path)?
            .into_iter()
            .find(|project| project.project_id == prepared.project_id)
            .ok_or_else(|| io::Error::other("live producer project registration is missing"))?;
        let conn = open_project_state_database_read_only(&project.state_db_path)?;
        conn.query_row(
            "SELECT requesting_connection_internal_id
               FROM evidence_capture_intents
              WHERE project_id = ?1 AND task_id = ?2",
            rusqlite::params![prepared.project_id, prepared.task_id],
            |row| row.get(0),
        )
        .map_err(Into::into)
    }

    fn evidence_producer_kind_text(kind: EvidenceProducerKind) -> &'static str {
        match kind {
            EvidenceProducerKind::VerifiedToolInvocation => "verified_tool_invocation",
            EvidenceProducerKind::RegisteredConnectionObservation => {
                "registered_connection_observation"
            }
            EvidenceProducerKind::VerifiedCommandExecution => "verified_command_execution",
            EvidenceProducerKind::UnverifiedCaller => "unverified_caller",
            EvidenceProducerKind::UserChannelObservation => "user_channel_observation",
            EvidenceProducerKind::ReusedEvidence => "reused_evidence",
        }
    }

    fn live_producer_resume_prompt(
        prepared: &PreparedLiveProducerBasis,
        source: &ActualLiveProducerSource,
        feature: HostFeature,
    ) -> Result<String, Box<dyn Error>> {
        let target = canonical_json_string(&prepared.target)?;
        let intent_ref = canonical_json_string(&serde_json::json!({
            "record_kind": "evidence_capture_intent",
            "record_id": source.capture_intent_id,
            "project_id": prepared.project_id,
            "task_id": prepared.task_id,
            "produced_at_state_version": source.intent_state_version
        }))?;
        let (source_kind, assurance_level) = match feature {
            HostFeature::VerifiedToolProducer => ("external_tool", "external_tool_result"),
            HostFeature::RegisteredConnectionObservation => {
                ("connection_observation", "registered_connection_observed")
            }
            _ => return Err(io::Error::other("unsupported producer resume feature").into()),
        };
        Ok(format!(
            concat!(
                "Use only the MCP server named `volicord`. {routing} ",
                "Do not edit files, run shell commands, inspect authentication material, or print tool bodies, prompts, transcripts, URLs, tokens, or credentials.\n\n",
                "1. Call `volicord.record_run` exactly once with `detail=full`, `task_id={task_id}`, `change_unit_id={change_unit_id}`, `kind=shaping_update`, `run_id=null`, `baseline_ref={baseline_ref}`, `write_ticket_id=null`, and summary exactly `{marker}`. Set observed changes to `changed_paths=[]`, `product_file_write_observed=false`, `sensitive_categories=[]`, and the same baseline. Set `artifact_inputs=[]`. Set exactly one evidence observation with target `{target}`, `source_kind={source_kind}`, `assurance_level={assurance_level}`, null observer/tool fields, empty tool metadata/source refs/output artifact refs/limitations, `input_refs=[{intent_ref}]`, and `observed_at={caller_observed_at}`. Set exactly one supported evidence update for the same target with all caller-supplied supporting/gap ref arrays empty. Set a non-null close assessment with result summary exactly `{marker}` and empty result refs, residual risks, sensitive categories, and recovery constraints. Require a committed result with exactly one producer, one registered receipt artifact, and one evidence observation; stop on any mismatch.\n",
                "2. Call `volicord.status` exactly once for Task `{task_id}` with `detail=full`. Require the producer-linked Run to be latest, evidence coverage supported/sufficient, close state ready, and zero close blockers.\n",
                "3. Call `volicord.check_close` exactly once for Task `{task_id}`. Require `close_state=ready`, zero blockers, the same state version, and the same evidence gate. Report only IDs, counts, digests, and booleans, then stop."
            ),
            routing = live_project_routing_instruction(&prepared.project_id),
            task_id = prepared.task_id,
            change_unit_id = prepared.change_unit_id,
            baseline_ref = prepared.baseline_ref,
            marker = prepared.run_marker,
            target = target,
            source_kind = source_kind,
            assurance_level = assurance_level,
            intent_ref = intent_ref,
            caller_observed_at = LIVE_PRODUCER_CALLER_OBSERVED_AT,
        ))
    }

    struct LiveProducerHostResume {
        record_run_calls: u64,
        status_calls: u64,
        check_close_calls: u64,
        ordered: bool,
    }

    fn assert_live_producer_resume_diagnostic(
        fixture: &LiveSmokeFixture,
        connection_id: &str,
        project_id: &str,
        cursor: DiagnosticEventCursor,
    ) -> Result<LiveProducerHostResume, Box<dyn Error>> {
        let conn = rusqlite::Connection::open_with_flags(
            diagnostics_db_path(&fixture.runtime_home_path),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )?;
        let observed = conn.query_row(
            "SELECT
                 COALESCE(SUM(CASE
                   WHEN e.tool_name = 'volicord.record_run'
                    AND e.core_committed = 1 AND e.outcome = 'success'
                   THEN 1 ELSE 0 END), 0),
                 COALESCE(SUM(CASE
                   WHEN e.tool_name = 'volicord.status'
                    AND e.core_committed = 0 AND e.outcome = 'success'
                   THEN 1 ELSE 0 END), 0),
                 COALESCE(SUM(CASE
                   WHEN e.tool_name = 'volicord.check_close'
                    AND e.core_committed = 0 AND e.outcome = 'success'
                   THEN 1 ELSE 0 END), 0),
                 MIN(CASE WHEN e.tool_name = 'volicord.record_run' THEN e.event_id END),
                 MIN(CASE WHEN e.tool_name = 'volicord.status' THEN e.event_id END),
                 MIN(CASE WHEN e.tool_name = 'volicord.check_close' THEN e.event_id END)
               FROM diagnostic_sessions s
               JOIN diagnostic_events e ON e.session_id = s.session_id
              WHERE s.connection_id = ?1 AND s.project_id = ?2 AND e.event_id > ?3",
            rusqlite::params![connection_id, project_id, cursor.0],
            |row| {
                Ok((
                    row.get::<_, u64>(0)?,
                    row.get::<_, u64>(1)?,
                    row.get::<_, u64>(2)?,
                    row.get::<_, Option<i64>>(3)?,
                    row.get::<_, Option<i64>>(4)?,
                    row.get::<_, Option<i64>>(5)?,
                ))
            },
        )?;
        let ordered = observed.3.zip(observed.4).zip(observed.5).is_some_and(
            |((record_run, status), check_close)| record_run < status && status < check_close,
        );
        if observed.0 != 1 || observed.1 != 1 || observed.2 != 1 || !ordered {
            return Err(io::Error::other(format!(
                "actual producer resume diagnostics are not exact and ordered: record_run={}, status={}, check_close={}, ordered={ordered}",
                observed.0, observed.1, observed.2
            ))
            .into());
        }
        Ok(LiveProducerHostResume {
            record_run_calls: observed.0,
            status_calls: observed.1,
            check_close_calls: observed.2,
            ordered,
        })
    }

    struct InspectedLiveProducerChain {
        producer_id: String,
        artifact_id: String,
        evidence_observation_id: String,
        run_id: String,
        state_version: u64,
        lifecycle_phase: String,
        strong_producer_chain: bool,
        criterion_coverage_projected: bool,
    }

    fn inspect_live_producer_chain(
        fixture: &LiveSmokeFixture,
        prepared: &PreparedLiveProducerBasis,
        source: &ActualLiveProducerSource,
        receipt: &InspectedLiveCaptureReceipt,
        connection_id: &str,
        feature: HostFeature,
        marker: &str,
    ) -> Result<InspectedLiveProducerChain, Box<dyn Error>> {
        let project = list_projects(&fixture.runtime_home_path)?
            .into_iter()
            .find(|project| project.project_id == prepared.project_id)
            .ok_or_else(|| io::Error::other("live producer project registration is missing"))?;
        let conn = open_project_state_database_read_only(&project.state_db_path)?;
        let producer_rows = conn
            .prepare(
                "SELECT p.evidence_producer_id, p.evidence_observation_id, p.artifact_id,
                        p.run_id, p.scope_revision, p.baseline_ref, p.producer_kind,
                        p.canonical_producer_json,
                        o.source_kind, o.assurance_level, o.input_refs_json,
                        o.output_artifact_refs_json, o.metadata_json,
                        r.kind, r.status, r.summary_json, r.observed_changes_json,
                        r.created_by_actor_source,
                        a.integrity_status, a.availability, a.redaction_state
                   FROM evidence_producers p
                   JOIN evidence_observations o
                     ON o.project_id = p.project_id
                    AND o.evidence_observation_id = p.evidence_observation_id
                   JOIN runs r ON r.project_id = p.project_id AND r.run_id = p.run_id
                   JOIN artifacts a ON a.project_id = p.project_id AND a.artifact_id = p.artifact_id
                  WHERE p.project_id = ?1 AND p.evidence_capture_intent_id = ?2",
            )?
            .query_map(
                rusqlite::params![prepared.project_id, source.capture_intent_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, u64>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, String>(6)?,
                        row.get::<_, String>(7)?,
                        row.get::<_, String>(8)?,
                        row.get::<_, String>(9)?,
                        row.get::<_, String>(10)?,
                        row.get::<_, String>(11)?,
                        row.get::<_, String>(12)?,
                        row.get::<_, String>(13)?,
                        row.get::<_, String>(14)?,
                        row.get::<_, String>(15)?,
                        row.get::<_, String>(16)?,
                        row.get::<_, String>(17)?,
                        row.get::<_, String>(18)?,
                        row.get::<_, String>(19)?,
                        row.get::<_, String>(20)?,
                    ))
                },
            )?
            .collect::<Result<Vec<_>, _>>()?;
        let [row] = producer_rows.as_slice() else {
            return Err(io::Error::other(format!(
                "producer finalization must create exactly one chain; found {}",
                producer_rows.len()
            ))
            .into());
        };
        let (
            producer_id,
            evidence_observation_id,
            artifact_id,
            run_id,
            scope_revision,
            baseline_ref,
            producer_kind,
            canonical_producer_json,
            source_kind,
            assurance_level,
            input_refs_json,
            output_artifact_refs_json,
            observation_metadata_json,
            run_kind,
            run_status,
            run_summary_json,
            observed_changes_json,
            created_by_actor_source,
            artifact_integrity,
            artifact_availability,
            artifact_redaction,
        ) = row;
        let producer: EvidenceProducer = serde_json::from_str(canonical_producer_json)?;
        let input_refs: Vec<StateRecordRef> = serde_json::from_str(input_refs_json)?;
        let output_artifact_refs: Vec<ArtifactRef> =
            serde_json::from_str(output_artifact_refs_json)?;
        let authority: PersistedEvidenceObservationAuthority =
            serde_json::from_str(observation_metadata_json)?;
        let run = live_run_observation(
            run_id,
            run_kind,
            run_summary_json,
            observed_changes_json,
            created_by_actor_source,
        )?;
        let expected_actor = format!("agent_connection:{connection_id}");
        let expected_kind = match feature {
            HostFeature::VerifiedToolProducer => EvidenceProducerKind::VerifiedToolInvocation,
            HostFeature::RegisteredConnectionObservation => {
                EvidenceProducerKind::RegisteredConnectionObservation
            }
            _ => return Err(io::Error::other("unsupported producer chain feature").into()),
        };
        let expected_kind_text = evidence_producer_kind_text(expected_kind);
        let expected_source = match feature {
            HostFeature::VerifiedToolProducer => ("external_tool", "external_tool_result"),
            HostFeature::RegisteredConnectionObservation => {
                ("connection_observation", "registered_connection_observed")
            }
            _ => unreachable!("producer feature was validated above"),
        };
        let expected_verification_basis = match feature {
            HostFeature::VerifiedToolProducer => "registered_guard_exact_invocation_v1",
            HostFeature::RegisteredConnectionObservation => "registered_connection_observation_v1",
            _ => unreachable!("producer feature was validated above"),
        };
        let [intent_ref] = input_refs.as_slice() else {
            return Err(io::Error::other(
                "producer observation does not contain one exact capture-intent ref",
            )
            .into());
        };
        let [receipt_artifact] = producer.receipt_artifact_refs.as_slice() else {
            return Err(io::Error::other(
                "producer does not contain one exact receipt artifact ref",
            )
            .into());
        };
        let producer_ref = authority
            .producer_anchor
            .producer_ref
            .as_ref()
            .ok_or_else(|| io::Error::other("producer observation has no producer ref"))?;
        let strong = producer_id == producer.evidence_producer_id.as_str()
            && evidence_observation_id == producer.observation_ref.record_id.as_str()
            && artifact_id == receipt_artifact.artifact_id.as_str()
            && run_id == producer.run_ref.record_id.as_str()
            && *scope_revision == producer.scope_revision
            && baseline_ref == producer.baseline_ref.as_str()
            && producer_kind == expected_kind_text
            && producer.producer_kind == expected_kind
            && producer.capture_intent_id.as_str() == source.capture_intent_id
            && producer.capture_receipt_id.as_str() == receipt.capture_receipt_id
            && producer.project_id.as_str() == prepared.project_id
            && producer.task_id.as_str() == prepared.task_id
            && producer.change_unit_id.as_str() == prepared.change_unit_id
            && producer.baseline_ref.as_str() == prepared.baseline_ref
            && producer.target == prepared.target
            && producer.input_sha256 == source.capture_input_sha256
            && producer.result_sha256 == receipt.result_sha256
            && producer.connection_id.as_str() == connection_id
            && producer.session_id.as_ref().map(|value| value.as_str())
                == Some(source.session_id.as_str())
            && producer.complete
            && producer.observed_by_actor_source.to_canonical_string() == expected_actor
            && source_kind == expected_source.0
            && assurance_level == expected_source.1
            && intent_ref.record_kind == StateRecordKind::EvidenceCaptureIntent
            && intent_ref.record_id.as_str() == source.capture_intent_id
            && intent_ref.produced_at_state_version.as_ref() == Some(&source.intent_state_version)
            && output_artifact_refs.as_slice() == [receipt_artifact.clone()]
            && authority.recorded_by_run_id.as_str() == run_id
            && authority.invocation_verification_basis
                == VERIFICATION_BASIS_MCP_STDIO_CONNECTION_BINDING
            && authority.producer_anchor.producer_kind == expected_kind
            && producer_ref.record_kind == StateRecordKind::EvidenceProducer
            && producer_ref.record_id.as_str() == producer_id
            && authority
                .producer_anchor
                .verification_basis
                .as_ref()
                .map(String::as_str)
                == Some(expected_verification_basis)
            && authority.relevance_assessment.status == EvidenceRelevanceStatus::Unassessed
            && run.kind == "shaping_update"
            && run_status == "recorded"
            && run.summary == marker
            && run.created_by_actor_source == expected_actor
            && !run.product_file_write_observed
            && run.changed_paths.is_empty()
            && artifact_integrity == "verified"
            && artifact_availability == "available"
            && artifact_redaction == "redacted";

        let mut summaries = conn.prepare(
            "SELECT status, coverage_json, metadata_json
               FROM evidence_summaries
              WHERE project_id = ?1 AND task_id = ?2",
        )?;
        let coverage_rows = summaries
            .query_map(
                rusqlite::params![prepared.project_id, prepared.task_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )?
            .collect::<Result<Vec<_>, _>>()?;
        let matching_coverage = coverage_rows
            .into_iter()
            .filter_map(|(status, coverage_json, metadata_json)| {
                let metadata =
                    serde_json::from_str::<PersistedEvidenceMetadata>(&metadata_json).ok()?;
                (metadata.updated_by_run_id.as_str() == run_id).then_some((status, coverage_json))
            })
            .collect::<Vec<_>>();
        let [(coverage_status, coverage_json)] = matching_coverage.as_slice() else {
            return Err(io::Error::other(
                "producer-linked Run has no unique current evidence summary",
            )
            .into());
        };
        let coverage_items: Vec<EvidenceCoverageItem> = serde_json::from_str(coverage_json)?;
        let [coverage] = coverage_items.as_slice() else {
            return Err(io::Error::other(
                "producer-linked evidence summary has no sole criterion coverage item",
            )
            .into());
        };
        let criterion_coverage = coverage_status == "sufficient"
            && coverage.target == prepared.target
            && coverage.coverage_state == EvidenceCoverageState::Supported
            && coverage.supporting_run_refs.len() == 1
            && coverage.supporting_run_refs[0].record_id.as_str() == run_id
            && coverage.observation_refs.len() == 1
            && coverage.observation_refs[0].record_id.as_str() == evidence_observation_id
            && coverage.supporting_artifact_refs.as_slice() == [receipt_artifact.clone()]
            && coverage.gap_refs.is_empty();
        let (state_version, lifecycle_phase): (u64, String) = conn.query_row(
            "SELECT ps.state_version, t.lifecycle_phase
               FROM project_state ps
               JOIN tasks t ON t.project_id = ps.project_id
              WHERE ps.project_id = ?1 AND t.task_id = ?2",
            rusqlite::params![prepared.project_id, prepared.task_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        if !strong || !criterion_coverage {
            return Err(io::Error::other(
                "producer finalization is not a complete Strong Evidence chain with criterion coverage",
            )
            .into());
        }
        Ok(InspectedLiveProducerChain {
            producer_id: producer_id.clone(),
            artifact_id: artifact_id.clone(),
            evidence_observation_id: evidence_observation_id.clone(),
            run_id: run_id.clone(),
            state_version,
            lifecycle_phase,
            strong_producer_chain: strong,
            criterion_coverage_projected: criterion_coverage,
        })
    }

    struct LiveProducerAssertionFamilies {
        actual_host_event: bool,
        intent_precedes_source: bool,
        exact_session_connection_actor_scope_baseline: bool,
        capture_receipt_bound: bool,
        strong_producer_chain: bool,
        criterion_coverage_projected: bool,
        negative_rejections_zero_effect: bool,
    }

    impl LiveProducerAssertionFamilies {
        fn all_passed(&self) -> bool {
            self.actual_host_event
                && self.intent_precedes_source
                && self.exact_session_connection_actor_scope_baseline
                && self.capture_receipt_bound
                && self.strong_producer_chain
                && self.criterion_coverage_projected
                && self.negative_rejections_zero_effect
        }
    }

    struct LiveProducerSummaryInput<'a> {
        identity: &'a LiveHostIdentity,
        feature: HostFeature,
        prepared: &'a PreparedLiveProducerBasis,
        source: &'a ActualLiveProducerSource,
        negative: &'a NegativeLiveCapture,
        receipt: &'a InspectedLiveCaptureReceipt,
        host_resume: &'a LiveProducerHostResume,
        chain: &'a InspectedLiveProducerChain,
        authority_receipt: &'a VerifiedLiveReceipt,
        assertions: &'a LiveProducerAssertionFamilies,
        host_feature_diagnostics: &'a ReleaseHostFeatureDiagnostics,
    }

    fn live_producer_completed_summary(input: LiveProducerSummaryInput<'_>) -> Value {
        let LiveProducerSummaryInput {
            identity,
            feature,
            prepared,
            source,
            negative,
            receipt,
            host_resume,
            chain,
            authority_receipt,
            assertions,
            host_feature_diagnostics,
        } = input;
        let result_kind = match feature {
            HostFeature::VerifiedToolProducer => LIVE_VERIFIED_TOOL_PRODUCER_RESULT_KIND,
            HostFeature::RegisteredConnectionObservation => {
                LIVE_REGISTERED_CONNECTION_OBSERVATION_RESULT_KIND
            }
            _ => unreachable!("producer summary requires a producer feature"),
        };
        serde_json::json!({
            "kind": result_kind,
            "result": if assertions.all_passed() { "passed" } else { "failed" },
            "host": {
                "kind": identity.host,
                "version": identity.host_version,
                "executable_sha256": identity.host_executable_sha256
            },
            "volicord": { "build_id": identity.volicord_build_id },
            "connection": { "connection_id": identity.connection_id },
            "host_feature_support": host_feature_diagnostics.host_feature_support,
            "final_output_authority_disclosure": host_feature_diagnostics.final_output_authority_disclosure,
            "actual_host_event": {
                "observed": true,
                "source_event_count": source.source_event_ids.len(),
                "source_event_ids": source.source_event_ids,
                "opaque_session_id": source.session_id,
                "guard_installation_id": source.guard_installation_id,
                "host_invocation_id": source.host_invocation_id
            },
            "capture_intent": {
                "capture_intent_id": source.capture_intent_id,
                "input_sha256": source.capture_input_sha256,
                "intent_state_version": source.intent_state_version,
                "task_id": prepared.task_id,
                "change_unit_id": prepared.change_unit_id,
                "baseline_ref": prepared.baseline_ref,
                "intent_precedes_source": source.intent_precedes_source,
                "exact_session_connection_actor_scope_baseline": source.exact_session_connection_actor_scope_baseline
            },
            "negative_capture": {
                "rejected": negative.rejected,
                "nonzero_exit": negative.exit_code != 0,
                "receipt_delta": 0,
                "staging_delta": 0,
                "producer_delta": 0,
                "artifact_delta": 0,
                "authority_event_delta": 0,
                "state_version_unchanged": true
            },
            "capture_receipt": {
                "capture_receipt_id": receipt.capture_receipt_id,
                "staged_receipt_handle_id": receipt.staged_receipt_handle_id,
                "safe_receipt_sha256": receipt.safe_receipt_sha256,
                "result_sha256": receipt.result_sha256,
                "source_claim_count": receipt.source_claim_count,
                "bound": receipt.capture_receipt_bound
            },
            "host_resume": {
                "record_run_calls": host_resume.record_run_calls,
                "status_calls": host_resume.status_calls,
                "check_close_calls": host_resume.check_close_calls,
                "ordered": host_resume.ordered,
                "same_connection": true
            },
            "producer_chain": {
                "producer_id": chain.producer_id,
                "artifact_id": chain.artifact_id,
                "evidence_observation_id": chain.evidence_observation_id,
                "run_id": chain.run_id,
                "strong": chain.strong_producer_chain,
                "criterion_coverage_projected": chain.criterion_coverage_projected
            },
            "close": {
                "project_id": authority_receipt.project_id,
                "task_id": authority_receipt.task_id,
                "state_version": authority_receipt.state_version,
                "latest_run_id": authority_receipt.latest_run_id,
                "ready": authority_receipt.close_state == StatusCloseState::Ready,
                "blocker_count": authority_receipt.close_blocker_count
            },
            "assertions": {
                "actual_host_event": assertions.actual_host_event,
                "intent_precedes_source": assertions.intent_precedes_source,
                "exact_session_connection_actor_scope_baseline": assertions.exact_session_connection_actor_scope_baseline,
                "capture_receipt_bound": assertions.capture_receipt_bound,
                "strong_producer_chain": assertions.strong_producer_chain,
                "criterion_coverage_projected": assertions.criterion_coverage_projected,
                "negative_rejections_zero_effect": assertions.negative_rejections_zero_effect
            },
            "sensitive_payloads": {
                "prompt_recorded": false,
                "tool_input_recorded": false,
                "tool_output_recorded": false,
                "transcript_recorded": false,
                "token_recorded": false,
                "url_recorded": false,
                "native_session_id_recorded": false
            }
        })
    }

    fn validate_live_producer_result_shape(
        value: &Value,
        feature: HostFeature,
    ) -> Result<(), Box<dyn Error>> {
        validate_release_host_feature_diagnostics(
            value,
            Some(IntegrationProfile::Detective),
            true,
            true,
        )?;
        let expected_kind = match feature {
            HostFeature::VerifiedToolProducer => LIVE_VERIFIED_TOOL_PRODUCER_RESULT_KIND,
            HostFeature::RegisteredConnectionObservation => {
                LIVE_REGISTERED_CONNECTION_OBSERVATION_RESULT_KIND
            }
            _ => {
                return Err(
                    io::Error::other("producer result validator got non-producer feature").into(),
                )
            }
        };
        for (pointer, keys) in [
            (
                "",
                &[
                    "kind",
                    "result",
                    "host",
                    "volicord",
                    "connection",
                    "host_feature_support",
                    "final_output_authority_disclosure",
                    "actual_host_event",
                    "capture_intent",
                    "negative_capture",
                    "capture_receipt",
                    "host_resume",
                    "producer_chain",
                    "close",
                    "assertions",
                    "sensitive_payloads",
                ][..],
            ),
            ("/host", &["kind", "version", "executable_sha256"][..]),
            ("/volicord", &["build_id"][..]),
            ("/connection", &["connection_id"][..]),
            (
                "/actual_host_event",
                &[
                    "observed",
                    "source_event_count",
                    "source_event_ids",
                    "opaque_session_id",
                    "guard_installation_id",
                    "host_invocation_id",
                ][..],
            ),
            (
                "/capture_intent",
                &[
                    "capture_intent_id",
                    "input_sha256",
                    "intent_state_version",
                    "task_id",
                    "change_unit_id",
                    "baseline_ref",
                    "intent_precedes_source",
                    "exact_session_connection_actor_scope_baseline",
                ][..],
            ),
            (
                "/negative_capture",
                &[
                    "rejected",
                    "nonzero_exit",
                    "receipt_delta",
                    "staging_delta",
                    "producer_delta",
                    "artifact_delta",
                    "authority_event_delta",
                    "state_version_unchanged",
                ][..],
            ),
            (
                "/capture_receipt",
                &[
                    "capture_receipt_id",
                    "staged_receipt_handle_id",
                    "safe_receipt_sha256",
                    "result_sha256",
                    "source_claim_count",
                    "bound",
                ][..],
            ),
            (
                "/host_resume",
                &[
                    "record_run_calls",
                    "status_calls",
                    "check_close_calls",
                    "ordered",
                    "same_connection",
                ][..],
            ),
            (
                "/producer_chain",
                &[
                    "producer_id",
                    "artifact_id",
                    "evidence_observation_id",
                    "run_id",
                    "strong",
                    "criterion_coverage_projected",
                ][..],
            ),
            (
                "/close",
                &[
                    "project_id",
                    "task_id",
                    "state_version",
                    "latest_run_id",
                    "ready",
                    "blocker_count",
                ][..],
            ),
            (
                "/assertions",
                &[
                    "actual_host_event",
                    "intent_precedes_source",
                    "exact_session_connection_actor_scope_baseline",
                    "capture_receipt_bound",
                    "strong_producer_chain",
                    "criterion_coverage_projected",
                    "negative_rejections_zero_effect",
                ][..],
            ),
            (
                "/sensitive_payloads",
                &[
                    "prompt_recorded",
                    "tool_input_recorded",
                    "tool_output_recorded",
                    "transcript_recorded",
                    "token_recorded",
                    "url_recorded",
                    "native_session_id_recorded",
                ][..],
            ),
        ] {
            require_exact_live_evidence_result_keys(value, pointer, keys)?;
        }
        if value["kind"] != expected_kind || value["result"] != "passed" {
            return Err(
                io::Error::other("producer result has wrong kind or terminal status").into(),
            );
        }
        validate_lower_hex(
            "producer host executable digest",
            required_result_string(value, "/host/executable_sha256")?,
            &[64],
        )?;
        required_result_string(value, "/host/kind")?;
        required_result_string(value, "/host/version")?;
        required_result_string(value, "/volicord/build_id")?;
        validate_lower_hex(
            "producer capture input digest",
            required_result_string(value, "/capture_intent/input_sha256")?,
            &[64],
        )?;
        for pointer in [
            "/capture_receipt/safe_receipt_sha256",
            "/capture_receipt/result_sha256",
        ] {
            validate_lower_hex(
                "producer receipt digest",
                required_result_string(value, pointer)?,
                &[64],
            )?;
        }
        let session_id = required_result_string(value, "/actual_host_event/opaque_session_id")?;
        validate_managed_summary_session_id("producer opaque session id", session_id)?;
        let source_ids = value["actual_host_event"]["source_event_ids"]
            .as_array()
            .ok_or_else(|| io::Error::other("producer result has no source event id array"))?;
        for source_id in source_ids {
            validate_domain_separated_correlation_id(
                "producer source event id",
                source_id
                    .as_str()
                    .ok_or_else(|| io::Error::other("producer source event id must be a string"))?,
                "guard_event",
            )?;
        }
        let unique_source_ids = source_ids
            .iter()
            .filter_map(Value::as_str)
            .collect::<BTreeSet<_>>();
        let expected_source_count = if feature == HostFeature::VerifiedToolProducer {
            2
        } else {
            1
        };
        let expected_claim_count = if feature == HostFeature::VerifiedToolProducer {
            3
        } else {
            1
        };
        let assertions = value["assertions"]
            .as_object()
            .ok_or_else(|| io::Error::other("producer assertions are not an object"))?;
        let all_assertions = assertions.values().all(|assertion| assertion == true);
        let negative = &value["negative_capture"];
        let sensitive = value["sensitive_payloads"]
            .as_object()
            .ok_or_else(|| io::Error::other("producer sensitive flags are not an object"))?;
        let all_sensitive_false = sensitive.values().all(|flag| flag == false);
        let connection_id = required_result_string(value, "/connection/connection_id")?;
        for pointer in [
            "/actual_host_event/guard_installation_id",
            "/capture_intent/capture_intent_id",
            "/capture_intent/task_id",
            "/capture_intent/change_unit_id",
            "/capture_intent/baseline_ref",
            "/capture_receipt/capture_receipt_id",
            "/capture_receipt/staged_receipt_handle_id",
            "/producer_chain/producer_id",
            "/producer_chain/artifact_id",
            "/producer_chain/evidence_observation_id",
            "/producer_chain/run_id",
            "/close/project_id",
            "/close/task_id",
            "/close/latest_run_id",
        ] {
            required_result_string(value, pointer)?;
        }
        let intent_state_version =
            required_result_u64(value, "/capture_intent/intent_state_version")?;
        let close_state_version = required_result_u64(value, "/close/state_version")?;
        let host_invocation_exact = match feature {
            HostFeature::VerifiedToolProducer => {
                validate_domain_separated_correlation_id(
                    "producer host invocation id",
                    value["actual_host_event"]["host_invocation_id"]
                        .as_str()
                        .ok_or_else(|| {
                            io::Error::other("producer host invocation id must be a string")
                        })?,
                    "managed_native_id",
                )?;
                true
            }
            HostFeature::RegisteredConnectionObservation => {
                value["actual_host_event"]["host_invocation_id"].is_null()
            }
            _ => unreachable!("producer feature was validated above"),
        };
        if !all_assertions
            || !all_sensitive_false
            || !host_invocation_exact
            || value["actual_host_event"]["observed"] != true
            || value["actual_host_event"]["source_event_count"] != expected_source_count
            || source_ids.len() != expected_source_count
            || source_ids
                .iter()
                .any(|id| id.as_str().is_none_or(str::is_empty))
            || unique_source_ids.len() != source_ids.len()
            || intent_state_version == 0
            || close_state_version <= intent_state_version
            || value["capture_intent"]["intent_precedes_source"] != true
            || value["capture_intent"]["exact_session_connection_actor_scope_baseline"] != true
            || negative["rejected"] != true
            || negative["nonzero_exit"] != true
            || negative["receipt_delta"] != 0
            || negative["staging_delta"] != 0
            || negative["producer_delta"] != 0
            || negative["artifact_delta"] != 0
            || negative["authority_event_delta"] != 0
            || negative["state_version_unchanged"] != true
            || value["capture_receipt"]["bound"] != true
            || value["capture_receipt"]["source_claim_count"] != expected_claim_count
            || value["host_resume"]["record_run_calls"] != 1
            || value["host_resume"]["status_calls"] != 1
            || value["host_resume"]["check_close_calls"] != 1
            || value["host_resume"]["ordered"] != true
            || value["host_resume"]["same_connection"] != true
            || value["producer_chain"]["strong"] != true
            || value["producer_chain"]["criterion_coverage_projected"] != true
            || value["close"]["project_id"]
                .as_str()
                .is_none_or(str::is_empty)
            || value["close"]["task_id"] != value["capture_intent"]["task_id"]
            || value["close"]["latest_run_id"] != value["producer_chain"]["run_id"]
            || value["close"]["ready"] != true
            || value["close"]["blocker_count"] != 0
            || connection_id.is_empty()
        {
            return Err(io::Error::other(
                "passing producer result does not close all seven semantic assertion families",
            )
            .into());
        }
        reject_forbidden_live_producer_fields(value)?;
        if serialize_live_host_result(value)?.len() >= MAX_LIVE_HOST_RESULT_BYTES {
            return Err(
                io::Error::other("producer result exceeds its bounded summary budget").into(),
            );
        }
        Ok(())
    }

    fn reject_forbidden_live_producer_fields(value: &Value) -> Result<(), Box<dyn Error>> {
        fn walk(value: &Value) -> Result<(), io::Error> {
            match value {
                Value::Object(object) => {
                    for (key, value) in object {
                        let normalized = key.to_ascii_lowercase();
                        if matches!(
                            normalized.as_str(),
                            "prompt"
                                | "tool_input"
                                | "tool_output"
                                | "tool_response"
                                | "command"
                                | "transcript"
                                | "transcript_path"
                                | "token"
                                | "url"
                                | "native_session_id"
                        ) {
                            return Err(io::Error::other(format!(
                                "producer result contains forbidden payload field {key:?}"
                            )));
                        }
                        walk(value)?;
                    }
                }
                Value::Array(values) => {
                    for value in values {
                        walk(value)?;
                    }
                }
                Value::String(text) => {
                    let normalized = text.to_ascii_lowercase();
                    if normalized.contains("https://")
                        || normalized.contains("http://")
                        || normalized.contains("bearer ")
                    {
                        return Err(io::Error::other(
                            "producer result contains forbidden URL or bearer payload text",
                        ));
                    }
                }
                Value::Null | Value::Bool(_) | Value::Number(_) => {}
            }
            Ok(())
        }
        walk(value).map_err(Into::into)
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
        let mut result_recorder = LiveResultRecorder::from_env_for_kind_and_profile(
            &recorder_host,
            host,
            LIVE_FINAL_OUTPUT_RESULT_KIND,
            Some(profile),
        )?;
        let release_candidate = result_recorder.release_candidate()?.clone();
        let fixture = LiveSmokeFixture::new_with_release_candidate_for_recorder(
            &recorder_host,
            &release_candidate,
            &mut result_recorder,
        )?;
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
        result_recorder.mark_installed_host_detected();
        let ObservedReleaseHostIdentity {
            host_version,
            host_executable_sha256,
            volicord_build_id,
        } = fixture.observe_and_bind_installed_host_identity(
            &mut result_recorder,
            executable_name,
            &executable,
        )?;
        let maintained_host_kind = match host {
            "codex" => HostKind::Codex,
            "claude-code" => HostKind::ClaudeCode,
            _ => return Err(io::Error::other("unsupported managed final-output host").into()),
        };
        let feature = match profile {
            IntegrationProfile::Record => HostFeature::RecordFinalOutput,
            IntegrationProfile::Detective => HostFeature::DetectiveFinalOutput,
        };
        if host_feature_implementation_for_version(
            maintained_host_kind,
            Some(&host_version),
            feature,
        ) == HostFeatureImplementation::UnsupportedByHost
        {
            let summary = final_output_unavailable_summary_with_host_identity(
                host,
                profile,
                "the maintained host capability matrix statically marks this final-output feature unsupported",
                &host_version,
                &host_executable_sha256,
                &volicord_build_id,
            );
            validate_final_output_result_shape(&summary, profile)?;
            result_recorder.record_final(&summary)?;
            return Ok(());
        }
        if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
            let reason = "authenticated live final-output validation requires interactive terminal stdin and stdout";
            let summary = final_output_unavailable_summary_with_host_identity(
                host,
                profile,
                reason,
                &host_version,
                &host_executable_sha256,
                &volicord_build_id,
            );
            validate_final_output_result_shape(&summary, profile)?;
            result_recorder.record_final(&summary)?;
            return Err(io::Error::other(reason).into());
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
        assert_success("volicord init for live final-output smoke", &init);
        let init_json = json_stdout(&init)?;
        assert_live_init_reported_action_required(
            &init_json,
            host,
            Some(&host_version),
            profile,
            expected_host_action,
        );
        let connection_id = bounded_identity(
            "Agent Connection id",
            init_json["connection"]["connection_id"]
                .as_str()
                .ok_or_else(|| io::Error::other("init result has no Agent Connection id"))?,
            MAX_CONNECTION_ID_BYTES,
        )?;
        let identity = LiveHostIdentity {
            host: host.to_owned(),
            host_version,
            host_executable_sha256,
            volicord_build_id,
            connection_id,
        };
        let project_id = live_fixture_project_id(&fixture)?;
        let config_fixture = verify_final_output_config_fixture(
            &fixture,
            host,
            Some(&identity.host_version),
            profile,
            &init_json,
        )?;

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
        let first_direct = fixture.run_generated_final_output_handler(host, &direct_event)?;
        let first_wire = verify_no_active_status_wire(&first_direct, no_active_private_prose)?;
        let after_first_direct = guard_observation_counts(&fixture, &project_id)?;
        let second_direct = fixture.run_generated_final_output_handler(host, &direct_event)?;
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
        let prompt = live_final_output_no_active_prompt(&project_id);
        println!(
            "\n=== Volicord live {host}/{} final-output smoke ===\nThis first authenticated host turn intentionally has no active Volicord Task. After the host answer, inspect the host-native final-output surface. Do not enter credentials into this test process.\n=== end instruction ===\n",
            profile.as_str()
        );
        let host_status = fixture.run_authenticated_interactive_host(
            host,
            &executable,
            &prompt,
            &mut result_recorder,
        )?;
        if !host_status.success() {
            return Err(io::Error::other(format!(
                "the interactive {host} process exited unsuccessfully with {}",
                status_text(host_status)
            ))
            .into());
        }
        let after_actual_host = guard_observation_counts(&fixture, &project_id)?;
        verify_live_connection_after_host_observation(&fixture, host, &identity.connection_id)?;
        confirm_final_output_ui(host, profile, FinalOutputUiExpectation::ManagedSurface)?;
        confirm_final_output_ui(
            host,
            profile,
            FinalOutputUiExpectation::NoActiveTaskStatus {
                complete_message: first_wire.system_message.clone(),
            },
        )?;

        let no_active_actual_event = match profile {
            IntegrationProfile::Record => {
                let managed_mcp_observation = verify_record_managed_mcp_host_turn(
                    &fixture,
                    &project_id,
                    &identity.connection_id,
                    before_actual_host,
                    after_actual_host,
                    Some("volicord.status"),
                )?;
                serde_json::json!({
                    "status": "verified",
                    "source": "authenticated_host_owned_surface_delivery",
                    "delivery_evidence": "managed_final_output_ui",
                    "managed_mcp_observation": managed_mcp_observation,
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
                    "decision": historical.decision,
                    "persistent_guard_event": true
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
            host,
            &executable,
            concat!(
                "Reply with exactly VOLICORD_LIVE_FINAL_OUTPUT_RECEIPT and then stop. ",
                "Do not call tools, MCP servers, shell commands, or edit files."
            ),
            &mut result_recorder,
        )?;
        if !receipt_host_status.success() {
            return Err(io::Error::other(format!(
                "the AuthorityReceipt interactive {host} process exited unsuccessfully with {}",
                status_text(receipt_host_status)
            ))
            .into());
        }
        let after_receipt_host = guard_observation_counts(&fixture, &project_id)?;
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
        let (receipt_actual_event, detective_decision) = match profile {
            IntegrationProfile::Record => {
                let managed_mcp_observation = verify_record_managed_mcp_host_turn(
                    &fixture,
                    &project_id,
                    &identity.connection_id,
                    before_receipt_host,
                    after_receipt_host,
                    None,
                )?;
                (
                    serde_json::json!({
                        "status": "verified",
                        "source": "authenticated_host_owned_surface_delivery",
                        "delivery_evidence": "managed_final_output_ui",
                        "managed_mcp_observation": managed_mcp_observation,
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
                        "decision": stop.decision,
                        "persistent_guard_event": true
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
        let first_active_direct =
            fixture.run_generated_final_output_handler(host, &active_direct_event)?;
        let first_active_wire = verify_authority_receipt_wire(
            &first_active_direct,
            &prepared.receipt,
            true,
            active_private_prose,
        )?;
        let after_first_active_direct = guard_observation_counts(&fixture, &project_id)?;
        let first_direct_historical = if profile == IntegrationProfile::Detective {
            Some(stored_stop_snapshot_for_native_session(
                &fixture,
                &project_id,
                host,
                &identity.connection_id,
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
        let second_active_direct =
            fixture.run_generated_final_output_handler(host, &active_direct_event)?;
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
                if stored_stop_snapshot_for_native_session(
                    &fixture,
                    &project_id,
                    host,
                    &identity.connection_id,
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
        let host_feature_support = config_fixture["host_feature_support"].clone();
        let final_output_authority_disclosure =
            config_fixture["final_output_authority_disclosure"].clone();

        let summary = serde_json::json!({
            "kind": LIVE_FINAL_OUTPUT_RESULT_KIND,
            "result": "incomplete",
            "host": {
                "kind": identity.host,
                "version": identity.host_version,
                "executable_sha256": identity.host_executable_sha256
            },
            "profile": profile.as_str(),
            "volicord": { "build_id": identity.volicord_build_id },
            "connection": { "connection_id": identity.connection_id },
            "host_feature_support": host_feature_support,
            "final_output_authority_disclosure": final_output_authority_disclosure,
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
        let mut result_recorder = LiveResultRecorder::from_env_for_kind_and_profile(
            &recorder_host,
            host,
            LIVE_CLI_FALLBACK_RESULT_KIND,
            Some(IntegrationProfile::Detective),
        )?;
        let release_candidate = result_recorder.release_candidate()?.clone();
        let fixture = LiveSmokeFixture::new_with_release_candidate_for_recorder(
            &recorder_host,
            &release_candidate,
            &mut result_recorder,
        )?;
        let executable = match find_executable(executable_name) {
            Some(executable) => executable,
            None => {
                let reason = format!("`{executable_name}` was not found on PATH");
                let summary =
                    live_cli_fallback_unavailable_summary(host, None, "host_executable", &reason);
                validate_release_host_feature_diagnostics(
                    &summary,
                    Some(IntegrationProfile::Detective),
                    false,
                    false,
                )?;
                result_recorder.record_final(&summary)?;
                return Err(io::Error::new(io::ErrorKind::NotFound, reason).into());
            }
        };
        result_recorder.mark_installed_host_detected();
        let ObservedReleaseHostIdentity {
            host_version,
            host_executable_sha256,
            volicord_build_id,
        } = fixture.observe_and_bind_installed_host_identity(
            &mut result_recorder,
            executable_name,
            &executable,
        )?;
        if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
            let reason = "authenticated live CLI-fallback validation requires interactive terminal stdin and stdout";
            let summary = live_cli_fallback_unavailable_summary(
                host,
                Some(&host_version),
                "interactive_terminal",
                reason,
            );
            validate_release_host_feature_diagnostics(
                &summary,
                Some(IntegrationProfile::Detective),
                false,
                false,
            )?;
            result_recorder.record_final(&summary)?;
            return Err(io::Error::other(reason).into());
        }
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
            Some(&host_version),
            IntegrationProfile::Detective,
            expected_host_action,
        );
        let host_feature_diagnostics = release_host_feature_diagnostics_from_init(
            &init_json,
            host,
            Some(&host_version),
            IntegrationProfile::Detective,
        )?;
        let connection_id = bounded_identity(
            "Agent Connection id",
            init_json["connection"]["connection_id"]
                .as_str()
                .ok_or_else(|| io::Error::other("init result has no Agent Connection id"))?,
            MAX_CONNECTION_ID_BYTES,
        )?;
        let identity = LiveHostIdentity {
            host: host.to_owned(),
            host_version,
            host_executable_sha256,
            volicord_build_id,
            connection_id,
        };
        observe_and_verify_live_connection_before_task(
            &fixture,
            host,
            &executable,
            &identity.connection_id,
            &mut result_recorder,
        )?;
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
        let diagnostic_cursor = diagnostic_event_cursor(&fixture)?;
        let prompt = live_cli_fallback_resume_prompt(&prepared)?;
        println!(
            "\n=== Volicord live {host} CLI-fallback smoke ===\nThe pending choice was resolved by the human operator through the actual `volicord inbox resolve --json` User Channel. The installed host must now resume that exact request through the same Agent Connection, consume the selected option, record its mapped no-write Run, read fresh status, and stop. Approve the repository or MCP entry if the host asks. Do not type credentials or secrets.\n\n{prompt}\n=== end instruction ===\n"
        );
        let status = fixture.run_authenticated_interactive_host(
            host,
            &executable,
            &prompt,
            &mut result_recorder,
        )?;
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
            diagnostic_cursor,
        )?;
        if let Err(error) = confirm_final_output_ui(
            host,
            IntegrationProfile::Detective,
            FinalOutputUiExpectation::CompleteAuthorityReceipt {
                canonical_json: canonical_json_string(&receipt.canonical_receipt)?,
            },
        ) {
            let summary = live_cli_fallback_completed_summary(LiveCliFallbackSummaryInput {
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
                host_feature_diagnostics: &host_feature_diagnostics,
            });
            validate_release_host_feature_diagnostics(
                &summary,
                Some(IntegrationProfile::Detective),
                true,
                true,
            )?;
            result_recorder.record_final(&summary)?;
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
            host_feature_diagnostics: &host_feature_diagnostics,
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
        let release_candidate = result_recorder.release_candidate()?.clone();
        let fixture = LiveSmokeFixture::new_with_release_candidate_for_recorder(
            &format!("{host}-user-action"),
            &release_candidate,
            &mut result_recorder,
        )?;
        let executable = match find_executable(executable_name) {
            Some(executable) => executable,
            None => {
                let reason = format!("`{executable_name}` was not found on PATH");
                let summary =
                    live_user_action_unavailable_summary(host, None, "host_executable", &reason);
                validate_release_host_feature_diagnostics(
                    &summary,
                    Some(IntegrationProfile::Detective),
                    false,
                    false,
                )?;
                result_recorder.record_final(&summary)?;
                return Err(io::Error::new(io::ErrorKind::NotFound, reason).into());
            }
        };
        result_recorder.mark_installed_host_detected();
        let ObservedReleaseHostIdentity {
            host_version,
            host_executable_sha256,
            volicord_build_id,
        } = fixture.observe_and_bind_installed_host_identity(
            &mut result_recorder,
            executable_name,
            &executable,
        )?;
        if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
            let reason = "authenticated live Judgment validation requires interactive terminal stdin and stdout";
            let summary = live_user_action_unavailable_summary(
                host,
                Some(&host_version),
                "interactive_terminal",
                reason,
            );
            validate_release_host_feature_diagnostics(
                &summary,
                Some(IntegrationProfile::Detective),
                false,
                false,
            )?;
            result_recorder.record_final(&summary)?;
            return Err(io::Error::other(reason).into());
        }
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
            Some(&host_version),
            IntegrationProfile::Detective,
            expected_host_action,
        );
        let host_feature_diagnostics = release_host_feature_diagnostics_from_init(
            &init_json,
            host,
            Some(&host_version),
            IntegrationProfile::Detective,
        )?;
        let connection_id = bounded_identity(
            "Agent Connection id",
            init_json["connection"]["connection_id"]
                .as_str()
                .ok_or_else(|| io::Error::other("init result has no Agent Connection id"))?,
            MAX_CONNECTION_ID_BYTES,
        )?;
        let identity = LiveHostIdentity {
            host: host.to_owned(),
            host_version,
            host_executable_sha256,
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
        observe_and_verify_live_connection_before_task(
            &fixture,
            host,
            &executable,
            &identity.connection_id,
            &mut result_recorder,
        )?;

        let marker = format!(
            "VOLICORD_LIVE_HOST_USER_ACTION_ROUND_TRIP_{}",
            host.replace('-', "_").to_ascii_uppercase()
        );
        let project_id = live_fixture_project_id(&fixture)?;
        let prompt = live_user_action_prompt(&marker, &project_id);
        let judgment_stop_cursor = stop_event_cursor(&fixture, &project_id)?;
        println!(
            "\n=== Volicord live {host} user-action smoke ===\nThe host will receive this initial instruction and may ask you to trust the repository or approve its MCP server. When the host-native user-action selector appears, choose one option yourself. Do not type credentials or secrets. Exit the host after it reports the final Volicord status.\n\n{prompt}\n=== end instruction ===\n"
        );
        let status = fixture.run_authenticated_interactive_host(
            host,
            &executable,
            &prompt,
            &mut result_recorder,
        )?;
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
            let summary = live_host_fallback_summary(
                &identity,
                &observation,
                &fallback,
                &host_feature_diagnostics,
            );
            validate_release_host_feature_diagnostics(
                &summary,
                Some(IntegrationProfile::Detective),
                true,
                true,
            )?;
            result_recorder.record_final(&summary)?;
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
            let summary = live_host_choice_mismatch_summary(
                &identity,
                &observation,
                &operator_choice_id,
                selected_option_id,
                &host_feature_diagnostics,
            );
            validate_release_host_feature_diagnostics(
                &summary,
                Some(IntegrationProfile::Detective),
                true,
                true,
            )?;
            result_recorder.record_final(&summary)?;
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
            let summary = live_host_completed_summary(LiveCompletedSummaryInput {
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
                host_feature_diagnostics: &host_feature_diagnostics,
            });
            validate_release_host_feature_diagnostics(
                &summary,
                Some(IntegrationProfile::Detective),
                true,
                true,
            )?;
            result_recorder.record_final(&summary)?;
            return Err(error);
        }
        let summary = live_host_completed_summary(LiveCompletedSummaryInput {
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
            host_feature_diagnostics: &host_feature_diagnostics,
        });
        validate_release_host_feature_diagnostics(
            &summary,
            Some(IntegrationProfile::Detective),
            true,
            true,
        )?;
        result_recorder.record_final(&summary)?;
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

    fn live_user_action_prompt(marker: &str, project_selector: &str) -> String {
        let routing_instruction = live_project_routing_instruction(project_selector);
        format!(
            concat!(
                "Run a human-in-the-loop Volicord connection smoke using the MCP server named `volicord`. ",
                "{routing_instruction} ",
                "Do not edit files, run shell commands, prepare a write, or answer on the user's behalf.\n\n",
                "1. Call `volicord.intake` with `detail=full`, `requested_mode=advisor`, `acceptance_policy=null`, and create-new resume behavior. The plain-language request must be exactly `{task_marker}`. Use a narrow no-write initial scope and exactly one acceptance criterion whose `evidence_requirement=not_required`. Retain the returned Task ID.\n",
                "2. For that Task, call `volicord.update_scope` with `detail=full`, `baseline_ref={baseline_ref}`, and a `change_unit` whose `operation=create_current`, `scope_summary` describes this no-write live-host user-action validation, and `affected_paths=[]`. Retain `state.active_change_unit_ref.record_id` and `state.baseline_ref`. Do not continue unless both are present.\n",
                "3. Call `volicord.request_user_action` exactly once and omit top-level `detail` so the default compact projection is exercised. Put every create field inside `request`: set `request.operation=create`, set `request.task_id` to the retained Task ID, set `request.change_unit_id` to the retained `state.active_change_unit_ref.record_id`, set `request.required_for=[\"close_complete\"]`, and set `request.expires_at=null`. Set `request.action` to the closed choice object with `request.action.action_type=choice`, `request.action.judgment_kind=product_decision`, `request.action.presentation=short`, `request.action.question=\"Which live-smoke route must the agent consume?\"`, `request.action.context={{\"summary\":\"A human operator must choose the live-smoke route.\",\"related_refs\":[],\"artifact_refs\":[],\"visible_risks\":[],\"constraints\":[]}}`, `request.action.affected_refs=[]`, and `request.action.sensitive_action_scope=null`. Do not add aliases such as `title` or `prompt`, and do not put `task_id` or `required_for` inside `action`. Provide exactly these two caller-authored options in this order:\n",
                "   - `option_id={alpha_option_id}`, label `Route alpha`, description `Select the alpha live-smoke route.`, consequence `The agent records the alpha choice-consumption Run marker.`, `is_default=false`.\n",
                "   - `option_id={beta_option_id}`, label `Route beta`, description `Select the beta live-smoke route.`, consequence `The agent records the beta choice-consumption Run marker.`, `is_default=false`.\n",
                "4. Wait for the host's native MCP elicitation/User Channel UI. The human running this smoke will choose the answer. Never infer, fabricate, or submit that answer yourself.\n",
                "5. After Volicord reports the user action resolved, consume `structuredContent.method_result.resolution_summary.selected_option_id` from that default compact result. If it is `{alpha_option_id}`, call `volicord.record_run` with summary exactly `{alpha_run_marker}`. If it is `{beta_option_id}`, call `volicord.record_run` with summary exactly `{beta_run_marker}`. Use the retained Task ID, Change Unit ID, and baseline ref; set `kind=shaping_update`, `run_id=null`, `write_ticket_id=null`, `artifact_inputs=[]`, `evidence_updates=[]`, and `evidence_observations=[]`; report `changed_paths=[]`, `product_file_write_observed=false`, `sensitive_categories=[]`, and the same baseline ref in `observed_changes`. Supply a non-null `close_assessment` whose `result_summary` is exactly the selected Run marker and whose `result_refs`, `residual_risks`, `sensitive_categories`, and `recovery_constraints` are all empty arrays. Do not record a Run if the selected option is absent or unrecognized.\n",
                "6. After that Run is recorded, call `volicord.status` for the Task and report the selected option ID, exact Run marker, lifecycle phase, close state, close-blocker count, and state version. Then stop.\n\n",
                "If a native prompt does not appear and Volicord returns only a pending `user_action_request_summary`, do not simulate a resolution or execute a fallback command. Report that the CLI User Channel is required and stop so the disposable harness can verify the trusted CLI inbox and resolve-command shape."
            ),
            routing_instruction = routing_instruction,
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
        let result_recorder =
            LiveResultRecorder::from_env_for_kind(host, LIVE_EVIDENCE_OBSERVATION_RESULT_KIND)?;
        execute_live_evidence_observation_round_trip(
            host,
            InstalledHostExecutable::discover(executable_name),
            expected_host_action,
            result_recorder,
        )
    }

    #[derive(Clone, Copy)]
    struct InstalledHostExecutable<'a> {
        name: &'a str,
        exact_path: Option<&'a Path>,
    }

    impl<'a> InstalledHostExecutable<'a> {
        fn discover(name: &'a str) -> Self {
            Self {
                name,
                exact_path: None,
            }
        }

        fn at_path(name: &'a str, exact_path: &'a Path) -> Self {
            Self {
                name,
                exact_path: Some(exact_path),
            }
        }

        fn resolve(self) -> Result<PathBuf, Box<dyn Error>> {
            match self.exact_path {
                Some(executable) => Ok(executable.to_path_buf()),
                None => find_executable(self.name).ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::NotFound,
                        format!("`{}` was not found on PATH", self.name),
                    )
                    .into()
                }),
            }
        }
    }

    fn execute_live_evidence_observation_round_trip(
        host: &str,
        installed_executable: InstalledHostExecutable<'_>,
        expected_host_action: &str,
        mut result_recorder: LiveResultRecorder,
    ) -> Result<(), Box<dyn Error>> {
        let release_candidate = result_recorder.release_candidate()?.clone();
        let mut stage = "preflight";
        let mut host_feature_diagnostics = None;
        let outcome = live_evidence_observation_round_trip_inner(
            host,
            installed_executable,
            expected_host_action,
            &release_candidate,
            &mut stage,
            &mut host_feature_diagnostics,
            &mut result_recorder,
        );
        match outcome {
            Ok(summary) if stage == "static_unsupported_by_host" => {
                let summary = result_recorder.with_observed_host_identity(&summary)?;
                validate_live_evidence_observation_incomplete_result_shape(&summary)?;
                result_recorder.record_final(&summary)
            }
            Ok(summary) => {
                stage = "result_validation";
                if let Err(error) = validate_live_evidence_observation_result_shape(&summary) {
                    let observed_host_version = result_recorder
                        .observed_host_coordinates
                        .as_ref()
                        .map(|coordinates| coordinates.host_version.clone());
                    let incomplete = live_evidence_observation_incomplete_summary(
                        host,
                        observed_host_version.as_deref(),
                        stage,
                        host_feature_diagnostics.as_ref(),
                    );
                    let incomplete = result_recorder.with_observed_host_identity(&incomplete)?;
                    validate_live_evidence_observation_incomplete_result_shape(&incomplete)?;
                    result_recorder.record_final(&incomplete)?;
                    return Err(error);
                }
                result_recorder.record_final(&summary)
            }
            Err(error) => {
                let observed_host_version = result_recorder
                    .observed_host_coordinates
                    .as_ref()
                    .map(|coordinates| coordinates.host_version.clone());
                let incomplete = live_evidence_observation_incomplete_summary(
                    host,
                    observed_host_version.as_deref(),
                    stage,
                    host_feature_diagnostics.as_ref(),
                );
                let incomplete = result_recorder.with_observed_host_identity(&incomplete)?;
                validate_live_evidence_observation_incomplete_result_shape(&incomplete)?;
                result_recorder.record_final(&incomplete)?;
                Err(error)
            }
        }
    }

    fn live_evidence_observation_round_trip_inner(
        host: &str,
        installed_executable: InstalledHostExecutable<'_>,
        expected_host_action: &str,
        release_candidate: &ReleaseCandidate,
        stage: &mut &'static str,
        host_feature_diagnostics: &mut Option<ReleaseHostFeatureDiagnostics>,
        result_recorder: &mut LiveResultRecorder,
    ) -> Result<Value, Box<dyn Error>> {
        *stage = "fixture_setup";
        let fixture = LiveSmokeFixture::new_with_release_candidate_for_recorder(
            &format!("{host}-evidence-observation"),
            release_candidate,
            result_recorder,
        )?;
        *stage = "host_executable";
        let executable = installed_executable.resolve()?;
        result_recorder.mark_installed_host_detected();
        let ObservedReleaseHostIdentity {
            host_version,
            host_executable_sha256,
            volicord_build_id,
        } = fixture.observe_and_bind_installed_host_identity(
            result_recorder,
            installed_executable.name,
            &executable,
        )?;
        let maintained_host_kind = match host {
            "codex" => HostKind::Codex,
            "claude-code" => HostKind::ClaudeCode,
            _ => {
                return Err(
                    io::Error::other("unsupported managed evidence-observation host").into(),
                )
            }
        };
        if host_feature_implementation_for_version(
            maintained_host_kind,
            Some(&host_version),
            HostFeature::LocalWebUserChannel,
        ) == HostFeatureImplementation::UnsupportedByHost
        {
            *stage = "static_unsupported_by_host";
            return Ok(live_evidence_observation_incomplete_summary(
                host,
                Some(&host_version),
                stage,
                None,
            ));
        }
        *stage = "interactive_terminal";
        if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
            return Err(io::Error::other(
                "authenticated live evidence-observation validation requires interactive terminal stdin and stdout",
            )
            .into());
        }
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
            Some(&host_version),
            IntegrationProfile::Detective,
            expected_host_action,
        )?;
        *host_feature_diagnostics = Some(release_host_feature_diagnostics_from_init(
            &init_json,
            host,
            Some(&host_version),
            IntegrationProfile::Detective,
        )?);
        let connection_id = bounded_identity(
            "Agent Connection id",
            init_json["connection"]["connection_id"]
                .as_str()
                .ok_or_else(|| io::Error::other("init result has no Agent Connection id"))?,
            MAX_CONNECTION_ID_BYTES,
        )?;
        let identity = LiveHostIdentity {
            host: host.to_owned(),
            host_version,
            host_executable_sha256,
            volicord_build_id,
            connection_id,
        };
        *stage = "connection_observation";
        observe_and_verify_live_connection_before_task(
            &fixture,
            host,
            &executable,
            &identity.connection_id,
            result_recorder,
        )?;
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
        let diagnostic_cursor = diagnostic_event_cursor(&fixture)?;
        let prompt = live_evidence_observation_prompt(&prepared);
        println!(
            "\n=== Volicord live {host} evidence-observation smoke ===\nThis cell is valid only if the MCP client negotiated `capabilities.experimental[\"{MODEL_INVISIBLE_USER_CHANNEL_CAPABILITY_NAMESPACE}\"].{MODEL_INVISIBLE_USER_CHANNEL_CAPABILITY_FIELD}=true` and the host consumes `CallToolResult._meta[\"{MODEL_INVISIBLE_USER_CHANNEL_CAPABILITY_NAMESPACE}\"]` in a separate host-owned model-invisible surface. That `_meta` handoff must never enter chat or model output. Never ask the agent to relay, quote, print, or open the URL. If that separate surface appears, use it yourself to select the sole target and artifact, choose `supported`, enter a non-secret one-line summary, and submit. If it does not appear—or if any URL, token, form, question, or request ref appears in returned tool-result content, returned structured output, or host diagnostic text—tell the host to stop without fallback; the harness will record this cell unavailable. Do not enter credentials, secrets, tokens, or private keys.\n\n{prompt}\n=== end instruction ===\n"
        );

        *stage = "host_process";
        let status = fixture.run_authenticated_interactive_host_with_local_web(
            host,
            &executable,
            &prompt,
            result_recorder,
        )?;
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
            diagnostic_cursor,
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
                host_feature_diagnostics: host_feature_diagnostics
                    .as_ref()
                    .expect("successful init stores the canonical release diagnostics"),
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
        let routing_instruction = live_project_routing_instruction(&prepared.project_id);
        format!(
            concat!(
                "Continue the prepared Volicord evidence-observation validation using the MCP server named `volicord`. ",
                "{routing_instruction} ",
                "Do not edit files, run shell commands, prepare a write, inspect or relay host-only metadata, open any consent URL, or answer on the user's behalf. The fixture contains no real credential or secret; its display metadata contains a routing marker solely so Volicord requires a user-only local consent page.\n\n",
                "1. Call `volicord.request_user_action` exactly once with `detail=full` and the closed create shape. Set `request.operation=create`, `request.task_id={task_id}`, `request.change_unit_id={change_unit_id}`, `request.required_for=[\"record_run\"]`, and `request.expires_at=null`. Set `request.action` to the closed evidence-observation object with `request.action.action_type=evidence_observation`, `request.action.question=\"{question}\"`, `request.action.context_summary=\"{context_summary}\"`, `request.action.target_candidates=[{target}]`, and `request.action.artifact_candidate_ids=[\"{artifact_id}\"]`. Do not put `request.task_id`, `request.change_unit_id`, `request.required_for`, or `request.expires_at` inside `request.action`, and do not add any other `request.action` members. Do not create any other user action.\n",
                "2. The returned public result must expose only `user_action_request_summary` with the exact keys `user_action_request_id`, `status=pending`, and `next_actor=user`; it must not expose a full request, request ref, inbox item, form, question, URL, token, command, or channel availability. The local-web handoff exists only in host-consumed top-level `CallToolResult._meta[\"{capability_namespace}\"]` and is owned entirely by a separate host model-invisible surface. Do not inspect, print, quote, relay, or open it. If the host does not render that separate surface, report only that the required User Channel is unavailable and stop without elicitation, prompt capture, or CLI fallback.\n",
                "3. Wait until the operator confirms completion without pasting any URL, token, form value, or observation summary into chat. Then call `volicord.request_user_action` exactly once with `detail=full` and the closed resume shape: set `request.operation=resume` and set `request.user_action_request_id` to the request ID returned in step 2. Do not include create-only fields in the resume `request`, and never use create again. Require `agent_workflow_result_replayed=true`, `current_status=resolved`, a non-null `user_action_resolution_ref`, and an evidence-observation resolution summary whose target equals `{target}`, whose sole artifact has ID `{artifact_id}`, and whose relevance is `supported`. Do not record a Run if any fact differs.\n",
                "4. Consume that exact resolution in one `volicord.record_run` call. Use `task_id={task_id}`, `change_unit_id={change_unit_id}`, `kind=shaping_update`, `run_id=null`, `baseline_ref={baseline_ref}`, `write_ticket_id=null`, summary exactly `{run_marker}`, no product-file changes, `artifact_inputs=[]`, and one supported evidence update for the resolved target with the exact resolved ArtifactRef. Add exactly one evidence observation for that target with `source_kind=user_observation`, `assurance_level=user_observed`, null observer/tool fields, empty tool metadata/source refs/limitations, `input_refs` containing only the exact resolution ref, `output_artifact_refs` containing only the exact resolved ArtifactRef, and `observed_at={caller_observed_at}`. Supply a close assessment with result summary exactly `{run_marker}` and empty result refs, risks, sensitive categories, and recovery constraints.\n",
                "5. Call `volicord.status` for Task `{task_id}`. Report only the request ID from the safe summary, resolution ID, Run ID, evidence observation ID, lifecycle phase, close state, blocker count, and state version. Do not repeat a request ref, form, question, URL, token, user summary, this prompt, or a transcript. Then stop."
            ),
            routing_instruction = routing_instruction,
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
        prepared: &PreparedCliFallbackAction,
    ) -> Result<String, io::Error> {
        let user_action_request_id = prepared
            .observation
            .user_action_request_id
            .as_deref()
            .ok_or_else(|| io::Error::other("prepared CLI fallback request id is missing"))?;
        let routing_instruction =
            live_project_routing_instruction(&prepared.observation.project_id);
        Ok(format!(
            concat!(
                "Continue the prepared Volicord CLI User Channel fallback using the MCP server named `volicord`. ",
                "{routing_instruction} ",
                "Do not edit files, run shell commands, create or resolve another user action, prepare a write, or answer on the user's behalf.\n\n",
                "1. Call `volicord.request_user_action` exactly once with nested `request.operation=resume` and `request.user_action_request_id={user_action_request_id}`. Do not use `request.operation=create`. The request was created by this same Agent Connection and has already been resolved by the human operator through `volicord inbox resolve --json`.\n",
                "2. Require the resumed result to report `current_status=resolved`, `agent_workflow_result_replayed=true`, and a non-null `user_channel_resolution.resolution_summary.selected_option_id`. Consume that selected option; do not infer it from this instruction.\n",
                "3. If the selected option is `{alpha_option_id}`, call `volicord.record_run` with summary exactly `{alpha_run_marker}`. If it is `{beta_option_id}`, use summary exactly `{beta_run_marker}`. Use `task_id={task_id}`, `change_unit_id={change_unit_id}`, `baseline_ref={baseline_ref}`, `kind=shaping_update`, `run_id=null`, `write_ticket_id=null`, `artifact_inputs=[]`, `evidence_updates=[]`, and `evidence_observations=[]`. Report `changed_paths=[]`, `product_file_write_observed=false`, `sensitive_categories=[]`, and the same baseline ref in `observed_changes`. Supply a non-null `close_assessment` whose `result_summary` is exactly the chosen Run marker and whose `result_refs`, `residual_risks`, `sensitive_categories`, and `recovery_constraints` are empty arrays. Do not record a Run if resume is pending or the option is absent or unrecognized.\n",
                "4. Call `volicord.status` for Task `{task_id}` and report the selected option ID, exact Run marker, lifecycle phase, close state, close-blocker count, and state version. Then stop."
            ),
            routing_instruction = routing_instruction,
            user_action_request_id = user_action_request_id,
            alpha_option_id = USER_ACTION_ROUTE_ALPHA_OPTION_ID,
            beta_option_id = USER_ACTION_ROUTE_BETA_OPTION_ID,
            alpha_run_marker = USER_ACTION_ROUTE_ALPHA_RUN_MARKER,
            beta_run_marker = USER_ACTION_ROUTE_BETA_RUN_MARKER,
            task_id = prepared.observation.task_id,
            change_unit_id = prepared.change_unit_id,
            baseline_ref = LIVE_CLI_FALLBACK_BASELINE_REF,
        ))
    }

    fn live_project_routing_instruction(project_selector: &str) -> String {
        format!(
            "Pass `project_selector={project_selector}` on every public Volicord method call in this turn. This is the fixture's exact opaque selector; never replace it with a repository name, folder name, host label, or remembered value."
        )
    }

    fn live_final_output_no_active_prompt(project_selector: &str) -> String {
        format!(
            "Use only the MCP server named `volicord` to call `volicord.status` once with `detail=full`, no task_id, and the prepared project routing. {} Confirm that it reports no active Task, then reply with exactly VOLICORD_LIVE_FINAL_OUTPUT_NO_ACTIVE_TASK and stop. Do not call any other tool, shell command, or edit files.",
            live_project_routing_instruction(project_selector)
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
            MAX_RECORDED_AT_BYTES,
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
            match event_type.as_str() {
                "user_action_requested"
                    if payload
                        .get("user_action_request_id")
                        .and_then(Value::as_str)
                        == Some(user_action_request_id) =>
                {
                    if payload.get("action_kind").and_then(Value::as_str)
                        != Some("product_decision")
                    {
                        return Err(io::Error::other(
                            "matching user_action_requested event has the wrong action kind",
                        )
                        .into());
                    }
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
        host_executable_sha256: String,
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
        host_feature_diagnostics: &'a ReleaseHostFeatureDiagnostics,
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
            host_feature_diagnostics,
        } = input;
        serde_json::json!({
            "kind": LIVE_EVIDENCE_OBSERVATION_RESULT_KIND,
            "result": "passed",
            "host": {
                "kind": identity.host,
                "version": identity.host_version,
                "executable_sha256": identity.host_executable_sha256
            },
            "volicord": {
                "build_id": identity.volicord_build_id
            },
            "connection": {
                "connection_id": identity.connection_id
            },
            "host_feature_support": host_feature_diagnostics.host_feature_support,
            "final_output_authority_disclosure": host_feature_diagnostics.final_output_authority_disclosure,
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
                    "effective_exact_capability_observed": false,
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

    fn live_evidence_observation_incomplete_summary(
        host: &str,
        host_version: Option<&str>,
        stage: &str,
        initialized_diagnostics: Option<&ReleaseHostFeatureDiagnostics>,
    ) -> Value {
        let result = match stage {
            "host_executable"
            | "interactive_terminal"
            | "host_delivery_boundary"
            | "static_unsupported_by_host" => "unavailable",
            _ => "failed",
        };
        let default_diagnostics;
        let diagnostics = match initialized_diagnostics {
            Some(diagnostics) => diagnostics,
            None => {
                default_diagnostics = canonical_release_host_feature_diagnostics_for_version(
                    host,
                    host_version,
                    IntegrationProfile::Detective,
                    false,
                    false,
                );
                &default_diagnostics
            }
        };
        serde_json::json!({
            "kind": LIVE_EVIDENCE_OBSERVATION_RESULT_KIND,
            "result": result,
            "host": { "kind": host },
            "stage": stage,
            "host_feature_support": diagnostics.host_feature_support,
            "final_output_authority_disclosure": diagnostics.final_output_authority_disclosure,
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

    fn native_user_action_result_shape_fixture(host: &str, executable_sha256: &str) -> Value {
        let diagnostics = canonical_release_host_feature_diagnostics(
            host,
            IntegrationProfile::Detective,
            true,
            true,
        );
        serde_json::json!({
            "kind": LIVE_USER_ACTION_RESULT_KIND,
            "result": "passed",
            "host": {
                "kind": host,
                "version": "fixture-host 1.0",
                "executable_sha256": executable_sha256
            },
            "volicord": { "build_id": "fixture-build-id" },
            "connection": { "connection_id": "CONN-native-fixture" },
            "host_feature_support": diagnostics.host_feature_support,
            "final_output_authority_disclosure": diagnostics.final_output_authority_disclosure,
            "task": {
                "project_id": "PRJ-native-fixture",
                "task_id": "TASK-native-fixture",
                "lifecycle_phase": "ready_to_close",
                "state_version": 5
            },
            "user_action": {
                "user_action_request_id": "UAR-native-fixture",
                "selected_option_id": USER_ACTION_ROUTE_ALPHA_OPTION_ID,
                "operator_confirmed_option_id": USER_ACTION_ROUTE_ALPHA_OPTION_ID,
                "stored_choice_matches_operator": true,
                "user_channel_basis": VERIFICATION_BASIS_MCP_ELICITATION_USER_CHANNEL
            },
            "choice_consumption": {
                "run_id": "RUN-native-fixture",
                "run_kind": "shaping_update",
                "run_marker": USER_ACTION_ROUTE_ALPHA_RUN_MARKER,
                "product_file_write_observed": false,
                "changed_path_count": 0
            },
            "authority_events": {
                "user_action_requested_event_seq": 10,
                "user_action_resolved_event_seq": 11,
                "run_recorded_event_seq": 12,
                "ordered": true
            },
            "native_ui": {
                "user_action_selector_confirmed": true,
                "operator_choice_confirmed": true,
                "stop_system_message_authority_receipt_confirmed": true
            },
            "stop_hook": {
                "guard_event_id": format!("guard_event_{}", "1".repeat(16)),
                "session_id": format!("mhs_{}", "2".repeat(64)),
                "connection_id": "CONN-native-fixture",
                "decision": "allow",
                "decision_observed_from_guard_event": true,
                "receipt_state_version": 5,
                "latest_run_id": "RUN-native-fixture"
            },
            "authority_receipt": {
                "project_id": "PRJ-native-fixture",
                "task_id": "TASK-native-fixture",
                "state_version": 5,
                "latest_run_id": "RUN-native-fixture",
                "close_state": "ready",
                "close_blocker_count": 0
            },
            "cli_fallback": { "verified": false }
        })
    }

    fn producer_result_shape_fixture(feature: HostFeature) -> Value {
        let diagnostics = canonical_release_host_feature_diagnostics(
            "codex",
            IntegrationProfile::Detective,
            true,
            true,
        );
        let (kind, source_event_ids, host_invocation_id, source_claim_count) = match feature {
            HostFeature::VerifiedToolProducer => (
                LIVE_VERIFIED_TOOL_PRODUCER_RESULT_KIND,
                serde_json::json!([
                    format!("guard_event_{}", "3".repeat(16)),
                    format!("guard_event_{}", "4".repeat(16))
                ]),
                Value::String(format!("managed_native_id_{}", "5".repeat(16))),
                3,
            ),
            HostFeature::RegisteredConnectionObservation => (
                LIVE_REGISTERED_CONNECTION_OBSERVATION_RESULT_KIND,
                serde_json::json!([format!("guard_event_{}", "6".repeat(16))]),
                Value::Null,
                1,
            ),
            _ => unreachable!("producer fixture requires a producer feature"),
        };
        let source_event_count = source_event_ids
            .as_array()
            .expect("fixture source event ids are an array")
            .len();
        serde_json::json!({
            "kind": kind,
            "result": "passed",
            "host": {
                "kind": "codex",
                "version": "fixture-host 1.0",
                "executable_sha256": "a".repeat(64)
            },
            "volicord": { "build_id": "fixture-build-id" },
            "connection": { "connection_id": "CONN-producer-fixture" },
            "host_feature_support": diagnostics.host_feature_support,
            "final_output_authority_disclosure": diagnostics.final_output_authority_disclosure,
            "actual_host_event": {
                "observed": true,
                "source_event_count": source_event_count,
                "source_event_ids": source_event_ids,
                "opaque_session_id": format!("mhs_{}", "b".repeat(64)),
                "guard_installation_id": "GI-producer-fixture",
                "host_invocation_id": host_invocation_id
            },
            "capture_intent": {
                "capture_intent_id": "ECI-producer-fixture",
                "input_sha256": "c".repeat(64),
                "intent_state_version": 4,
                "task_id": "TASK-producer-fixture",
                "change_unit_id": "CU-producer-fixture",
                "baseline_ref": "baseline_producer_fixture",
                "intent_precedes_source": true,
                "exact_session_connection_actor_scope_baseline": true
            },
            "negative_capture": {
                "rejected": true,
                "nonzero_exit": true,
                "receipt_delta": 0,
                "staging_delta": 0,
                "producer_delta": 0,
                "artifact_delta": 0,
                "authority_event_delta": 0,
                "state_version_unchanged": true
            },
            "capture_receipt": {
                "capture_receipt_id": "ECR-producer-fixture",
                "staged_receipt_handle_id": "STAGE-producer-fixture",
                "safe_receipt_sha256": "d".repeat(64),
                "result_sha256": "e".repeat(64),
                "source_claim_count": source_claim_count,
                "bound": true
            },
            "host_resume": {
                "record_run_calls": 1,
                "status_calls": 1,
                "check_close_calls": 1,
                "ordered": true,
                "same_connection": true
            },
            "producer_chain": {
                "producer_id": "EP-producer-fixture",
                "artifact_id": "ART-producer-fixture",
                "evidence_observation_id": "EOBS-producer-fixture",
                "run_id": "RUN-producer-fixture",
                "strong": true,
                "criterion_coverage_projected": true
            },
            "close": {
                "project_id": "PRJ-producer-fixture",
                "task_id": "TASK-producer-fixture",
                "state_version": 6,
                "latest_run_id": "RUN-producer-fixture",
                "ready": true,
                "blocker_count": 0
            },
            "assertions": {
                "actual_host_event": true,
                "intent_precedes_source": true,
                "exact_session_connection_actor_scope_baseline": true,
                "capture_receipt_bound": true,
                "strong_producer_chain": true,
                "criterion_coverage_projected": true,
                "negative_rejections_zero_effect": true
            },
            "sensitive_payloads": {
                "prompt_recorded": false,
                "tool_input_recorded": false,
                "tool_output_recorded": false,
                "transcript_recorded": false,
                "token_recorded": false,
                "url_recorded": false,
                "native_session_id_recorded": false
            }
        })
    }

    fn evidence_observation_result_shape_fixture() -> Value {
        let diagnostics = canonical_release_host_feature_diagnostics(
            "codex",
            IntegrationProfile::Detective,
            true,
            true,
        );
        serde_json::json!({
            "kind": LIVE_EVIDENCE_OBSERVATION_RESULT_KIND,
            "result": "passed",
            "host": {
                "kind": "codex",
                "version": "fixture-version",
                "executable_sha256": "a".repeat(64)
            },
            "host_feature_support": diagnostics.host_feature_support,
            "final_output_authority_disclosure": diagnostics.final_output_authority_disclosure,
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
                "guard_event_id": format!("guard_event_{}", "7".repeat(16)),
                "session_id": format!("mhs_{}", "8".repeat(64)),
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
                "host_feature_support",
                "final_output_authority_disclosure",
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
            ("/host", &["kind", "version", "executable_sha256"][..]),
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
        let observed_identity = value.pointer("/host/version").is_some()
            || value.pointer("/host/executable_sha256").is_some()
            || value.pointer("/volicord/build_id").is_some();
        if observed_identity {
            require_exact_live_evidence_result_keys(
                value,
                "",
                &[
                    "kind",
                    "result",
                    "host",
                    "volicord",
                    "stage",
                    "host_feature_support",
                    "final_output_authority_disclosure",
                    "evidence_scope",
                    "sensitive_payloads",
                ],
            )?;
            require_exact_live_evidence_result_keys(
                value,
                "/host",
                &["kind", "version", "executable_sha256"],
            )?;
            require_exact_live_evidence_result_keys(value, "/volicord", &["build_id"])?;
            required_result_string(value, "/host/version")?;
            validate_lower_hex(
                "incomplete evidence-observation host executable digest",
                required_result_string(value, "/host/executable_sha256")?,
                &[64],
            )?;
            required_result_string(value, "/volicord/build_id")?;
        } else {
            require_exact_live_evidence_result_keys(
                value,
                "",
                &[
                    "kind",
                    "result",
                    "host",
                    "stage",
                    "host_feature_support",
                    "final_output_authority_disclosure",
                    "evidence_scope",
                    "sensitive_payloads",
                ],
            )?;
            require_exact_live_evidence_result_keys(value, "/host", &["kind"])?;
        }
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
        validate_release_host_feature_diagnostics(
            value,
            Some(IntegrationProfile::Detective),
            true,
            true,
        )?;
        reject_forbidden_live_evidence_result_fields(value)?;
        required_result_string(value, "/host/kind")?;
        required_result_string(value, "/host/version")?;
        validate_lower_hex(
            "host executable_sha256",
            required_result_string(value, "/host/executable_sha256")?,
            &[64],
        )?;
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
        validate_domain_separated_correlation_id(
            "local-web guard event id",
            required_result_string(value, "/stop_hook/guard_event_id")?,
            "guard_event",
        )?;
        validate_managed_summary_session_id(
            "local-web session id",
            required_result_string(value, "/stop_hook/session_id")?,
        )?;
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
        let configured = value["final_output_authority_disclosure"]["configured"]
            .as_bool()
            .ok_or_else(|| io::Error::other("incomplete release result has no configured fact"))?;
        let configuration_verified = value["final_output_authority_disclosure"]
            ["configuration_verified"]
            .as_bool()
            .ok_or_else(|| {
                io::Error::other("incomplete release result has no configuration_verified fact")
            })?;
        if configuration_verified && !configured {
            return Err(io::Error::other(
                "configuration_verified cannot be true when configured is false",
            )
            .into());
        }
        validate_release_host_feature_diagnostics(
            value,
            Some(IntegrationProfile::Detective),
            configured,
            configuration_verified,
        )?;
        reject_forbidden_live_evidence_result_fields(value)?;
        required_result_string(value, "/host/kind")?;
        let stage = required_result_string(value, "/stage")?;
        let expected_result = match stage {
            "host_executable"
            | "interactive_terminal"
            | "host_delivery_boundary"
            | "static_unsupported_by_host" => "unavailable",
            "fixture_setup"
            | "connection_observation"
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
        let diagnostics = canonical_release_host_feature_diagnostics(
            "codex",
            IntegrationProfile::Detective,
            true,
            true,
        );
        serde_json::json!({
            "kind": LIVE_CLI_FALLBACK_RESULT_KIND,
            "result": "passed",
            "host": { "kind": "codex" },
            "host_feature_support": diagnostics.host_feature_support,
            "final_output_authority_disclosure": diagnostics.final_output_authority_disclosure,
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
        let host = "claude-code";
        let diagnostics = canonical_release_host_feature_diagnostics(host, profile, true, true);
        let config_host_feature_support = diagnostics.host_feature_support.clone();
        let config_final_output_authority_disclosure =
            diagnostics.final_output_authority_disclosure.clone();
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
        let (status_fallback_event, authority_receipt_event) = match profile {
            IntegrationProfile::Record => {
                let managed_mcp_observation = |session_id: &str, tool_name: Option<&str>| {
                    let mut lifecycle_events = vec![
                        "managed_host_startup",
                        "managed_host_initialize_response",
                        "managed_host_tools_list",
                    ];
                    if tool_name.is_some() {
                        lifecycle_events
                            .extend(["managed_host_tool_call", "managed_host_tool_call_completed"]);
                    }
                    serde_json::json!({
                        "agent_session_delta": 1,
                        "all_new_sessions_validated": true,
                        "complete_session_ids": [session_id],
                        "connection_id": "CONN-fixture",
                        "guard_mode": "record",
                        "lifecycle_events": lifecycle_events,
                        "session_ids": [session_id],
                        "tool_name": tool_name
                    })
                };
                (
                    serde_json::json!({
                        "status": "verified",
                        "source": "authenticated_host_owned_surface_delivery",
                        "delivery_evidence": "managed_final_output_ui",
                        "managed_mcp_observation": managed_mcp_observation(
                            &format!("mhs_{}", "9".repeat(64)),
                            Some("volicord.status"),
                        ),
                        "persistent_guard_event": false,
                        "non_observing": true
                    }),
                    serde_json::json!({
                        "status": "verified",
                        "source": "authenticated_host_owned_surface_delivery",
                        "delivery_evidence": "managed_final_output_ui",
                        "managed_mcp_observation": managed_mcp_observation(
                            &format!("mhs_{}", "a".repeat(64)),
                            None,
                        ),
                        "persistent_guard_event": false,
                        "non_observing": true
                    }),
                )
            }
            IntegrationProfile::Detective => {
                let event = serde_json::json!({
                    "status": "verified",
                    "source": "persisted_guard_event",
                    "persistent_guard_event": true,
                    "guard_event_id": format!("guard_event_{}", "b".repeat(16)),
                    "session_id": format!("mhs_{}", "c".repeat(64)),
                    "decision": "allow"
                });
                (event.clone(), event)
            }
        };
        serde_json::json!({
            "kind": LIVE_FINAL_OUTPUT_RESULT_KIND,
            "result": "incomplete",
            "host": { "kind": host },
            "profile": profile.as_str(),
            "host_feature_support": diagnostics.host_feature_support,
            "final_output_authority_disclosure": diagnostics.final_output_authority_disclosure,
            "evidence": {
                "config_fixture": {
                    "status": "verified",
                    "host_feature_support": config_host_feature_support,
                    "final_output_authority_disclosure": config_final_output_authority_disclosure
                },
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
                    "status_fallback_event": status_fallback_event,
                    "authority_receipt_event": authority_receipt_event
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
        let diagnostics = canonical_release_host_feature_diagnostics(host, profile, false, false);
        let config_host_feature_support = diagnostics.host_feature_support.clone();
        let config_final_output_authority_disclosure =
            diagnostics.final_output_authority_disclosure.clone();
        serde_json::json!({
            "kind": LIVE_FINAL_OUTPUT_RESULT_KIND,
            "result": "incomplete",
            "host": { "kind": host },
            "profile": profile.as_str(),
            "host_feature_support": diagnostics.host_feature_support,
            "final_output_authority_disclosure": diagnostics.final_output_authority_disclosure,
            "evidence": {
                "config_fixture": {
                    "status": "unavailable",
                    "reason": reason,
                    "host_feature_support": config_host_feature_support,
                    "final_output_authority_disclosure": config_final_output_authority_disclosure
                },
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

    fn final_output_unavailable_summary_with_host_identity(
        host: &str,
        profile: IntegrationProfile,
        reason: &str,
        host_version: &str,
        host_executable_sha256: &str,
        volicord_build_id: &str,
    ) -> Value {
        let mut summary = final_output_unavailable_summary(host, profile, reason);
        let diagnostics = canonical_release_host_feature_diagnostics_for_version(
            host,
            Some(host_version),
            profile,
            false,
            false,
        );
        summary["host_feature_support"] = diagnostics.host_feature_support.clone();
        summary["final_output_authority_disclosure"] =
            diagnostics.final_output_authority_disclosure.clone();
        summary["evidence"]["config_fixture"]["host_feature_support"] =
            diagnostics.host_feature_support;
        summary["evidence"]["config_fixture"]["final_output_authority_disclosure"] =
            diagnostics.final_output_authority_disclosure;
        summary["host"] = serde_json::json!({
            "kind": host,
            "version": host_version,
            "executable_sha256": host_executable_sha256
        });
        summary["volicord"] = serde_json::json!({ "build_id": volicord_build_id });
        summary
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
        let config_fixture = evidence
            .get("config_fixture")
            .expect("config_fixture was required above");
        let configuration_verified = match status(config_fixture, "config_fixture")? {
            "verified" => true,
            "unavailable" | "failed" => false,
            other => {
                return Err(io::Error::other(format!(
                    "final-output config fixture cannot use status {other:?}"
                ))
                .into())
            }
        };
        validate_release_host_feature_diagnostics(
            value,
            Some(profile),
            configuration_verified,
            configuration_verified,
        )?;
        let top_level_support = value.get("host_feature_support").ok_or_else(|| {
            io::Error::other("live final-output result has no top-level host_feature_support")
        })?;
        let top_level_disclosure =
            value
                .get("final_output_authority_disclosure")
                .ok_or_else(|| {
                    io::Error::other(
                    "live final-output result has no top-level final_output_authority_disclosure",
                )
                })?;
        if config_fixture.get("host_feature_support") != Some(top_level_support)
            || config_fixture.get("final_output_authority_disclosure") != Some(top_level_disclosure)
        {
            return Err(io::Error::other(
                "live final-output top-level diagnostics do not exactly match config_fixture",
            )
            .into());
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
                        let managed = &event["managed_mcp_observation"];
                        let agent_session_delta = managed["agent_session_delta"]
                            .as_u64()
                            .filter(|delta| {
                                (1..=MAX_MANAGED_MCP_SESSIONS_PER_HOST_TURN).contains(delta)
                            })
                            .ok_or_else(|| {
                                io::Error::other(format!(
                                    "verified Record {branch} has no bounded positive managed MCP AgentSession delta"
                                ))
                            })?;
                        let session_ids = managed["session_ids"].as_array().ok_or_else(|| {
                            io::Error::other(format!(
                                "verified Record {branch} has no managed MCP session-id set"
                            ))
                        })?;
                        let complete_session_ids = managed["complete_session_ids"]
                            .as_array()
                            .filter(|values| !values.is_empty())
                            .ok_or_else(|| {
                                io::Error::other(format!(
                                    "verified Record {branch} has no complete managed MCP session set"
                                ))
                            })?;
                        let unique_session_ids = session_ids
                            .iter()
                            .filter_map(Value::as_str)
                            .collect::<BTreeSet<_>>();
                        let unique_complete_session_ids = complete_session_ids
                            .iter()
                            .filter_map(Value::as_str)
                            .collect::<BTreeSet<_>>();
                        for session_id in session_ids.iter().chain(complete_session_ids) {
                            validate_managed_summary_session_id(
                                &format!("verified Record {branch} session id"),
                                session_id.as_str().ok_or_else(|| {
                                    io::Error::other(format!(
                                        "verified Record {branch} session id must be a string"
                                    ))
                                })?,
                            )?;
                        }
                        if session_ids.len() != usize::try_from(agent_session_delta)?
                            || session_ids
                                .iter()
                                .any(|value| value.as_str().is_none_or(str::is_empty))
                            || unique_session_ids.len() != session_ids.len()
                            || complete_session_ids
                                .iter()
                                .any(|value| value.as_str().is_none_or(str::is_empty))
                            || unique_complete_session_ids.len() != complete_session_ids.len()
                            || complete_session_ids
                                .iter()
                                .any(|complete| !session_ids.contains(complete))
                            || managed["all_new_sessions_validated"] != true
                            || managed["guard_mode"] != IntegrationProfile::Record.as_str()
                            || managed["connection_id"].as_str().is_none_or(str::is_empty)
                        {
                            return Err(io::Error::other(format!(
                                "verified Record {branch} must separate every bounded registered managed MCP AgentSession from non-observing final-output delivery"
                            ))
                            .into());
                        }
                        let lifecycle_events = managed["lifecycle_events"]
                            .as_array()
                            .ok_or_else(|| {
                                io::Error::other(format!(
                                    "verified Record {branch} has no managed MCP lifecycle events"
                                ))
                            })?
                            .iter()
                            .filter_map(Value::as_str)
                            .collect::<Vec<_>>();
                        for required in [
                            "managed_host_startup",
                            "managed_host_initialize_response",
                            "managed_host_tools_list",
                        ] {
                            if !lifecycle_events.contains(&required) {
                                return Err(io::Error::other(format!(
                                    "verified Record {branch} is missing managed MCP lifecycle event {required:?}"
                                ))
                                .into());
                            }
                        }
                        let expected_tool_name =
                            (branch == "status_fallback_event").then_some("volicord.status");
                        if managed["tool_name"].as_str() != expected_tool_name {
                            return Err(io::Error::other(format!(
                                "verified Record {branch} has the wrong managed MCP tool-call binding"
                            ))
                            .into());
                        }
                        for tool_event in
                            ["managed_host_tool_call", "managed_host_tool_call_completed"]
                        {
                            if lifecycle_events.contains(&tool_event)
                                != expected_tool_name.is_some()
                            {
                                return Err(io::Error::other(format!(
                                    "verified Record {branch} has inconsistent {tool_event:?} lifecycle evidence"
                                ))
                                .into());
                            }
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
                        let event = &evidence["actual_host_event"][branch];
                        validate_domain_separated_correlation_id(
                            &format!("verified Detective {branch} guard event id"),
                            event["guard_event_id"].as_str().ok_or_else(|| {
                                io::Error::other(format!(
                                    "verified Detective {branch} has no guard event id"
                                ))
                            })?,
                            "guard_event",
                        )?;
                        validate_managed_summary_session_id(
                            &format!("verified Detective {branch} session id"),
                            event["session_id"].as_str().ok_or_else(|| {
                                io::Error::other(format!(
                                    "verified Detective {branch} has no session id"
                                ))
                            })?,
                        )?;
                        if event["decision"].as_str().is_none_or(str::is_empty)
                            || evidence["actual_host_event"][branch]["source"]
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
        max_agent_session_rowid: i64,
    }

    #[derive(Clone, Copy, Debug)]
    struct StopEventCursor(i64);

    #[derive(Clone, Copy, Debug)]
    struct DiagnosticEventCursor(i64);

    #[derive(Debug, Eq, PartialEq)]
    struct ProjectAuthoritySnapshot {
        state_version: u64,
        task_count: u64,
    }

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
        let guard_events =
            conn.query_row("SELECT COUNT(*) FROM guard_events", [], |row| row.get(0))?;
        let (agent_sessions, max_agent_session_rowid) = conn.query_row(
            "SELECT COUNT(*), COALESCE(MAX(rowid), 0) FROM agent_sessions",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        Ok(GuardObservationCounts {
            guard_events,
            agent_sessions,
            max_agent_session_rowid,
        })
    }

    fn verify_record_managed_mcp_host_turn(
        fixture: &LiveSmokeFixture,
        project_id: &str,
        connection_id: &str,
        before: GuardObservationCounts,
        after: GuardObservationCounts,
        expected_tool_name: Option<&str>,
    ) -> Result<Value, Box<dyn Error>> {
        if after.guard_events != before.guard_events {
            return Err(io::Error::other(
                "Record actual-host final-output handling persisted a GuardEvent",
            )
            .into());
        }
        let agent_session_delta = after
            .agent_sessions
            .checked_sub(before.agent_sessions)
            .filter(|delta| *delta > 0)
            .ok_or_else(|| {
                io::Error::other(format!(
                    "Record actual-host turn did not add a managed MCP AgentSession: before={}, after={}",
                    before.agent_sessions, after.agent_sessions
                ))
            })?;
        if agent_session_delta > MAX_MANAGED_MCP_SESSIONS_PER_HOST_TURN {
            return Err(io::Error::other(format!(
                "Record actual-host turn added an unbounded number of managed MCP AgentSessions: delta={agent_session_delta}, maximum={MAX_MANAGED_MCP_SESSIONS_PER_HOST_TURN}"
            ))
            .into());
        }
        if after.max_agent_session_rowid <= before.max_agent_session_rowid {
            return Err(io::Error::other(
                "Record actual-host AgentSession count advanced without a new rowid",
            )
            .into());
        }

        let project = list_projects(&fixture.runtime_home_path)?
            .into_iter()
            .find(|project| project.project_id == project_id)
            .ok_or_else(|| io::Error::other("live smoke project registration is missing"))?;
        let conn = open_project_state_database_read_only(&project.state_db_path)?;
        let mut statement = conn.prepare(
            "SELECT rowid, session_id, connection_internal_id, host_kind, guard_mode, metadata_json
               FROM agent_sessions
              WHERE project_id = ?1
                AND rowid > ?2
                AND rowid <= ?3
              ORDER BY rowid",
        )?;
        let new_sessions = statement
            .query_map(
                rusqlite::params![
                    project_id,
                    before.max_agent_session_rowid,
                    after.max_agent_session_rowid
                ],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                    ))
                },
            )?
            .collect::<Result<Vec<_>, _>>()?;
        if u64::try_from(new_sessions.len())? != agent_session_delta {
            return Err(io::Error::other(format!(
                "Record actual-host AgentSession delta does not match the bounded rowid window: delta={agent_session_delta}, rows={}",
                new_sessions.len()
            ))
            .into());
        }

        let mut session_ids = Vec::with_capacity(new_sessions.len());
        let mut complete_session_ids = Vec::new();
        let mut observed_lifecycle_events = BTreeSet::new();
        let mut expected_tool_calls_received = 0_u64;
        let mut expected_tool_calls_completed = 0_u64;
        let mut expected_tool_received_session_id = None;
        let mut expected_tool_completed_session_id = None;
        for (_, session_id, observed_connection_id, host_kind, guard_mode, session_metadata_json) in
            new_sessions
        {
            if observed_connection_id != connection_id {
                return Err(io::Error::other(format!(
                    "Record actual-host turn added an AgentSession for unexpected connection {observed_connection_id:?}"
                ))
                .into());
            }
            if host_kind != "codex" {
                return Err(io::Error::other(format!(
                    "Record managed MCP AgentSession used unexpected host_kind {host_kind:?}"
                ))
                .into());
            }
            if guard_mode != IntegrationProfile::Record.as_str() {
                return Err(io::Error::other(format!(
                    "Record managed MCP AgentSession used unexpected guard_mode {guard_mode:?}"
                ))
                .into());
            }
            let session_metadata: Value = serde_json::from_str(&session_metadata_json)?;
            if session_metadata["source"] != "volicord_session_watch"
                || session_metadata["session_watch_initialized"] != true
            {
                return Err(io::Error::other(
                    "Record managed MCP AgentSession is missing its bounded session-watch initialization metadata",
                )
                .into());
            }

            let baseline = latest_watch_baseline_for_session(
                &fixture.runtime_home_path,
                project_id,
                &session_id,
            )?
            .ok_or_else(|| {
                io::Error::other(format!(
                    "Record managed MCP AgentSession {session_id:?} has no canonical session-watch baseline"
                ))
            })?;
            if baseline.connection_internal_id != connection_id {
                return Err(io::Error::other(format!(
                    "Record managed MCP baseline used unexpected connection {:?}",
                    baseline.connection_internal_id
                ))
                .into());
            }
            let baseline_metadata: Value = serde_json::from_str(&baseline.metadata_json)?;
            if baseline_metadata["source"] != "volicord_session_watch"
                || baseline_metadata["coverage_basis"] != "mcp_start"
                || baseline_metadata["launch_origin"] != "managed_host"
                || baseline_metadata["host_kind"] != "codex"
                || baseline_metadata["connection_id"] != connection_id
                || baseline_metadata["project_id"] != project_id
            {
                return Err(io::Error::other(
                    "Record host turn did not persist the expected registered managed MCP lifecycle binding",
                )
                .into());
            }
            let lifecycle_events = baseline_metadata["lifecycle_events"]
                .as_array()
                .ok_or_else(|| io::Error::other("managed MCP lifecycle metadata has no events"))?;
            for event in lifecycle_events {
                if event["connection_id"] != connection_id
                    || event["project_id"] != project_id
                    || event["host_kind"] != "codex"
                    || event["launch_origin"] != "managed_host"
                {
                    return Err(io::Error::other(format!(
                        "Record managed MCP AgentSession {session_id:?} contains a lifecycle event outside its registered managed-host binding"
                    ))
                    .into());
                }
            }
            let lifecycle_event_names = lifecycle_events
                .iter()
                .filter_map(|event| event["lifecycle_event"].as_str())
                .map(str::to_owned)
                .collect::<Vec<_>>();
            if !lifecycle_event_names
                .iter()
                .any(|event| event == "managed_host_startup")
            {
                return Err(io::Error::other(
                    "Record host turn added a managed MCP AgentSession without startup evidence",
                )
                .into());
            }

            for event in lifecycle_events {
                let Some(lifecycle_event) = event["lifecycle_event"].as_str() else {
                    continue;
                };
                let is_received = lifecycle_event == "managed_host_tool_call";
                let is_completed = lifecycle_event == "managed_host_tool_call_completed";
                if !is_received && !is_completed {
                    continue;
                }
                let observed_tool_name = event["tool_name"].as_str();
                if observed_tool_name != expected_tool_name {
                    return Err(io::Error::other(format!(
                        "Record actual-host turn observed unexpected MCP tool call {observed_tool_name:?}; expected {expected_tool_name:?}"
                    ))
                    .into());
                }
                if is_received {
                    expected_tool_calls_received += 1;
                    expected_tool_received_session_id = Some(session_id.clone());
                }
                if is_completed {
                    expected_tool_calls_completed += 1;
                    expected_tool_completed_session_id = Some(session_id.clone());
                }
            }

            let complete_lifecycle = [
                "managed_host_startup",
                "managed_host_initialize_response",
                "managed_host_tools_list",
            ]
            .iter()
            .all(|required| lifecycle_event_names.iter().any(|event| event == required));
            if complete_lifecycle {
                complete_session_ids.push(session_id.clone());
            }
            observed_lifecycle_events.extend(lifecycle_event_names);
            session_ids.push(session_id);
        }

        match expected_tool_name {
            Some(expected_tool_name)
                if expected_tool_calls_received != 1 || expected_tool_calls_completed != 1 =>
            {
                return Err(io::Error::other(format!(
                    "Record host turn must observe exactly one received and completed {expected_tool_name:?} call across its managed MCP sessions: received={expected_tool_calls_received}, completed={expected_tool_calls_completed}"
                ))
                .into());
            }
            Some(expected_tool_name)
                if expected_tool_received_session_id != expected_tool_completed_session_id =>
            {
                return Err(io::Error::other(format!(
                    "Record host turn observed the received and completed {expected_tool_name:?} call in different managed MCP sessions"
                ))
                .into());
            }
            None if expected_tool_calls_received != 0 || expected_tool_calls_completed != 0 => {
                return Err(io::Error::other(
                    "Record AuthorityReceipt host turn unexpectedly called an MCP tool",
                )
                .into());
            }
            _ => {}
        }
        if complete_session_ids.is_empty() {
            return Err(io::Error::other(
                "Record actual-host turn has no complete registered managed MCP lifecycle",
            )
            .into());
        }
        session_ids.sort();
        complete_session_ids.sort();

        Ok(serde_json::json!({
            "agent_session_delta": agent_session_delta,
            "all_new_sessions_validated": true,
            "complete_session_ids": complete_session_ids,
            "connection_id": connection_id,
            "guard_mode": IntegrationProfile::Record.as_str(),
            "lifecycle_events": observed_lifecycle_events.into_iter().collect::<Vec<_>>(),
            "session_ids": session_ids,
            "tool_name": expected_tool_name,
        }))
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

    fn diagnostic_event_cursor(
        fixture: &LiveSmokeFixture,
    ) -> Result<DiagnosticEventCursor, Box<dyn Error>> {
        let path = diagnostics_db_path(&fixture.runtime_home_path);
        if !path.exists() {
            return Ok(DiagnosticEventCursor(0));
        }
        let conn = rusqlite::Connection::open_with_flags(
            path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )?;
        Ok(DiagnosticEventCursor(conn.query_row(
            "SELECT COALESCE(MAX(event_id), 0) FROM diagnostic_events",
            [],
            |row| row.get(0),
        )?))
    }

    fn project_authority_snapshot(
        fixture: &LiveSmokeFixture,
        project_id: &str,
    ) -> Result<ProjectAuthoritySnapshot, Box<dyn Error>> {
        let project = list_projects(&fixture.runtime_home_path)?
            .into_iter()
            .find(|project| project.project_id == project_id)
            .ok_or_else(|| io::Error::other("live smoke project registration is missing"))?;
        let conn = open_project_state_database_read_only(&project.state_db_path)?;
        let state_version = conn.query_row(
            "SELECT state_version FROM project_state WHERE project_id = ?1",
            [project_id],
            |row| row.get(0),
        )?;
        let task_count = conn.query_row(
            "SELECT COUNT(*) FROM tasks WHERE project_id = ?1",
            [project_id],
            |row| row.get(0),
        )?;
        Ok(ProjectAuthoritySnapshot {
            state_version,
            task_count,
        })
    }

    fn observe_and_verify_live_connection_before_task(
        fixture: &LiveSmokeFixture,
        host: &str,
        executable: &Path,
        connection_id: &str,
        result_recorder: &mut LiveResultRecorder,
    ) -> Result<(), Box<dyn Error>> {
        let project_selector = live_fixture_project_id(fixture)?;
        let authority_before = project_authority_snapshot(fixture, &project_selector)?;
        if authority_before.task_count != 0 {
            return Err(io::Error::other(
                "the connection-observation probe requires a project with no Task",
            )
            .into());
        }
        let diagnostic_cursor = diagnostic_event_cursor(fixture)?;
        let prompt = live_final_output_no_active_prompt(&project_selector);
        println!(
            "\n=== Volicord live {host} connection-observation probe ===\nThis authenticated host turn intentionally has no active Volicord Task. It must expose and call the installed Volicord MCP server before the administrative verification step can store a complete Agent Connection result. Approve the repository or MCP entry if the host asks. Do not type credentials or secrets.\n\n{prompt}\n=== end instruction ===\n"
        );
        let status = fixture.run_authenticated_interactive_host(
            host,
            executable,
            &prompt,
            result_recorder,
        )?;
        smoke_note(
            host,
            format!(
                "connection-observation host exited with {}",
                status_text(status)
            ),
        );
        if !status.success() {
            return Err(io::Error::other(format!(
                "the connection-observation {host} process exited unsuccessfully with {}",
                status_text(status)
            ))
            .into());
        }
        let authority_after = project_authority_snapshot(fixture, &project_selector)?;
        if authority_after != authority_before {
            return Err(io::Error::other(format!(
                "the connection-observation probe changed project authority state: before={authority_before:?}, after={authority_after:?}"
            ))
            .into());
        }
        assert_connection_observation_diagnostic(
            fixture,
            connection_id,
            &project_selector,
            diagnostic_cursor,
        )?;
        verify_live_connection_after_host_observation(fixture, host, connection_id)
    }

    fn verify_live_connection_after_host_observation(
        fixture: &LiveSmokeFixture,
        host: &str,
        connection_id: &str,
    ) -> Result<(), Box<dyn Error>> {
        let verification = fixture.run_volicord_with_host_environment([
            "connection",
            "verify",
            host,
            "--repo",
            fixture.repo_arg(),
            "--shared",
            "--json",
        ])?;
        require_success(
            "volicord connection verify after live host observation",
            &verification,
        )?;
        let value = json_stdout(&verification)?;
        let observed_status = value["status"].as_str().unwrap_or("missing");
        let observed_connection_id = value["connection"]["connection_id"]
            .as_str()
            .unwrap_or("missing");
        if observed_status != "complete" || observed_connection_id != connection_id {
            let action_ids = value["actions"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|action| action["id"].as_str())
                .collect::<Vec<_>>();
            return Err(io::Error::other(format!(
                "the post-observation administrative verification did not complete the prepared Agent Connection: status={observed_status:?}, connection_id_matches={}, actions={action_ids:?}",
                observed_connection_id == connection_id
            ))
            .into());
        }
        assert_live_connection_verified(fixture, connection_id)
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

    fn stored_stop_snapshot_for_native_session(
        fixture: &LiveSmokeFixture,
        project_id: &str,
        host: &str,
        connection_id: &str,
        native_session_id: &str,
    ) -> Result<StoredStopSnapshot, Box<dyn Error>> {
        let normalized_host = match host {
            "codex" => "codex",
            "claude-code" | "claude_code" => "claude_code",
            _ => {
                return Err(io::Error::other(format!(
                    "unsupported managed host kind {host:?} for Stop snapshot lookup"
                ))
                .into())
            }
        };
        let session_id =
            managed_host_session_id(normalized_host, connection_id, native_session_id)?;
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
        .optional()?
        .ok_or_else(|| {
            io::Error::other(format!(
                "no stored Stop snapshot exists for project {project_id:?} and session {session_id:?}"
            ))
            .into()
        })
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
        host_feature_diagnostics: &'a ReleaseHostFeatureDiagnostics,
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
            host_feature_diagnostics,
        } = input;
        serde_json::json!({
            "kind": LIVE_CLI_FALLBACK_RESULT_KIND,
            "result": result,
            "host": {
                "kind": identity.host,
                "version": identity.host_version,
                "executable_sha256": identity.host_executable_sha256
            },
            "volicord": {
                "build_id": identity.volicord_build_id
            },
            "connection": {
                "connection_id": identity.connection_id
            },
            "host_feature_support": host_feature_diagnostics.host_feature_support,
            "final_output_authority_disclosure": host_feature_diagnostics.final_output_authority_disclosure,
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

    fn live_cli_fallback_unavailable_summary(
        host: &str,
        host_version: Option<&str>,
        stage: &str,
        reason: &str,
    ) -> Value {
        let diagnostics = canonical_release_host_feature_diagnostics_for_version(
            host,
            host_version,
            IntegrationProfile::Detective,
            false,
            false,
        );
        serde_json::json!({
            "kind": LIVE_CLI_FALLBACK_RESULT_KIND,
            "result": "unavailable",
            "host": { "kind": host },
            "stage": stage,
            "reason": reason,
            "host_feature_support": diagnostics.host_feature_support,
            "final_output_authority_disclosure": diagnostics.final_output_authority_disclosure
        })
    }

    fn validate_live_cli_fallback_result_shape(value: &Value) -> Result<(), Box<dyn Error>> {
        validate_release_host_feature_diagnostics(
            value,
            Some(IntegrationProfile::Detective),
            true,
            true,
        )?;
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

    fn validate_live_user_action_result_shape(value: &Value) -> Result<(), Box<dyn Error>> {
        validate_release_host_feature_diagnostics(
            value,
            Some(IntegrationProfile::Detective),
            true,
            true,
        )?;
        for (pointer, keys) in [
            (
                "",
                &[
                    "kind",
                    "result",
                    "host",
                    "volicord",
                    "connection",
                    "host_feature_support",
                    "final_output_authority_disclosure",
                    "task",
                    "user_action",
                    "choice_consumption",
                    "authority_events",
                    "native_ui",
                    "stop_hook",
                    "authority_receipt",
                    "cli_fallback",
                ][..],
            ),
            ("/host", &["kind", "version", "executable_sha256"][..]),
            ("/volicord", &["build_id"][..]),
            ("/connection", &["connection_id"][..]),
            (
                "/task",
                &["project_id", "task_id", "lifecycle_phase", "state_version"][..],
            ),
            (
                "/user_action",
                &[
                    "user_action_request_id",
                    "selected_option_id",
                    "operator_confirmed_option_id",
                    "stored_choice_matches_operator",
                    "user_channel_basis",
                ][..],
            ),
            (
                "/choice_consumption",
                &[
                    "run_id",
                    "run_kind",
                    "run_marker",
                    "product_file_write_observed",
                    "changed_path_count",
                ][..],
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
                "/native_ui",
                &[
                    "user_action_selector_confirmed",
                    "operator_choice_confirmed",
                    "stop_system_message_authority_receipt_confirmed",
                ][..],
            ),
            (
                "/stop_hook",
                &[
                    "guard_event_id",
                    "session_id",
                    "connection_id",
                    "decision",
                    "decision_observed_from_guard_event",
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
                ][..],
            ),
            ("/cli_fallback", &["verified"][..]),
        ] {
            require_exact_live_evidence_result_keys(value, pointer, keys)?;
        }
        if value["kind"] != LIVE_USER_ACTION_RESULT_KIND || value["result"] != "passed" {
            return Err(io::Error::other(
                "passing native-user-action result has the wrong kind or result",
            )
            .into());
        }
        required_result_string(value, "/host/kind")?;
        required_result_string(value, "/host/version")?;
        validate_lower_hex(
            "native-user-action host executable digest",
            required_result_string(value, "/host/executable_sha256")?,
            &[64],
        )?;
        required_result_string(value, "/volicord/build_id")?;
        let connection_id = required_result_string(value, "/connection/connection_id")?;
        let project_id = required_result_string(value, "/task/project_id")?;
        let task_id = required_result_string(value, "/task/task_id")?;
        required_result_string(value, "/task/lifecycle_phase")?;
        let task_state_version = required_result_u64(value, "/task/state_version")?;
        required_result_string(value, "/user_action/user_action_request_id")?;
        let selected_option = required_result_string(value, "/user_action/selected_option_id")?;
        let operator_option =
            required_result_string(value, "/user_action/operator_confirmed_option_id")?;
        let expected_run_marker =
            run_marker_for_selected_option(selected_option).ok_or_else(|| {
                io::Error::other("native-user-action result stores an unknown choice")
            })?;
        let run_id = required_result_string(value, "/choice_consumption/run_id")?;
        let requested_event_seq =
            required_result_u64(value, "/authority_events/user_action_requested_event_seq")?;
        let resolved_event_seq =
            required_result_u64(value, "/authority_events/user_action_resolved_event_seq")?;
        let run_event_seq = required_result_u64(value, "/authority_events/run_recorded_event_seq")?;
        validate_domain_separated_correlation_id(
            "native-user-action guard event id",
            required_result_string(value, "/stop_hook/guard_event_id")?,
            "guard_event",
        )?;
        validate_managed_summary_session_id(
            "native-user-action session id",
            required_result_string(value, "/stop_hook/session_id")?,
        )?;
        if selected_option != operator_option
            || value["user_action"]["stored_choice_matches_operator"] != true
            || value["user_action"]["user_channel_basis"]
                != VERIFICATION_BASIS_MCP_ELICITATION_USER_CHANNEL
            || value["choice_consumption"]["run_kind"] != "shaping_update"
            || value["choice_consumption"]["run_marker"] != expected_run_marker
            || value["choice_consumption"]["product_file_write_observed"] != false
            || value["choice_consumption"]["changed_path_count"] != 0
            || requested_event_seq == 0
            || !(requested_event_seq < resolved_event_seq && resolved_event_seq < run_event_seq)
            || value["authority_events"]["ordered"] != true
            || value["native_ui"]["user_action_selector_confirmed"] != true
            || value["native_ui"]["operator_choice_confirmed"] != true
            || value["native_ui"]["stop_system_message_authority_receipt_confirmed"] != true
            || value["stop_hook"]["connection_id"] != connection_id
            || value["stop_hook"]["decision"] != "allow"
            || value["stop_hook"]["decision_observed_from_guard_event"] != true
            || value["stop_hook"]["receipt_state_version"] != task_state_version
            || value["stop_hook"]["latest_run_id"] != run_id
            || value["authority_receipt"]["project_id"] != project_id
            || value["authority_receipt"]["task_id"] != task_id
            || value["authority_receipt"]["state_version"] != task_state_version
            || value["authority_receipt"]["latest_run_id"] != run_id
            || value["authority_receipt"]["close_state"] != "ready"
            || value["authority_receipt"]["close_blocker_count"] != 0
            || value["cli_fallback"]["verified"] != false
        {
            return Err(io::Error::other(
                "passing native-user-action result does not preserve its exact semantic evidence",
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
        host_feature_diagnostics: &'a ReleaseHostFeatureDiagnostics,
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
            host_feature_diagnostics,
        } = input;
        serde_json::json!({
            "kind": "live_host_user_action_release_validation",
            "result": result,
            "host": {
                "kind": identity.host,
                "version": identity.host_version,
                "executable_sha256": identity.host_executable_sha256
            },
            "volicord": {
                "build_id": identity.volicord_build_id
            },
            "connection": {
                "connection_id": identity.connection_id
            },
            "host_feature_support": host_feature_diagnostics.host_feature_support,
            "final_output_authority_disclosure": host_feature_diagnostics.final_output_authority_disclosure,
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
        host_feature_diagnostics: &ReleaseHostFeatureDiagnostics,
    ) -> Value {
        serde_json::json!({
            "kind": "live_host_user_action_release_validation",
            "result": "failed_choice_mismatch",
            "host": {
                "kind": identity.host,
                "version": identity.host_version,
                "executable_sha256": identity.host_executable_sha256
            },
            "volicord": {
                "build_id": identity.volicord_build_id
            },
            "connection": {
                "connection_id": identity.connection_id
            },
            "host_feature_support": host_feature_diagnostics.host_feature_support,
            "final_output_authority_disclosure": host_feature_diagnostics.final_output_authority_disclosure,
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
        host_feature_diagnostics: &ReleaseHostFeatureDiagnostics,
    ) -> Value {
        serde_json::json!({
            "kind": "live_host_user_action_release_validation",
            "result": "failed_native_elicitation",
            "host": {
                "kind": identity.host,
                "version": identity.host_version,
                "executable_sha256": identity.host_executable_sha256
            },
            "volicord": {
                "build_id": identity.volicord_build_id
            },
            "connection": {
                "connection_id": identity.connection_id
            },
            "host_feature_support": host_feature_diagnostics.host_feature_support,
            "final_output_authority_disclosure": host_feature_diagnostics.final_output_authority_disclosure,
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

    fn live_user_action_unavailable_summary(
        host: &str,
        host_version: Option<&str>,
        stage: &str,
        reason: &str,
    ) -> Value {
        let diagnostics = canonical_release_host_feature_diagnostics_for_version(
            host,
            host_version,
            IntegrationProfile::Detective,
            false,
            false,
        );
        serde_json::json!({
            "kind": LIVE_USER_ACTION_RESULT_KIND,
            "result": "unavailable",
            "host": { "kind": host },
            "stage": stage,
            "reason": reason,
            "host_feature_support": diagnostics.host_feature_support,
            "final_output_authority_disclosure": diagnostics.final_output_authority_disclosure
        })
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct ObservedReleaseHostCoordinates {
        host_version: String,
        host_executable_sha256: String,
    }

    impl ObservedReleaseHostCoordinates {
        fn new(
            host_version: String,
            host_executable_sha256: String,
        ) -> Result<Self, Box<dyn Error>> {
            bounded_identity(
                "observed host version",
                &host_version,
                MAX_HOST_VERSION_BYTES,
            )?;
            validate_lower_hex(
                "observed host executable digest",
                &host_executable_sha256,
                &[64],
            )?;
            Ok(Self {
                host_version,
                host_executable_sha256,
            })
        }
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct ObservedReleaseHostIdentity {
        host_version: String,
        host_executable_sha256: String,
        volicord_build_id: String,
    }

    impl ObservedReleaseHostIdentity {
        fn new(
            host_version: String,
            host_executable_sha256: String,
            volicord_build_id: String,
        ) -> Result<Self, Box<dyn Error>> {
            ObservedReleaseHostCoordinates::new(
                host_version.clone(),
                host_executable_sha256.clone(),
            )?;
            bounded_identity(
                "observed Volicord build id",
                &volicord_build_id,
                MAX_BUILD_ID_BYTES,
            )?;
            Ok(Self {
                host_version,
                host_executable_sha256,
                volicord_build_id,
            })
        }
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct ObservedInitializedClientInfo(ManagedMcpClientInfo);

    impl ObservedInitializedClientInfo {
        fn new(name: String, version: String) -> Result<Self, Box<dyn Error>> {
            Ok(Self(ManagedMcpClientInfo::new(name, version)?))
        }

        fn name(&self) -> &str {
            self.0.name()
        }

        fn version(&self) -> &str {
            self.0.version()
        }
    }

    fn initialized_client_info_from_watch_metadata(
        metadata: &Value,
        expected_host_kind: &str,
        connection_id: &str,
        project_id: &str,
    ) -> Result<Option<ObservedInitializedClientInfo>, Box<dyn Error>> {
        let exact_managed_binding = metadata["source"] == "volicord_session_watch"
            && metadata["launch_origin"] == "managed_host"
            && metadata["host_kind"] == expected_host_kind
            && metadata["connection_id"] == connection_id
            && metadata["project_id"] == project_id;
        if !exact_managed_binding {
            return Ok(None);
        }
        let initialize_observed = metadata["lifecycle_events"]
            .as_array()
            .is_some_and(|events| {
                events.iter().any(|event| {
                    event["lifecycle_event"] == "managed_host_initialize_response"
                        && event["launch_origin"] == "managed_host"
                        && event["host_kind"] == expected_host_kind
                        && event["connection_id"] == connection_id
                        && event["project_id"] == project_id
                })
            });
        if !initialize_observed {
            return Ok(None);
        }
        match (metadata.get("client_name"), metadata.get("client_version")) {
            (Some(Value::String(name)), Some(Value::String(version))) => Ok(Some(
                ObservedInitializedClientInfo::new(name.clone(), version.clone())?,
            )),
            (None | Some(Value::Null), None | Some(Value::Null)) => Ok(None),
            _ => Err(io::Error::other(
                "managed initialize metadata contains a partial or malformed client identity",
            )
            .into()),
        }
    }

    fn required_initialized_client_info_from_watch_metadata(
        metadata: &Value,
        expected_host_kind: &str,
        connection_id: &str,
        project_id: &str,
    ) -> Result<ObservedInitializedClientInfo, Box<dyn Error>> {
        initialized_client_info_from_watch_metadata(
            metadata,
            expected_host_kind,
            connection_id,
            project_id,
        )?
        .ok_or_else(|| {
            io::Error::other(
                "captured managed baseline does not contain one exact initialized client identity",
            )
            .into()
        })
    }

    fn is_exact_managed_host_turn_baseline(
        baseline: &WatchBaselineRecord,
        expected_project_id: &str,
        expected_connection_id: &str,
    ) -> bool {
        baseline.project_id == expected_project_id
            && baseline.connection_internal_id == expected_connection_id
            && validate_managed_host_session_id(&baseline.session_id).is_ok()
            && baseline.watch_baseline_id == format!("watch_base_managed_{}", baseline.session_id)
    }

    fn managed_baseline_metadata_fingerprint(metadata_json: &str) -> String {
        format!("{:x}", Sha256::digest(metadata_json.as_bytes()))
    }

    #[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
    struct ObservedHostTurnBaseline {
        project_id: String,
        watch_baseline_id: String,
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct ManagedBaselineObservation {
        metadata_fingerprint: String,
        initialize_event_fingerprints: BTreeSet<String>,
    }

    impl ManagedBaselineObservation {
        fn from_metadata_json(metadata_json: &str) -> Result<Self, Box<dyn Error>> {
            let metadata: Value = serde_json::from_str(metadata_json)?;
            let initialize_event_fingerprints = metadata["lifecycle_events"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|event| event["lifecycle_event"] == "managed_host_initialize_response")
                .map(canonical_json_bare_sha256)
                .collect::<Result<BTreeSet<_>, _>>()?;
            Ok(Self {
                metadata_fingerprint: managed_baseline_metadata_fingerprint(metadata_json),
                initialize_event_fingerprints,
            })
        }

        fn records_new_initialize_since(&self, previous: &Self) -> bool {
            self.initialize_event_fingerprints.len() > previous.initialize_event_fingerprints.len()
                && self
                    .initialize_event_fingerprints
                    .is_superset(&previous.initialize_event_fingerprints)
        }
    }

    type ManagedBaselineObservations =
        BTreeMap<ObservedHostTurnBaseline, ManagedBaselineObservation>;
    type ManagedBaselineFingerprints = BTreeMap<ObservedHostTurnBaseline, String>;

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct LiveRunnerEnvironment {
        runner_os: String,
        runner_os_version: String,
        runner_arch: String,
    }

    impl LiveRunnerEnvironment {
        fn measure() -> Result<Self, Box<dyn Error>> {
            fn uname(flag: &str, label: &str) -> Result<String, Box<dyn Error>> {
                let output = Command::new("uname").arg(flag).output()?;
                if !output.status.success() {
                    return Err(io::Error::other(format!(
                        "could not measure live runner {label}: {}",
                        String::from_utf8_lossy(&output.stderr).trim()
                    ))
                    .into());
                }
                let value = String::from_utf8(output.stdout)?.trim().to_owned();
                bounded_identity(label, &value, 256)
            }

            Ok(Self {
                runner_os: uname("-s", "runner_os")?,
                runner_os_version: uname("-r", "runner_os_version")?,
                runner_arch: uname("-m", "runner_arch")?,
            })
        }
    }

    struct LiveResultRecorder {
        result_host: String,
        profile: Option<IntegrationProfile>,
        result_kind: &'static str,
        result_path: Option<PathBuf>,
        release_path_context: Option<ValidationContext>,
        publication_domain: Option<LivePublicationDomain>,
        release_path_validation_failed: bool,
        run_id: String,
        started_at: String,
        started: bool,
        finalized: bool,
        release_candidate: Option<ReleaseCandidate>,
        release_feature: Option<HostFeature>,
        release_requested_verified: Option<bool>,
        installed_host_detected: bool,
        observed_host_coordinates: Option<ObservedReleaseHostCoordinates>,
        observed_volicord_build_id: Option<String>,
        observed_initialized_client_info: Option<ObservedInitializedClientInfo>,
        observed_runtime_home: Option<PathBuf>,
        observed_host_turn_baselines: ManagedBaselineFingerprints,
        runner_environment: LiveRunnerEnvironment,
    }

    impl LiveResultRecorder {
        fn from_env(host: &str) -> Result<Self, Box<dyn Error>> {
            let result_path = required_live_result_path(env::var_os(LIVE_HOST_RESULT_PATH_ENV))?;
            let mut recorder = Self::new(host, Some(result_path))?;
            recorder.load_release_candidate_from_env()?;
            recorder.release_feature = Some(HostFeature::NativeUserAction);
            Ok(recorder)
        }

        fn from_env_for_kind(
            host: &str,
            result_kind: &'static str,
        ) -> Result<Self, Box<dyn Error>> {
            let result_path = required_live_result_path(env::var_os(LIVE_HOST_RESULT_PATH_ENV))?;
            let mut recorder = Self::new_for_kind(host, result_kind, Some(result_path))?;
            recorder.load_release_candidate_from_env()?;
            recorder.release_feature = release_feature_for_result(result_kind, None);
            Ok(recorder)
        }

        fn from_env_for_kind_and_profile(
            recorder_label: &str,
            result_host: &str,
            result_kind: &'static str,
            profile: Option<IntegrationProfile>,
        ) -> Result<Self, Box<dyn Error>> {
            let result_path = required_live_result_path(env::var_os(LIVE_HOST_RESULT_PATH_ENV))?;
            let mut recorder = Self::new_for_kind_and_profile(
                recorder_label,
                result_host,
                result_kind,
                profile,
                Some(result_path),
            )?;
            recorder.load_release_candidate_from_env()?;
            recorder.release_feature = release_feature_for_result(result_kind, profile);
            Ok(recorder)
        }

        fn new(host: &str, result_path: Option<PathBuf>) -> Result<Self, Box<dyn Error>> {
            Self::new_for_kind_and_profile(
                host,
                host,
                LIVE_USER_ACTION_RESULT_KIND,
                Some(IntegrationProfile::Detective),
                result_path,
            )
        }

        fn new_for_kind(
            host: &str,
            result_kind: &'static str,
            result_path: Option<PathBuf>,
        ) -> Result<Self, Box<dyn Error>> {
            let profile = match result_kind {
                LIVE_USER_ACTION_RESULT_KIND
                | LIVE_EVIDENCE_OBSERVATION_RESULT_KIND
                | LIVE_CLI_FALLBACK_RESULT_KIND
                | LIVE_VERIFIED_TOOL_PRODUCER_RESULT_KIND
                | LIVE_REGISTERED_CONNECTION_OBSERVATION_RESULT_KIND => {
                    Some(IntegrationProfile::Detective)
                }
                LIVE_FINAL_OUTPUT_RESULT_KIND => None,
                _ => None,
            };
            Self::new_for_kind_and_profile(host, host, result_kind, profile, result_path)
        }

        fn new_for_kind_and_profile(
            recorder_label: &str,
            result_host: &str,
            result_kind: &'static str,
            profile: Option<IntegrationProfile>,
            result_path: Option<PathBuf>,
        ) -> Result<Self, Box<dyn Error>> {
            let release_path_context = if let Some(path) = result_path.as_deref() {
                let context = release_validation_context()?;
                if release_feature_for_result(result_kind, profile).is_some() {
                    ResultRootLease::prevalidate_cell_path(&context, path)?;
                } else {
                    ResultRootLease::prevalidate_auxiliary_path(&context, path)?;
                }
                Some(context)
            } else {
                None
            };
            let started_at = recorded_at_now()?;
            let run_id = bounded_identity(
                "live validation run_id",
                &format!(
                    "{}-{}-{}",
                    recorder_label.replace('-', "_"),
                    std::process::id(),
                    epoch_duration()?.as_nanos()
                ),
                MAX_VALIDATION_RUN_ID_BYTES,
            )?;
            let mut recorder = Self {
                result_host: result_host.to_owned(),
                profile,
                result_kind,
                result_path,
                release_path_context,
                publication_domain: None,
                release_path_validation_failed: false,
                run_id,
                started_at,
                started: false,
                finalized: false,
                release_candidate: None,
                release_feature: None,
                release_requested_verified: parse_release_requested_verified_claim(
                    env::var_os(RELEASE_REQUEST_VERIFIED_ENV).as_deref(),
                )?,
                installed_host_detected: false,
                observed_host_coordinates: None,
                observed_volicord_build_id: None,
                observed_initialized_client_info: None,
                observed_runtime_home: None,
                observed_host_turn_baselines: BTreeMap::new(),
                runner_environment: LiveRunnerEnvironment::measure()?,
            };
            recorder.started = true;
            Ok(recorder)
        }

        fn required_release_path_context(&self) -> Result<&ValidationContext, Box<dyn Error>> {
            self.release_path_context.as_ref().ok_or_else(|| {
                io::Error::other(
                    "live result recorder has no canonical release-path validation context",
                )
                .into()
            })
        }

        fn load_release_candidate_from_env(&mut self) -> Result<(), Box<dyn Error>> {
            let descriptor_path =
                required_release_candidate_path(env::var_os(RELEASE_CANDIDATE_PATH_ENV));
            let descriptor_path = match descriptor_path {
                Ok(path) => path,
                Err(error) => {
                    self.release_path_validation_failed = true;
                    return Err(error);
                }
            };
            self.load_release_candidate(&descriptor_path)
        }

        fn load_release_candidate(&mut self, descriptor_path: &Path) -> Result<(), Box<dyn Error>> {
            let candidate = ReleaseCandidate::from_descriptor_path(
                self.required_release_path_context()?,
                descriptor_path,
            );
            match candidate {
                Ok(candidate) => {
                    self.release_candidate = Some(candidate);
                    Ok(())
                }
                Err(error) => {
                    self.release_path_validation_failed = true;
                    Err(error)
                }
            }
        }

        fn effective_release_path_context(&self) -> Result<ValidationContext, Box<dyn Error>> {
            let mut context = match &self.release_path_context {
                Some(context) => context.clone(),
                None => release_validation_context()?,
            };
            if let Some(runtime_home) = self.observed_runtime_home.as_deref() {
                context.add_runtime_home(runtime_home)?;
            }
            Ok(context)
        }

        fn validate_retained_release_paths(
            &self,
            context: &ValidationContext,
        ) -> Result<(), Box<dyn Error>> {
            if let Some(path) = self.result_path.as_deref() {
                if release_feature_for_result(self.result_kind, self.profile).is_some() {
                    ResultRootLease::prevalidate_cell_path(context, path)?;
                } else {
                    ResultRootLease::prevalidate_auxiliary_path(context, path)?;
                }
                if let Some(publication_domain) = &self.publication_domain {
                    publication_domain.validate_before_publication(context, path)?;
                }
            }
            if let Some(candidate) = &self.release_candidate {
                candidate.validate_external_paths(context)?;
            }
            Ok(())
        }

        fn require_publication_domain_ready(&mut self) -> Result<(), Box<dyn Error>> {
            if self.release_path_validation_failed {
                return Err(io::Error::other(
                    "live result recorder is poisoned by a rejected publication domain",
                )
                .into());
            }
            let context = self.effective_release_path_context()?;
            let ready = (|| -> Result<(), Box<dyn Error>> {
                self.validate_retained_release_paths(&context)?;
                if self.publication_domain.is_none() {
                    if let Some(path) = self.result_path.as_deref() {
                        let publication_domain =
                            if release_feature_for_result(self.result_kind, self.profile).is_some()
                            {
                                LivePublicationDomain::acquire_for_cell(&context, path)?
                            } else {
                                LivePublicationDomain::acquire_for_auxiliary(&context, path)?
                            };
                        self.publication_domain = Some(publication_domain);
                    }
                }
                self.validate_retained_release_paths(&context)
            })();
            if let Err(error) = ready {
                self.release_path_validation_failed = true;
                return Err(error);
            }
            Ok(())
        }

        fn release_candidate(&self) -> Result<&ReleaseCandidate, Box<dyn Error>> {
            self.release_candidate.as_ref().ok_or_else(|| {
                io::Error::other(
                    "selected live release cell has no exact release-candidate binding",
                )
                .into()
            })
        }

        fn failed_before_completion_summary(&self) -> Value {
            let diagnostics = canonical_release_host_feature_diagnostics_for_profile(
                &self.result_host,
                self.observed_host_coordinates
                    .as_ref()
                    .map(|coordinates| coordinates.host_version.as_str()),
                self.profile,
                false,
                false,
            );
            let mut summary = serde_json::json!({
                "kind": self.result_kind,
                "result": "failed_before_completion",
                "host": { "kind": self.result_host },
                "host_feature_support": diagnostics.host_feature_support,
                "final_output_authority_disclosure": diagnostics.final_output_authority_disclosure
            });
            if self.result_kind == LIVE_FINAL_OUTPUT_RESULT_KIND {
                summary["profile"] = self
                    .profile
                    .map(IntegrationProfile::as_str)
                    .map_or(Value::Null, |profile| Value::String(profile.to_owned()));
            }
            summary
        }

        fn bind_observed_host_identity(
            &mut self,
            identity: ObservedReleaseHostIdentity,
        ) -> Result<(), Box<dyn Error>> {
            self.bind_observed_host_coordinates(ObservedReleaseHostCoordinates::new(
                identity.host_version,
                identity.host_executable_sha256,
            )?)?;
            self.bind_observed_volicord_build_id(identity.volicord_build_id)
        }

        fn mark_installed_host_detected(&mut self) {
            self.installed_host_detected = true;
        }

        fn bind_observed_host_coordinates(
            &mut self,
            coordinates: ObservedReleaseHostCoordinates,
        ) -> Result<(), Box<dyn Error>> {
            self.mark_installed_host_detected();
            if let Some(existing) = &self.observed_host_coordinates {
                if existing != &coordinates {
                    return Err(io::Error::other(
                        "live result recorder cannot replace observed host coordinates",
                    )
                    .into());
                }
            } else {
                self.observed_host_coordinates = Some(coordinates);
            }
            Ok(())
        }

        fn bind_observed_volicord_build_id(
            &mut self,
            volicord_build_id: String,
        ) -> Result<(), Box<dyn Error>> {
            let volicord_build_id = bounded_identity(
                "observed Volicord build id",
                &volicord_build_id,
                MAX_BUILD_ID_BYTES,
            )?;
            if let Some(existing) = &self.observed_volicord_build_id {
                if existing != &volicord_build_id {
                    return Err(io::Error::other(
                        "live result recorder cannot replace an observed Volicord build id",
                    )
                    .into());
                }
            } else {
                self.observed_volicord_build_id = Some(volicord_build_id);
            }
            Ok(())
        }

        fn bind_observed_runtime_home(
            &mut self,
            runtime_home: &Path,
        ) -> Result<(), Box<dyn Error>> {
            let prospective =
                (|| -> Result<Option<(ValidationContext, PathBuf)>, Box<dyn Error>> {
                    let runtime_home = fs::canonicalize(runtime_home)?;
                    if let Some(existing) = &self.observed_runtime_home {
                        if existing != &runtime_home {
                            return Err(io::Error::other(
                                "live result recorder cannot replace its observed runtime home",
                            )
                            .into());
                        }
                        return Ok(None);
                    }
                    let mut context = match &self.release_path_context {
                        Some(context) => context.clone(),
                        None => release_validation_context()?,
                    };
                    context.add_runtime_home(&runtime_home)?;
                    self.validate_retained_release_paths(&context)?;
                    ensure_unregistered_runtime_home_read_only(&runtime_home)?;
                    Ok(Some((context, runtime_home)))
                })();
            match prospective {
                Ok(Some((context, runtime_home))) => {
                    self.release_path_context = Some(context);
                    self.observed_runtime_home = Some(runtime_home);
                    Ok(())
                }
                Ok(None) => Ok(()),
                Err(error) => {
                    self.release_path_validation_failed = true;
                    Err(error)
                }
            }
        }

        fn bind_observed_host_turn_baselines(
            &mut self,
            before: &ManagedBaselineObservations,
            after: &ManagedBaselineObservations,
        ) -> Result<(), Box<dyn Error>> {
            if self.observed_runtime_home.is_none() {
                return Err(io::Error::other(
                    "host-turn baseline observation requires a bound disposable runtime home",
                )
                .into());
            }
            for (baseline, expected_fingerprint) in &self.observed_host_turn_baselines {
                match before
                    .get(baseline)
                    .map(|observation| &observation.metadata_fingerprint)
                {
                    Some(before_fingerprint) if before_fingerprint == expected_fingerprint => {}
                    Some(_) => {
                        return Err(io::Error::other(
                            "captured managed baseline changed before a later host turn",
                        )
                        .into())
                    }
                    None => {
                        return Err(io::Error::other(
                            "captured managed baseline disappeared before a later host turn",
                        )
                        .into())
                    }
                }
                if !after.contains_key(baseline) {
                    return Err(io::Error::other(
                        "captured managed baseline disappeared during a later host turn",
                    )
                    .into());
                }
            }
            let mut next = self.observed_host_turn_baselines.clone();
            for (baseline, after_observation) in after {
                let before_observation = before.get(baseline);
                let metadata_changed = before_observation.is_none_or(|observation| {
                    observation.metadata_fingerprint != after_observation.metadata_fingerprint
                });
                if !metadata_changed {
                    continue;
                }
                let already_retained = next.contains_key(baseline);
                let newly_initialized = before_observation.is_none_or(|observation| {
                    after_observation.records_new_initialize_since(observation)
                });
                if already_retained || newly_initialized {
                    next.insert(
                        baseline.clone(),
                        after_observation.metadata_fingerprint.clone(),
                    );
                }
            }
            self.observed_host_turn_baselines = next;
            Ok(())
        }

        fn bind_observed_initialized_client_info(
            &mut self,
            client_info: ObservedInitializedClientInfo,
        ) -> Result<(), Box<dyn Error>> {
            if let Some(existing) = &self.observed_initialized_client_info {
                if existing != &client_info {
                    return Err(io::Error::other(
                        "live result recorder observed more than one initialized MCP client identity",
                    )
                    .into());
                }
            } else {
                self.observed_initialized_client_info = Some(client_info);
            }
            Ok(())
        }

        fn refresh_observed_initialized_client_info(
            &mut self,
            summary: &Value,
        ) -> Result<(), Box<dyn Error>> {
            let Some(runtime_home) = self.observed_runtime_home.as_deref() else {
                return Ok(());
            };
            let Some(connection_id) = summary
                .pointer("/connection/connection_id")
                .and_then(Value::as_str)
            else {
                return if self.observed_host_turn_baselines.is_empty() {
                    Ok(())
                } else {
                    Err(io::Error::other(
                        "captured managed baselines require an exact release-cell connection",
                    )
                    .into())
                };
            };
            let connection_id = bounded_identity(
                "release-cell observed Agent Connection id",
                connection_id,
                MAX_CONNECTION_ID_BYTES,
            )?;
            let expected_host_kind = match self.result_host.as_str() {
                "codex" => "codex",
                "claude-code" | "claude_code" => "claude_code",
                _ => {
                    return Err(io::Error::other(
                        "initialized-client observation requires a maintained managed host",
                    )
                    .into())
                }
            };
            let mut observed = BTreeSet::new();
            for (observed_baseline, expected_fingerprint) in &self.observed_host_turn_baselines {
                let baseline = watch_baseline(
                    runtime_home,
                    &observed_baseline.project_id,
                    &observed_baseline.watch_baseline_id,
                )?
                .ok_or_else(|| {
                    io::Error::other(
                        "captured managed baseline disappeared before final cell recording",
                    )
                })?;
                let actual_fingerprint =
                    managed_baseline_metadata_fingerprint(&baseline.metadata_json);
                if &actual_fingerprint != expected_fingerprint {
                    return Err(io::Error::other(
                        "captured managed baseline metadata changed before final cell recording",
                    )
                    .into());
                }
                if !is_exact_managed_host_turn_baseline(
                    &baseline,
                    &observed_baseline.project_id,
                    &connection_id,
                ) {
                    return Err(io::Error::other(
                        "captured managed baseline was replaced outside its exact cell coordinates",
                    )
                    .into());
                }
                let metadata: Value = serde_json::from_str(&baseline.metadata_json)?;
                let client_info = required_initialized_client_info_from_watch_metadata(
                    &metadata,
                    expected_host_kind,
                    &connection_id,
                    &observed_baseline.project_id,
                )?;
                observed.insert((
                    client_info.name().to_owned(),
                    client_info.version().to_owned(),
                ));
            }
            if observed.len() > 1 {
                return Err(io::Error::other(
                    "one live release cell observed multiple initialized MCP client identities",
                )
                .into());
            }
            if let Some((name, version)) = observed.into_iter().next() {
                self.bind_observed_initialized_client_info(ObservedInitializedClientInfo::new(
                    name, version,
                )?)?;
            }
            Ok(())
        }

        fn with_observed_host_identity(&self, summary: &Value) -> Result<Value, Box<dyn Error>> {
            if self.observed_host_coordinates.is_none() && self.observed_volicord_build_id.is_none()
            {
                return Ok(summary.clone());
            }
            let mut summary = summary.clone();
            let object = summary.as_object_mut().ok_or_else(|| {
                io::Error::other("live-host result summary must be a JSON object")
            })?;
            if let Some(coordinates) = &self.observed_host_coordinates {
                let host = object
                    .entry("host".to_owned())
                    .or_insert_with(|| serde_json::json!({ "kind": self.result_host }))
                    .as_object_mut()
                    .ok_or_else(|| io::Error::other("live-host result host must be an object"))?;
                match host.get("kind").and_then(Value::as_str) {
                    Some(kind) if kind == self.result_host => {}
                    Some(_) => {
                        return Err(io::Error::other(
                            "live-host result host kind conflicts with the recorder host",
                        )
                        .into())
                    }
                    None => {
                        host.insert("kind".to_owned(), Value::String(self.result_host.clone()));
                    }
                }
                for (key, expected) in [
                    ("version", coordinates.host_version.as_str()),
                    (
                        "executable_sha256",
                        coordinates.host_executable_sha256.as_str(),
                    ),
                ] {
                    match host.get(key) {
                        Some(Value::String(value)) if value == expected => {}
                        Some(_) => {
                            return Err(io::Error::other(format!(
                                "live-host result host {key} conflicts with the observed identity"
                            ))
                            .into())
                        }
                        None => {
                            host.insert(key.to_owned(), Value::String(expected.to_owned()));
                        }
                    }
                }
            }
            if let Some(volicord_build_id) = &self.observed_volicord_build_id {
                let volicord = object
                    .entry("volicord".to_owned())
                    .or_insert_with(|| serde_json::json!({}))
                    .as_object_mut()
                    .ok_or_else(|| {
                        io::Error::other("live-host result volicord must be an object")
                    })?;
                match volicord.get("build_id") {
                    Some(Value::String(value)) if value == volicord_build_id => {}
                    Some(_) => {
                        return Err(io::Error::other(
                            "live-host result build id conflicts with the observed identity",
                        )
                        .into())
                    }
                    None => {
                        volicord.insert(
                            "build_id".to_owned(),
                            Value::String(volicord_build_id.clone()),
                        );
                    }
                }
            }
            Ok(summary)
        }

        fn record_final(&mut self, summary: &Value) -> Result<(), Box<dyn Error>> {
            if self.release_path_validation_failed {
                return Err(io::Error::other(
                    "live result recorder is poisoned by a rejected release path",
                )
                .into());
            }
            self.require_publication_domain_ready()?;
            let path_context = match self.effective_release_path_context() {
                Ok(context) => context,
                Err(error) => {
                    self.release_path_validation_failed = true;
                    return Err(error);
                }
            };
            if let Err(error) = self.validate_retained_release_paths(&path_context) {
                self.release_path_validation_failed = true;
                return Err(error);
            }
            let summary = self.with_observed_host_identity(summary)?;
            validate_terminal_release_host_feature_diagnostics(&summary)?;
            self.refresh_observed_initialized_client_info(&summary)?;
            let summary = self.with_validation_run(&summary)?;
            let serialized = serialize_live_host_result(&summary)?;
            let release_feature = self.release_feature;
            let result_path = self.result_path.clone();
            let rendered = match (release_feature, result_path.as_deref()) {
                (Some(feature), Some(cell_path)) => self.write_release_cell(
                    &path_context,
                    feature,
                    cell_path,
                    &summary,
                    &serialized,
                )?,
                (_, Some(path)) => {
                    let publication_domain = self.publication_domain.as_ref().ok_or_else(|| {
                        io::Error::other("live result recorder has no publication domain")
                    })?;
                    write_new_live_host_result(
                        &path_context,
                        publication_domain,
                        path,
                        &serialized,
                    )?;
                    serialized
                }
                (_, None) => serialized,
            };
            if let Some(publication_domain) = self.publication_domain.as_mut() {
                if let Err(error) = publication_domain.complete_publication_attempt(&path_context) {
                    self.release_path_validation_failed = true;
                    return Err(error);
                }
            }
            println!("{rendered}");
            self.finalized = true;
            self.publication_domain.take();
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
                    "recorded_at": recorded_at_now()?,
                    "client_name": self
                        .observed_initialized_client_info
                        .as_ref()
                        .map(ObservedInitializedClientInfo::name),
                    "client_version": self
                        .observed_initialized_client_info
                        .as_ref()
                        .map(ObservedInitializedClientInfo::version)
                }),
            );
            Ok(summary)
        }

        fn write_release_cell(
            &mut self,
            path_context: &ValidationContext,
            feature: HostFeature,
            cell_path: &Path,
            summary: &Value,
            evidence_serialized: &str,
        ) -> Result<String, Box<dyn Error>> {
            let candidate = self.release_candidate()?.clone();
            candidate.validate_with_context(path_context)?;
            let host_kind = summary
                .pointer("/host/kind")
                .and_then(Value::as_str)
                .ok_or_else(|| io::Error::other("release-cell evidence has no host kind"))?;
            let (canonical_host_kind, maintained_host_kind) = match host_kind {
                "codex" => ("codex", HostKind::Codex),
                "claude-code" | "claude_code" => ("claude_code", HostKind::ClaudeCode),
                _ => {
                    return Err(io::Error::other(
                        "release-cell evidence names an unsupported managed host",
                    )
                    .into())
                }
            };
            let host_version = match summary.pointer("/host/version") {
                Some(Value::String(value)) => Some(bounded_identity(
                    "release-cell host_version",
                    value,
                    MAX_HOST_VERSION_BYTES,
                )?),
                Some(Value::Null) | None => None,
                Some(_) => {
                    return Err(io::Error::other(
                        "release-cell host version must be a string or null",
                    )
                    .into())
                }
            };
            let host_executable_sha256 = match summary.pointer("/host/executable_sha256") {
                Some(Value::String(value)) => {
                    validate_lower_hex("host_executable_sha256", value, &[64])?;
                    Some(value.clone())
                }
                Some(Value::Null) | None => None,
                Some(_) => {
                    return Err(io::Error::other(
                        "release-cell host executable digest must be a string or null",
                    )
                    .into())
                }
            };
            if host_version.is_some() != host_executable_sha256.is_some() {
                return Err(io::Error::other(
                    "release-cell host version and executable digest must both be present or both be null",
                )
                .into());
            }
            if self.installed_host_detected && host_version.is_none() {
                return Err(io::Error::other(
                    "an installed host cannot be recorded with a null release availability coordinate",
                )
                .into());
            }
            let initialized_client_info = match (
                summary.pointer("/validation_run/client_name"),
                summary.pointer("/validation_run/client_version"),
            ) {
                (Some(Value::String(name)), Some(Value::String(version))) => Some(
                    ObservedInitializedClientInfo::new(name.clone(), version.clone())?,
                ),
                (Some(Value::Null), Some(Value::Null)) => None,
                _ => {
                    return Err(io::Error::other(
                        "release-cell client_name and client_version must both be strings or both be null",
                    )
                    .into())
                }
            };
            if host_version.is_none() && initialized_client_info.is_some() {
                return Err(io::Error::other(
                    "release-cell initialized client identity requires an available host coordinate",
                )
                .into());
            }
            let client_name = initialized_client_info
                .as_ref()
                .map(|client_info| client_info.name().to_owned());
            let client_version = initialized_client_info
                .as_ref()
                .map(|client_info| client_info.version().to_owned());
            let adapter_version = match summary.pointer("/volicord/build_id") {
                Some(Value::String(value)) => {
                    bounded_identity("release-cell adapter_version", value, MAX_BUILD_ID_BYTES)?
                }
                Some(Value::Null) | None => candidate.adapter_build_id(path_context)?,
                Some(_) => {
                    return Err(io::Error::other(
                        "release-cell adapter build id must be a string or null",
                    )
                    .into())
                }
            };
            let adapter_profile = match feature {
                HostFeature::RecordFinalOutput | HostFeature::DetectiveFinalOutput => summary
                    .get("profile")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        io::Error::other("final-output release cell has no exact profile")
                    })?,
                _ => IntegrationProfile::Detective.as_str(),
            };
            let implementation = host_feature_implementation_for_version(
                maintained_host_kind,
                host_version.as_deref(),
                feature,
            );
            let static_unsupported = implementation == HostFeatureImplementation::UnsupportedByHost;
            let host_identity_available = host_version.is_some();
            let client_identity_exact = initialized_client_info.as_ref().is_some_and(|client| {
                host_version.as_deref() == Some(client.version())
                    && (maintained_host_kind != HostKind::Codex
                        || host_version.as_deref() != Some(REVIEWED_CODEX_HOST_VERSION)
                        || client.name() == REVIEWED_CODEX_MCP_CLIENT_NAME)
            });
            let requested_verified = resolve_release_requested_verified(
                self.release_requested_verified,
                static_unsupported,
                host_identity_available,
            )?;
            let completed_pass = summary.get("result").and_then(Value::as_str) == Some("passed");
            if completed_pass {
                validate_release_cell_passed_summary(feature, summary)?;
            }
            let assertions = release_cell_assertions(feature, static_unsupported, completed_pass);
            let evidence_path = if static_unsupported {
                None
            } else {
                Some(release_evidence_path(cell_path)?)
            };
            let evidence_sha256 = evidence_path
                .as_ref()
                .map(|_| serialized_live_host_result_sha256(evidence_serialized));
            let (evidence_artifact_path, evidence_artifact_sha256) =
                match (evidence_path.as_ref(), evidence_sha256.as_ref()) {
                    (Some(path), Some(sha256)) => (
                        Value::String(path_text(path)),
                        Value::String(sha256.clone()),
                    ),
                    (None, None) => (Value::Null, Value::Null),
                    _ => unreachable!("evidence path and digest are derived together"),
                };
            let claimed_status = if static_unsupported {
                "unsupported_by_host"
            } else if completed_pass && client_identity_exact {
                "verified"
            } else {
                "implemented_unverified"
            };
            let recorded_at = recorded_at_now()?;
            let cell = serde_json::json!({
                "schema": RELEASE_CELL_SCHEMA,
                "candidate_id": candidate.candidate_id,
                "binary_sha256": candidate.binary_sha256,
                "source_revision": candidate.source_revision,
                "target_triple": candidate.target_triple,
                "release_profile": candidate.release_profile,
                "host_kind": canonical_host_kind,
                "host_version": host_version,
                "client_name": client_name,
                "client_version": client_version,
                "adapter_profile": adapter_profile,
                "adapter_version": adapter_version,
                "feature": feature.as_str(),
                "implementation_disposition": if static_unsupported { "unsupported_by_host" } else { "implemented" },
                "requested_verified": requested_verified,
                "claimed_status": claimed_status,
                "run_state": if static_unsupported {
                    "not_applicable"
                } else if host_identity_available {
                    "completed"
                } else {
                    "ignored"
                },
                "started_at": self.started_at,
                "recorded_at": recorded_at,
                "environment": {
                    "runner_os": self.runner_environment.runner_os,
                    "runner_os_version": self.runner_environment.runner_os_version,
                    "runner_arch": self.runner_environment.runner_arch,
                    "host_executable_sha256": host_executable_sha256,
                    "host_kind": canonical_host_kind,
                    "host_version": host_version,
                    "client_name": client_name,
                    "client_version": client_version,
                    "adapter_profile": adapter_profile,
                    "adapter_version": adapter_version
                },
                "assertions": assertions,
                "evidence_artifact_path": evidence_artifact_path,
                "evidence_artifact_sha256": evidence_artifact_sha256
            });
            let serialized = serde_json::to_string(&cell)?;
            if serialized.len() > 1024 * 1024 {
                return Err(io::Error::other("release cell exceeds the 1 MiB bound").into());
            }
            candidate.validate_with_context(path_context)?;
            let publication = (|| -> Result<(), Box<dyn Error>> {
                let publication_domain = self.publication_domain.as_ref().ok_or_else(|| {
                    io::Error::other("release-cell recorder has no publication domain")
                })?;
                publication_domain.validate_before_publication(path_context, cell_path)?;
                if let Some(expected_evidence_path) = evidence_path {
                    let evidence_stage = publication_domain.stage(
                        path_context,
                        &expected_evidence_path,
                        evidence_serialized,
                        MAX_LIVE_HOST_RESULT_BYTES,
                    )?;
                    if Some(evidence_stage.sha256()) != evidence_sha256.as_deref() {
                        return Err(io::Error::other(
                            "held release evidence stage digest changed before publication",
                        )
                        .into());
                    }
                    let cell_stage = publication_domain.stage(
                        path_context,
                        cell_path,
                        &serialized,
                        1024 * 1024,
                    )?;
                    candidate.validate_with_context(path_context)?;
                    publication_domain.validate_attached(path_context)?;
                    evidence_stage.publish(path_context, publication_domain)?;
                    cell_stage.publish(path_context, publication_domain)?;
                } else {
                    let cell_stage = publication_domain.stage(
                        path_context,
                        cell_path,
                        &serialized,
                        1024 * 1024,
                    )?;
                    candidate.validate_with_context(path_context)?;
                    cell_stage.publish(path_context, publication_domain)?;
                }
                Ok(())
            })();
            if let Err(error) = publication {
                self.release_path_validation_failed = true;
                return Err(error);
            }
            Ok(serialized)
        }
    }

    fn ensure_unregistered_runtime_home_read_only(
        runtime_home: &Path,
    ) -> Result<(), Box<dyn Error>> {
        match inspect_runtime_home(runtime_home).registry {
            DatabaseInspection::Missing { .. } => Ok(()),
            DatabaseInspection::Present(snapshot) if snapshot.projects.is_empty() => Ok(()),
            DatabaseInspection::Present(_) => Err(io::Error::other(
                "live result recorder must bind a disposable runtime home before project registration",
            )
            .into()),
            DatabaseInspection::Unsupported { .. } => Err(io::Error::other(
                "live result recorder cannot bind an unsupported Runtime Home registry",
            )
            .into()),
            DatabaseInspection::Malformed { .. } => Err(io::Error::other(
                "live result recorder cannot bind a malformed Runtime Home registry",
            )
            .into()),
            DatabaseInspection::Unreadable { .. } => Err(io::Error::other(
                "live result recorder cannot bind an unreadable Runtime Home registry",
            )
            .into()),
        }
    }

    fn release_feature_for_result(
        result_kind: &str,
        profile: Option<IntegrationProfile>,
    ) -> Option<HostFeature> {
        match result_kind {
            LIVE_USER_ACTION_RESULT_KIND => Some(HostFeature::NativeUserAction),
            LIVE_EVIDENCE_OBSERVATION_RESULT_KIND => Some(HostFeature::LocalWebUserChannel),
            LIVE_VERIFIED_TOOL_PRODUCER_RESULT_KIND => Some(HostFeature::VerifiedToolProducer),
            LIVE_REGISTERED_CONNECTION_OBSERVATION_RESULT_KIND => {
                Some(HostFeature::RegisteredConnectionObservation)
            }
            LIVE_FINAL_OUTPUT_RESULT_KIND => match profile {
                Some(IntegrationProfile::Record) => Some(HostFeature::RecordFinalOutput),
                Some(IntegrationProfile::Detective) => Some(HostFeature::DetectiveFinalOutput),
                None => None,
            },
            _ => None,
        }
    }

    fn validate_release_cell_passed_summary(
        feature: HostFeature,
        summary: &Value,
    ) -> Result<(), Box<dyn Error>> {
        let mut evidence = summary.clone();
        let object = evidence.as_object_mut().ok_or_else(|| {
            io::Error::other("release-cell evidence summary must be a JSON object")
        })?;
        object.remove("validation_run");
        match feature {
            HostFeature::NativeUserAction => validate_live_user_action_result_shape(&evidence),
            HostFeature::LocalWebUserChannel => {
                validate_live_evidence_observation_result_shape(&evidence)
            }
            HostFeature::VerifiedToolProducer | HostFeature::RegisteredConnectionObservation => {
                validate_live_producer_result_shape(&evidence, feature)
            }
            HostFeature::RecordFinalOutput => {
                validate_final_output_result_shape(&evidence, IntegrationProfile::Record)
            }
            HostFeature::DetectiveFinalOutput => {
                validate_final_output_result_shape(&evidence, IntegrationProfile::Detective)
            }
        }
    }

    fn release_cell_assertions(
        feature: HostFeature,
        static_unsupported: bool,
        passed: bool,
    ) -> Vec<Value> {
        let mut assertion_ids = if static_unsupported {
            vec!["static_unsupported_by_host"]
        } else {
            match feature {
                HostFeature::NativeUserAction => vec![
                    "actual_host_session",
                    "native_user_selector_observed",
                    "operator_choice_confirmed",
                    "same_connection_resume",
                    "authority_receipt_observed",
                ],
                HostFeature::LocalWebUserChannel => vec![
                    "actual_host_session",
                    "trusted_capability_current",
                    "host_owned_surface_observed",
                    "model_visible_payload_absence_observed",
                    "browser_submission_observed",
                    "same_connection_resume",
                    "strong_evidence_close_chain",
                ],
                HostFeature::VerifiedToolProducer => vec![
                    "actual_host_tool_event",
                    "intent_precedes_source",
                    "exact_session_connection_actor_scope_baseline",
                    "capture_receipt_bound",
                    "strong_producer_chain",
                    "criterion_coverage_projected",
                    "negative_rejections_zero_effect",
                ],
                HostFeature::RegisteredConnectionObservation => vec![
                    "actual_host_connection_event",
                    "intent_precedes_source",
                    "exact_session_connection_actor_scope_baseline",
                    "capture_receipt_bound",
                    "strong_producer_chain",
                    "criterion_coverage_projected",
                    "negative_rejections_zero_effect",
                ],
                HostFeature::RecordFinalOutput => vec![
                    "actual_host_session",
                    "authority_display_observed",
                    "authenticated_exact_replay_observed",
                ],
                HostFeature::DetectiveFinalOutput => vec![
                    "actual_host_session",
                    "authority_display_observed",
                    "authenticated_exact_replay_observed",
                    "block_finalization_observed",
                ],
            }
        };
        assertion_ids.sort_unstable();
        assertion_ids
            .into_iter()
            .map(|assertion_id| {
                let assertion_passed = static_unsupported || passed;
                let mut assertion = serde_json::json!({
                    "assertion_id": assertion_id,
                    "passed": assertion_passed
                });
                if !assertion_passed {
                    assertion["finding_codes"] = serde_json::json!(["live_cell_incomplete"]);
                }
                assertion
            })
            .collect()
    }

    fn release_evidence_path(cell_path: &Path) -> Result<PathBuf, Box<dyn Error>> {
        let cell_dir = cell_path
            .parent()
            .ok_or_else(|| io::Error::other("release cell path has no parent directory"))?;
        let result_root = cell_dir
            .parent()
            .ok_or_else(|| io::Error::other("release cell directory has no result-root parent"))?;
        let evidence_dir = result_root.join("evidence");
        let file_name = cell_path
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| io::Error::other("release cell filename must be UTF-8"))?;
        Ok(evidence_dir.join(format!("{file_name}.evidence.json")))
    }

    struct PinnedLiveResultDirectory {
        file: fs::File,
        path: PathBuf,
        device: u64,
        inode: u64,
    }

    impl PinnedLiveResultDirectory {
        fn open(context: &ValidationContext, path: &Path) -> Result<Self, Box<dyn Error>> {
            context.validate_existing_directory(path)?;
            let file = fs::File::open(path)?;
            let metadata = file.metadata()?;
            if !metadata.is_dir() {
                return Err(io::Error::other(format!(
                    "live publication directory is not a directory: {}",
                    path.display()
                ))
                .into());
            }
            let directory = Self {
                file,
                path: path.to_path_buf(),
                device: metadata.dev(),
                inode: metadata.ino(),
            };
            directory.validate_attached(context)?;
            Ok(directory)
        }

        fn try_clone(&self) -> Result<Self, Box<dyn Error>> {
            Ok(Self {
                file: self.file.try_clone()?,
                path: self.path.clone(),
                device: self.device,
                inode: self.inode,
            })
        }

        fn validate_attached(&self, context: &ValidationContext) -> Result<(), Box<dyn Error>> {
            context.validate_existing_directory(&self.path)?;
            let metadata = fs::symlink_metadata(&self.path)?;
            if metadata.file_type().is_symlink()
                || !metadata.is_dir()
                || metadata.dev() != self.device
                || metadata.ino() != self.inode
            {
                return Err(io::Error::other(format!(
                    "live publication directory was replaced: {}",
                    self.path.display()
                ))
                .into());
            }
            Ok(())
        }
    }

    struct LivePublicationDomain {
        lease: ResultRootLease,
        output_directory: PinnedLiveResultDirectory,
        evidence_directory: Option<PinnedLiveResultDirectory>,
    }

    impl LivePublicationDomain {
        fn acquire_for_cell(
            context: &ValidationContext,
            cell_path: &Path,
        ) -> Result<Self, Box<dyn Error>> {
            let lease = ResultRootLease::acquire_exclusive_for_cell_path(context, cell_path)?;
            let output_directory =
                PinnedLiveResultDirectory::open(context, lease.output_directory())?;
            let evidence_directory = Some(PinnedLiveResultDirectory::open(
                context,
                lease.evidence_directory().ok_or_else(|| {
                    io::Error::other("release-cell lease has no evidence directory")
                })?,
            )?);
            let mut domain = Self {
                lease,
                output_directory,
                evidence_directory,
            };
            domain.validate_before_publication(context, cell_path)?;
            domain.lease.begin_publication_attempt()?;
            domain.validate_before_publication(context, cell_path)?;
            Ok(domain)
        }

        fn acquire_for_auxiliary(
            context: &ValidationContext,
            output_path: &Path,
        ) -> Result<Self, Box<dyn Error>> {
            let lease =
                ResultRootLease::acquire_exclusive_for_auxiliary_path(context, output_path)?;
            let output_directory =
                PinnedLiveResultDirectory::open(context, lease.output_directory())?;
            let domain = Self {
                lease,
                output_directory,
                evidence_directory: None,
            };
            domain.validate_before_publication(context, output_path)?;
            Ok(domain)
        }

        fn validate_attached(&self, context: &ValidationContext) -> Result<(), Box<dyn Error>> {
            self.lease.validate_attached(context)?;
            self.output_directory.validate_attached(context)?;
            if let Some(directory) = &self.evidence_directory {
                directory.validate_attached(context)?;
            }
            Ok(())
        }

        fn validate_before_publication(
            &self,
            context: &ValidationContext,
            output_path: &Path,
        ) -> Result<(), Box<dyn Error>> {
            self.validate_attached(context)?;
            if output_path.parent() != Some(self.output_directory.path.as_path()) {
                return Err(io::Error::other(
                    "live result is not a direct child of its pinned output directory",
                )
                .into());
            }
            context.validate_new_output(output_path)?;
            if let Some(evidence_directory) = &self.evidence_directory {
                let evidence_path = release_evidence_path(output_path)?;
                if evidence_path.parent() != Some(evidence_directory.path.as_path()) {
                    return Err(io::Error::other(
                        "release evidence is not a direct child of its pinned evidence directory",
                    )
                    .into());
                }
                context.validate_new_output(&evidence_path)?;
            }
            Ok(())
        }

        fn stage(
            &self,
            context: &ValidationContext,
            final_path: &Path,
            serialized: &str,
            max_bytes: usize,
        ) -> Result<StagedLiveHostResult, Box<dyn Error>> {
            let directory = if final_path.parent() == Some(self.output_directory.path.as_path()) {
                &self.output_directory
            } else if self
                .evidence_directory
                .as_ref()
                .is_some_and(|directory| final_path.parent() == Some(directory.path.as_path()))
            {
                self.evidence_directory
                    .as_ref()
                    .expect("evidence directory was matched")
            } else {
                return Err(io::Error::other(
                    "live publication final path is outside its pinned directories",
                )
                .into());
            };
            StagedLiveHostResult::create(context, directory, final_path, serialized, max_bytes)
        }

        fn complete_publication_attempt(
            &mut self,
            context: &ValidationContext,
        ) -> Result<(), Box<dyn Error>> {
            self.validate_attached(context)?;
            self.lease.complete_publication_attempt()?;
            Ok(())
        }
    }

    struct StagedLiveHostResult {
        _file: fs::File,
        directory: PinnedLiveResultDirectory,
        stage_name: OsString,
        final_name: OsString,
        final_path: PathBuf,
        sha256: String,
    }

    impl StagedLiveHostResult {
        fn create(
            context: &ValidationContext,
            directory: &PinnedLiveResultDirectory,
            final_path: &Path,
            serialized: &str,
            max_bytes: usize,
        ) -> Result<Self, Box<dyn Error>> {
            directory.validate_attached(context)?;
            context.validate_new_output(final_path)?;
            if final_path.parent() != Some(directory.path.as_path()) {
                return Err(io::Error::other(
                    "staged live result is not a direct child of its pinned directory",
                )
                .into());
            }
            let final_name = final_path
                .file_name()
                .ok_or_else(|| io::Error::other("live result final path has no filename"))?
                .to_os_string();
            let mut expected = serialized.as_bytes().to_vec();
            expected.push(b'\n');
            if expected.len() > max_bytes {
                return Err(io::Error::other(format!(
                    "staged live result exceeds the {max_bytes}-byte limit"
                ))
                .into());
            }

            let (stage_name, descriptor) = (0..64)
                .find_map(|_| {
                    let stage_name = random_live_stage_name().ok()?;
                    match rustix::fs::openat(
                        &directory.file,
                        &stage_name,
                        rustix::fs::OFlags::RDWR
                            | rustix::fs::OFlags::CREATE
                            | rustix::fs::OFlags::EXCL
                            | rustix::fs::OFlags::CLOEXEC
                            | rustix::fs::OFlags::NOFOLLOW,
                        rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR,
                    ) {
                        Ok(descriptor) => Some(Ok((stage_name, descriptor))),
                        Err(error) if error == rustix::io::Errno::EXIST => None,
                        Err(error) => Some(Err(io::Error::from(error))),
                    }
                })
                .transpose()?
                .ok_or_else(|| {
                    io::Error::other("could not allocate a private live-result stage")
                })?;
            let mut file = fs::File::from(descriptor);
            let stage_path = directory.path.join(&stage_name);
            let stage_result = (|| -> Result<String, Box<dyn Error>> {
                file.write_all(&expected)?;
                file.sync_all()?;
                file.seek(SeekFrom::Start(0))?;
                let mut observed = Vec::with_capacity(expected.len());
                Read::by_ref(&mut file)
                    .take(max_bytes as u64 + 1)
                    .read_to_end(&mut observed)?;
                if observed != expected {
                    return Err(io::Error::other(
                        "held live-result stage bytes changed before publication",
                    )
                    .into());
                }
                Ok(format!("{:x}", Sha256::digest(&observed)))
            })();
            let sha256 = stage_result.map_err(|error| {
                io::Error::other(format!(
                    "cannot complete private live-result stage {}: {error}",
                    stage_path.display()
                ))
            })?;

            Ok(Self {
                _file: file,
                directory: directory.try_clone()?,
                stage_name,
                final_name,
                final_path: final_path.to_path_buf(),
                sha256,
            })
        }

        fn sha256(&self) -> &str {
            &self.sha256
        }

        fn stage_path(&self) -> PathBuf {
            self.directory.path.join(&self.stage_name)
        }

        fn publish(
            self,
            context: &ValidationContext,
            domain: &LivePublicationDomain,
        ) -> Result<(), Box<dyn Error>> {
            domain.validate_attached(context)?;
            self.directory.validate_attached(context)?;
            context.validate_new_output(&self.final_path)?;
            rename_live_stage_no_replace(&self.directory.file, &self.stage_name, &self.final_name)
                .map_err(|error| {
                    io::Error::other(format!(
                        "cannot publish staged live result {} as {}: {error}",
                        self.stage_path().display(),
                        self.final_path.display()
                    ))
                })?;
            self.directory.file.sync_all()?;
            domain.validate_attached(context)?;
            Ok(())
        }
    }

    fn random_live_stage_name() -> io::Result<OsString> {
        let mut random = [0_u8; 16];
        getrandom::fill(&mut random)
            .map_err(|error| io::Error::other(format!("stage randomness failed: {error}")))?;
        let token = random
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        Ok(OsString::from(format!(".volicord-live-stage-{token}")))
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn rename_live_stage_no_replace(
        parent: &fs::File,
        source: &OsStr,
        destination: &OsStr,
    ) -> io::Result<()> {
        use rustix::fs::{renameat_with, RenameFlags};

        renameat_with(parent, source, parent, destination, RenameFlags::NOREPLACE)
            .map_err(io::Error::from)
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    fn rename_live_stage_no_replace(
        _parent: &fs::File,
        _source: &OsStr,
        _destination: &OsStr,
    ) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "atomic no-replace live-result publication is unavailable on this Unix platform",
        ))
    }
    fn required_live_result_path(value: Option<OsString>) -> Result<PathBuf, Box<dyn Error>> {
        value
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .ok_or_else(|| {
            io::Error::other(format!(
                "{LIVE_HOST_RESULT_PATH_ENV} must name a new path satisfying the canonical external release-path policy"
            ))
            .into()
        })
    }

    fn required_release_candidate_path(value: Option<OsString>) -> Result<PathBuf, Box<dyn Error>> {
        value
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .ok_or_else(|| {
                io::Error::other(format!(
                    "{RELEASE_CANDIDATE_PATH_ENV} must name the exact external release-candidate descriptor"
                ))
                .into()
            })
    }

    fn parse_release_requested_verified_claim(
        value: Option<&OsStr>,
    ) -> Result<Option<bool>, Box<dyn Error>> {
        match value.and_then(OsStr::to_str) {
            None if value.is_none() => Ok(None),
            Some("0") => Ok(Some(false)),
            Some("1") => Ok(Some(true)),
            Some(_) | None => Err(io::Error::other(format!(
                "{RELEASE_REQUEST_VERIFIED_ENV} must be exactly `0` or `1` when present"
            ))
            .into()),
        }
    }

    fn resolve_release_requested_verified(
        explicit: Option<bool>,
        static_unsupported: bool,
        _host_identity_available: bool,
    ) -> Result<bool, Box<dyn Error>> {
        if static_unsupported {
            return match explicit {
                Some(true) => Err(io::Error::other(format!(
                    "{RELEASE_REQUEST_VERIFIED_ENV}=1 is forbidden for a statically unsupported release cell"
                ))
                .into()),
                Some(false) | None => Ok(false),
            };
        }
        match explicit {
            Some(value) => Ok(value),
            None => Ok(true),
        }
    }

    impl Drop for LiveResultRecorder {
        fn drop(&mut self) {
            if !self.started || self.finalized || self.result_path.is_none() {
                return;
            }
            let summary = self.failed_before_completion_summary();
            let _ = self.record_final(&summary);
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

    fn serialized_live_host_result_sha256(serialized: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(serialized.as_bytes());
        hasher.update(b"\n");
        format!("{:x}", hasher.finalize())
    }

    fn release_validation_context() -> Result<ValidationContext, Box<dyn Error>> {
        Ok(ValidationContext::from_process(&env::current_dir()?)?)
    }

    fn write_new_live_host_result(
        context: &ValidationContext,
        publication_domain: &LivePublicationDomain,
        path: &Path,
        serialized: &str,
    ) -> Result<(), Box<dyn Error>> {
        publication_domain.validate_before_publication(context, path)?;
        publication_domain
            .stage(context, path, serialized, MAX_LIVE_HOST_RESULT_BYTES)?
            .publish(context, publication_domain)
    }

    fn epoch_duration() -> Result<Duration, Box<dyn Error>> {
        Ok(SystemTime::now().duration_since(UNIX_EPOCH)?)
    }

    fn recorded_at_now() -> Result<String, Box<dyn Error>> {
        let timestamp =
            DateTime::<Utc>::from(SystemTime::now()).to_rfc3339_opts(SecondsFormat::Secs, true);
        validate_canonical_second_timestamp("live validation recorded_at", &timestamp)?;
        bounded_identity(
            "live validation recorded_at",
            &timestamp,
            MAX_RECORDED_AT_BYTES,
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
        bounded_identity("host version", version, MAX_HOST_VERSION_BYTES)
    }

    fn canonical_codex_version_summary(output: &TimedOutput) -> Result<String, Box<dyn Error>> {
        let stdout = stdout(output);
        let stderr = stderr(output);
        if !stderr.is_empty() {
            return Err(io::Error::other(
                "Codex --version must not write a second version envelope to stderr",
            )
            .into());
        }
        let envelope = stdout
            .strip_suffix('\n')
            .ok_or_else(|| io::Error::other("Codex --version must end with exactly one LF"))?;
        if envelope.contains('\n') || envelope.contains('\r') {
            return Err(io::Error::other(
                "Codex --version must contain exactly one canonical line",
            )
            .into());
        }
        let version = canonical_codex_host_version_from_probe(envelope).ok_or_else(|| {
            io::Error::other(
                "Codex --version did not return a canonical `codex-cli VERSION` envelope",
            )
        })?;
        bounded_identity(
            "canonical Codex host version",
            version,
            MAX_HOST_VERSION_BYTES,
        )
    }

    fn validate_codex_chatgpt_login_status(
        stdout: &str,
        stderr: &str,
    ) -> Result<(), Box<dyn Error>> {
        let combined = format!("{stdout}\n{stderr}");
        let lines = combined
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>();
        if lines.as_slice() != ["Logged in using ChatGPT"]
            || combined.to_ascii_lowercase().contains("api key")
        {
            return Err(io::Error::other(
                "codex login status must say exactly `Logged in using ChatGPT`; API-key or ambiguous authentication is not allowed for live release cells",
            )
            .into());
        }
        Ok(())
    }

    fn release_build_id_from_version_output(
        label: &str,
        output: &TimedOutput,
    ) -> Result<String, Box<dyn Error>> {
        require_success(label, output)?;
        let line = stdout(output);
        let line = line.trim();
        let (_, build_id) = line
            .strip_prefix("volicord ")
            .and_then(|value| value.strip_suffix(')'))
            .and_then(|value| value.rsplit_once(" (build_id="))
            .ok_or_else(|| {
                io::Error::other(
                    "release candidate --version did not expose the exact build_id envelope",
                )
            })?;
        bounded_identity("Volicord build_id", build_id, MAX_BUILD_ID_BYTES)
    }

    fn bounded_identity(
        label: &str,
        value: &str,
        max_bytes: usize,
    ) -> Result<String, Box<dyn Error>> {
        if value.is_empty() || value.chars().any(char::is_control) {
            return Err(
                io::Error::other(format!("{label} must be one non-empty printable line")).into(),
            );
        }
        if value.len() > max_bytes {
            return Err(io::Error::other(format!(
                "{label} exceeds the {max_bytes}-byte result limit"
            ))
            .into());
        }
        Ok(value.to_owned())
    }

    fn assert_connection_observation_diagnostic(
        fixture: &LiveSmokeFixture,
        connection_id: &str,
        project_id: &str,
        cursor: DiagnosticEventCursor,
    ) -> Result<(), Box<dyn Error>> {
        let conn = rusqlite::Connection::open_with_flags(
            diagnostics_db_path(&fixture.runtime_home_path),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )?;
        let observed = conn.query_row(
            "SELECT
                 COALESCE(SUM(CASE
                       WHEN e.tool_name = 'volicord.status'
                        AND e.core_committed = 0
                        AND e.outcome = 'success'
                       THEN 1 ELSE 0 END), 0),
                 COALESCE(SUM(CASE
                       WHEN e.core_committed = 1
                       THEN 1 ELSE 0 END), 0)
               FROM diagnostic_sessions s
               JOIN diagnostic_events e ON e.session_id = s.session_id
              WHERE s.connection_id = ?1
                AND s.project_id = ?2
                AND e.event_id > ?3",
            rusqlite::params![connection_id, project_id, cursor.0],
            |row| Ok((row.get::<_, u64>(0)?, row.get::<_, u64>(1)?)),
        )?;
        if observed.0 < 1 || observed.1 != 0 {
            return Err(io::Error::other(format!(
                "the connection-observation probe did not remain a read-only status observation: successful_status={}, committed_events={}",
                observed.0, observed.1
            ))
            .into());
        }
        Ok(())
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
        cursor: DiagnosticEventCursor,
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
                       THEN 1 ELSE 0 END), 0),
                 MIN(CASE
                       WHEN e.tool_name = 'volicord.request_user_action'
                        AND e.replayed = 1
                        AND e.outcome = 'success'
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
              WHERE s.connection_id = ?1
                AND s.project_id = ?2
                AND e.event_id > ?3",
            rusqlite::params![connection_id, project_id, cursor.0],
            |row| {
                Ok((
                    row.get::<_, u64>(0)?,
                    row.get::<_, u64>(1)?,
                    row.get::<_, u64>(2)?,
                    row.get::<_, Option<u64>>(3)?,
                    row.get::<_, Option<u64>>(4)?,
                    row.get::<_, Option<u64>>(5)?,
                ))
            },
        )?;
        let ordered = observed.3.zip(observed.4).zip(observed.5).is_some_and(
            |((resume, record_run), status)| resume < record_run && record_run < status,
        );
        if observed.0 < 1 || observed.1 != 1 || observed.2 < 1 || !ordered {
            return Err(io::Error::other(format!(
                "the authenticated host diagnostics did not show one ordered same-connection resume path after the active-run cursor: replayed request_user_action={}, committed record_run={}, status={}, ordered={ordered}",
                observed.0, observed.1, observed.2,
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
        cursor: DiagnosticEventCursor,
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
              WHERE s.connection_id = ?1
                AND s.project_id = ?2
                AND e.event_id > ?3",
            rusqlite::params![connection_id, project_id, cursor.0],
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

    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(deny_unknown_fields)]
    struct ReleaseCandidateBuildEnvironment {
        runner_os: String,
        runner_os_version: String,
        runner_arch: String,
        git_version: String,
        rustc_version: String,
        cargo_version: String,
    }

    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(deny_unknown_fields)]
    struct ReleaseCandidate {
        #[serde(skip)]
        descriptor_path: Option<PathBuf>,
        schema: String,
        candidate_id: String,
        candidate_path: String,
        source_revision: String,
        source_clean: bool,
        source_archive_algorithm: String,
        source_archive_sha256: String,
        target_triple: String,
        release_profile: String,
        binary_sha256: String,
        build_environment: ReleaseCandidateBuildEnvironment,
        recorded_at: String,
    }

    impl ReleaseCandidate {
        fn from_descriptor_path(
            context: &ValidationContext,
            descriptor_path: &Path,
        ) -> Result<Self, Box<dyn Error>> {
            let bytes = read_bounded_external_file(
                context,
                descriptor_path,
                MAX_RELEASE_CANDIDATE_DESCRIPTOR_BYTES as u64,
            )?;
            let mut candidate: Self = serde_json::from_slice(&bytes)?;
            candidate.descriptor_path = Some(descriptor_path.to_path_buf());
            candidate.validate_with_context(context)?;
            Ok(candidate)
        }

        fn validate(&self) -> Result<(), Box<dyn Error>> {
            let context = release_validation_context()?;
            self.validate_with_context(&context)
        }

        fn validate_external_paths(
            &self,
            context: &ValidationContext,
        ) -> Result<(), Box<dyn Error>> {
            if let Some(descriptor_path) = self.descriptor_path.as_deref() {
                validate_external_regular_input(
                    context,
                    descriptor_path,
                    MAX_RELEASE_CANDIDATE_DESCRIPTOR_BYTES as u64,
                    RELEASE_CANDIDATE_PATH_ENV,
                )?;
            }
            validate_external_regular_input(
                context,
                self.executable_path(),
                MAX_RELEASE_CANDIDATE_BINARY_BYTES,
                "candidate_path",
            )
        }

        fn validate_with_context(&self, context: &ValidationContext) -> Result<(), Box<dyn Error>> {
            if self.schema != RELEASE_CANDIDATE_SCHEMA {
                return Err(io::Error::other(format!(
                    "release candidate schema must be {RELEASE_CANDIDATE_SCHEMA}"
                ))
                .into());
            }
            bounded_identity("candidate_id", &self.candidate_id, 256)?;
            validate_lower_hex("source_revision", &self.source_revision, &[40, 64])?;
            if !self.source_clean {
                return Err(io::Error::other("release candidate source_clean must be true").into());
            }
            if self.source_archive_algorithm != RELEASE_SOURCE_ARCHIVE_ALGORITHM {
                return Err(io::Error::other(format!(
                    "release candidate source archive algorithm must be {RELEASE_SOURCE_ARCHIVE_ALGORITHM}"
                ))
                .into());
            }
            validate_lower_hex("source_archive_sha256", &self.source_archive_sha256, &[64])?;
            validate_lower_hex("binary_sha256", &self.binary_sha256, &[64])?;
            bounded_identity("target_triple", &self.target_triple, 256)?;
            bounded_identity("release_profile", &self.release_profile, 128)?;
            if self.release_profile != "release" {
                return Err(io::Error::other(
                    "live release validation requires the exact release profile",
                )
                .into());
            }
            validate_canonical_second_timestamp("candidate recorded_at", &self.recorded_at)?;
            for (name, value) in [
                ("runner_os", self.build_environment.runner_os.as_str()),
                (
                    "runner_os_version",
                    self.build_environment.runner_os_version.as_str(),
                ),
                ("runner_arch", self.build_environment.runner_arch.as_str()),
                ("git_version", self.build_environment.git_version.as_str()),
                (
                    "rustc_version",
                    self.build_environment.rustc_version.as_str(),
                ),
                (
                    "cargo_version",
                    self.build_environment.cargo_version.as_str(),
                ),
            ] {
                bounded_identity(name, value, 256)?;
            }
            let candidate_path = Path::new(&self.candidate_path);
            self.validate_external_paths(context)?;
            let actual_sha256 = sha256_external_file(
                context,
                candidate_path,
                Some(MAX_RELEASE_CANDIDATE_BINARY_BYTES),
            )?;
            if actual_sha256 != self.binary_sha256 {
                return Err(io::Error::other(
                    "release candidate executable bytes do not match binary_sha256",
                )
                .into());
            }
            Ok(())
        }

        fn executable_path(&self) -> &Path {
            Path::new(&self.candidate_path)
        }

        fn adapter_build_id(&self, context: &ValidationContext) -> Result<String, Box<dyn Error>> {
            self.validate_with_context(context)?;
            let mut command = Command::new(self.executable_path());
            command
                .arg("--version")
                .env("NO_COLOR", "1")
                .env_remove(LIVE_HOST_RESULT_PATH_ENV)
                .env_remove(RELEASE_CANDIDATE_PATH_ENV)
                .env_remove(RELEASE_REQUEST_VERIFIED_ENV);
            LiveSmokeFixture::remove_inherited_host_control_env(&mut command);
            LiveSmokeFixture::remove_inherited_auth_secret_env(&mut command);
            let outcome = run_with_timeout(command, COMMAND_TIMEOUT);
            self.validate_with_context(context)?;
            let output = outcome?;
            release_build_id_from_version_output(
                "exact release candidate volicord --version",
                &output,
            )
        }
    }

    fn validate_lower_hex(
        name: &str,
        value: &str,
        allowed_lengths: &[usize],
    ) -> Result<(), Box<dyn Error>> {
        if !allowed_lengths.contains(&value.len())
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(io::Error::other(format!(
                "{name} must be lowercase hexadecimal with length in {allowed_lengths:?}"
            ))
            .into());
        }
        Ok(())
    }

    fn validate_managed_summary_session_id(name: &str, value: &str) -> Result<(), Box<dyn Error>> {
        validate_managed_host_session_id(value)
            .map_err(|error| io::Error::other(format!("{name} is invalid: {error}")).into())
    }

    fn validate_domain_separated_correlation_id(
        name: &str,
        value: &str,
        domain: &str,
    ) -> Result<(), Box<dyn Error>> {
        let prefix = format!("{domain}_");
        let digest = value.strip_prefix(&prefix).ok_or_else(|| {
            io::Error::other(format!(
                "{name} must use the {domain:?} opaque-id domain prefix"
            ))
        })?;
        validate_lower_hex(name, digest, &[16])
    }

    fn validate_canonical_second_timestamp(name: &str, value: &str) -> Result<(), Box<dyn Error>> {
        if value.len() != 20
            || !value.ends_with('Z')
            || value.as_bytes().get(4) != Some(&b'-')
            || value.as_bytes().get(7) != Some(&b'-')
            || value.as_bytes().get(10) != Some(&b'T')
            || value.as_bytes().get(13) != Some(&b':')
            || value.as_bytes().get(16) != Some(&b':')
            || DateTime::parse_from_rfc3339(value).is_err()
        {
            return Err(io::Error::other(format!(
                "{name} must be canonical UTC RFC 3339 with second precision"
            ))
            .into());
        }
        Ok(())
    }

    fn validate_external_regular_input(
        context: &ValidationContext,
        path: &Path,
        max_bytes: u64,
        name: &str,
    ) -> Result<(), Box<dyn Error>> {
        context.validate_existing_file(path)?;
        let metadata = fs::symlink_metadata(path)?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(
                io::Error::other(format!("{name} must name a non-symlink regular file")).into(),
            );
        }
        if metadata.len() > max_bytes {
            return Err(
                io::Error::other(format!("{name} exceeds its {max_bytes}-byte bound")).into(),
            );
        }
        Ok(())
    }

    fn sha256_file(path: &Path, max_bytes: u64) -> Result<String, Box<dyn Error>> {
        let mut file = fs::File::open(path)?;
        let mut hasher = Sha256::new();
        let mut total = 0_u64;
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let count = file.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            total = total
                .checked_add(u64::try_from(count)?)
                .ok_or_else(|| io::Error::other("file size overflow while hashing"))?;
            if total > max_bytes {
                return Err(io::Error::other(format!(
                    "{} exceeded its {max_bytes}-byte bound while hashing",
                    path.display()
                ))
                .into());
            }
            hasher.update(&buffer[..count]);
        }
        Ok(format!("{:x}", hasher.finalize()))
    }

    struct LiveSmokeFixture {
        _runtime_home: TempRuntimeHome,
        host_home_root: PathBuf,
        runtime_home_path: PathBuf,
        release_artifact_root: PathBuf,
        repo_root: PathBuf,
        repo_arg: String,
        runtime_home_arg: String,
        env_path: OsString,
        home: PathBuf,
        codex_home: PathBuf,
        xdg_config_home: PathBuf,
        claude_config_dir: PathBuf,
        volicord_path: PathBuf,
        expected_volicord_sha256: String,
    }

    impl Drop for LiveSmokeFixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.host_home_root);
        }
    }

    fn create_disposable_host_home(
        path_context: &ValidationContext,
    ) -> Result<(PathBuf, PathBuf), Box<dyn Error>> {
        let base = path_context
            .target_directory()
            .join("volicord-live-host-homes");
        fs::create_dir_all(&base)?;
        let base_metadata = fs::symlink_metadata(&base)?;
        if base_metadata.file_type().is_symlink() || !base_metadata.is_dir() {
            return Err(io::Error::other(
                "live-host home base must be a real Cargo target directory",
            )
            .into());
        }
        let canonical_base = fs::canonicalize(&base)?;
        let canonical_temp = fs::canonicalize(env::temp_dir())?;
        if canonical_base.starts_with(canonical_temp) {
            return Err(io::Error::other(
                "Codex canonical version probes require a disposable home outside the system temporary directory",
            )
            .into());
        }
        let root = base.join(format!(
            "host-home-{}-{}",
            std::process::id(),
            epoch_duration()?.as_nanos()
        ));
        fs::create_dir(&root)?;
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700))?;
        let codex_home = root.join("codex");
        if let Err(error) = fs::create_dir(&codex_home) {
            let _ = fs::remove_dir(&root);
            return Err(error.into());
        }
        fs::set_permissions(&codex_home, fs::Permissions::from_mode(0o700))?;
        Ok((root, codex_home))
    }

    impl LiveSmokeFixture {
        fn new(prefix: &str) -> Result<Self, Box<dyn Error>> {
            Self::new_with_volicord(prefix, Path::new(volicord_bin()), None, None, None)
        }

        fn new_with_release_candidate(
            prefix: &str,
            candidate: &ReleaseCandidate,
        ) -> Result<Self, Box<dyn Error>> {
            Self::new_with_volicord(
                prefix,
                candidate.executable_path(),
                Some(candidate.binary_sha256.as_str()),
                Some(candidate),
                None,
            )
        }

        fn new_with_release_candidate_for_recorder(
            prefix: &str,
            candidate: &ReleaseCandidate,
            recorder: &mut LiveResultRecorder,
        ) -> Result<Self, Box<dyn Error>> {
            Self::new_with_volicord(
                prefix,
                candidate.executable_path(),
                Some(candidate.binary_sha256.as_str()),
                Some(candidate),
                Some(recorder),
            )
        }

        fn new_with_volicord(
            prefix: &str,
            source_volicord: &Path,
            expected_sha256: Option<&str>,
            release_candidate: Option<&ReleaseCandidate>,
            publication_recorder: Option<&mut LiveResultRecorder>,
        ) -> Result<Self, Box<dyn Error>> {
            let mut path_context = release_validation_context()?;
            let runtime_home = TempRuntimeHome::new(&format!("live-host-smoke-{prefix}"))?;
            let runtime_home_path = runtime_home.path().to_path_buf();
            if let Some(recorder) = publication_recorder {
                recorder.bind_observed_runtime_home(&runtime_home_path)?;
                recorder.require_publication_domain_ready()?;
            }
            path_context.add_runtime_home(&runtime_home_path)?;
            if let Some(candidate) = release_candidate {
                candidate.validate_with_context(&path_context)?;
            }
            let release_artifact_root = runtime_home.product_repo_path("release-artifacts");
            fs::create_dir_all(&release_artifact_root)?;
            path_context.validate_existing_directory(&release_artifact_root)?;
            let repo_root = runtime_home.create_product_repo("product-repo")?;
            initialize_git_repository(&repo_root)?;
            fs::write(
                repo_root.join("README.md"),
                "Volicord live smoke repository\n",
            )?;

            let bin_dir = release_artifact_root.join("live-bin");
            fs::create_dir_all(&bin_dir)?;
            let volicord_path = bin_dir.join("volicord-release-candidate");
            fs::copy(source_volicord, &volicord_path)?;
            let copied_sha256 = sha256_file(&volicord_path, MAX_RELEASE_CANDIDATE_BINARY_BYTES)?;
            let expected_volicord_sha256 = expected_sha256.unwrap_or(&copied_sha256).to_owned();
            if copied_sha256 != expected_volicord_sha256 {
                return Err(io::Error::other(
                    "private live fixture copy does not match the release candidate digest",
                )
                .into());
            }
            let mut permissions = fs::metadata(&volicord_path)?.permissions();
            permissions.set_mode(0o555);
            fs::set_permissions(&volicord_path, permissions)?;
            write_volicord_shim(&bin_dir, &volicord_path)?;

            let home = runtime_home_path.join("isolated-home");
            let (host_home_root, codex_home) = create_disposable_host_home(&path_context)?;
            let xdg_config_home = runtime_home_path.join("isolated-xdg-config");
            let claude_config_dir = runtime_home_path.join("isolated-claude-config");
            for path in [&home, &xdg_config_home, &claude_config_dir] {
                fs::create_dir_all(path)?;
            }

            let env_path = path_with_prefix(&bin_dir)?;
            let repo_arg = path_text(&repo_root);
            let runtime_home_arg = path_text(&runtime_home_path);
            Ok(Self {
                _runtime_home: runtime_home,
                host_home_root,
                runtime_home_path,
                release_artifact_root,
                repo_root,
                repo_arg,
                runtime_home_arg,
                env_path,
                home,
                codex_home,
                xdg_config_home,
                claude_config_dir,
                volicord_path,
                expected_volicord_sha256,
            })
        }

        fn repo_arg(&self) -> &str {
            &self.repo_arg
        }

        fn live_bin(&self) -> &Path {
            self.volicord_path
                .parent()
                .expect("live fixture candidate must have a bin directory")
        }

        fn runtime_home_arg(&self) -> &str {
            &self.runtime_home_arg
        }

        fn run_volicord<const N: usize>(
            &self,
            args: [&str; N],
        ) -> Result<TimedOutput, Box<dyn Error>> {
            let mut command = Command::new(&self.volicord_path);
            command.args(args).current_dir(&self.repo_root);
            self.apply_isolated_env(&mut command);
            self.run_candidate_command(command)
        }

        fn release_build_id(&self) -> Result<String, Box<dyn Error>> {
            let output = self.run_volicord(["--version"])?;
            release_build_id_from_version_output("release candidate volicord --version", &output)
        }

        fn run_volicord_with_host_environment<const N: usize>(
            &self,
            args: [&str; N],
        ) -> Result<TimedOutput, Box<dyn Error>> {
            let mut command = Command::new(&self.volicord_path);
            command
                .args(args)
                .current_dir(&self.repo_root)
                .env("VOLICORD_HOME", &self.runtime_home_path)
                .env("PATH", &self.env_path)
                .env("NO_COLOR", "1")
                .env_remove(LIVE_HOST_RESULT_PATH_ENV)
                .env_remove(RELEASE_REQUEST_VERIFIED_ENV);
            Self::remove_inherited_host_control_env(&mut command);
            Self::remove_inherited_auth_secret_env(&mut command);
            self.run_candidate_command(command)
        }

        fn run_host_command<const N: usize>(
            &self,
            program: &Path,
            args: [&str; N],
        ) -> Result<TimedOutput, Box<dyn Error>> {
            let mut command = Command::new(program);
            command.args(args).current_dir(&self.repo_root);
            self.apply_isolated_env(&mut command);
            self.with_private_candidate_digest_guard(|| {
                run_with_timeout(command, COMMAND_TIMEOUT).map_err(Into::into)
            })
        }

        fn run_installed_host_version_probe(
            &self,
            program: &Path,
        ) -> Result<TimedOutput, Box<dyn Error>> {
            let mut command = Command::new(program);
            command
                .arg("--version")
                .current_dir(&self.repo_root)
                .env("PATH", &self.env_path)
                .env("NO_COLOR", "1")
                .env_remove("CODEX_HOME");
            Self::remove_inherited_host_control_env(&mut command);
            Self::remove_inherited_auth_secret_env(&mut command);
            self.with_private_candidate_digest_guard(|| {
                run_with_timeout(command, COMMAND_TIMEOUT).map_err(Into::into)
            })
        }

        fn observe_and_bind_installed_host_identity(
            &self,
            recorder: &mut LiveResultRecorder,
            executable_name: &str,
            executable: &Path,
        ) -> Result<ObservedReleaseHostIdentity, Box<dyn Error>> {
            recorder.bind_observed_runtime_home(&self.runtime_home_path)?;
            recorder.require_publication_domain_ready()?;
            recorder.mark_installed_host_detected();
            let host_executable_sha256 =
                sha256_file(executable, MAX_RELEASE_CANDIDATE_BINARY_BYTES)?;
            let host_version_output = self.run_installed_host_version_probe(executable)?;
            let host_version = if executable_name == "codex" {
                canonical_codex_version_summary(&host_version_output)?
            } else {
                host_version_summary(&host_version_output)?
            };
            recorder.bind_observed_host_coordinates(ObservedReleaseHostCoordinates::new(
                host_version.clone(),
                host_executable_sha256.clone(),
            )?)?;
            require_success(
                &format!("{executable_name} --version for live release cell"),
                &host_version_output,
            )?;
            let volicord_build_id = self.release_build_id()?;
            recorder.bind_observed_volicord_build_id(volicord_build_id.clone())?;
            ObservedReleaseHostIdentity::new(
                host_version,
                host_executable_sha256,
                volicord_build_id,
            )
        }

        fn managed_baseline_observations(
            &self,
        ) -> Result<ManagedBaselineObservations, Box<dyn Error>> {
            let mut observations = BTreeMap::new();
            for project in list_projects(&self.runtime_home_path)? {
                let connection = open_project_state_database_read_only(&project.state_db_path)?;
                let mut statement = connection.prepare(
                    "SELECT watch_baseline_id, metadata_json
                       FROM session_watch_baselines
                      WHERE project_id = ?1
                      ORDER BY watch_baseline_id",
                )?;
                let rows = statement.query_map([&project.project_id], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })?;
                for row in rows {
                    let (watch_baseline_id, metadata_json) = row?;
                    observations.insert(
                        ObservedHostTurnBaseline {
                            project_id: project.project_id.clone(),
                            watch_baseline_id,
                        },
                        ManagedBaselineObservation::from_metadata_json(&metadata_json)?,
                    );
                }
            }
            Ok(observations)
        }

        fn run_authenticated_interactive_host(
            &self,
            host: &str,
            program: &Path,
            prompt: &str,
            recorder: &mut LiveResultRecorder,
        ) -> Result<ExitStatus, Box<dyn Error>> {
            recorder.require_publication_domain_ready()?;
            let before = self.managed_baseline_observations()?;
            self.require_codex_chatgpt_login_immediately_before_cell(host, program)?;
            let mut command = Command::new(program);
            command
                .arg(prompt)
                .current_dir(&self.repo_root)
                .env("VOLICORD_HOME", &self.runtime_home_path)
                .env("PATH", &self.env_path)
                .env_remove(LIVE_HOST_RESULT_PATH_ENV)
                .env_remove(RELEASE_REQUEST_VERIFIED_ENV)
                .stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit());
            Self::remove_inherited_host_control_env(&mut command);
            Self::remove_inherited_auth_secret_env(&mut command);
            let result = self.run_candidate_host_turn(command);
            let after = self.managed_baseline_observations()?;
            recorder.bind_observed_host_turn_baselines(&before, &after)?;
            result
        }

        fn run_authenticated_interactive_host_with_local_web(
            &self,
            host: &str,
            program: &Path,
            prompt: &str,
            recorder: &mut LiveResultRecorder,
        ) -> Result<ExitStatus, Box<dyn Error>> {
            recorder.require_publication_domain_ready()?;
            let before = self.managed_baseline_observations()?;
            self.require_codex_chatgpt_login_immediately_before_cell(host, program)?;
            let mut command = Command::new(program);
            command
                .arg(prompt)
                .current_dir(&self.repo_root)
                .env("VOLICORD_HOME", &self.runtime_home_path)
                .env("PATH", &self.env_path)
                .env_remove(LIVE_HOST_RESULT_PATH_ENV)
                .env_remove(RELEASE_REQUEST_VERIFIED_ENV)
                .stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit());
            Self::remove_inherited_host_control_env(&mut command);
            Self::remove_inherited_auth_secret_env(&mut command);
            command.env("VOLICORD_LOCAL_WEB_CONSENT", "1");
            let result = self.run_candidate_host_turn(command);
            let after = self.managed_baseline_observations()?;
            recorder.bind_observed_host_turn_baselines(&before, &after)?;
            result
        }

        fn require_codex_chatgpt_login_immediately_before_cell(
            &self,
            host: &str,
            program: &Path,
        ) -> Result<(), Box<dyn Error>> {
            if host != "codex" {
                return Ok(());
            }
            let mut command = Command::new(program);
            command
                .args(["login", "status"])
                .current_dir(&self.repo_root)
                .env("VOLICORD_HOME", &self.runtime_home_path)
                .env("PATH", &self.env_path)
                .env("NO_COLOR", "1")
                .env_remove(LIVE_HOST_RESULT_PATH_ENV)
                .env_remove(RELEASE_CANDIDATE_PATH_ENV)
                .env_remove(RELEASE_REQUEST_VERIFIED_ENV);
            Self::remove_inherited_host_control_env(&mut command);
            Self::remove_inherited_auth_secret_env(&mut command);
            let output = self.with_private_candidate_digest_guard(|| {
                run_with_timeout(command, COMMAND_TIMEOUT).map_err(Into::into)
            })?;
            require_success("codex login status", &output)?;
            validate_codex_chatgpt_login_status(&stdout(&output), &stderr(&output))
        }

        fn remove_inherited_host_control_env(command: &mut Command) {
            for name in [
                LIVE_HOST_RESULT_PATH_ENV,
                RELEASE_CANDIDATE_PATH_ENV,
                RELEASE_REQUEST_VERIFIED_ENV,
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

        fn remove_inherited_auth_secret_env(command: &mut Command) {
            for name in [
                "OPENAI_API_KEY",
                "CODEX_API_KEY",
                "ANTHROPIC_API_KEY",
                "ANTHROPIC_AUTH_TOKEN",
                "CLAUDE_CODE_OAUTH_TOKEN",
                "CLAUDE_CODE_API_KEY",
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
                .env("NO_COLOR", "1");
            Self::remove_inherited_host_control_env(command);
            Self::remove_inherited_auth_secret_env(command);
        }

        fn verify_private_candidate_digest(&self) -> Result<(), Box<dyn Error>> {
            let metadata = fs::symlink_metadata(&self.volicord_path)?;
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                return Err(io::Error::other(
                    "private live fixture candidate must remain a non-symlink regular file",
                )
                .into());
            }
            if metadata.permissions().mode() & 0o222 != 0 {
                return Err(io::Error::other(
                    "private live fixture candidate must remain read-only",
                )
                .into());
            }
            let actual = sha256_file(&self.volicord_path, MAX_RELEASE_CANDIDATE_BINARY_BYTES)?;
            if actual != self.expected_volicord_sha256 {
                return Err(io::Error::other(
                    "private live fixture candidate digest changed after fixture setup",
                )
                .into());
            }
            Ok(())
        }

        fn run_candidate_command(&self, command: Command) -> Result<TimedOutput, Box<dyn Error>> {
            self.with_private_candidate_digest_guard(|| {
                run_with_timeout(command, COMMAND_TIMEOUT).map_err(Into::into)
            })
        }

        fn run_candidate_host_turn(
            &self,
            mut command: Command,
        ) -> Result<ExitStatus, Box<dyn Error>> {
            self.with_private_candidate_digest_guard(|| command.status().map_err(Into::into))
        }

        fn run_generated_final_output_handler(
            &self,
            host: &str,
            event: &Value,
        ) -> Result<Output, Box<dyn Error>> {
            self.with_private_candidate_digest_guard(|| {
                run_generated_final_output_handler(
                    &self.runtime_home_path,
                    &self.repo_root,
                    &self.env_path,
                    host,
                    event,
                )
            })
        }

        fn with_private_candidate_digest_guard<T>(
            &self,
            operation: impl FnOnce() -> Result<T, Box<dyn Error>>,
        ) -> Result<T, Box<dyn Error>> {
            self.verify_private_candidate_digest()?;
            let outcome = catch_unwind(AssertUnwindSafe(operation));
            let stability = self.verify_private_candidate_digest();
            match (outcome, stability) {
                (_, Err(error)) => Err(error),
                (Ok(Ok(value)), Ok(())) => Ok(value),
                (Ok(Err(error)), Ok(())) => Err(error),
                (Err(payload), Ok(())) => resume_unwind(payload),
            }
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
        let mut command = Command::new(generated_stop_wrapper_path(repo_root, host)?);
        command
            .env("VOLICORD_HOME", runtime_home)
            .env("PATH", path)
            .current_dir(repo_root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        LiveSmokeFixture::remove_inherited_host_control_env(&mut command);
        LiveSmokeFixture::remove_inherited_auth_secret_env(&mut command);
        let mut child = command.spawn()?;
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
        host_version: Option<&str>,
        profile: IntegrationProfile,
        init: &Value,
    ) -> Result<Value, Box<dyn Error>> {
        require_init_host_feature_support(init, host, host_version, profile)?;
        let wrapper_path = generated_stop_wrapper_path(&fixture.repo_root, host)?;
        let wrapper = fs::read_to_string(&wrapper_path)?;
        let volicord_command =
            generated_script_word(path_text(&fs::canonicalize(&fixture.volicord_path)?).as_str());
        let expected_command = match profile {
            IntegrationProfile::Record => format!("exec {volicord_command} _final-output"),
            IntegrationProfile::Detective => format!("exec {volicord_command} _hook stop"),
        };
        let runtime_home_assignment = format!(
            "VOLICORD_HOME={}",
            generated_script_word(fixture.runtime_home_arg())
        );
        if !wrapper.contains(&expected_command)
            || !wrapper.lines().any(|line| line == runtime_home_assignment)
            || !wrapper.lines().any(|line| line == "export VOLICORD_HOME")
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
            "host_feature_support": init["states"]["host_feature_support"].clone(),
            "final_output_authority_disclosure": init["states"]["final_output_authority_disclosure"].clone()
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

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct ReleaseHostFeatureDiagnostics {
        host_feature_support: Value,
        final_output_authority_disclosure: Value,
    }

    fn maintained_host_kind(host: &str) -> HostKind {
        match host {
            "codex" => HostKind::Codex,
            "claude-code" => HostKind::ClaudeCode,
            other => panic!("unexpected maintained live-host fixture kind: {other}"),
        }
    }

    fn canonical_release_host_feature_diagnostics(
        host: &str,
        profile: IntegrationProfile,
        configured: bool,
        configuration_verified: bool,
    ) -> ReleaseHostFeatureDiagnostics {
        canonical_release_host_feature_diagnostics_for_version(
            host,
            None,
            profile,
            configured,
            configuration_verified,
        )
    }

    fn canonical_release_host_feature_diagnostics_for_version(
        host: &str,
        host_version: Option<&str>,
        profile: IntegrationProfile,
        configured: bool,
        configuration_verified: bool,
    ) -> ReleaseHostFeatureDiagnostics {
        let projection = HostFeatureDiagnosticProjection::baseline_for_version(
            maintained_host_kind(host),
            host_version,
            profile,
            configured,
            configuration_verified,
        );
        ReleaseHostFeatureDiagnostics {
            host_feature_support: projection.host_feature_support_json(),
            final_output_authority_disclosure: projection.final_output_authority_disclosure_json(),
        }
    }

    fn canonical_release_host_feature_diagnostics_for_profile(
        host: &str,
        host_version: Option<&str>,
        profile: Option<IntegrationProfile>,
        configured: bool,
        configuration_verified: bool,
    ) -> ReleaseHostFeatureDiagnostics {
        match profile {
            Some(profile) => canonical_release_host_feature_diagnostics_for_version(
                host,
                host_version,
                profile,
                configured,
                configuration_verified,
            ),
            None => ReleaseHostFeatureDiagnostics {
                host_feature_support: default_host_feature_support_json_for_version(
                    maintained_host_kind(host),
                    host_version,
                ),
                final_output_authority_disclosure: Value::Null,
            },
        }
    }

    fn release_host_feature_diagnostics_from_init(
        value: &Value,
        host: &str,
        host_version: Option<&str>,
        profile: IntegrationProfile,
    ) -> Result<ReleaseHostFeatureDiagnostics, Box<dyn Error>> {
        require_init_host_feature_support(value, host, host_version, profile)?;
        Ok(ReleaseHostFeatureDiagnostics {
            host_feature_support: value["states"]["host_feature_support"].clone(),
            final_output_authority_disclosure: value["states"]["final_output_authority_disclosure"]
                .clone(),
        })
    }

    fn validate_release_host_feature_diagnostics(
        value: &Value,
        profile: Option<IntegrationProfile>,
        configured: bool,
        configuration_verified: bool,
    ) -> Result<(), Box<dyn Error>> {
        let host = value["host"]["kind"]
            .as_str()
            .ok_or_else(|| io::Error::other("release result has no exact host kind"))?;
        let host_version = match value["host"].get("version") {
            Some(Value::String(version)) => Some(version.as_str()),
            Some(Value::Null) | None => None,
            Some(_) => {
                return Err(io::Error::other(
                    "release result host version must be a string or null",
                )
                .into())
            }
        };
        let expected = canonical_release_host_feature_diagnostics_for_profile(
            host,
            host_version,
            profile,
            configured,
            configuration_verified,
        );
        let actual_support = value
            .get("host_feature_support")
            .ok_or_else(|| io::Error::other("release result has no host_feature_support"))?;
        let actual_disclosure =
            value
                .get("final_output_authority_disclosure")
                .ok_or_else(|| {
                    io::Error::other("release result has no final_output_authority_disclosure")
                })?;
        if actual_support != &expected.host_feature_support
            || actual_disclosure != &expected.final_output_authority_disclosure
        {
            return Err(io::Error::other(
                "release result does not use the exact canonical host-feature projection",
            )
            .into());
        }
        Ok(())
    }

    fn validate_terminal_release_host_feature_diagnostics(
        value: &Value,
    ) -> Result<(), Box<dyn Error>> {
        let kind = value["kind"]
            .as_str()
            .ok_or_else(|| io::Error::other("terminal release result has no kind"))?;
        let profile = match kind {
            LIVE_USER_ACTION_RESULT_KIND
            | LIVE_EVIDENCE_OBSERVATION_RESULT_KIND
            | LIVE_CLI_FALLBACK_RESULT_KIND
            | LIVE_VERIFIED_TOOL_PRODUCER_RESULT_KIND
            | LIVE_REGISTERED_CONNECTION_OBSERVATION_RESULT_KIND => {
                Some(IntegrationProfile::Detective)
            }
            LIVE_FINAL_OUTPUT_RESULT_KIND => match value["profile"].as_str() {
                Some("record") => Some(IntegrationProfile::Record),
                Some("detective") => Some(IntegrationProfile::Detective),
                Some(_) | None => None,
            },
            _ => {
                return Err(io::Error::other(format!(
                    "terminal release result has unsupported kind {kind:?}"
                ))
                .into())
            }
        };
        let disclosure = value
            .get("final_output_authority_disclosure")
            .ok_or_else(|| {
                io::Error::other("terminal release result has no final_output_authority_disclosure")
            })?;
        let (configured, configuration_verified) = if disclosure.is_null() {
            (false, false)
        } else {
            let configured = disclosure["configured"].as_bool().ok_or_else(|| {
                io::Error::other("terminal release result has no configured fact")
            })?;
            let configuration_verified = disclosure["configuration_verified"]
                .as_bool()
                .ok_or_else(|| {
                    io::Error::other("terminal release result has no configuration_verified fact")
                })?;
            if configuration_verified && !configured {
                return Err(io::Error::other(
                    "configuration_verified cannot be true when configured is false",
                )
                .into());
            }
            (configured, configuration_verified)
        };
        validate_release_host_feature_diagnostics(
            value,
            profile,
            configured,
            configuration_verified,
        )
    }

    fn expected_host_feature_support(host: &str, host_version: Option<&str>) -> Value {
        canonical_release_host_feature_diagnostics_for_version(
            host,
            host_version,
            IntegrationProfile::Record,
            false,
            false,
        )
        .host_feature_support
    }

    fn expected_final_output_authority_disclosure(
        host: &str,
        host_version: Option<&str>,
        profile: IntegrationProfile,
    ) -> Value {
        canonical_release_host_feature_diagnostics_for_version(
            host,
            host_version,
            profile,
            true,
            true,
        )
        .final_output_authority_disclosure
    }

    fn require_init_host_feature_support(
        value: &Value,
        host: &str,
        host_version: Option<&str>,
        profile: IntegrationProfile,
    ) -> Result<(), Box<dyn Error>> {
        let expected_support = expected_host_feature_support(host, host_version);
        let expected_disclosure =
            expected_final_output_authority_disclosure(host, host_version, profile);
        if value["states"]["host_feature_support"] != expected_support
            || value["states"]["final_output_authority_disclosure"] != expected_disclosure
            || value["host_hook"].get("host_feature_support").is_some()
            || value["host_hook"]
                .get("final_output_authority_disclosure")
                .is_some()
        {
            return Err(io::Error::other(format!(
                "{host}/{} init host-feature diagnostics do not match the canonical projection",
                profile.as_str()
            ))
            .into());
        }
        Ok(())
    }

    fn assert_init_host_feature_support(
        value: &Value,
        host: &str,
        host_version: Option<&str>,
        profile: IntegrationProfile,
    ) {
        assert_eq!(
            value["states"]["host_feature_support"],
            expected_host_feature_support(host, host_version),
            "{host}/{} init did not emit the exact six-key support map: {value}",
            profile.as_str()
        );
        assert_eq!(
            value["states"]["final_output_authority_disclosure"],
            expected_final_output_authority_disclosure(host, host_version, profile),
            "{host}/{} init did not emit exact selected-profile final-output detail: {value}",
            profile.as_str()
        );
        assert!(value["host_hook"].get("host_feature_support").is_none());
        assert!(value["host_hook"]
            .get("final_output_authority_disclosure")
            .is_none());
    }

    fn require_live_init_reported_action_required(
        value: &Value,
        host: &str,
        host_version: Option<&str>,
        profile: IntegrationProfile,
        host_action: &str,
    ) -> Result<(), Box<dyn Error>> {
        let actions = value["actions"]
            .as_array()
            .ok_or_else(|| io::Error::other("live init actions are not an array"))?;
        let has_action = |expected: &str| actions.iter().any(|action| action["id"] == expected);
        require_init_host_feature_support(value, host, host_version, profile)?;
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
        host_version: Option<&str>,
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
        assert_init_host_feature_support(value, host, host_version, profile);
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
        host_version: Option<&str>,
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
        assert_init_host_feature_support(value, host, host_version, profile);
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
        assert_eq!(
            value.get("env_vars"),
            Some(&serde_json::json!(["VOLICORD_HOME"])),
            "Codex MCP entry must forward only the selected Runtime Home binding: {value}"
        );
        assert!(value
            .get("env")
            .is_none_or(|env| env.as_object().is_some_and(serde_json::Map::is_empty)));
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

    fn generated_script_word(value: &str) -> String {
        if !value.is_empty()
            && value.chars().all(|ch| {
                ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/' | ':' | '=')
            })
        {
            value.to_owned()
        } else {
            format!("'{}'", value.replace('\'', "'\\''"))
        }
    }

    fn volicord_bin() -> &'static str {
        env!("CARGO_BIN_EXE_volicord")
    }
}
