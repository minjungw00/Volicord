use std::{error::Error, path::PathBuf};

use volicord_host_contract::{
    project_mcp_tool, CanonicalToolName, CodexHookPromptCorrelation, CodexHookToolCorrelation,
    CodexMcpCorrelation, HostCallableName, HostContractProfileId, HostNativeCorrelation,
    HostSessionId, HostThreadId, HostToolUseId, HostTurnId, McpServerKey,
};
use volicord_test_support::TempRuntimeHome;
use volicord_types::{
    guard_manifest_from_json, AgentToolId, GuardDecision, GuardHookContractStatus,
    GuardProbeResult, GuardVerificationRepairReason, GuardVerificationRetryPolicy,
    IntegrationRevision, McpRuntimeSessionSource, PolicyHash, SequenceDurableIdGenerator,
};

use crate::{
    agent_connections::{
        add_connection_project, agent_connection_record_read_only, ensure_agent_connection,
        AgentConnectionRegistration, ConnectionProjectRegistration, CONNECTION_INTENT_SHARED,
        CONNECTION_MODE_WORKFLOW, HOST_KIND_CODEX, HOST_SCOPE_PROJECT,
    },
    bootstrap::{
        initialize_runtime_home, register_project, ProjectRegistration, ACTIVE_PROJECT_STATUS,
    },
    guards::{
        bind_agent_session_runtime, insert_guard_event, observe_host_correlation,
        test_guard_manifest_json, upsert_guard_installation, AgentSessionRuntimeBinding,
        GuardEventInsert, GuardInstallationUpsert, HostCorrelationObservation,
    },
    integration_verification::{
        acknowledge_guard_integration_probe, begin_guard_integration_verification_with_generator,
        observe_guard_probe_hook_event,
        row::{mark_repair_required, run_by_id},
        BeginGuardIntegrationVerificationInput, GuardIntegrationVerificationCaller,
        GuardIntegrationVerificationRunRecord, GuardProbeHookEvidence,
    },
    mutation::{with_test_runtime_home_setup, TestRuntimeHomeAdmission},
    operational_sessions::{start_mcp_runtime_session_for_test, McpRuntimeSessionStart},
    sqlite::{open_registry_database_for_test, registry_db_path},
    RuntimeHomeMutationContext, StoreResult,
};

pub(super) const POLICY_HASH: &str =
    "sha256:0000000000000000000000000000000000000000000000000000000000000000";
pub(super) const STALE_HASH: &str =
    "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
pub(super) const PROJECT_ID: &str = "project_verification";
pub(super) const CONNECTION_ID: &str = "connection_verification";
pub(super) const SERVER_KEY: &str = "volicord-verification";
pub(super) const INSTALLATION_ID: &str = "guard_installation_verification";
pub(super) const HOST_SESSION_ID: &str = "host_session_verification";
pub(super) const HOST_THREAD_ID: &str = "host_thread_verification";
pub(super) const HOST_TURN_ID: &str = "host_turn_verification";
pub(super) const BEGIN_AT: &str = "2026-07-23T00:00:03Z";
pub(super) const ACK_AT: &str = "2026-07-23T00:00:04Z";

pub(super) struct VerificationFixture {
    mutation: TestRuntimeHomeAdmission,
    pub runtime_home: TempRuntimeHome,
    pub repo_root: PathBuf,
    pub runtime_session_id: String,
    pub project_session_id: String,
    pub integration_revision: String,
}

pub(super) struct ToolEventFixture<'a> {
    pub event_id: &'a str,
    pub phase: &'a str,
    pub turn: &'a str,
    pub tool_use_id: &'a str,
    pub tool_name: &'a str,
    pub verification_id: &'a str,
    pub occurred_at: &'a str,
    pub digest: Option<&'a str>,
    pub policy_hash: Option<&'a str>,
    pub integration_revision: Option<&'a str>,
}

impl VerificationFixture {
    pub(super) fn new(prefix: &str) -> Result<Self, Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new(prefix)?;
        let repo_root = runtime_home.create_product_repo("verification-repo")?;
        with_test_runtime_home_setup(runtime_home.path(), |context| {
            initialize_runtime_home(context, &format!("runtime_home_{prefix}"), "{}")?;
            register_project(
                context,
                ProjectRegistration {
                    project_id: PROJECT_ID.to_owned(),
                    repo_root: repo_root.clone(),
                    project_home: None,
                    status: ACTIVE_PROJECT_STATUS.to_owned(),
                    metadata_json: "{}".to_owned(),
                },
            )?;
            ensure_agent_connection(
                context,
                AgentConnectionRegistration {
                    connection_internal_id: CONNECTION_ID.to_owned(),
                    host_kind: HOST_KIND_CODEX.to_owned(),
                    intent: CONNECTION_INTENT_SHARED.to_owned(),
                    host_scope: HOST_SCOPE_PROJECT.to_owned(),
                    server_name: SERVER_KEY.to_owned(),
                    config_target: runtime_home
                        .path()
                        .join("connection-verification")
                        .to_string_lossy()
                        .into_owned(),
                    mode: CONNECTION_MODE_WORKFLOW.to_owned(),
                    enabled: true,
                    managed_fingerprint: "fingerprint:verification".to_owned(),
                    metadata_json: "{}".to_owned(),
                },
            )?;
            add_connection_project(
                context,
                ConnectionProjectRegistration {
                    connection_internal_id: CONNECTION_ID.to_owned(),
                    project_id: PROJECT_ID.to_owned(),
                },
            )?;
            Ok(())
        })?;
        let mutation = TestRuntimeHomeAdmission::shared(runtime_home.path())?;
        let context = mutation.context()?;
        let connection = agent_connection_record_read_only(runtime_home.path(), CONNECTION_ID)?
            .expect("test connection");
        upsert_guard_installation(
            &context,
            GuardInstallationUpsert {
                guard_installation_id: INSTALLATION_ID.to_owned(),
                connection_internal_id: CONNECTION_ID.to_owned(),
                project_id: PROJECT_ID.to_owned(),
                manifest_json: test_guard_manifest_json(
                    &connection,
                    PROJECT_ID,
                    &repo_root,
                    INSTALLATION_ID,
                    POLICY_HASH,
                ),
            },
        )?;
        let integration_revision =
            crate::operational_sessions::connection_integration_revision(&connection)?
                .as_str()
                .to_owned();
        let runtime_session_id = start_mcp_runtime_session_for_test(
            &context,
            McpRuntimeSessionStart {
                connection_internal_id: CONNECTION_ID.to_owned(),
                session_source: McpRuntimeSessionSource::ManagedHost,
                observed_host_executable_version: None,
                process_id: 42,
                process_started_at: "2026-07-23T00:00:00Z".to_owned(),
            },
        )?
        .runtime_session_id;

        let prompt = prompt_correlation(HOST_TURN_ID);
        observe_event_correlation(&context, prompt.clone(), "2026-07-23T00:00:01Z")?;
        insert_test_event(
            &context,
            EventFixture {
                integration_revision: &integration_revision,
                event_id: "guard_event_prompt",
                correlation: prompt,
                phase: "prompt_capture",
                occurred_at: "2026-07-23T00:00:01Z",
                verification_id: None,
                digest: None,
                policy_hash: POLICY_HASH,
            },
        )?;
        let session = bind_agent_session_runtime(
            &context,
            PROJECT_ID,
            AgentSessionRuntimeBinding {
                runtime_session_id: runtime_session_id.clone(),
                connection_internal_id: CONNECTION_ID.to_owned(),
                guard_installation_id: Some(INSTALLATION_ID.to_owned()),
                correlation: mcp_correlation(HOST_TURN_ID),
                observed_at: "2026-07-23T00:00:02Z".to_owned(),
            },
        )?;
        Ok(Self {
            mutation,
            runtime_home,
            repo_root,
            runtime_session_id,
            project_session_id: session.session_id,
            integration_revision,
        })
    }

    pub(super) fn context(&self) -> StoreResult<RuntimeHomeMutationContext<'_>> {
        self.mutation.context()
    }

    pub(super) fn caller(&self) -> GuardIntegrationVerificationCaller {
        self.caller_for_turn(HOST_TURN_ID)
    }

    pub(super) fn caller_for_turn(&self, turn: &str) -> GuardIntegrationVerificationCaller {
        GuardIntegrationVerificationCaller {
            connection_internal_id: CONNECTION_ID.to_owned(),
            runtime_session_id: self.runtime_session_id.clone(),
            host_session_id: HOST_SESSION_ID.to_owned(),
            host_turn_id: turn.to_owned(),
        }
    }

    pub(super) fn begin(&self) -> StoreResult<GuardIntegrationVerificationRunRecord> {
        self.begin_at(BEGIN_AT, ["one"])
    }

    pub(super) fn begin_at<const N: usize>(
        &self,
        observed_at: &str,
        ids: [&str; N],
    ) -> StoreResult<GuardIntegrationVerificationRunRecord> {
        begin_guard_integration_verification_with_generator(
            &self.context()?,
            BeginGuardIntegrationVerificationInput {
                caller: self.caller(),
                project_id: PROJECT_ID.to_owned(),
                project_session_id: self.project_session_id.clone(),
                observed_at: observed_at.to_owned(),
            },
            &SequenceDurableIdGenerator::new(ids),
        )
    }

    pub(super) fn acknowledge(
        &self,
        verification_id: &str,
        observed_at: &str,
    ) -> StoreResult<GuardProbeResult> {
        acknowledge_guard_integration_probe(
            &self.context()?,
            verification_id,
            &self.caller(),
            observed_at,
        )
    }

    pub(super) fn record(
        &self,
        verification_id: &str,
    ) -> StoreResult<GuardIntegrationVerificationRunRecord> {
        let conn = open_registry_database_for_test(registry_db_path(self.runtime_home.path()))?;
        Ok(run_by_id(&conn, verification_id)?.expect("verification record"))
    }

    pub(super) fn set_policy_hash(
        &self,
        _verification_id: &str,
        policy_hash: &str,
    ) -> StoreResult<()> {
        self.set_current_manifest_field("$.policy_hash", policy_hash)
    }

    pub(super) fn set_hook_contract_digest(
        &self,
        _verification_id: &str,
        hook_contract_digest: &str,
    ) -> StoreResult<()> {
        self.set_current_manifest_field("$.host_contract_digest", hook_contract_digest)
    }

    pub(super) fn set_integration_revision(
        &self,
        _verification_id: &str,
        integration_revision: &str,
    ) -> StoreResult<()> {
        self.set_current_manifest_field("$.integration_revision", integration_revision)
    }

    pub(super) fn set_expected_host_callable_name(
        &self,
        verification_id: &str,
        expected_host_callable_name: &str,
    ) -> StoreResult<()> {
        let conn = open_registry_database_for_test(registry_db_path(self.runtime_home.path()))?;
        conn.execute(
            "UPDATE guard_integration_verification_runs
                SET expected_host_callable_name = ?2
              WHERE verification_id = ?1",
            rusqlite::params![verification_id, expected_host_callable_name],
        )?;
        Ok(())
    }

    fn set_current_manifest_field(&self, path: &str, value: &str) -> StoreResult<()> {
        let conn = open_registry_database_for_test(registry_db_path(self.runtime_home.path()))?;
        let manifest_json: String = conn.query_row(
            "SELECT manifest_json
               FROM guard_installations
              WHERE guard_installation_id = ?1",
            [INSTALLATION_ID],
            |row| row.get(0),
        )?;
        let updated_manifest = if path == "$.policy_hash" {
            let connection =
                agent_connection_record_read_only(self.runtime_home.path(), CONNECTION_ID)?
                    .expect("fixture connection");
            test_guard_manifest_json(
                &connection,
                PROJECT_ID,
                &self.repo_root,
                INSTALLATION_ID,
                PolicyHash::parse(value)
                    .expect("fixture policy hash is canonical")
                    .as_str(),
            )
        } else {
            let mut manifest = guard_manifest_from_json(&manifest_json)
                .expect("fixture starts with an exact current Guard manifest");
            match path {
                "$.host_contract_digest" => manifest.host_contract_digest = value.to_owned(),
                "$.integration_revision" => {
                    manifest.integration_revision = IntegrationRevision::parse(value)
                        .expect("fixture integration revision is canonical")
                }
                _ => panic!("unsupported fixture manifest field"),
            }
            serde_json::to_string(&manifest).expect("serialize fixture manifest")
        };
        guard_manifest_from_json(&updated_manifest)
            .unwrap_or_else(|error| panic!("{path} must retain exact manifest semantics: {error}"));
        conn.execute(
            "UPDATE guard_installations
                SET manifest_json = ?2
              WHERE guard_installation_id = ?1",
            rusqlite::params![INSTALLATION_ID, updated_manifest],
        )?;
        Ok(())
    }

    pub(super) fn force_repair(
        &self,
        verification_id: &str,
        reason: GuardVerificationRepairReason,
        retry_policy: GuardVerificationRetryPolicy,
    ) -> StoreResult<()> {
        let conn = open_registry_database_for_test(registry_db_path(self.runtime_home.path()))?;
        mark_repair_required(
            &conn,
            verification_id,
            ACK_AT,
            reason,
            retry_policy,
            reason.as_str(),
            "Test-owned terminal repair condition.",
        )
    }

    pub(super) fn begin_new_turn<const N: usize>(
        &self,
        turn: &str,
        prompt_event_id: &str,
        observed_at: &str,
        ids: [&str; N],
    ) -> StoreResult<GuardIntegrationVerificationRunRecord> {
        let prompt = prompt_correlation(turn);
        observe_event_correlation(&self.context()?, prompt.clone(), observed_at)?;
        insert_test_event(
            &self.context()?,
            EventFixture {
                integration_revision: &self.integration_revision,
                event_id: prompt_event_id,
                correlation: prompt,
                phase: "prompt_capture",
                occurred_at: observed_at,
                verification_id: None,
                digest: None,
                policy_hash: POLICY_HASH,
            },
        )?;
        let session = bind_agent_session_runtime(
            &self.context()?,
            PROJECT_ID,
            AgentSessionRuntimeBinding {
                runtime_session_id: self.runtime_session_id.clone(),
                connection_internal_id: CONNECTION_ID.to_owned(),
                guard_installation_id: Some(INSTALLATION_ID.to_owned()),
                correlation: mcp_correlation(turn),
                observed_at: observed_at.to_owned(),
            },
        )?;
        begin_guard_integration_verification_with_generator(
            &self.context()?,
            BeginGuardIntegrationVerificationInput {
                caller: self.caller_for_turn(turn),
                project_id: PROJECT_ID.to_owned(),
                project_session_id: session.session_id,
                observed_at: observed_at.to_owned(),
            },
            &SequenceDurableIdGenerator::new(ids),
        )
    }

    pub(super) fn insert_tool_event(&self, event: ToolEventFixture<'_>) -> StoreResult<()> {
        let correlation = tool_correlation(event.turn, event.tool_use_id, event.tool_name);
        observe_event_correlation(&self.context()?, correlation.clone(), event.occurred_at)?;
        insert_test_event(
            &self.context()?,
            EventFixture {
                integration_revision: event
                    .integration_revision
                    .unwrap_or(&self.integration_revision),
                event_id: event.event_id,
                correlation,
                phase: event.phase,
                occurred_at: event.occurred_at,
                verification_id: Some(event.verification_id),
                digest: event.digest,
                policy_hash: event.policy_hash.unwrap_or(POLICY_HASH),
            },
        )?;
        observe_guard_probe_hook_event(
            &self.context()?,
            PROJECT_ID,
            event.event_id,
            GuardProbeHookEvidence::present(Some(event.verification_id.to_owned())),
        )?;
        Ok(())
    }

    pub(super) fn insert_exact_tool_events(&self, verification_id: &str) -> StoreResult<()> {
        let probe_name = host_callable_name(AgentToolId::GUARD_PROBE);
        self.insert_tool_event(ToolEventFixture {
            event_id: "guard_event_pre",
            phase: "pre_tool",
            turn: HOST_TURN_ID,
            tool_use_id: "tool_use_probe",
            tool_name: probe_name.as_str(),
            verification_id,
            occurred_at: "2026-07-23T00:00:03.500Z",
            digest: None,
            policy_hash: None,
            integration_revision: None,
        })?;
        self.insert_tool_event(ToolEventFixture {
            event_id: "guard_event_post",
            phase: "post_tool",
            turn: HOST_TURN_ID,
            tool_use_id: "tool_use_probe",
            tool_name: probe_name.as_str(),
            verification_id,
            occurred_at: "2026-07-23T00:00:04.500Z",
            digest: None,
            policy_hash: None,
            integration_revision: None,
        })
    }

    pub(super) fn insert_incompatible_tool_event(
        &self,
        event_id: &str,
        phase: &str,
        occurred_at: &str,
    ) -> StoreResult<()> {
        insert_guard_event(
            &self.context()?,
            PROJECT_ID,
            GuardEventInsert {
                guard_event_id: event_id.to_owned(),
                correlation: None,
                connection_internal_id: CONNECTION_ID.to_owned(),
                guard_installation_id: INSTALLATION_ID.to_owned(),
                policy_hash: POLICY_HASH.to_owned(),
                integration_revision: self.integration_revision.clone(),
                event_kind: phase.to_owned(),
                contract_status: GuardHookContractStatus::Malformed.as_str().to_owned(),
                decision: GuardDecision::Warn.as_str().to_owned(),
                subject_json: "{}".to_owned(),
                result_json: "{}".to_owned(),
                occurred_at: occurred_at.to_owned(),
                metadata_json: serde_json::json!({
                    "host_contract_digest":
                        HostContractProfileId::CodexCommandHooks.contract_digest()
                })
                .to_string(),
            },
        )?;
        observe_guard_probe_hook_event(
            &self.context()?,
            PROJECT_ID,
            event_id,
            GuardProbeHookEvidence::absent(),
        )?;
        Ok(())
    }

    pub(super) fn complete(
        &self,
        verification_id: &str,
    ) -> StoreResult<GuardIntegrationVerificationRunRecord> {
        self.acknowledge(verification_id, ACK_AT)?;
        self.insert_exact_tool_events(verification_id)?;
        self.record(verification_id)
    }
}

pub(super) fn host_callable_name(tool: AgentToolId) -> HostCallableName {
    let server = McpServerKey::parse(SERVER_KEY).expect("fixture server key");
    project_mcp_tool(&server, tool)
        .expect("fixture callable projection")
        .callable_name()
        .clone()
}

struct EventFixture<'a> {
    integration_revision: &'a str,
    event_id: &'a str,
    correlation: HostNativeCorrelation,
    phase: &'a str,
    occurred_at: &'a str,
    verification_id: Option<&'a str>,
    digest: Option<&'a str>,
    policy_hash: &'a str,
}

fn mcp_correlation(turn: &str) -> CodexMcpCorrelation {
    CodexMcpCorrelation {
        session_id: HostSessionId::parse(HOST_SESSION_ID).expect("session"),
        thread_id: HostThreadId::parse(HOST_THREAD_ID).expect("thread"),
        turn_id: HostTurnId::parse(turn).expect("turn"),
    }
}

fn prompt_correlation(turn: &str) -> HostNativeCorrelation {
    HostNativeCorrelation::CodexHookPrompt(CodexHookPromptCorrelation {
        session_id: HostSessionId::parse(HOST_SESSION_ID).expect("session"),
        turn_id: HostTurnId::parse(turn).expect("turn"),
    })
}

fn tool_correlation(turn: &str, tool_use: &str, tool_name: &str) -> HostNativeCorrelation {
    HostNativeCorrelation::CodexHookTool(CodexHookToolCorrelation {
        session_id: HostSessionId::parse(HOST_SESSION_ID).expect("session"),
        turn_id: HostTurnId::parse(turn).expect("turn"),
        tool_use_id: HostToolUseId::parse(tool_use).expect("tool use"),
        tool_name: CanonicalToolName::parse(tool_name).expect("tool name"),
    })
}

fn observe_event_correlation(
    context: &RuntimeHomeMutationContext<'_>,
    correlation: HostNativeCorrelation,
    observed_at: &str,
) -> StoreResult<()> {
    observe_host_correlation(
        context,
        PROJECT_ID,
        HostCorrelationObservation {
            connection_internal_id: CONNECTION_ID.to_owned(),
            guard_installation_id: Some(INSTALLATION_ID.to_owned()),
            correlation,
            observed_at: observed_at.to_owned(),
        },
    )?;
    Ok(())
}

fn insert_test_event(
    context: &RuntimeHomeMutationContext<'_>,
    event: EventFixture<'_>,
) -> StoreResult<()> {
    insert_guard_event(
        context,
        PROJECT_ID,
        GuardEventInsert {
            guard_event_id: event.event_id.to_owned(),
            correlation: Some(event.correlation),
            connection_internal_id: CONNECTION_ID.to_owned(),
            guard_installation_id: INSTALLATION_ID.to_owned(),
            policy_hash: event.policy_hash.to_owned(),
            integration_revision: event.integration_revision.to_owned(),
            event_kind: event.phase.to_owned(),
            contract_status: GuardHookContractStatus::Compatible.as_str().to_owned(),
            decision: GuardDecision::Allow.as_str().to_owned(),
            subject_json: serde_json::json!({
                "raw_event": {
                    "tool_input": event.verification_id.map(|id| serde_json::json!({
                        "verification_id": id
                    }))
                }
            })
            .to_string(),
            result_json: "{}".to_owned(),
            occurred_at: event.occurred_at.to_owned(),
            metadata_json: serde_json::json!({
                "host_contract_digest": event
                    .digest
                    .map(str::to_owned)
                    .unwrap_or_else(|| HostContractProfileId::CodexCommandHooks.contract_digest())
            })
            .to_string(),
        },
    )?;
    Ok(())
}
