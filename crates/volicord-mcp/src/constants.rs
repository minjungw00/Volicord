use std::{sync::atomic::AtomicU64, time::Duration};

use volicord_types::VERIFICATION_BASIS_MCP_STDIO_CONNECTION_BINDING;

pub(crate) const SUPPORTED_PROTOCOL_VERSION: &str = "2025-11-25";
pub(crate) const SERVER_NAME: &str = "volicord-mcp";
pub(crate) const DEFAULT_INVOCATION_BINDING_BASIS: &str =
    VERIFICATION_BASIS_MCP_STDIO_CONNECTION_BINDING;
pub(crate) const DEFAULT_LOCALE: &str = "en-US";
pub(crate) const CHECK_CLOSE_TOOL_NAME: &str = "volicord.check_close";
pub(crate) const ELICITATION_CREATE_METHOD: &str = "elicitation/create";
pub(crate) const LOCAL_WEB_CONSENT_PATH: &str = "/consent";
pub(crate) const LOCAL_WEB_CONSENT_TOKEN_TTL_SECONDS: u64 = 10 * 60;
pub(crate) static REQUEST_SEQUENCE: AtomicU64 = AtomicU64::new(1);

/// Agent-facing Volicord tools exposed through workflow MCP connections.
pub const PUBLIC_METHOD_TOOL_NAMES: [&str; 10] = [
    "volicord.intake",
    "volicord.update_scope",
    "volicord.status",
    "volicord.prepare_write",
    "volicord.stage_artifact",
    "volicord.record_run",
    "volicord.request_user_judgment",
    "volicord.reconcile_changes",
    CHECK_CLOSE_TOOL_NAME,
    "volicord.close_task",
];

/// Public method tools exposed through read-only MCP connections.
pub const READ_ONLY_METHOD_TOOL_NAMES: [&str; 2] = ["volicord.status", CHECK_CLOSE_TOOL_NAME];

/// Adapter-owned MCP utility tools that are not public Core methods.
pub const ADAPTER_UTILITY_TOOL_NAMES: [&str; 1] = ["volicord.list_projects"];

pub(crate) const LIST_PROJECTS_TOOL_NAME: &str = "volicord.list_projects";
pub(crate) const SERVER_INSTRUCTIONS: &str = "Volicord records task scope, write tickets, evidence, runs, user-owned judgment requests, artifacts, and close readiness for explicitly registered Product Repositories. If project selection is unclear, call volicord.list_projects and use one listed project_selector; do not guess from folders, roots, labels, or memory. Volicord state management is separate from permission to edit product files: product-file edits still require the host/user path and any required write ticket. A write ticket records intended product-file changes; it is not OS permission, review bypass, access control, or a promise of automatic tool use.";
pub(crate) const WATCH_METADATA_SOURCE: &str = "volicord_session_watch";
pub(crate) const TRANSPORT_DISCLOSURE_TEXT: &str = "disclosure: transport diagnostics only; not OS sandboxing, network isolation, malware defense, full write prevention, actor identity proof, correctness proof, test sufficiency proof, or human review replacement";
pub(crate) const FIRST_PROJECT_SELECTION_PARTIAL_COVERAGE_WARNING: &str =
    "Session-watch coverage starts at first explicit project selection; Product Repository changes before project selection are outside watcher coverage.";
pub(crate) const METHOD_BOUNDARY_PARTIAL_COVERAGE_WARNING: &str =
    "Session-watch coverage starts at a method boundary; Product Repository changes before that boundary are outside watcher coverage.";
pub(crate) const HTTP_HEADER_LIMIT_BYTES: usize = 16 * 1024;
pub(crate) const HTTP_BODY_LIMIT_BYTES: usize = 1024 * 1024;
pub(crate) const HTTP_READ_TIMEOUT: Duration = Duration::from_secs(5);
