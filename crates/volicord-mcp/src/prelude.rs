// This module is compiled only for the MCP unit-test target and supplies the
// shared protocol, Store, Core, and fixture vocabulary used across that suite.
#![allow(unused_imports)]

pub(crate) use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    error::Error,
    ffi::OsString,
    fmt,
    io::{self, BufRead, Write},
    path::{Path, PathBuf},
    str,
    sync::atomic::Ordering,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

pub(crate) use serde::Serialize;
pub(crate) use serde_json::{json, Map, Value};
pub(crate) use volicord_core::{
    rejected_response, tool_error, validate_authority_status, AuthorityStatusExpectation,
    CorePipelineError, CoreService, CurrentUserActionFacts, GitWorkspaceContext, InvocationContext,
    PipelineResponse,
};
pub(crate) use volicord_mcp_protocol::{
    ClientCapabilitiesShape, CommittedResultRecovery, InitializedNotification, JsonRpcBatching,
    McpProtocolCapabilities, McpProtocolProfile, ProtocolRegistry, ServerCapabilityField,
    ToolDefinitionField, ToolResultCarrier, ToolResultField,
};
pub(crate) use volicord_platform_fs::{
    canonical_runtime_home_path, CanonicalRuntimeHomePath, RuntimeHomeMutationLeaseError,
};
pub(crate) use volicord_store::{
    agent_connections::{
        agent_connection_project_access_read_only, agent_connection_record_read_only,
        list_agent_connections_read_only, list_connection_projects_read_only,
        AgentConnectionRecord, ConnectionProjectRecord, CONNECTION_INTENT_SHARED,
        CONNECTION_MODE_READ_ONLY, CONNECTION_MODE_WORKFLOW, HOST_SCOPE_PROJECT,
    },
    bootstrap::{
        project_record_by_repo_root_read_only, require_installation_profile_read_only,
        runtime_home_record_read_only, ACTIVE_PROJECT_STATUS,
    },
    core_pipeline::CoreProjectStore,
    diagnostic_findings::insert_occurrence_finding,
    diagnostics::{
        record_diagnostic_event, record_workflow_metric_event, start_diagnostic_session,
        DiagnosticEvent, DiagnosticEventKind, DiagnosticFallbackKind, DiagnosticHostKind,
        DiagnosticOutcome, DiagnosticSessionStart, DiagnosticTransport, WorkflowMetricEvent,
        WorkflowMetricKind, WorkflowMetricOutcome,
    },
    guards::{
        bind_agent_session_runtime, current_project_agent_session_coordinates,
        list_guard_installations, AgentSessionRuntimeBinding,
    },
    integration_verification::{
        acknowledge_guard_integration_probe, begin_guard_integration_verification,
        get_guard_integration_verification, BeginGuardIntegrationVerificationInput,
        GuardIntegrationVerificationCaller,
    },
    managed_launch_leases::{
        consume_managed_mcp_launch_lease_and_start_runtime, ManagedMcpLaunchLeaseConsumption,
    },
    operational_sessions::{
        mcp_runtime_session, record_mcp_graceful_close, record_mcp_initialize_attempt,
        record_mcp_initialize_completion, record_mcp_initialized_notification,
        record_mcp_terminal_finding, record_mcp_tools_list,
        record_mcp_verification_tool_observation, start_mcp_runtime_session,
        McpRuntimeSessionStart,
    },
    runtime_home::{
        resolve_runtime_home as resolve_shared_runtime_home, RuntimeHomeResolutionError,
    },
    sqlite::{open_project_state_database_read_only, project_state_database_write_capability},
    RuntimeHomeMutationContext, RuntimeHomeMutationSetupInProgress, StoreError,
};

#[cfg(test)]
pub(crate) use volicord_store::{
    managed_launch_leases::{issue_managed_mcp_launch_lease, ManagedMcpLaunchLeaseIssue},
    operational_sessions::connection_integration_revision,
};

#[cfg(test)]
pub(crate) use volicord_store::guards::guard_health_record;
pub(crate) use volicord_types::diagnostics::OccurrenceDiagnosticFinding;
pub(crate) use volicord_types::ids::{
    AgentConnectionId, AgentRuntimeSessionId, AgentSessionId, IdempotencyKey, ProjectId, RecordId,
    RequestId, TaskId, UserActionRequestId,
};
pub(crate) use volicord_types::integration_revision::McpRuntimeSessionSource;
pub(crate) use volicord_types::integration_verification::{
    BeginIntegrationVerificationArguments, IntegrationVerificationIdArguments,
};
pub(crate) use volicord_types::methods::{
    mcp_request_schema, mcp_response_schema, CheckCloseRequest, CloseTaskRequest,
    GetOperationResultRequest, IntakeRequest, McpAuthoritativeRefreshFailure,
    McpCheckCloseArguments, McpCloseTaskArguments, McpGetOperationResultArguments,
    McpIntakeArguments, McpMutationEffectSummary, McpMutationFullResponse,
    McpMutationPostEffectFailure, McpMutationProjectionErrorCode,
    McpMutationResponseBudgetExceeded, McpMutationSummaryResponse, McpMutationWorkflowResponse,
    McpPostEffectFailureCode, McpPrepareEvidenceCaptureArguments,
    McpPrepareEvidenceCaptureCompactResult, McpPrepareWriteArguments, McpPrepareWriteCompactResult,
    McpReconcileChangesArguments, McpReconcileChangesCompactResult, McpRecordRunArguments,
    McpRecordRunCloseBasisAnchor, McpRecordRunCompactResult, McpRequestUserActionArguments,
    McpRequestUserActionCompactResult, McpRequestUserActionOperation, McpRequestUserActionResponse,
    McpStageArtifactArguments, McpStageArtifactCompactResult, McpStatusArguments, McpToolErrorCode,
    McpToolErrorIssue, McpToolErrorResponse, McpToolIssueCode, McpUpdateScopeArguments,
    MethodOperationCategory, OperationResultRef, PrepareEvidenceCaptureRequest,
    PrepareEvidenceCaptureResult, PrepareWriteRequest, PrepareWriteResult, ReconcileChangesRequest,
    ReconcileChangesResult, RecordRunRequest, RecordRunResult, RequestUserActionRequest,
    RequestUserActionResponse, RequestUserActionResult, StageArtifactRequest, StageArtifactResult,
    StatusRequest, UpdateScopeRequest, MAX_MCP_TOOL_ERROR_RESULT_BYTES,
    MAX_MCP_TOOL_ISSUE_MESSAGE_BYTES, MAX_MCP_TOOL_ISSUE_PATH_BYTES, MAX_VALIDATION_ISSUES,
};
pub(crate) use volicord_types::schema::{
    AuthorityReceipt, NextActionSummary, RequiredNullable, StateRecordRef, ToolEnvelope,
    ToolResultBase,
};
pub(crate) use volicord_types::tool_names::{
    AgentToolCategory, AgentToolId, AgentToolOwner, ToolVerificationRole,
};
pub(crate) use volicord_types::values::{
    ActorSource, AgentConnectionMode, EffectKind, ErrorCode, IntegrationProfile, MethodName,
    MutationDetailLevel, OperationCategory, StateRecordKind, StatusDetailLevel, UserActionStatus,
    UtcTimestamp,
};

pub(crate) use crate::constants::{
    server_instructions, DEFAULT_LOCALE, REQUEST_SEQUENCE, SERVER_NAME, TRANSPORT_DISCLOSURE_TEXT,
};
