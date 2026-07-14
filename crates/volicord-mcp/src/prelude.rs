pub(crate) use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    error::Error,
    ffi::OsString,
    fmt,
    io::{self, BufRead, Read, Write},
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, TcpListener, TcpStream},
    path::{Path, PathBuf},
    str,
    sync::atomic::Ordering,
    thread,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

pub(crate) use serde::Serialize;
pub(crate) use serde_json::{json, Map, Value};
pub(crate) use volicord_core::{
    local_web_channel_submission_id, rejected_response, tool_error, validate_authority_status,
    AuthorityStatusExpectation, CoreBoundary, CorePipelineError, CoreService,
    CurrentUserActionProjection, GitWorkspaceContext, InvocationContext,
    LocalWebConsentCompletionMetadata, LocalWebConsentUserActionProjectionOutcome,
    LocalWebConsentUserActionProjectionRequest, LocalWebConsentUserActionRequest, PipelineResponse,
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
    diagnostics::{
        record_diagnostic_event, start_diagnostic_session, DiagnosticEvent, DiagnosticEventKind,
        DiagnosticFallbackKind, DiagnosticHostKind, DiagnosticOutcome, DiagnosticSessionStart,
        DiagnosticTransport, DiagnosticUserChannelKind,
    },
    guards::{agent_session, guard_health_record, insert_agent_session, AgentSessionInsert},
    host_capabilities::{
        evaluate_current_host_capability_verification_read_only,
        HostCapabilityVerificationExpectation, HOST_CAPABILITY_ADAPTER_PROFILE_LOCAL_WEB_V1,
        HOST_CAPABILITY_MODEL_INVISIBLE_USER_SURFACE,
    },
    runtime_home::{
        resolve_runtime_home as resolve_shared_runtime_home, RuntimeHomeResolutionError,
    },
    session_watch::{
        create_watch_baseline, latest_watch_baseline_for_session, snapshot_product_repository,
        update_watch_status, SessionWatchStatus as StoreSessionWatchStatus, WatchBaselineCreate,
        WatchBaselineRecord, WatchSnapshotOptions, WatchStatusUpdate,
    },
    sqlite::{open_project_state_database_read_only, sqlite_database_write_capability},
    user_action_channel::{
        create_user_action_channel_token, user_action_channel_current_timestamp,
        validate_user_action_channel_token, UserActionChannelTokenCheck,
        UserActionChannelTokenCreate, UserActionChannelTokenRecord,
        UserActionChannelTokenRejection, UserActionChannelTokenValidation,
    },
    StoreError,
};
pub(crate) use volicord_types::{
    canonical_json_bare_sha256, managed_host_session_id, mcp_request_schema, mcp_response_schema,
    validate_managed_host_native_session_id, validate_managed_host_session_id, ActorSource,
    AgentConnectionId, AgentConnectionMode, AuthorityReceipt, CheckCloseRequest, CloseTaskRequest,
    EffectKind, ErrorCode, EvidenceRelevanceStatus, GetOperationResultRequest, GuaranteeDisclosure,
    IdempotencyKey, IntakeRequest, IntegrationProfile, McpAuthoritativeRefreshFailure,
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
    McpToolErrorIssue, McpToolErrorResponse, McpToolIssueCode, McpUpdateScopeArguments, MethodName,
    MethodOperationCategory, MutationDetailLevel, NextActionSummary, OperationCategory,
    OperationResultRef, PrepareEvidenceCaptureRequest, PrepareEvidenceCaptureResult,
    PrepareWriteRequest, PrepareWriteResult, ProjectId, ReconcileChangesRequest,
    ReconcileChangesResult, RecordId, RecordRunRequest, RecordRunResult, RequestId,
    RequestUserActionRequest, RequestUserActionResponse, RequestUserActionResult, RequiredNullable,
    ResolveUserActionRequest, SessionWatchCoverageBasis, SessionWatchScanSummary,
    SessionWatchStatus, StageArtifactRequest, StageArtifactResult, StateRecordKind, StateRecordRef,
    StatusDetailLevel, StatusRequest, TaskId, ToolEnvelope, ToolResultBase, UpdateScopeRequest,
    UserActionChannelKind, UserActionInboxForm, UserActionInboxItem, UserActionOptionAction,
    UserActionPresentationForm, UserActionPresentationPlan, UserActionPresentationSafety,
    UserActionRequest, UserActionRequestBody, UserActionRequestId, UserActionResolutionInput,
    UserActionStatus, MANAGED_HOST_SESSION_ID_PREFIX, MAX_MCP_TOOL_ERROR_RESULT_BYTES,
    MAX_MCP_TOOL_ISSUE_MESSAGE_BYTES, MAX_MCP_TOOL_ISSUE_PATH_BYTES, MAX_VALIDATION_ISSUES,
    USER_ACTION_FORM_MAX_BYTES, VERIFICATION_BASIS_LOCAL_USER_LOCAL_WEB,
    VERIFICATION_BASIS_MCP_ELICITATION_USER_CHANNEL,
    VERIFICATION_BASIS_MCP_LOCAL_HTTP_CONNECTION_BINDING,
    VERIFICATION_BASIS_MCP_STDIO_CONNECTION_BINDING, VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
};

pub(crate) use crate::constants::*;
