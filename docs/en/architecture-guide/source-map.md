# Source Map

This map routes maintainers to current implementation owners. It is not a
product contract; use the focused Reference document for exact behavior.

## Shared Types

| Path | Responsibility |
|---|---|
| `crates/volicord-types/src/schema.rs` | Shared request, response, and stored-record shapes. |
| `crates/volicord-types/src/values.rs` | Closed product value sets. |
| `crates/volicord-types/src/ids.rs` | Opaque identifiers. |
| `crates/volicord-types/src/canonical.rs` | Canonical serialization and hashing. |
| `crates/volicord-types/src/codex_support_catalog.rs` and `contracts/codex-support-catalog.json` | Embedded exact Codex runtime support policy. |
| `crates/volicord-types/src/codex_release_evidence.rs` | Strict external release-evidence types and parsing without evidence embedding. |
| `crates/volicord-types/src/tool_names.rs` | Public MCP tool-name registry. |

## Store

| Path | Responsibility |
|---|---|
| `crates/volicord-store/src/schema/registry.sql` | Canonical Runtime Home registry DDL source. |
| `crates/volicord-store/src/schema/project.sql` | Canonical project Store DDL source. |
| `crates/volicord-store/src/bootstrap.rs` | Runtime Home and Store bootstrap. |
| `crates/volicord-store/src/agent_connections.rs` | Agent Connection records and project allowlists. |
| `crates/volicord-store/src/workflow_records.rs` | Workflow record reads and writes. |
| `crates/volicord-store/src/core_pipeline/` | Core-open, validation, replay, commit, and mutation application. |
| `crates/volicord-store/src/guards.rs` | Guard observations, expected writes, and suppression inputs. |
| `crates/volicord-store/src/evidence_capture.rs` | Evidence-capture intent and producer records. |
| `crates/volicord-store/src/artifacts.rs` | Artifact staging and durable body validation. |
| `crates/volicord-store/src/error.rs` | Store failure classification. |

## Core

| Path | Responsibility |
|---|---|
| `crates/volicord-core/src/pipeline.rs` | Common preflight, replay, plan selection, response, and commit orchestration. |
| `crates/volicord-core/src/methods/` | Method-specific structural validation and planning. |
| `crates/volicord-core/src/policy/` | Reusable access, workflow, evidence, continuity, write-ticket, and close-readiness policy. |
| `crates/volicord-core/src/authority_status.rs` | Typed status and authority-receipt correspondence. |

## CLI And Codex Adapter

| Path | Responsibility |
|---|---|
| `crates/volicord-cli/src/main.rs` | Process entry and administrative command dispatch. |
| `crates/volicord-cli/src/connection_command/` | Connection add, list, status, verify, mode, and remove orchestration. |
| `crates/volicord-cli/src/host_integration/codex/` | Codex configuration, executable identity, trust facts, and verification. |
| `crates/volicord-cli/src/guard_command/` | Guard event decoding and bounded observations. |
| `crates/volicord-cli/src/user_command.rs` | CLI inbox and local-user resolution. |
| `crates/volicord-cli/src/doctor_command.rs` | Diagnostic fact collection and rendering. |

## MCP Adapter

| Path | Responsibility |
|---|---|
| `crates/volicord-mcp/src/stdio.rs` | stdio lifecycle, framing, initialization, and process preflight. |
| `crates/volicord-mcp/src/adapter.rs` | Public argument decoding, server-owned context, Core dispatch, and wrapping. |
| `crates/volicord-mcp/src/tool_registry.rs` | Compact public tool descriptors. |
| `crates/volicord-mcp/src/schema_validation.rs` | Public schema validation. |
| `crates/volicord-mcp/src/repository_discovery.rs` | Bound Product Repository discovery. |

## Tests

| Path | Responsibility |
|---|---|
| `crates/*/tests/` and module-local `tests` | Crate boundary and unit tests. |
| `tests/conformance/` | Cross-method conformance scenarios. |
| `tests/release-validation/` | Exact finalized Codex and Volicord artifact validation plus the external four-platform evidence manifest. |
| `crates/volicord-test-support/` | Disposable Runtime Home, repository, Store, and request helpers. |

Update this map when a durable responsibility moves. Do not list removed,
generated, or private scratch paths.
