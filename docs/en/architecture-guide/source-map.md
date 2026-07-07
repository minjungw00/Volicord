# Source map

This page owns the guide-level source path map for the current Volicord Rust
workspace. It documents source paths and implementation responsibilities so
maintainers can route code-reading and code-change questions to the right
module.

This page does not describe execution flow, request lifecycles, storage
transactions, public API behavior, schema meaning, storage effects, security
guarantees, Core authority semantics, or product contracts. Use
[Implementation Architecture](architecture.md) for high-level architecture and
dependency boundaries, [Codebase Tour](codebase-tour.md) for a first-pass
learning route, [CLI Workflows](cli-workflows.md) for administrative CLI
workflow boundaries, [Request Lifecycle](request-lifecycle.md) for
representative method traces, [Storage and Transactions](storage-and-transactions.md)
for Store commit and artifact boundaries, [Testing Strategy](testing-strategy.md)
for test-layer choice, and the [Reference Index](../reference/README.md) for
exact contracts.

All source and test paths are relative to the repository root.

## Workspace members

| Path | Cargo package | Source-map role |
|---|---|---|
| `crates/volicord-types` | `volicord-types` | Shared Rust request, response, schema-shaped, value-set, MCP tool-name, identifier, and canonical-hash types. |
| `crates/volicord-store` | `volicord-store` | SQLite, Runtime Home, bootstrap, project Store, artifact storage, inspection, guard/session observation storage, local web consent storage, export snapshots, and storage-error implementation. |
| `crates/volicord-core` | `volicord-core` | Core service, shared request pipeline, method planning, policy checks, response construction, and Store coordination. |
| `crates/volicord-cli` | `volicord-cli` | Local `volicord` administrative binary, reusable command modules, Runtime Home setup, project and Agent Connection registration, host adapters, guard hooks, User Channel commands, and public `volicord mcp` process dispatch. |
| `crates/volicord-mcp` | `volicord-mcp` | Local MCP adapter library for startup validation, tool listing, `tools/call` decoding and dispatch, stdio framing, local HTTP transport, and Core invocation. |
| `crates/volicord-test-support` | `volicord-test-support` | Disposable Runtime Home, Product Repository, Store, Core, Agent Connection, and fixture helpers shared by implementation tests. |
| `tests/conformance` | `volicord-conformance-tests` | Baseline cross-method scenarios that exercise owner-defined behavior through Core-facing APIs and shared fixtures. |
| `tests/integration` | `volicord-integration-tests` | Cross-layer MCP, Core, Store, Agent Connection binding, operation-category, and public schema snapshot tests. |
| `xtask` | `xtask` | Repository maintenance tooling for documentation validation. It is not part of Volicord runtime architecture. |

## Shared types

| Source path | Responsibility |
|---|---|
| `crates/volicord-types/src/lib.rs` | Public crate surface for shared Rust API and domain-shaped values. |
| `crates/volicord-types/src/methods.rs` | Typed public request and result models, method request schema generation, and method-to-`operation_category` mapping. |
| `crates/volicord-types/src/schema.rs` | Shared envelope, response, state, artifact, judgment, display, and persisted helper shapes. |
| `crates/volicord-types/src/tool_names.rs` | Shared MCP-visible tool-name constants for public method and adapter utility tool sets. |
| `crates/volicord-types/src/values.rs` | Controlled Rust enums and constants for documented value names. |
| `crates/volicord-types/src/ids.rs` | Opaque identifier wrappers and durable ID generation helpers. |
| `crates/volicord-types/src/canonical.rs` | Deterministic canonical JSON serialization and request hashing. |

## Store

| Source path | Responsibility |
|---|---|
| `crates/volicord-store/src/lib.rs` | Public crate surface for storage records, schema initialization, artifact plumbing, and Store helpers. |
| `crates/volicord-store/src/runtime_home.rs` | Runtime Home path resolution and Runtime Home/Product Repository location validation helpers. |
| `crates/volicord-store/src/bootstrap.rs` | Runtime Home metadata initialization, project registration, current-project helpers, and User Channel registration. |
| `crates/volicord-store/src/agent_connections.rs` | Agent Connection rows, natural keys, Connection Projects membership, mode/status values, and Agent Connection lookup/update helpers. |
| `crates/volicord-store/src/schema.rs` and `crates/volicord-store/src/schema/` | Canonical registry and project SQL sources plus schema initialization and validation wiring. |
| `crates/volicord-store/src/sqlite.rs` | Registry/project SQLite path helpers, open/validation helpers, and transaction helpers. |
| `crates/volicord-store/src/core_pipeline.rs` | Store-facing Core records, read helpers, mutation types, commit input/output types, replay helpers, and public Store boundary for Core. |
| `crates/volicord-store/src/core_pipeline/open.rs` | Project-local Store handle opening and execution-context validation. |
| `crates/volicord-store/src/core_pipeline/replay.rs` | Replay-row lookup and replay-context matching. |
| `crates/volicord-store/src/core_pipeline/commit.rs` | Atomic Core mutation commit transaction, state-version advance, authority-event append, and replay-row insert. |
| `crates/volicord-store/src/core_pipeline/mutation_apply.rs` | Transaction-scoped SQL application of `CoreStorageMutation` values. |
| `crates/volicord-store/src/core_pipeline/validation.rs` | Shared persisted-value validation and decoding helpers. |
| `crates/volicord-store/src/artifacts.rs` | Transient artifact staging and persistent artifact body verification helpers. |
| `crates/volicord-store/src/guards.rs` | Guard installation records, guard event records, prompt capture records, expected-write records, and unrecorded-change observation storage helpers. |
| `crates/volicord-store/src/session_watch.rs` | Session-level Product Repository watch snapshot, observation, and unresolved-change helper storage. |
| `crates/volicord-store/src/local_consent.rs` | Local web consent token creation, validation, and completion storage helpers. |
| `crates/volicord-store/src/inspection.rs` | Read-only Runtime Home, registry, project, Agent Connection, and setup-state inspection snapshots. |
| `crates/volicord-store/src/export.rs` | Read-only authority bundle snapshot assembly for project records and artifact metadata. |
| `crates/volicord-store/src/error.rs` | Store error types and storage failure routing. |

## Core

| Source path | Responsibility |
|---|---|
| `crates/volicord-core/src/lib.rs` | Public crate surface for Core-facing services and adapter-independent method entry points. |
| `crates/volicord-core/src/pipeline.rs` | `CoreService`, invocation context, common preflight, request hashing, Store opening, replay handling, effect-path selection, response construction, and Core commit orchestration. |
| `crates/volicord-core/src/methods/` | Method-specific validation, planning, storage mutation lists, event payloads, dry-run summaries, and result fields. |
| `crates/volicord-core/src/methods/status.rs` | `volicord.status` planning and read-only result construction. |
| `crates/volicord-core/src/methods/intake.rs` | `volicord.intake` planning and task/change-unit mutation preparation. |
| `crates/volicord-core/src/methods/update_scope.rs` | `volicord.update_scope` planning and scope mutation preparation. |
| `crates/volicord-core/src/methods/prepare_write.rs` | `volicord.prepare_write` planning, compatibility checks, and write-ticket mutation preparation. |
| `crates/volicord-core/src/methods/record_run.rs` | `volicord.record_run` planning for run and evidence-related mutations. |
| `crates/volicord-core/src/methods/reconcile_changes.rs` | `volicord.reconcile_changes` planning for unresolved Product Repository observations. |
| `crates/volicord-core/src/methods/judgment.rs` | User-judgment request and recording method planning. |
| `crates/volicord-core/src/methods/close_task.rs` | `volicord.close_task` planning and close-readiness result handling. |
| `crates/volicord-core/src/methods/session_watch.rs` | Session-watch method planning and observation coordination. |
| `crates/volicord-core/src/methods/stage_artifact.rs` | Transient artifact staging method handling. |
| `crates/volicord-core/src/policy/` | Reusable Core policy helpers for access checks, replay context, Product Repository path normalization, write-ticket compatibility, evidence status, judgment relevance, continuity, rationale, effect contracts, and close-readiness calculations. |
| `crates/volicord-core/src/methods/tests/` | Core method and pipeline tests close to the method planners. |

## CLI

| Source path | Responsibility |
|---|---|
| `crates/volicord-cli/src/main.rs` | `volicord` process entry, administrative command dispatch, `volicord mcp` and local HTTP process-mode handoff, setup gating, and binary exit behavior. |
| `crates/volicord-cli/src/lib.rs` | Shared administrative CLI crate surface for reusable command modules. |
| `crates/volicord-cli/src/setup_command.rs` and `crates/volicord-cli/src/setup_command/` | Setup command entry, setup workflow execution, executable discovery, command-link planning, shell startup planning, interactive choices, and setup output rendering. |
| `crates/volicord-cli/src/connection_command.rs` and `crates/volicord-cli/src/connection_command/` | `volicord init`, `volicord connection add`, `volicord connection list`, and `volicord connection status/verify/mode/remove` parsing, provisioning, selection, verification, MCP process checks, and output rendering. |
| `crates/volicord-cli/src/guard_command.rs` and `crates/volicord-cli/src/guard_command/` | Guard hook command dispatch, argument parsing, host event normalization, tool observation extraction, mutation classification, phase handling, prompt capture, prompt-embedded judgment commands, write-ticket checks, and hook output rendering. |
| `crates/volicord-cli/src/guard_integration/` | Guard integration planning, generated guard file application, capability metadata, policy helpers, host-specific guard hook planning, and factual audit helpers used by connection status and doctor diagnostics. |
| `crates/volicord-cli/src/guard_integration/plan.rs` | Guard integration plan assembly across host capability, profile, project, and runtime facts. |
| `crates/volicord-cli/src/guard_integration/files.rs` | Generated guard file and managed policy file plans. |
| `crates/volicord-cli/src/guard_integration/apply.rs` | Application of generated guard files and managed projections. |
| `crates/volicord-cli/src/guard_integration/capability.rs` | Capability metadata and recorded guard installation metadata helpers. |
| `crates/volicord-cli/src/guard_integration/policy.rs` | Guard policy helper values and lifecycle phase helpers. |
| `crates/volicord-cli/src/guard_integration/hooks.rs` and `crates/volicord-cli/src/guard_integration/hosts/` | Host hook command planning and host-specific generated file planning. |
| `crates/volicord-cli/src/guard_integration/audit.rs` | Factual checks over recorded capability metadata, generated files, wrapper scripts, hook command paths, and managed projections. These facts are diagnostic observations, not security guarantees, human approval records, or correctness proofs. |
| `crates/volicord-cli/src/doctor_command.rs` | Installation, connection, host, and guard fact gathering for diagnostic reporting. |
| `crates/volicord-cli/src/user_command.rs` | Local User Channel status and `volicord inbox` command parsing and orchestration. |
| `crates/volicord-cli/src/host_integration/` | Shared host kinds, scopes, capabilities, lifecycle phases, config editing, integration contracts, generic-host guidance, and diagnostic status types. |
| `crates/volicord-cli/src/host_integration/codex/` | Codex adapter internals for config planning, executable checks, managed identity, trust facts, and verification. |
| `crates/volicord-cli/src/host_integration/claude_code/` | Claude Code adapter internals for CLI command construction, config planning, managed identity checks, host-native output parsing, and verification. |
| `crates/volicord-cli/src/registration.rs` | Agent Connection, Connection Project, and User Channel metadata construction. |
| `crates/volicord-cli/src/project_context.rs` | Product Repository root detection and `volicord project ...` command orchestration. |
| `crates/volicord-cli/src/export_command.rs` | Authority bundle export command parsing and rendering. |
| `crates/volicord-cli/src/changes_command.rs` | Local change-reconciliation command parsing and orchestration. |
| `crates/volicord-cli/src/serve_command.rs` | Local HTTP service command parsing and server configuration handoff. |
| `crates/volicord-cli/src/disclosure.rs`, `managed_block.rs`, `shell_path.rs`, `setup_report.rs`, and `summary_card.rs` | Shared CLI helpers for disclosure text, managed file blocks, shell path handling, setup report data, and compact status summaries. |

## MCP adapter

| Source path | Responsibility |
|---|---|
| `crates/volicord-mcp/src/lib.rs` | Public crate surface and re-exported adapter entry points used by the CLI binary. |
| `crates/volicord-mcp/src/tool_registry.rs` | MCP-visible tool metadata and tool sets by Agent Connection mode. |
| `crates/volicord-mcp/src/routing.rs` | Agent Connection startup inspection, project availability, project allowlist checks, and request-time project selection helpers. |
| `crates/volicord-mcp/src/adapter.rs` | Typed public `tools/call` decoding, adapter utility calls, `operation_category` and `actor_source` derivation, Core invocation, and response wrapping helpers. |
| `crates/volicord-mcp/src/stdio.rs` | JSON-RPC stdio framing, initialization, response wrapping, elicitation handling, and stdio/preflight runners used by `volicord mcp`. |
| `crates/volicord-mcp/src/local_http.rs` | Local loopback HTTP server setup, endpoint routing, token handling, and local HTTP MCP serving. |
| `crates/volicord-mcp/src/local_web_consent.rs` | Local web consent request and completion handling for User Channel answers. |
| `crates/volicord-mcp/src/http.rs` | Shared HTTP parsing and response helpers. |
| `crates/volicord-mcp/src/constants.rs` | Adapter constants shared by MCP modules. |
| `crates/volicord-mcp/src/errors.rs` | MCP adapter and local HTTP error types. |
| `crates/volicord-mcp/src/prelude.rs` and `crates/volicord-mcp/src/util.rs` | Internal shared imports and small adapter utility helpers. |
| `crates/volicord-mcp/src/tests.rs` | Crate-local MCP adapter and transport tests. |

## Tests and maintenance support

| Source path | Responsibility |
|---|---|
| `crates/volicord-test-support/src/lib.rs` | Disposable Runtime Home helpers, Core fixtures, request builders, fixture-only Store helpers, and shared assertions for implementation tests. |
| `crates/volicord-cli/tests/support/` | Binary fixtures, fake hosts, fake MCP processes, JSON helpers, assertions, and guard lifecycle fixtures for CLI integration tests. |
| `crates/volicord-cli/tests/binary_admin.rs` | Binary-level administrative CLI coverage for setup, project, connection, status, inbox, preflight, and host configuration behavior. |
| `crates/volicord-cli/tests/guard_command.rs` | Guard hook lifecycle, prompt capture, observed mutation, expected-write, write-ticket matching, and guarded init/status coverage. |
| `crates/volicord-cli/tests/mcp_transport.rs` | `volicord mcp` subcommand, `--check`, stdio framing, reconnection, and MCP response wrapping coverage. |
| `crates/volicord-cli/tests/serve_transport.rs` | Local HTTP service command and transport coverage. |
| `crates/volicord-cli/tests/live_host_smoke.rs` | Host smoke-test coverage guarded by test environment availability. |
| `tests/conformance/baseline.rs` | Cross-method baseline scenarios through Core-facing APIs. |
| `tests/integration/mcp_connection.rs` | Cross-layer MCP/Core/Store and Agent Connection behavior coverage. |
| `tests/integration/public_contract_snapshots.rs` and `tests/integration/snapshots/` | Public schema and MCP tool snapshot contract coverage. |
| `xtask/src/main.rs` and `xtask/src/lib.rs` | Read-only repository maintenance commands, including documentation validation. |

These source descriptions are implementation placement guidance. If this map
and a focused Reference owner appear to disagree about product behavior, treat
that as an owner-routing or implementation gap rather than inferring a product
contract from source placement.
