# Codebase tour

This tour explains the current Rust workspace by tracing Cargo members, source
files, symbols, dependency direction, and tests. It is a learning guide, not a
contract owner. Exact API behavior, storage effects, schemas, security
guarantees, runtime boundaries, and Core authority semantics remain in
Reference documents.

Code and test paths are written relative to the repository root. Source links
from this page use relative Markdown targets so they can be opened directly.

## First pass reading path

Read in this order when you are learning the public method path:

1. `harness-types` for typed request, response, value-set, identifier, and
   canonical-hash shapes.
2. `harness-store` for Runtime Home, project Store, artifact, migration, and
   commit boundaries.
3. `harness-core` for the shared request pipeline, method planners, policies,
   and Store coordination.
4. `harness-mcp` for stdio startup, tool registration, typed argument decoding,
   invocation-context derivation, dispatch, and response wrapping.
5. `harness-test-support`, `tests/integration`, and `tests/conformance` for
   disposable fixtures and cross-layer proof points.

For administrative setup behavior, read `harness-cli` after `harness-store`.
The CLI path is local setup and registration, not public Core method behavior.

For repository documentation validation, read `xtask` after the Maintain
policies. It is maintenance tooling and not part of the public method path.

## Dependency shape

Normal internal dependency direction from the current Cargo manifests:

- `harness-types` has no internal dependencies.
- `harness-store` depends on `harness-types`.
- `harness-core` depends on `harness-store` and `harness-types`.
- `harness-cli` depends on `harness-store` and `harness-types`.
- `harness-mcp` depends on `harness-core`, `harness-store`, and
  `harness-types`.
- `harness-test-support` depends on `harness-store` and `harness-types`.
- `xtask` has no internal product-crate dependencies; its documentation-parser
  dependencies stay isolated in the maintenance package.

Test-only composition adds `harness-test-support` to implementation crates and
lets `tests/conformance` and `tests/integration` compose the implementation
crates they exercise. Core still does not depend on CLI or MCP adapters.

## `crates/harness-types`

Why it exists:

`harness-types` is the shared Rust type boundary for public API and
domain-shaped values. It gives adapters, Core, Store, and tests one place to
use the same serde models, JsonSchema generation, controlled value types,
opaque identifiers, and canonical request hashing.

Owns in the implementation:

- Public request and result Rust shapes for supported methods.
- Shared schema-shaped structs such as `ToolEnvelope`, `ToolResultBase`,
  `StateRecordRef`, `StateSummary`, `WriteAuthorizationSummary`,
  `EvidenceSummary`, `CloseReadinessBlocker`, and `ArtifactRef`.
- Controlled value enums such as `MethodName`, `AccessClass`, `EffectKind`,
  `ResponseKind`, `ResumePolicy`, `PrepareWriteDecision`, and `ErrorCode`.
- Opaque identifier wrappers and durable ID generation helpers.
- Deterministic canonical JSON and request hashing.

Does not own:

- Core method behavior.
- Store mutations, DDL, migrations, or storage effects.
- MCP or CLI transport behavior.
- Product contract meaning for schemas or value sets.

Recommended first file:

- [`crates/harness-types/src/lib.rs`](../../../crates/harness-types/src/lib.rs)

Important modules:

- [`crates/harness-types/src/methods.rs`](../../../crates/harness-types/src/methods.rs)
  for `MethodAccessClass`, method request structs, method result structs, and
  `public_request_schema`.
- [`crates/harness-types/src/schema.rs`](../../../crates/harness-types/src/schema.rs)
  for shared envelope, response, state, artifact, judgment, and display shapes.
- [`crates/harness-types/src/values.rs`](../../../crates/harness-types/src/values.rs)
  for controlled enums and constants.
- [`crates/harness-types/src/ids.rs`](../../../crates/harness-types/src/ids.rs)
  for ID wrappers, `DurableIdKind`, `DurableIdGenerator`,
  `RandomDurableIdGenerator`, and `SequenceDurableIdGenerator`.
- [`crates/harness-types/src/canonical.rs`](../../../crates/harness-types/src/canonical.rs)
  for `canonical_json_string`, `canonical_json_sha256`, and
  `canonical_request_hash`.

Important current symbols:

- `MethodAccessClass`, `IntakeRequest`, `StatusRequest`,
  `PrepareWriteRequest`, `RecordRunRequest`, `CloseTaskRequest`
- `ToolEnvelope`, `ToolResponse`, `ToolRejectedResponse`,
  `ToolDryRunResponse`, `ToolError`, `DryRunSummary`
- `MethodName`, `AccessClass`, `EffectKind`, `ResponseKind`, `ErrorCode`
- `RequiredNullable<T>`, `StateSummary`, `StateRecordRef`,
  `WriteAuthorizationSummary`, `AuthorizedAttemptScope`
- `canonical_request_hash`, `DurableIdGenerator`, `DURABLE_ID_RETRY_LIMIT`

Most relevant tests:

- Unit tests in [`crates/harness-types/src/lib.rs`](../../../crates/harness-types/src/lib.rs),
  including `typed_requests_derive_documented_access_classes`,
  `unknown_top_level_fields_are_rejected_on_public_requests`, and
  `authority_looking_request_fields_are_rejected`.

Recommended next component:

- Read `harness-core` if you want to see how typed requests become method
  behavior. Read `harness-mcp` if you want to see how MCP arguments become
  these typed requests.

## `crates/harness-store`

Why it exists:

`harness-store` owns SQLite-backed Runtime Home and project Store mechanics:
opening databases, validating schema, bootstrapping local records, applying
migrations, inspecting setup state, staging artifacts, classifying storage
failures, and atomically committing Core mutations.

Owns in the implementation:

- Runtime Home resolution and registry/project path helpers.
- Runtime Home initialization, project registration, and surface registration.
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

- [`crates/harness-store/src/lib.rs`](../../../crates/harness-store/src/lib.rs)

Important modules:

- [`crates/harness-store/src/runtime_home.rs`](../../../crates/harness-store/src/runtime_home.rs)
  for `resolve_runtime_home` and `RuntimeHomeResolutionError`.
- [`crates/harness-store/src/bootstrap.rs`](../../../crates/harness-store/src/bootstrap.rs)
  for `initialize_runtime_home`, `register_project`, `register_surface`,
  `ProjectRecord`, and `SurfaceRecord`.
- [`crates/harness-store/src/sqlite.rs`](../../../crates/harness-store/src/sqlite.rs)
  for database paths, opening, validation, and `begin_immediate_transaction`.
- [`crates/harness-store/src/migrations.rs`](../../../crates/harness-store/src/migrations.rs)
  for baseline migration constants and migration application.
- [`crates/harness-store/src/core_pipeline.rs`](../../../crates/harness-store/src/core_pipeline.rs)
  for Core-facing Store reads, `CoreStorageMutation`, and commit outcomes.
- [`crates/harness-store/src/artifacts.rs`](../../../crates/harness-store/src/artifacts.rs)
  for `CoreProjectStore::create_artifact_staging` and
  `verify_persistent_artifact_body`.
- [`crates/harness-store/src/inspection.rs`](../../../crates/harness-store/src/inspection.rs)
  for read-only Runtime Home and project-state inspection.
- [`crates/harness-store/src/error.rs`](../../../crates/harness-store/src/error.rs)
  for `StoreError` and storage failure routing.

Important current symbols:

- `CoreProjectStore`, `ProjectStateHeader`, `ProjectEnforcementProfileRecord`
- `ToolInvocationRecord`, `VerifiedReplayContext`, `PendingTaskEvent`
- `CommitMutationInput`, `MutationCommitOutcome`, `CommittedMutationFacts`
- `CoreStorageMutation`, `StorageEffectCounts`, `ProjectMutation`
- `RuntimeHomeRecord`, `ProjectRegistration`, `SurfaceRegistration`
- `ArtifactStagingInsert`, `ArtifactStagingRecord`,
  `PersistentArtifactVerification`
- `inspect_runtime_home`, `inspect_registry_database`,
  `inspect_project_state_database`

Most relevant tests:

- Colocated unit tests in the Store modules.
- Core method tests in
  [`crates/harness-core/src/methods/tests.rs`](../../../crates/harness-core/src/methods/tests.rs)
  for Store-visible effects.
- Cross-layer storage checks in
  [`tests/integration/mcp_surface.rs`](../../../tests/integration/mcp_surface.rs)
  and [`tests/conformance/baseline.rs`](../../../tests/conformance/baseline.rs).

Recommended next component:

- Read `harness-core` to see how method planners choose Store reads and
  `CoreStorageMutation` values. Read `harness-cli` to see local setup use Store
  bootstrap and inspection directly.

## `crates/harness-core`

Why it exists:

`harness-core` owns Core-facing services for public Harness method behavior. It
keeps adapter-independent method behavior in one crate and coordinates Store
reads, policy checks, method plans, dry-run previews, committed mutations, and
common response construction.

Owns in the implementation:

- `CoreService` and the public method entry functions on it.
- Common preflight for envelope shape, adapter binding, request hashing, Store
  opening, project state, surface verification, replay, Task resolution,
  state-version freshness, and access checks.
- Method-specific planning in `crates/harness-core/src/methods/`.
- Reusable policy helpers in `crates/harness-core/src/policy/`.
- Core response construction and routing to read-only, no-effect, dry-run, or
  committed mutation branches.

Does not own:

- MCP stdio framing or CLI setup behavior.
- SQLite DDL, migration definitions, or raw storage layout contracts.
- Product-file writes in `Product Repository`.
- Public schema contracts or exact value-set meaning.

Recommended first file:

- [`crates/harness-core/src/lib.rs`](../../../crates/harness-core/src/lib.rs),
  then [`crates/harness-core/src/pipeline.rs`](../../../crates/harness-core/src/pipeline.rs)

Important modules:

- [`crates/harness-core/src/pipeline.rs`](../../../crates/harness-core/src/pipeline.rs)
  for `CoreService`, `InvocationContext`, `MethodPolicy`,
  `OwnerPipelineBranch`, `PreparedRequest`, `PipelineResponse`,
  `CoreService::prepare_request`, and
  `CoreService::execute_prepared_request`.
- [`crates/harness-core/src/methods/`](../../../crates/harness-core/src/methods/)
  for method-specific entry functions and planners.
- [`crates/harness-core/src/methods/status.rs`](../../../crates/harness-core/src/methods/status.rs)
  for `CoreService::status`, `status_task`, and `status_result_fields`.
- [`crates/harness-core/src/methods/intake.rs`](../../../crates/harness-core/src/methods/intake.rs)
  for `CoreService::intake` and `plan_intake`.
- [`crates/harness-core/src/methods/prepare_write.rs`](../../../crates/harness-core/src/methods/prepare_write.rs)
  for `CoreService::prepare_write`, `prepare_write_policy`, and
  `plan_prepare_write`.
- [`crates/harness-core/src/policy/`](../../../crates/harness-core/src/policy/)
  for access, replay, path, write-authorization, evidence, judgment relevance,
  and close-readiness helpers.

Important current symbols:

- `CoreService`, `CoreResult`, `CorePipelineError`
- `AdapterSessionBinding`, `InvocationContext`, `VerifiedSurfaceContext`,
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

- [`crates/harness-core/src/pipeline.rs`](../../../crates/harness-core/src/pipeline.rs)
  has unit tests for replay, freshness, branch shape, no-effect behavior, and
  Store failure routing.
- [`crates/harness-core/src/methods/tests.rs`](../../../crates/harness-core/src/methods/tests.rs)
  exercises method plans and effects. Start with
  `status_is_read_only_including_dry_run`,
  `intake_commits_once_and_replays_without_effect`,
  `prepare_write_allowed_creates_one_authorization_with_post_commit_basis`,
  `prepare_write_dry_run_has_no_authorization_effect`, and
  `status_read_only_rejects_corrupt_owner_state_without_effect`.
- Cross-layer confirmation lives in
  [`tests/integration/mcp_surface.rs`](../../../tests/integration/mcp_surface.rs)
  and [`tests/conformance/baseline.rs`](../../../tests/conformance/baseline.rs).

Recommended next component:

- Read `harness-store` for commit mechanics and `harness-mcp` for adapter
  dispatch into `CoreService`.

## `crates/harness-cli`

Why it exists:

`harness-cli` implements the local `harness` administrative executable and
reusable agent setup modules. It handles Runtime Home initialization, project
and surface registration, Agent Integration Profile installation, host-specific
MCP configuration, optional repository guidance, and preflight execution.

Owns in the implementation:

- Process entry and administrative command dispatch for the `harness` binary.
- `harness agent` option parsing, storage preparation, host plan construction,
  preflight invocation, status/verify/project membership/uninstall/guidance
  commands, and output.
- Codex, Claude Code, and generic export host integration planning.
- Optional Product Repository guidance rendering and managed-block updates.
- Capability-profile and local-access metadata generation for registered
  surfaces.

Does not own:

- Public Harness API method behavior.
- MCP `tools/call` semantics.
- Core state transitions or method policy.
- Exact CLI command contracts.

Recommended first file:

- [`crates/harness-cli/src/main.rs`](../../../crates/harness-cli/src/main.rs)

Important modules:

- [`crates/harness-cli/src/main.rs`](../../../crates/harness-cli/src/main.rs)
  for process dispatch, `run_cli`, `command_init`, `command_project`, and
  `command_surface`.
- [`crates/harness-cli/src/agent_command.rs`](../../../crates/harness-cli/src/agent_command.rs)
  for `harness agent` install, project membership, status, verification,
  uninstall, and guidance command orchestration.
- [`crates/harness-cli/src/host_integration/`](../../../crates/harness-cli/src/host_integration/)
  for Codex, Claude Code, and generic host integration adapters.
- [`crates/harness-cli/src/repository_guidance.rs`](../../../crates/harness-cli/src/repository_guidance.rs)
  for managed Product Repository guidance discovery, apply, status, and removal.
- [`crates/harness-cli/src/guidance_template.rs`](../../../crates/harness-cli/src/guidance_template.rs)
  for the Codex and Claude Code guidance body.
- [`crates/harness-cli/src/registration.rs`](../../../crates/harness-cli/src/registration.rs)
  for `capability_profile_json`, `local_access_json`, and access-class helpers.

Important current symbols:

- `run_cli`, `CliError`
- `run_agent_command`, `agent_usage`, `AgentCommandError`,
  `AgentProcessOutput`
- `command_install`, `command_status`, `command_verify`,
  `command_uninstall`
- `command_project_add`, `command_project_remove`
- `command_guidance_apply`, `command_guidance_status`,
  `command_guidance_remove`
- `HostKind`, `HostScope`, `HostPlan`, `HostAdapter`, `Verification`
- `GuidanceTarget`, `GuidancePlan`, `guidance_status`,
  `plan_guidance_apply`, `apply_guidance_plan`, `plan_guidance_remove`
- `capability_profile_json`, `local_access_json`,
  `normalized_access_classes_from_local_access`

Most relevant tests:

- [`crates/harness-cli/tests/binary_admin.rs`](../../../crates/harness-cli/tests/binary_admin.rs)
  exercises the `harness` binary for administrative setup, dry-run behavior,
  `harness agent` host setup, repository guidance, setup-command rejection,
  preflight handling, and config-file safety.
- Colocated unit tests in CLI modules cover parsing, planning, rendering,
  registration metadata, and host/guidance behavior.

Recommended next component:

- Read `harness-store` for bootstrap, inspection, and registry storage calls.
  Read `harness-mcp` for the `harness-mcp --check --integration` preflight path
  that agent setup validates.

## `crates/harness-mcp`

Why it exists:

`harness-mcp` is the local MCP stdio adapter. It registers public Harness method
tools, validates startup/session binding, decodes `tools/call` arguments into
typed requests, derives trusted invocation context from the local session, calls
Core, and wraps Core's JSON response in an MCP `tools/call` result.

Owns in the implementation:

- `harness-mcp` binary command modes: stdio, `--check`, help, and version.
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

- [`crates/harness-mcp/src/lib.rs`](../../../crates/harness-mcp/src/lib.rs)

Important modules:

- [`crates/harness-mcp/src/lib.rs`](../../../crates/harness-mcp/src/lib.rs)
  for `PUBLIC_METHOD_TOOL_NAMES`, `McpIntegrationStartupInspection`,
  `McpIntegrationContext`, `McpAdapter`, `McpAdapter::call_tool`,
  `prepare_integration_arguments`, `public_method_tools`, `run_stdio`,
  `handle_json_rpc_request`, and `call_tool_result`.
- [`crates/harness-mcp/src/main.rs`](../../../crates/harness-mcp/src/main.rs)
  for process-mode dispatch through `dispatch_args`.

Important current symbols:

- `PUBLIC_METHOD_TOOL_NAMES`, `McpToolDefinition`, `public_method_tools`
- `McpIntegrationStartupInspection`, `McpIntegrationContext`,
  `McpDerivedInvocationContext`
- `McpAdapter`, `McpAdapter::derive_invocation_context`,
  `McpAdapter::call_tool`
- `prepare_typed_request`, `prepare_integration_arguments`, `decode_params`
- `run_stdio`, `run_stdio_from_env`, `run_preflight_check_from_env`,
  `preflight_check`
- `McpAdapterError`, `call_tool_result`, `json_rpc_error_for_adapter`

Most relevant tests:

- Unit tests in [`crates/harness-mcp/src/lib.rs`](../../../crates/harness-mcp/src/lib.rs),
  including `stdio_tools_list_exposes_exactly_public_method_tools`,
  `bootstrap_registered_surface_can_call_status_through_adapter`,
  `adapter_and_direct_core_status_have_equivalent_response_meaning`,
  `adapter_and_direct_core_intake_dry_run_have_equivalent_response_meaning`,
  and `adapter_derives_access_class_per_method_call`.
- [`crates/harness-mcp/tests/binary_transport.rs`](../../../crates/harness-mcp/tests/binary_transport.rs)
  exercises the binary, `--check`, stdio framing, reconnect behavior, and MCP
  response wrapping.
- [`tests/integration/mcp_surface.rs`](../../../tests/integration/mcp_surface.rs)
  exercises cross-layer MCP/Core/Store behavior.

Recommended next component:

- Read `harness-core` for the method semantics behind each `McpAdapter` branch.
  Read `harness-store` for startup validation and session-binding reads.

## `crates/harness-test-support`

Why it exists:

`harness-test-support` provides disposable fixture infrastructure shared by
implementation, integration, and conformance tests. It keeps Runtime Home,
Product Repository, project registration, surface registration, request
builders, and direct Store inspection helpers out of production crates.

Owns in the implementation:

- Temporary Runtime Home helpers under the system temporary directory.
- Shared `CoreFixture` setup with one registered project and surface.
- Request builders for public method tests.
- Fixture-only Store inspection and mutation helpers used by tests.
- Small marker modules for future fixture and golden-output helpers.

Does not own:

- Product contracts.
- Public API behavior.
- Durable Runtime Home data.
- Generated reports or runtime output.

Recommended first file:

- [`crates/harness-test-support/src/lib.rs`](../../../crates/harness-test-support/src/lib.rs)

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
  [`crates/harness-core/src/methods/tests.rs`](../../../crates/harness-core/src/methods/tests.rs),
  [`tests/integration/mcp_surface.rs`](../../../tests/integration/mcp_surface.rs),
  and [`tests/conformance/baseline.rs`](../../../tests/conformance/baseline.rs).

Recommended next component:

- Read whichever test package is using the fixture. Start with
  `tests/integration` for adapter behavior or `tests/conformance` for
  cross-method baseline scenarios.

## `tests/conformance`

Why it exists:

`tests/conformance` is a Cargo workspace member containing the
`harness-conformance-tests` package and the `baseline` test target. It exercises
baseline cross-method scenarios through Core-facing APIs and shared fixtures.

Owns in the implementation:

- Baseline scenario coverage that composes Core-facing public methods.
- Cross-method checks for effect branches, idempotency, write authorization,
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
- `idempotency_replay_is_bound_to_verified_access_context`
- `committed_non_allow_prepare_write_audit_and_replay_are_exact`
- `prepare_write_allocates_authorization_only_on_committed_allowed_effect`
- `status_projection_matches_public_close_check_and_stays_read_only`
- Shared helpers such as `core`, `invocation`,
  `create_task_with_change_unit`, and `prepare_write_authorization`

Most relevant tests:

- The package exposes the `baseline` test target from
  [`tests/conformance/baseline.rs`](../../../tests/conformance/baseline.rs).

Recommended next component:

- Read `harness-core` method tests for smaller focused cases, then return to
  Reference owners for exact behavior questions.

## `tests/integration`

Why it exists:

`tests/integration` is a Cargo workspace member containing the
`harness-integration-tests` package and the `mcp_surface` test target. It
verifies the cross-layer MCP, Core, Store, surface-binding, and access-path
composition.

Owns in the implementation:

- Tool exposure and schema exposure through MCP.
- MCP session binding, invocation-context derivation, and access-class routing.
- MCP/Core response parity for representative requests.
- Cross-layer storage effects and no-effect checks.
- Stdio protocol error handling that should not mutate Store state.

Does not own:

- Public method contracts.
- MCP transport contracts.
- Store contracts.
- Core authority semantics.

Recommended first file:

- [`tests/integration/mcp_surface.rs`](../../../tests/integration/mcp_surface.rs)

Important current symbols:

- `mcp_exposes_exactly_the_documented_public_methods`
- `stdio_tools_list_exposes_exactly_the_public_method_set`
- `one_mcp_session_with_baseline_workflow_surface_runs_full_access_workflow`
- `missing_write_authorization_grant_blocks_prepare_write`
- `mcp_session_derives_access_per_method_call`
- `stdio_invalid_params_returns_protocol_error_without_storage_effect`
- `mcp_and_direct_status_omit_same_excluded_projection_fields`
- Helpers such as `adapter`, `adapter_for_surface`, `invocation`, and
  `assert_rejected_code`

Most relevant tests:

- The package exposes the `mcp_surface` test target from
  [`tests/integration/mcp_surface.rs`](../../../tests/integration/mcp_surface.rs).

Recommended next component:

- Read `harness-mcp` for the adapter path under test, then `harness-core` and
  `harness-store` for the behavior behind successful calls.

## `xtask`

Why it exists:

`xtask` is a repository maintenance package for deterministic documentation
validation. It exposes `cargo run -p xtask -- docs-check` and keeps
documentation-tooling dependencies out of product and test-support crates.

Owns in the implementation:

- Version 2 `docs/doc-index.yaml` structural validation.
- Bilingual maintained Markdown coverage checks for `docs/en/` and `docs/ko/`.
- Local Markdown link and fragment validation, including hidden anchors.
- `docs/terminology-map.yaml` repository-document path validation.
- Retired documentation path detection in maintained Markdown and YAML route
  metadata.

Does not own:

- Harness runtime behavior.
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
