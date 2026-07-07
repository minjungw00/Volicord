# Codebase tour

This tour is a reading guide for maintainers learning the Volicord Rust
workspace. It suggests a useful order for opening code, explains why each crate
exists, and names a few stable entry symbols or flows to follow.

It is not the source ownership map. Use the [Source Map](source-map.md) for
exact source path responsibilities and module placement. Use
[Implementation Architecture](architecture.md) for the workspace dependency
graph and top-level runtime maps, [CLI Workflows](cli-workflows.md) for local
administrative execution flows, [Request Lifecycle](request-lifecycle.md) for
representative MCP-to-Core-to-Store method traces, [Storage and Transactions](storage-and-transactions.md)
for commit and artifact boundaries, [Testing Strategy](testing-strategy.md) for
test-layer choice, and the [Implementation Guide](change-guide.md) when you are
ready to make a change.

Exact API behavior, request and response schemas, storage effects, security
wording, runtime boundaries, error meaning, and Core authority semantics remain
with the focused [Reference Index](../reference/README.md) owners.

Code and test paths below are relative to the repository root.

## Recommended reading order

For a first pass through public method execution, read the code in this order:

1. `volicord-types`: learn the typed request, response, value, identifier, and
   canonical-hash shapes that the other crates share.
2. `volicord-mcp`: follow how an MCP `tools/call` becomes a typed request and
   a Core invocation.
3. `volicord-core`: follow the shared preflight, method planning, branch
   selection, and response construction.
4. `volicord-store`: follow project Store reads, `CoreStorageMutation` values,
   normal commit, replay, and artifact boundaries.
5. `tests/integration` and `tests/conformance`: see how the path is exercised
   across MCP/Core/Store and across baseline method scenarios.

For local operator and setup behavior, branch after `volicord-store` into
`volicord-cli`, then read [CLI Workflows](cli-workflows.md). The CLI path is
local administrative orchestration, not an alternate implementation of public
Core method behavior.

For storage questions, read `volicord-store` with
[Storage and Transactions](storage-and-transactions.md) open beside it. For
exact record, DDL, artifact, or storage-effect meaning, switch to the storage
Reference owners from the [Reference Index](../reference/README.md).

For a change, do not use this tour as the final routing authority. Use this
tour to get oriented, [Source Map](source-map.md) to locate the exact path,
[Testing Strategy](testing-strategy.md) to choose validation layers, and
[Implementation Guide](change-guide.md) to route owner documents and completion
checks.

## Workspace mental model

The shortest dependency mental model is:

- `volicord-types` sits at the shared type boundary.
- `volicord-store` uses shared types to manage Runtime Home and project Store
  mechanics.
- `volicord-core` uses shared types and Store to implement adapter-independent
  method handling.
- `volicord-mcp` and `volicord-cli` are adapters and local administrative
  entry points around Core and Store.
- `volicord-test-support`, `tests/integration`, and `tests/conformance` compose
  crates for disposable verification.
- `xtask` is repository maintenance tooling, separate from product runtime
  architecture.

For the exact Cargo dependency graph, use
[Implementation Architecture](architecture.md). For exact source placement, use
[Source Map](source-map.md).

## `crates/volicord-types`

Start here when you need to understand the data shape that adapters, Core,
Store, and tests share. This crate is the boundary for typed requests and
results, schema-shaped structs, controlled Rust values, opaque identifiers,
MCP-visible tool names, and canonical request hashing.

Open [`crates/volicord-types/src/lib.rs`](../../../crates/volicord-types/src/lib.rs)
first, then follow these anchors:

- [`crates/volicord-types/src/methods.rs`](../../../crates/volicord-types/src/methods.rs)
  for public request/result structs, `MethodOperationCategory`, and
  `public_request_schema`.
- [`crates/volicord-types/src/schema.rs`](../../../crates/volicord-types/src/schema.rs)
  for shared envelope, response, state, artifact, judgment, and display shapes
  such as `ToolEnvelope`, `StateSummary`, `EvidenceSummary`, and `ArtifactRef`.
- [`crates/volicord-types/src/values.rs`](../../../crates/volicord-types/src/values.rs)
  for controlled values such as `MethodName`, `OperationCategory`,
  `EffectKind`, `ResponseKind`, and `ErrorCode`.
- [`crates/volicord-types/src/ids.rs`](../../../crates/volicord-types/src/ids.rs)
  and [`crates/volicord-types/src/canonical.rs`](../../../crates/volicord-types/src/canonical.rs)
  for opaque IDs, `DurableIdGenerator`, and `canonical_request_hash`.

When reading tests, start with the type-shape and canonical-hash tests in
`crates/volicord-types/src/lib.rs`. Then move to `volicord-mcp` to see typed
requests produced from MCP arguments, or to `volicord-core` to see those
requests planned.

## `crates/volicord-mcp`

Read `volicord-mcp` when you want to see how a local MCP host reaches Core. The
adapter registers public tools, validates startup/session context, decodes
`tools/call` arguments, derives trusted invocation context from the local Agent
Connection, calls Core, and wraps Core JSON as MCP content.

Open [`crates/volicord-mcp/src/lib.rs`](../../../crates/volicord-mcp/src/lib.rs)
for the crate surface, then follow this path:

1. [`crates/volicord-mcp/src/tool_registry.rs`](../../../crates/volicord-mcp/src/tool_registry.rs)
   to see the public tool list and `PUBLIC_METHOD_TOOL_NAMES`.
2. [`crates/volicord-mcp/src/routing.rs`](../../../crates/volicord-mcp/src/routing.rs)
   to see startup inspection, connection context, project allowlists, and
   request-time project selection.
3. [`crates/volicord-mcp/src/adapter.rs`](../../../crates/volicord-mcp/src/adapter.rs)
   to follow `McpAdapter::call_tool`, typed decoding, generated envelope facts,
   and `McpDerivedInvocationContext::core_invocation`.
4. [`crates/volicord-mcp/src/stdio.rs`](../../../crates/volicord-mcp/src/stdio.rs)
   to see JSON-RPC stdio dispatch, preflight, and response wrapping.

Keep the boundary in mind: direct Store reads during MCP startup and routing are
validation and selection work before public method dispatch. Public method
semantics still route through `volicord-core`. For representative call traces,
use [Request Lifecycle](request-lifecycle.md); for exact MCP transport behavior,
use [MCP Transport](../reference/mcp-transport.md).

## `crates/volicord-core`

Read `volicord-core` when you want the adapter-independent method path. Core
coordinates shared preflight, Store opening, replay checks, method-specific
planning, branch selection, and response construction.

Open [`crates/volicord-core/src/lib.rs`](../../../crates/volicord-core/src/lib.rs),
then [`crates/volicord-core/src/pipeline.rs`](../../../crates/volicord-core/src/pipeline.rs).
The main symbols to follow are `CoreService`, `InvocationContext`,
`MethodPolicy`, `PreparedRequest`, `OwnerPipelineBranch`,
`CoreService::prepare_request`, and
`CoreService::execute_prepared_request`.

After the pipeline, read one method module from
[`crates/volicord-core/src/methods/`](../../../crates/volicord-core/src/methods/):

- `status.rs` shows a read-only branch.
- `intake.rs` shows a planned committed mutation branch.
- `prepare_write.rs` shows policy-heavy planning and write-ticket decisions.
- `record_run.rs`, `judgment.rs`, `reconcile_changes.rs`, and
  `close_task.rs` show how later workflow facts are planned without moving
  exact method contracts into Core prose.

The reusable policy helpers under
[`crates/volicord-core/src/policy/`](../../../crates/volicord-core/src/policy/)
are worth reading after you understand one method. Use
[Implementation Design Patterns](design-patterns.md) for the recurring
structures and [Request Lifecycle](request-lifecycle.md) for traced examples.

For tests, start with `crates/volicord-core/src/pipeline.rs` for branch and
preflight edges, then `crates/volicord-core/src/methods/tests/mod.rs` for method
plans and Store-visible effects.

## `crates/volicord-store`

Read `volicord-store` when you need the runtime data and transaction mechanics.
Store manages Runtime Home path handling, registry and project database setup,
schema validation, project Store reads, normal Core mutation commits, replay
rows, artifact staging, inspection, and storage-error routing.

Open [`crates/volicord-store/src/lib.rs`](../../../crates/volicord-store/src/lib.rs),
then follow the path that matches your question:

- Setup and local registration: [`runtime_home.rs`](../../../crates/volicord-store/src/runtime_home.rs),
  [`bootstrap.rs`](../../../crates/volicord-store/src/bootstrap.rs), and
  [`agent_connections.rs`](../../../crates/volicord-store/src/agent_connections.rs).
- SQLite shape and validation:
  [`sqlite.rs`](../../../crates/volicord-store/src/sqlite.rs),
  [`schema.rs`](../../../crates/volicord-store/src/schema.rs), and
  [`schema/`](../../../crates/volicord-store/src/schema/).
- Core-facing Store work:
  [`core_pipeline.rs`](../../../crates/volicord-store/src/core_pipeline.rs)
  for `CoreProjectStore`, `CoreStorageMutation`, `CommitMutationInput`,
  `MutationCommitOutcome`, and `CoreProjectStore::commit_mutation`.
- Artifact work: [`artifacts.rs`](../../../crates/volicord-store/src/artifacts.rs)
  for staging and persistent body verification helpers.
- Read-only setup and diagnostic views:
  [`inspection.rs`](../../../crates/volicord-store/src/inspection.rs).

Use [Storage and Transactions](storage-and-transactions.md) while reading the
commit path. That page explains the planning-to-mutation split, atomic commit
boundary, replay, state-version, artifact, and failure boundaries at guide
level. Use [Source Map](source-map.md) for the exact Store submodule map.

## `crates/volicord-cli`

Read `volicord-cli` when you need local operator workflows: installation
profile setup, project detection, Agent Connection registration, host adapter
planning, guard integration, connection status and verification, doctor
diagnostics, authority bundle export, User Channel commands, and the public
`volicord mcp` process-mode handoff.

Open [`crates/volicord-cli/src/main.rs`](../../../crates/volicord-cli/src/main.rs)
to see `run_cli` and process dispatch. Then choose the workflow you are
studying:

- Setup workflow: `run_setup_command` and `run_setup_workflow` in
  [`setup_command.rs`](../../../crates/volicord-cli/src/setup_command.rs) and
  [`setup_command/`](../../../crates/volicord-cli/src/setup_command/).
- Agent Connection provisioning and verification: `run_init_command`,
  `run_connection_command`, `provision_connection`, `select_connection`,
  `verify_connection`, and rendering under
  [`connection_command.rs`](../../../crates/volicord-cli/src/connection_command.rs)
  and [`connection_command/`](../../../crates/volicord-cli/src/connection_command/).
- Guard hook lifecycle: `run_guard_command`, `guard_envelope`,
  `tool_observation`, `handle_prompt_capture`, and `render_guard_output` under
  [`guard_command.rs`](../../../crates/volicord-cli/src/guard_command.rs) and
  [`guard_command/`](../../../crates/volicord-cli/src/guard_command/).
- Host and guard integration: `HostKind`, `HostAdapter`,
  `plan_guard_integration`, and `apply_guard_integration` under
  [`host_integration/`](../../../crates/volicord-cli/src/host_integration/) and
  [`guard_integration/`](../../../crates/volicord-cli/src/guard_integration/).
- User Channel commands:
  [`user_command.rs`](../../../crates/volicord-cli/src/user_command.rs).

Read [CLI Workflows](cli-workflows.md) before trying to reason from scattered
CLI modules. It owns the architecture-level execution-flow boundaries; exact
command contracts remain in [Administrative CLI](../reference/admin-cli.md).

For tests, start with `crates/volicord-cli/tests/binary_admin.rs` for
binary-visible administrative workflows, `guard_command.rs` for hook
lifecycle behavior, and `mcp_transport.rs` or `serve_transport.rs` for process
transport paths.

## `crates/volicord-test-support`

Read `volicord-test-support` when tests feel hard to set up. It provides
disposable Runtime Home fixtures, registered project and Agent Connection setup,
Core request builders, Store inspection helpers, and shared fixture utilities.

Open [`crates/volicord-test-support/src/lib.rs`](../../../crates/volicord-test-support/src/lib.rs)
and look for `disposable_runtime_home`, `TempRuntimeHome`, `CoreFixture`, and
the method request builders. Treat these helpers as test composition, not
production behavior or product-contract ownership.

Use [Testing Strategy](testing-strategy.md) to decide when fixture changes need
consumer tests in Core, CLI, integration, or conformance packages.

## `tests/integration`

Read `tests/integration` when you want the cross-layer MCP view. The main
starting point is
[`tests/integration/mcp_connection.rs`](../../../tests/integration/mcp_connection.rs),
which composes MCP, Core, Store, Agent Connection binding, project selection,
operation-category routing, response parity, and no-effect checks through
representative calls.

Use these tests to understand how layers are composed. Do not treat them as
the owner of public method contracts, MCP transport contracts, Store contracts,
or Core authority semantics.

## `tests/conformance`

Read `tests/conformance` when you want baseline cross-method scenarios through
Core-facing APIs. Start with
[`tests/conformance/baseline.rs`](../../../tests/conformance/baseline.rs) after
you have read one Core method test.

This package is useful for seeing replay, write tickets, artifact lifecycle,
judgment paths, close-readiness checks, error routing, and corruption handling
across methods. Product meaning still routes to the focused Reference owners.

## `xtask`

Read `xtask` only when you are maintaining documentation validation. It is a
repository maintenance package for read-only documentation checks such as
`cargo run -p xtask -- docs-check`.

Open [`xtask/src/lib.rs`](../../../xtask/src/lib.rs), then
[`xtask/src/main.rs`](../../../xtask/src/main.rs). The tests in
[`xtask/tests/docs_check.rs`](../../../xtask/tests/docs_check.rs) use small
fixture trees for metadata, bilingual path coverage, links, anchors, command
examples, terminology roles, terminology paths, and public-language checks.

Use [Validation](../maintain/validation.md) for the maintenance policy that
defines the command boundary and separates automated structure checks from
manual semantic review.

## Boundary reminders

- This page gives a reading order and concrete first-open anchors; it does not
  own exact source path responsibilities. Use [Source Map](source-map.md) for
  that.
- Core-facing code stays independent of CLI and MCP adapter crates.
- MCP startup and routing may read Store before Core dispatch; that is not
  alternate public method semantics.
- `Volicord Runtime Home` and `Product Repository` are separate boundaries.
- Tests verify owner-defined facts, but tests and fixtures are not product
  contract owners.
- Learning pages should name durable files, symbols, and flows, not unstable
  line numbers.
