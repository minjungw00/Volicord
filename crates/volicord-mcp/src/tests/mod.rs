//! MCP adapter tests partitioned by durable contract.

use std::{
    collections::BTreeSet,
    error::Error,
    ffi::OsString,
    fs,
    io::{BufReader, Cursor},
    panic::{catch_unwind, AssertUnwindSafe},
};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use crate::prelude::*;
use crate::stdio::{
    handle_json_rpc_message, run_managed_stdio_with_test_lease,
    run_manual_stdio_with_ignored_env_marker, tool_execution_error_result, ConnectionPhase,
    ConnectionState, MAX_MCP_COMPACT_MUTATION_RESULT_BYTES, MAX_MCP_FULL_MUTATION_RESULT_BYTES,
    MAX_MCP_MUTATION_COMPATIBILITY_TEXT_BYTES,
};
use crate::{
    adapter::{AgentSessionCoordinates, ManagedAgentSessionBinding},
    routing::McpStorageCapability,
    tool_registry::{
        canonical_tool_examples, compact_runtime_schema, mcp_tool_naming_style,
        mcp_tools_for_mode_and_storage, mcp_tools_for_mode_and_storage_with_detail,
        validate_tools_list_json_compatibility, validate_tools_list_schema_compatibility,
        ToolSchemaDetail, CHECK_CLOSE_MISSING_FINAL_ACCEPTANCE_EXAMPLE_ID,
        GET_OPERATION_RESULT_FIRST_PAGE_EXAMPLE_ID, MAX_RUNTIME_TOOLS_LIST_BYTES,
        PREPARE_EVIDENCE_CAPTURE_VERIFIED_COMMAND_EXAMPLE_ID,
        PREPARE_EVIDENCE_CAPTURE_VERIFIED_TOOL_EXAMPLE_ID, PREPARE_WRITE_SIMPLE_EXAMPLE_ID,
        RECORD_RUN_ADVISOR_NO_PRODUCT_WRITE_EXAMPLE_ID,
        REQUEST_USER_ACTION_FINAL_ACCEPTANCE_EXAMPLE_ID, STATUS_READ_ONLY_EXAMPLE_ID,
        UPDATE_SCOPE_KEEP_CURRENT_EXAMPLE_ID,
    },
};
use volicord_core::CoreBoundary;
use volicord_host_contract::{
    CodexMcpCorrelation, HostNativeCorrelation, HostSessionId, HostThreadId, HostTurnId,
};
use volicord_mcp_protocol::ToolResultField;
use volicord_store::agent_connections::{
    add_connection_project, agent_connection_record, ensure_agent_connection,
    set_connection_enabled, AgentConnectionRegistration, ConnectionProjectRegistration,
    CONNECTION_MODE_READ_ONLY,
};
use volicord_store::bootstrap::{register_project, ProjectRegistration, ACTIVE_PROJECT_STATUS};
use volicord_store::diagnostic_findings::{
    diagnostic_occurrences_for_runtime_session, stored_diagnostic_findings_by_ids,
};
use volicord_store::diagnostics::{
    diagnostics_db_path, read_diagnostic_session, read_workflow_metric_aggregates,
    WorkflowMetricAggregateRow,
};
use volicord_store::guards::{agent_session, upsert_guard_installation, GuardInstallationUpsert};
use volicord_store::operational_sessions::{latest_managed_runtime_session, mcp_runtime_session};
use volicord_store::sqlite::{open_registry_database_read_only, registry_db_path};
use volicord_test_support::core_fixtures::{
    artifact_input_for_handle, CoreFixture, ResolveUserActionFixture, UpdateScopeFixture,
    UserActionFixture,
};
use volicord_types::{
    AgentConnectionMode, ChangeUnitOperation, CloseAssessmentInput, OperationCategory,
    ResidualRiskInput, StagedArtifactHandle, CODEX_MANAGED_MCP_CLIENT_NAME,
};

fn production_profiles(
) -> impl ExactSizeIterator<Item = &'static volicord_mcp_protocol::McpProtocolProfile> {
    ProtocolRegistry::production().oldest_to_newest()
}

use super::*;

const CODEX_TEST_SESSION_ID: &str = "fixture_codex_session";
const CODEX_TEST_THREAD_ID: &str = "fixture_codex_thread";
const CODEX_TEST_TURN_ID: &str = "fixture_codex_turn";
const CODEX_TEST_CLIENT_VERSION: &str = "test-codex-client";

mod batching;
mod conformance;
mod diagnostics;
mod lifecycle;
mod managed_host_observation;
mod protocol_projection;
mod support;
mod tool_calls;

use support::*;
