# Codebase tour

This tour explains the current Rust workspace by tracing Cargo members, source
files, symbols, dependency direction, and tests. It is a learning guide, not a
contract owner. Exact API behavior, storage effects, schemas, security
guarantees, runtime boundaries, and Core authority semantics remain in
Reference documents.

Code and test paths are written relative to the repository root. Source links
from this page use relative Markdown targets so they can be opened directly.
For the workspace-wide Cargo dependency diagram and runtime maps, use
[Implementation Architecture](architecture.md). For validation-layer choice
after you identify an affected area, use [Testing Strategy](testing-strategy.md).

## First pass reading path

Read in this order when you are learning the public method path:

1. `volicord-types` for typed request, response, value-set, identifier, and
   canonical-hash shapes.
2. `volicord-store` for Runtime Home, project Store, artifact, migration, and
   commit boundaries.
3. `volicord-core` for the shared request pipeline, method planners, policies,
   and Store coordination.
4. `volicord-mcp` for stdio startup, tool registration, typed argument decoding,
   invocation-context derivation, dispatch, and response wrapping.
5. `volicord-test-support`, `tests/integration`, and `tests/conformance` for
   disposable fixtures and cross-layer verification points.

For administrative setup behavior, read `volicord-cli` after `volicord-store`.
The CLI path is local setup and registration, not public Core method behavior.

For repository documentation validation, read `xtask` after the Maintain
policies. It is maintenance tooling and not part of the public method path.

## Cargo Dependency Shape

Normal internal Cargo dependency direction from the current manifests:

- `volicord-types` has no internal dependencies.
- `volicord-store` depends on `volicord-types`.
- `volicord-core` depends on `volicord-store` and `volicord-types`.
- `volicord-cli` depends on `volicord-core`, `volicord-store`, and
  `volicord-types`.
- `volicord-mcp` depends on `volicord-core`, `volicord-store`, and
  `volicord-types`.
- `volicord-test-support` depends on `volicord-store` and `volicord-types`.
- `xtask` has no internal product-crate dependencies; its documentation-parser
  dependencies stay isolated in the maintenance package.

Test-only composition adds `volicord-test-support` to implementation crates and
lets `tests/conformance` and `tests/integration` compose the implementation
crates they exercise. Core still does not depend on CLI or MCP adapters. The
diagram version of this Cargo dependency graph lives in
[Implementation Architecture](architecture.md).

## `crates/volicord-types`

Why it exists:

`volicord-types` is the shared Rust type boundary for public API and
domain-shaped values. It gives adapters, Core, Store, and tests one place to
use the same serde models, JsonSchema generation, controlled value types,
opaque identifiers, and canonical request hashing.

Owns in the implementation:

- Public request and result Rust shapes for supported methods.
- Shared schema-shaped structs such as `ToolEnvelope`, `ToolResultBase`,
  `StateRecordRef`, `StateSummary`, `WriteCheckSummary`,
  `EvidenceSummary`, `CloseReadinessBlocker`, and `ArtifactRef`.
- Controlled value enums such as `MethodName`, `OperationCategory`, `EffectKind`,
  `ResponseKind`, `ResumePolicy`, `PrepareWriteDecision`, and `ErrorCode`.
- Opaque identifier wrappers and durable ID generation helpers.
- Deterministic canonical JSON and request hashing.

Does not own:

- Core method behavior.
- Store mutations, DDL, migrations, or storage effects.
- MCP or CLI transport behavior.
- Product contract meaning for schemas or value sets.

Recommended first file:

- [`crates/volicord-types/src/lib.rs`](../../../crates/volicord-types/src/lib.rs)

Important modules:

- [`crates/volicord-types/src/methods.rs`](../../../crates/volicord-types/src/methods.rs)
  for `MethodOperationCategory`, method request structs, method result structs, and
  `public_request_schema`.
- [`crates/volicord-types/src/schema.rs`](../../../crates/volicord-types/src/schema.rs)
  for shared envelope, response, state, artifact, judgment, and display shapes.
- [`crates/volicord-types/src/values.rs`](../../../crates/volicord-types/src/values.rs)
  for controlled enums and constants.
- [`crates/volicord-types/src/ids.rs`](../../../crates/volicord-types/src/ids.rs)
  for ID wrappers, `DurableIdKind`, `DurableIdGenerator`,
  `RandomDurableIdGenerator`, and `SequenceDurableIdGenerator`.
- [`crates/volicord-types/src/canonical.rs`](../../../crates/volicord-types/src/canonical.rs)
  for `canonical_json_string`, `canonical_json_sha256`, and
  `canonical_request_hash`.

Important current symbols:

- `MethodOperationCategory`, `IntakeRequest`, `StatusRequest`,
  `PrepareWriteRequest`, `RecordRunRequest`, `CloseTaskRequest`
- `ToolEnvelope`, `ToolResponse`, `ToolRejectedResponse`,
  `ToolDryRunResponse`, `ToolError`, `DryRunSummary`
- `MethodName`, `OperationCategory`, `EffectKind`, `ResponseKind`, `ErrorCode`
- `RequiredNullable<T>`, `StateSummary`, `StateRecordRef`,
  `WriteCheckSummary`, `WriteCheckAttemptScope`
- `canonical_request_hash`, `DurableIdGenerator`, `DURABLE_ID_RETRY_LIMIT`

Most relevant tests:

- Unit tests in [`crates/volicord-types/src/lib.rs`](../../../crates/volicord-types/src/lib.rs),
  including `typed_requests_derive_documented_operation_categories`,
  `unknown_top_level_fields_are_rejected_on_public_requests`, and
  `authority_looking_request_fields_are_rejected`.

Recommended next component:

- Read `volicord-core` if you want to see how typed requests become method
  behavior. Read `volicord-mcp` if you want to see how MCP arguments become
  these typed requests.

## `crates/volicord-store`

Why it exists:

`volicord-store` owns SQLite-backed Runtime Home and project Store mechanics:
opening databases, validating schema, bootstrapping local records, applying
migrations, inspecting setup state, staging artifacts, classifying storage
failures, and atomically committing Core mutations.

Owns in the implementation:

- Runtime Home resolution and registry/project path helpers.
- Runtime Home initialization, project registration, and Agent Connection registration.
- SQLite open, schema validation, migration, and transaction helpers.
- `CoreProjectStore` read helpers and `CoreStorageMutation` application.
- The `CoreProjectStore::commit_mutation` atomic transaction boundary.
- Transient artifact staging and persistent artifact body verification helpers.
- Read-only inspection snapshots used by setup and diagnostics.
- `StoreError`, `StoreFailureRoute`, and storage failure classification.

Does not own:

- Public method behavior or method policy.
- Adapter semantics for MCP or CLI.
- Product-file writes in `Product Repository`.
- Exact storage contracts, DDL meaning, or storage effect contracts.

Recommended first file:

- [`crates/volicord-store/src/lib.rs`](../../../crates/volicord-store/src/lib.rs)

Important modules:

- [`crates/volicord-store/src/runtime_home.rs`](../../../crates/volicord-store/src/runtime_home.rs)
  for `resolve_runtime_home` and `RuntimeHomeResolutionError`.
- [`crates/volicord-store/src/bootstrap.rs`](../../../crates/volicord-store/src/bootstrap.rs)
  for `initialize_runtime_home`, `register_project`, `ProjectRegistration`,
  and `ProjectRecord`.
- [`crates/volicord-store/src/agent_connections.rs`](../../../crates/volicord-store/src/agent_connections.rs)
  for `AgentConnectionRecord`, `AgentConnectionRegistration`,
  `ensure_agent_connection`, and `add_connection_project`.
- [`crates/volicord-store/src/sqlite.rs`](../../../crates/volicord-store/src/sqlite.rs)
  for database paths, opening, validation, and `begin_immediate_transaction`.
- [`crates/volicord-store/src/migrations.rs`](../../../crates/volicord-store/src/migrations.rs)
  for baseline migration constants and migration application.
- [`crates/volicord-store/src/core_pipeline.rs`](../../../crates/volicord-store/src/core_pipeline.rs)
  for Core-facing Store reads, `CoreStorageMutation`, and commit outcomes.
- [`crates/volicord-store/src/artifacts.rs`](../../../crates/volicord-store/src/artifacts.rs)
  for `CoreProjectStore::create_artifact_staging` and
  `verify_persistent_artifact_body`.
- [`crates/volicord-store/src/inspection.rs`](../../../crates/volicord-store/src/inspection.rs)
  for read-only Runtime Home and project-state inspection.
- [`crates/volicord-store/src/error.rs`](../../../crates/volicord-store/src/error.rs)
  for `StoreError` and storage failure routing.

Important current symbols:

- `CoreProjectStore`, `ProjectStateHeader`, `ProjectEnforcementProfileRecord`
- `ToolInvocationRecord`, `VerifiedReplayContext`, `PendingTaskEvent`
- `CommitMutationInput`, `MutationCommitOutcome`, `CommittedMutationFacts`
- `CoreStorageMutation`, `StorageEffectCounts`, `ProjectMutation`
- `RuntimeHomeRecord`, `ProjectRegistration`, `AgentConnectionRegistration`
- `ArtifactStagingInsert`, `ArtifactStagingRecord`,
  `PersistentArtifactVerification`
- `inspect_runtime_home`, `inspect_registry_database`,
  `inspect_project_state_database`

Most relevant tests:

- Colocated unit tests in the Store modules.
- Core method tests in
  [`crates/volicord-core/src/methods/tests.rs`](../../../crates/volicord-core/src/methods/tests.rs)
  for Store-visible effects.
- Cross-layer storage checks in
  [`tests/integration/mcp_connection.rs`](../../../tests/integration/mcp_connection.rs)
  and [`tests/conformance/baseline.rs`](../../../tests/conformance/baseline.rs).

Recommended next component:

- Read `volicord-core` to see how method planners choose Store reads and
  `CoreStorageMutation` values. Read `volicord-cli` to see local setup use Store
  bootstrap and inspection directly.

## `crates/volicord-core`

Why it exists:

`volicord-core` owns Core-facing services for public Volicord method behavior. It
keeps adapter-independent method behavior in one crate and coordinates Store
reads, policy checks, method plans, dry-run previews, committed mutations, and
common response construction.

Owns in the implementation:

- `CoreService` and the public method entry functions on it.
- Common preflight for envelope shape, adapter binding, request hashing, Store
  opening, project state, invocation verification, replay, Task resolution,
  state-version freshness, and operation-category checks.
- Method-specific planning in `crates/volicord-core/src/methods/`.
- Reusable policy helpers in `crates/volicord-core/src/policy/`.
- Core response construction and routing to read-only, no-effect, dry-run, or
  committed mutation branches.

Does not own:

- MCP stdio framing or CLI setup behavior.
- SQLite DDL, migration definitions, or raw storage layout contracts.
- Product-file writes in `Product Repository`.
- Public schema contracts or exact value-set meaning.

Recommended first file:

- [`crates/volicord-core/src/lib.rs`](../../../crates/volicord-core/src/lib.rs),
  then [`crates/volicord-core/src/pipeline.rs`](../../../crates/volicord-core/src/pipeline.rs)

Important modules:

- [`crates/volicord-core/src/pipeline.rs`](../../../crates/volicord-core/src/pipeline.rs)
  for `CoreService`, `InvocationContext`, `MethodPolicy`,
  `OwnerPipelineBranch`, `PreparedRequest`, `PipelineResponse`,
  `CoreService::prepare_request`, and
  `CoreService::execute_prepared_request`.
- [`crates/volicord-core/src/methods/`](../../../crates/volicord-core/src/methods/)
  for method-specific entry functions and planners.
- [`crates/volicord-core/src/methods/status.rs`](../../../crates/volicord-core/src/methods/status.rs)
  for `CoreService::status`, `status_task`, and `status_result_fields`.
- [`crates/volicord-core/src/methods/intake.rs`](../../../crates/volicord-core/src/methods/intake.rs)
  for `CoreService::intake` and `plan_intake`.
- [`crates/volicord-core/src/methods/prepare_write.rs`](../../../crates/volicord-core/src/methods/prepare_write.rs)
  for `CoreService::prepare_write`, `prepare_write_policy`, and
  `plan_prepare_write`.
- [`crates/volicord-core/src/policy/`](../../../crates/volicord-core/src/policy/)
  for access, replay, path, Write Check, evidence, judgment relevance,
  and close-readiness helpers.

Important current symbols:

- `CoreService`, `CoreResult`, `CorePipelineError`
- `AdapterSessionBinding`, `InvocationContext`, `VerifiedInvocationContext`,
  `VerifiedActorContext`
- `MethodPolicy`, `TaskRequirement`, `ReplayPolicy`, `FreshnessPolicy`,
  `MethodEffectPolicy`
- `OwnerPipelineBranch`, `PreparedRequest`, `VerifiedRequestContext`,
  `PipelinePreflightOutcome`, `PipelineResponse`
- `prepare_or_response`, `mutation_method_policy`, `validation_rejected`
- `CoreService::status`, `CoreService::intake`,
  `CoreService::prepare_write`, `CoreService::record_run`,
  `CoreService::close_task`

Most relevant tests:

- [`crates/volicord-core/src/pipeline.rs`](../../../crates/volicord-core/src/pipeline.rs)
  has unit tests for replay, freshness, branch shape, no-effect behavior, and
  Store failure routing.
- [`crates/volicord-core/src/methods/tests.rs`](../../../crates/volicord-core/src/methods/tests.rs)
  exercises method plans and effects. Start with
  `status_is_read_only_including_dry_run`,
  `intake_commits_once_and_replays_without_effect`,
  `prepare_write_allowed_creates_one_write_check_with_post_commit_basis`,
  `prepare_write_dry_run_has_no_write_check_effect`, and
  `status_read_only_rejects_corrupt_owner_state_without_effect`.
- Cross-layer confirmation lives in
  [`tests/integration/mcp_connection.rs`](../../../tests/integration/mcp_connection.rs)
  and [`tests/conformance/baseline.rs`](../../../tests/conformance/baseline.rs).

Recommended next component:

- Read `volicord-store` for commit mechanics and `volicord-mcp` for adapter
  dispatch into `CoreService`.

## `crates/volicord-cli`

Why it exists:

`volicord-cli` implements the local `volicord` administrative executable and
reusable agent setup modules. It handles Runtime Home initialization, project
and Agent Connection registration, Agent Connection setup, host-specific
MCP configuration, connection-project membership, and preflight execution.

Owns in the implementation:

- Process entry and administrative command dispatch for the `volicord` binary.
- `volicord agent` option parsing, storage preparation, host plan construction,
  preflight invocation, connect/status/verify/project membership/uninstall
  commands, and output.
- Codex, Claude Code, and generic export host integration planning.
- Managed host configuration planning and safety checks.
- Agent Connection and invocation metadata generation.

Does not own:

- Public Volicord API method behavior.
- MCP `tools/call` semantics.
- Core state transitions or method policy.
- Exact CLI command contracts.

Recommended first file:

- [`crates/volicord-cli/src/main.rs`](../../../crates/volicord-cli/src/main.rs)

Important modules:

- [`crates/volicord-cli/src/main.rs`](../../../crates/volicord-cli/src/main.rs)
  for process dispatch, `run_cli`, `command_init`, and `command_project`.
- [`crates/volicord-cli/src/agent_command.rs`](../../../crates/volicord-cli/src/agent_command.rs)
  for `volicord agent` connection, project membership, status, verification,
  and uninstall command orchestration.
- [`crates/volicord-cli/src/host_integration/`](../../../crates/volicord-cli/src/host_integration/)
  for Codex, Claude Code, and generic host integration adapters.
- [`crates/volicord-cli/src/registration.rs`](../../../crates/volicord-cli/src/registration.rs)
  for Agent Connection, Connection Project, and User Channel registry helpers.
- [`crates/volicord-cli/src/user_command.rs`](../../../crates/volicord-cli/src/user_command.rs)
  for local User Channel status and judgment commands.

Important current symbols:

- `run_cli`, `CliError`
- `run_agent_command`, `agent_usage`, `AgentCommandError`,
  `AgentProcessOutput`
- `command_connect`, `command_status`, `command_verify`,
  `command_uninstall`
- `command_project_add`, `command_project_remove`
- `HostKind`, `HostScope`, `HostPlan`, `HostAdapter`, `Verification`
- `AgentConnectionRegistration`, `ConnectionProjectRegistration`,
  `AgentConnectionRecord`
- `actor_source`, `operation_category`, `connection_id`,
  `verification_basis`

Most relevant tests:

- [`crates/volicord-cli/tests/binary_admin.rs`](../../../crates/volicord-cli/tests/binary_admin.rs)
  exercises the `volicord` binary for administrative setup, dry-run behavior,
  `volicord agent` host setup, connection-project membership, setup-command rejection,
  preflight handling, and config-file safety.
- Colocated unit tests in CLI modules cover parsing, planning, rendering,
  registration metadata, and host-configuration behavior.

Recommended next component:

- Read `volicord-store` for bootstrap, inspection, and registry storage calls.
  Read `volicord-mcp` for the `volicord-mcp --check --connection` preflight path
  that agent setup validates.

## `crates/volicord-mcp`

Why it exists:

`volicord-mcp` is the local MCP stdio adapter. It registers public Volicord method
tools, validates startup/session binding, decodes `tools/call` arguments into
typed requests, derives trusted invocation context from the local session, calls
Core, and wraps Core's JSON response in an MCP `tools/call` result.

Owns in the implementation:

- `volicord-mcp` binary command modes: stdio, `--check`, help, and version.
- Runtime Home and session binding validation for MCP startup.
- Tool metadata returned by `tools/list`.
- `tools/call` dispatch, typed argument decoding, and invocation-context
  derivation.
- JSON-RPC stdio framing and MCP response wrapping.

Does not own:

- Public method behavior once Core is called.
- Store mutation policy.
- Administrative CLI setup behavior.
- Product-file writes in `Product Repository`.

Recommended first file:

- [`crates/volicord-mcp/src/lib.rs`](../../../crates/volicord-mcp/src/lib.rs)

Important modules:

- [`crates/volicord-mcp/src/lib.rs`](../../../crates/volicord-mcp/src/lib.rs)
  for `PUBLIC_METHOD_TOOL_NAMES`, `McpConnectionStartupInspection`,
  `McpConnectionContext`, `McpAdapter`, `McpAdapter::call_tool`,
  `prepare_connection_arguments`, `public_method_tools`, `run_stdio_from_env`,
  `handle_json_rpc_request`, and `call_tool_result`.
- [`crates/volicord-mcp/src/main.rs`](../../../crates/volicord-mcp/src/main.rs)
  for process-mode dispatch through `dispatch_args`.

Important current symbols:

- `PUBLIC_METHOD_TOOL_NAMES`, `McpToolDefinition`, `public_method_tools`
- `McpConnectionStartupInspection`, `McpConnectionContext`,
  `McpDerivedInvocationContext`
- `McpAdapter`, `McpAdapter::derive_invocation_context`,
  `McpAdapter::call_tool`
- `prepare_typed_request`, `prepare_connection_arguments`, `decode_params`
- `run_stdio_from_env`, `run_preflight_check_from_env`,
  `preflight_check`
- `McpAdapterError`, `call_tool_result`, `json_rpc_error_for_adapter`

Most relevant tests:

- Unit tests in [`crates/volicord-mcp/src/lib.rs`](../../../crates/volicord-mcp/src/lib.rs),
  including `tool_sets_follow_connection_mode_and_exclude_user_only_recording`,
  `connection_context_resolves_and_preflight_reports_allowed_project`,
  `adapter_auto_selects_single_project_and_injects_connection_invocation`,
  `read_only_mode_rejects_agent_workflow_calls_before_core`, and
  `mcp_visible_schemas_make_project_selector_optional`.
- [`crates/volicord-mcp/tests/binary_transport.rs`](../../../crates/volicord-mcp/tests/binary_transport.rs)
  exercises the binary, `--check`, stdio framing, reconnect behavior, and MCP
  response wrapping.
- [`tests/integration/mcp_connection.rs`](../../../tests/integration/mcp_connection.rs)
  exercises cross-layer MCP/Core/Store behavior.

Recommended next component:

- Read `volicord-core` for the method semantics behind each `McpAdapter` branch.
  Read `volicord-store` for startup validation and session-binding reads.

## `crates/volicord-test-support`

Why it exists:

`volicord-test-support` provides disposable fixture infrastructure shared by
implementation, integration, and conformance tests. It keeps Runtime Home,
Product Repository, project registration, Agent Connection registration, request
builders, and direct Store inspection helpers out of production crates.

Owns in the implementation:

- Temporary Runtime Home helpers under the system temporary directory.
- Shared `CoreFixture` setup with one registered project and Agent Connection.
- Request builders for public method tests.
- Fixture-only Store inspection and mutation helpers used by tests.
- Small marker modules for future fixture and golden-output helpers.

Does not own:

- Product contracts.
- Public API behavior.
- Durable Runtime Home data.
- Generated reports or runtime output.

Recommended first file:

- [`crates/volicord-test-support/src/lib.rs`](../../../crates/volicord-test-support/src/lib.rs)

Important modules:

- `fixtures` and `golden` marker modules.
- `core_fixtures` for `CoreFixture`, request builders, and fixture utilities.

Important current symbols:

- `disposable_runtime_home`, `TempRuntimeHome`
- `CoreFixture`, `CoreFixture::new`, `CoreFixture::store`,
  `CoreFixture::counts`, `CoreFixture::conn`
- Fixture request builders such as `intake_request`, `status_request`,
  `prepare_write_request`, `update_scope_request`, `record_run_request`,
  `request_user_judgment_request`, `record_user_judgment_request`, and
  `close_task_request`
- `UpdateScopeFixture`, `RecordJudgmentFixture`, `CloseTaskFixture`,
  `UserJudgmentFixture`
- `supported_evidence_update`, `unsupported_evidence_update`,
  `artifact_input_for_handle`

Most relevant tests:

- This crate is primarily exercised through
  [`crates/volicord-core/src/methods/tests.rs`](../../../crates/volicord-core/src/methods/tests.rs),
  [`tests/integration/mcp_connection.rs`](../../../tests/integration/mcp_connection.rs),
  and [`tests/conformance/baseline.rs`](../../../tests/conformance/baseline.rs).

Recommended next component:

- Read whichever test package is using the fixture. Start with
  `tests/integration` for adapter behavior or `tests/conformance` for
  cross-method baseline scenarios.

## `tests/conformance`

Why it exists:

`tests/conformance` is a Cargo workspace member containing the
`volicord-conformance-tests` package and the `baseline` test target. It exercises
baseline cross-method scenarios through Core-facing APIs and shared fixtures.

Owns in the implementation:

- Baseline scenario coverage that composes Core-facing public methods.
- Cross-method checks for effect branches, idempotency, Write Check,
  artifact lifecycle, judgment boundaries, close readiness, error routing, and
  corruption handling.

Does not own:

- Product contract meaning or conformance authority.
- Public API schemas.
- Store DDL or storage effect definitions.
- Adapter transport behavior.

Recommended first file:

- [`tests/conformance/baseline.rs`](../../../tests/conformance/baseline.rs)

Important current symbols:

- `no_effect_branches_state_version_and_idempotency_are_stable`
- `idempotency_replay_rejects_actor_source_mismatch`
- `idempotency_replay_rejects_operation_category_mismatch`
- `committed_non_allow_prepare_write_audit_and_replay_are_exact`
- `prepare_write_allocates_write_check_only_on_committed_allowed_effect`
- `status_projection_matches_public_close_check_and_stays_read_only`
- Shared helpers such as `core`, `invocation`,
  `create_task_with_change_unit`, and `prepare_write_check`

Most relevant tests:

- The package exposes the `baseline` test target from
  [`tests/conformance/baseline.rs`](../../../tests/conformance/baseline.rs).

Recommended next component:

- Read `volicord-core` method tests for smaller focused cases, then return to
  Reference owners for exact behavior questions.

## `tests/integration`

Why it exists:

`tests/integration` is a Cargo workspace member containing the
`volicord-integration-tests` package and the `mcp_connection` test target. It
verifies the cross-layer MCP, Core, Store, connection binding, and invocation-path
composition.

Owns in the implementation:

- Tool exposure and schema exposure through MCP.
- MCP session binding, invocation-context derivation, and operation-category routing.
- MCP/Core response parity for representative requests.
- Cross-layer storage effects and no-effect checks.
- Stdio protocol error handling that should not mutate Store state.

Does not own:

- Public method contracts.
- MCP transport contracts.
- Store contracts.
- Core authority semantics.

Recommended first file:

- [`tests/integration/mcp_connection.rs`](../../../tests/integration/mcp_connection.rs)

Important current symbols:

- `workflow_tools_include_agent_workflow_and_read_tools_but_exclude_user_only`
- `read_only_tools_expose_only_read_operations_and_project_discovery`
- `connection_invocation_is_injected_and_single_project_is_auto_selected`
- `read_only_mode_rejects_agent_workflow_methods_before_core`
- `multiple_allowed_projects_require_explicit_project_id`
- `explicit_project_outside_allowlist_is_rejected_before_core`
- `explicit_allowed_project_routes_to_that_project`
- Helpers such as `adapter`, `invocation`, `set_connection_mode`, and
  `set_project_id`

Most relevant tests:

- The package exposes the `mcp_connection` test target from
  [`tests/integration/mcp_connection.rs`](../../../tests/integration/mcp_connection.rs).

Recommended next component:

- Read `volicord-mcp` for the adapter path under test, then `volicord-core` and
  `volicord-store` for the behavior behind successful calls.

## `xtask`

Why it exists:

`xtask` is a repository maintenance package for deterministic documentation
validation. It exposes `cargo run -p xtask -- docs-check` and keeps
documentation-tooling dependencies out of product and test-support crates.

Owns in the implementation:

- Version 3 `docs/doc-index.yaml` structural validation, including
  owner-area, date, and applicability metadata.
- Bilingual maintained Markdown coverage checks for `docs/en/` and `docs/ko/`.
- Local Markdown link and fragment validation, including hidden anchors.
- `docs/terminology-map.yaml` repository-document path validation.
- Retired documentation path detection in maintained Markdown and YAML route
  metadata.

Does not own:

- Volicord runtime behavior.
- Public API, schema, storage, security, or Core authority contracts.
- Semantic translation review or contract-owner technical review.
- Automatic file rewriting.

Recommended first file:

- [`xtask/src/lib.rs`](../../../xtask/src/lib.rs), then
  [`xtask/src/main.rs`](../../../xtask/src/main.rs)

Most relevant tests:

- [`xtask/tests/docs_check.rs`](../../../xtask/tests/docs_check.rs) uses small
  temporary fixture trees for metadata, pairing, link, fragment, retired-path,
  and terminology-path cases.

Recommended next component:

- Read [Validation](../maintain/validation.md) for the maintenance policy that
  names the command and separates automated structure checks from manual review.
