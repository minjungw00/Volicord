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
| `crates/volicord-types/src/platform.rs` | Shared platform-environment and platform-path types. |
| `crates/volicord-types/src/host_configuration.rs` | Shared connection-intent and host-scope configuration types. |
| `crates/volicord-types/src/connection_verification.rs` | Canonical connection status, check, action, and verification-report types. |
| `crates/volicord-types/src/integration_revision.rs` | Typed Connection/project integration revision bases and derivation. |
| `crates/volicord-types/src/guard_manifest.rs` | Canonical Guard manifest, managed-artifact, hook-phase, and typed command contracts. |
| `crates/volicord-types/src/tool_names.rs` | Public MCP tool-name registry. |

## Platform Filesystem Boundary

| Path | Responsibility |
|---|---|
| `crates/volicord-platform-fs/src/lib.rs` | Current process target and platform observation, native Linux/WSL2 kernel classification, WSL2 `/etc/os-release` distribution validation, path-filesystem observation, platform-native namespace operations, and canonical read-only Git layout discovery. |
| `crates/volicord-cli/src/host_integration/process.rs` | Process-target validation and target-path filesystem enforcement from platform-boundary observations. |

## Platform Process Boundary

| Path | Responsibility |
|---|---|
| `crates/volicord-platform-process/src/lib.rs` | Safe APIs and deterministic error categories for bounded child-process containment, command configuration, attachment, process-tree termination, and nonblocking child-pipe polling. |
| `crates/volicord-platform-process/src/unix.rs` | Unix process-group containment and nonblocking pipe primitives. |
| `crates/volicord-platform-process/src/windows.rs` | Private Windows Job Object ownership and anonymous-pipe readiness primitives. |

## Store

| Path | Responsibility |
|---|---|
| `crates/volicord-store/src/schema/registry.sql` | Canonical Runtime Home registry DDL source. |
| `crates/volicord-store/src/schema/project.sql` | Canonical project Store DDL source. |
| `crates/volicord-store/src/bootstrap.rs` | Runtime Home and Store bootstrap. |
| `crates/volicord-store/src/agent_connections.rs` | Agent Connection records, project allowlists, managed fingerprints, and persisted verification-report boundary. |
| `crates/volicord-store/src/operational_sessions.rs` | Managed runtime sessions, protocol milestones, revision-scoped project sessions, and exact cross-database bindings. |
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
| `crates/volicord-core/src/agent_session.rs` | Current Connection, project membership, mode, and managed runtime/project-session validation. |
| `crates/volicord-core/src/authority_status.rs` | Typed status and authority-receipt correspondence. |

## CLI And Codex Adapter

| Path | Responsibility |
|---|---|
| `crates/volicord-cli/src/main.rs` | Process entry and administrative command dispatch. |
| `crates/volicord-cli/src/connection_command/` | Connection add, list, status, verify, mode, and remove orchestration. |
| `crates/volicord-cli/src/connection_command/mcp_process/` | Managed launch materialization, bounded child-process supervision policy and deadlines, preflight interpretation, stdio JSON-RPC framing and probe sequencing, exchange progress, and typed lifecycle or protocol diagnostics. Low-level containment and pipe readiness route through `volicord-platform-process`. |
| `crates/volicord-cli/src/connection_command/output/report.rs` | Canonical selected-Connection command report, operation results, rendering input, and aggregate status. |
| `crates/volicord-cli/src/host_integration/codex/` | Codex configuration parsing and serialization, canonical managed-entry validation, preservation of the allowed tool-approval overlay, managed configuration mutation, diagnostic executable observations, and connection verification. |
| `crates/volicord-cli/src/guard_integration/manifest.rs` | Guard manifest and canonical managed-artifact expectation generation. |
| `crates/volicord-cli/src/guard_integration/audit.rs` | Current Guard owner, artifact, command, marker, and executable-behavior audit. |
| `crates/volicord-cli/src/guard_command/` | Guard event decoding and bounded observations. |
| `crates/volicord-cli/src/user_command.rs` | CLI inbox and local-user resolution. |
| `crates/volicord-cli/src/doctor_command.rs` | Diagnostic fact collection and rendering. |

## MCP Adapter

| Path | Responsibility |
|---|---|
| `crates/volicord-mcp/src/managed_launch.rs` | Canonical typed personal/shared managed MCP command, arguments, static and forwarded environment bindings, strict launch-shape validation, projection, and fingerprint inputs. |
| `crates/volicord-mcp/src/stdio.rs` | stdio lifecycle, framing, initialization, and process preflight. |
| `crates/volicord-mcp/src/adapter.rs` | Public argument decoding, server-owned context, Core dispatch, and wrapping. |
| `crates/volicord-mcp/src/tool_registry.rs` | Compact public tool descriptors. |
| `crates/volicord-mcp/src/schema_validation.rs` | Public schema validation. |
| `crates/volicord-mcp/src/routing.rs` | Bound Product Repository discovery and current Connection/project routing. |

## Tests

| Path | Responsibility |
|---|---|
| `crates/*/tests/` and module-local `tests` | Crate boundary and unit tests. |
| `tests/conformance/` | Cross-method conformance scenarios. |
| `tests/conformance/mcp-spec/` | Versioned official MCP schemas, release and handshake-family metadata, immutable upstream pins, license attribution, and checksums used as offline conformance inputs. |
| `tests/release-integrity/` | Generic five-target, version, canonical-byte, package, checksum, and release-workflow integrity tests. |
| `crates/volicord-test-support/` | Disposable Runtime Home, repository, Store, and request helpers. |

## Repository Maintenance Tooling

| Path | Responsibility |
|---|---|
| `xtask/src/mcp_spec.rs` | Offline pinned-spec validation and explicit networked synchronization through a verified temporary candidate. |
| `xtask/tests/mcp_spec.rs` | Manifest parsing, classification, immutable-pin, checksum, required-artifact, ordering, and offline-success coverage. |

Update this map when a durable responsibility moves. Do not list removed,
generated, or private scratch paths.
