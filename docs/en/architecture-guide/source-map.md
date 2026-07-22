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
| `crates/volicord-types/src/diagnostics.rs` | Lifecycle-specific occurrence/current finding types, opaque `DiagnosticSubjectIdentity`, `CurrentDiagnosticKey` canonical identity and fixed digest ID derivation, lifecycle-aware `StoredDiagnosticFinding` and `StoredDiagnosticGraph`, separate `DiagnosticLookupReport`, shared read-only `DiagnosticFinding` and selected-Connection `DiagnosticReport` types, stable namespaced-code validation, bounded redacting projection of typed owner facts, cause-graph validation, and unexpected-failure fallback. |
| `crates/volicord-types/src/platform.rs` | Shared platform-environment and platform-path types. |
| `crates/volicord-types/src/host_configuration.rs` | Shared connection-intent and host-scope configuration types. |
| `crates/volicord-types/src/connection_verification.rs` | Canonical connection status, check, action, and verification-report types. |
| `crates/volicord-types/src/integration_revision.rs` | Typed Connection/project integration revision bases and derivation. |
| `crates/volicord-types/src/guard_manifest.rs` | Canonical Guard manifest, managed-artifact, hook-phase, and typed command contracts. |
| `crates/volicord-types/src/tool_names.rs` | Closed `AgentToolId` catalog, `MethodName` reuse for Core-owned tools, category and mode metadata, compile-time verification-role binding, and stable MCP wire-name projection. |

## Host Wire Contracts

| Path | Responsibility |
|---|---|
| `crates/volicord-host-contract/src/lib.rs` | Explicit `codex-mcp-2025-06-18-v1` and `codex-hooks-v1` parsing, deterministic profile digests, bounded values and errors, and source-specific `CodexMcpCorrelation`, `CodexHookPromptCorrelation`, and `CodexHookToolCorrelation`. |
| `crates/volicord-host-contract/tests/host_contracts.rs` | Contract parsing, source-type separation, required-field and bound enforcement, MCP consistency, and pinned-fixture manifest/checksum/profile parity. |
| `tests/conformance/codex-host/` | Reviewed offline Codex hook and MCP host-wire fixtures plus their production-coverage manifest and checksums. |

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
| `crates/volicord-store/src/diagnostic_findings/mod.rs` | Lifecycle-specific diagnostic persistence facade and public Store API exports. |
| `crates/volicord-store/src/diagnostic_findings/occurrence.rs` | Insert-only occurrence persistence and atomic runtime terminal-finding links. |
| `crates/volicord-store/src/diagnostic_findings/current_state.rs` | Current snapshot activation, replacement, resolution, and reactivation. |
| `crates/volicord-store/src/diagnostic_findings/graph.rs` | Cause-graph validation, current-report root selection, and bounded deterministic lifecycle-aware exact traversal. |
| `crates/volicord-store/src/diagnostic_findings/queries.rs` | Lifecycle-aware exact identifier, current-report projection, runtime-session occurrence, and active-current-scope queries. |
| `crates/volicord-store/src/diagnostic_findings/row.rs` | Internal finding row encoding, decoding, and lifecycle identity validation. |
| `crates/volicord-store/src/operational_sessions.rs` | Managed runtime sessions, protocol milestones, revision-scoped managed MCP project sessions, and exact cross-database bindings. |
| `crates/volicord-store/src/workflow_records.rs` | Workflow record reads and writes. |
| `crates/volicord-store/src/core_pipeline/` | Core-open, validation, replay, commit, and mutation application. |
| `crates/volicord-store/src/guards.rs` | Typed host-correlation normalization, MCP-only project anchors, phase-specific Guard observations, prompt captures, expected writes, and suppression inputs. |
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
| `crates/volicord-cli/src/connection_command/verification/mod.rs` | Connection verification coordinator, shared step/report types, and bounded package exports. |
| `crates/volicord-cli/src/connection_command/verification/host_checks.rs` | Managed configuration, host executable, project trust, and managed-host session checks. |
| `crates/volicord-cli/src/connection_command/verification/mcp_checks.rs` | MCP preflight/handshake check projection and MCP finding-ID inputs. |
| `crates/volicord-cli/src/connection_command/verification/guard_checks.rs` | Guard file, hook-execution, and observation check evaluation. |
| `crates/volicord-cli/src/connection_command/verification/dependency_graph.rs` | Cause attachment, `Blocked` propagation, graph finalization, action selection, and canonical check construction. |
| `crates/volicord-cli/src/connection_command/verification/finding_projection.rs` | Process, host, peer-version, and Guard observation projection into lifecycle-specific findings. |
| `crates/volicord-cli/src/connection_command/verification/report_inputs.rs` | Active verification and current-status report input assembly. |
| `crates/volicord-cli/src/operational_diagnostics/mod.rs` | Typed operational-diagnostic module facade and bounded internal exports. |
| `crates/volicord-cli/src/operational_diagnostics/definitions.rs` | Immutable CLI operational-diagnostic definitions and exhaustive closed diagnostic-value mappings. |
| `crates/volicord-cli/src/operational_diagnostics/subjects.rs` | Closed typed operational subjects, subject-family canonical encoding and opaque identity derivation, scope ownership, and separate safe display projection. |
| `crates/volicord-cli/src/operational_diagnostics/facts.rs` | Typed bounded operational fact projections. |
| `crates/volicord-cli/src/operational_diagnostics/actions.rs` | Recommended-action selection from diagnostic definitions, typed facts, and typed check state. |
| `crates/volicord-cli/src/operational_diagnostics/projection.rs` | Current and occurrence finding construction plus explicit active-current report projection. |
| `crates/volicord-cli/src/operational_diagnostics/persistence.rs` | Owner-scoped activation and explicit resolution through Store lifecycle APIs. |
| `crates/volicord-cli/src/connection_command/mcp_process/` | Managed launch materialization, bounded child-process supervision policy and deadlines, preflight interpretation, stdio JSON-RPC framing and probe sequencing, exchange progress, and typed lifecycle or protocol diagnostics. Low-level containment and pipe readiness route through `volicord-platform-process`. |
| `crates/volicord-cli/src/connection_command/mcp_process/host_compatibility.rs` | Independently pinned host-profile fixtures and Codex request/tool-call shapes; these are not derived from the production protocol registry. |
| `crates/volicord-cli/src/connection_command/mcp_process/pinned_schema.rs` | Revision-specific validation of initialize, `tools/list`, and `tools/call` probe messages against the pinned offline schemas. |
| `crates/volicord-cli/src/connection_command/output/` | Canonical selected-Connection diagnostic report construction, aggregate status and roots, and concise, verbose, and lossless JSON presentation of that same report. |
| `crates/volicord-cli/src/diagnostics_command.rs` | Finding-ID and runtime-session detail commands, bounded lifecycle-aware cause traversal, lookup-specific JSON and human projection, and lookup-status exit outcomes independent of finding severity. |
| `crates/volicord-cli/src/host_integration/codex/` | Codex configuration parsing and serialization, canonical managed-entry validation, preservation of the allowed tool-approval overlay, managed configuration mutation, diagnostic executable observations, and connection verification. |
| `crates/volicord-cli/src/guard_integration/manifest.rs` | Guard manifest, exact host-contract profile/digest, and canonical managed-artifact expectation generation. |
| `crates/volicord-cli/src/guard_integration/audit.rs` | Current Guard owner, artifact, command, marker, and executable-behavior audit. |
| `crates/volicord-cli/src/guard_command/` | Explicit `codex-hooks-v1` event decoding and bounded source-specific observations. |
| `crates/volicord-cli/src/user_command.rs` | CLI inbox and local-user resolution. |
| `crates/volicord-cli/src/doctor_command.rs` | Diagnostic fact collection and rendering. |

## MCP Protocol Profiles

| Path | Responsibility |
|---|---|
| `crates/volicord-mcp-protocol/src/lib.rs` | Closed typed MCP revision parsing, production profile lookup, message/tool/schema feature declarations, deterministic supported-revision ordering, tracked pre-release classification, and the separately selected preferred server revision. |
| `crates/volicord-mcp-protocol/tests/protocol_registry.rs` | Pinned manifest parity, exact schema-feature parity, ordering, duplicate exclusion, exact parsing, preferred-revision membership, and pre-release exclusion. |

## MCP Adapter

| Path | Responsibility |
|---|---|
| `crates/volicord-mcp/src/managed_launch.rs` | Canonical typed personal/shared managed MCP command, arguments, static and forwarded environment bindings, strict launch-shape validation, projection, and fingerprint inputs. |
| `crates/volicord-mcp/src/stdio.rs` | stdio lifecycle and framing, typed initialization-profile selection, explicit `codex-mcp-2025-06-18-v1` turn-metadata parsing, revision-aware message handling, and process preflight. |
| `crates/volicord-mcp/src/adapter.rs` | Public argument decoding, server-owned context, Core dispatch, and wrapping. |
| `crates/volicord-mcp/src/tool_registry.rs` | Assembly of `AgentToolId`-keyed schemas and metadata into canonical tool definitions/results, plus revision-specific wire-name projection through the selected protocol profile. |
| `crates/volicord-mcp/src/schema_validation.rs` | Public schema validation. |
| `crates/volicord-mcp/src/routing.rs` | Bound Product Repository discovery and current Connection/project routing. |

## Tests

| Path | Responsibility |
|---|---|
| `crates/*/tests/` and module-local `tests` | Crate boundary and unit tests. |
| `crates/volicord-mcp/src/tests/lifecycle.rs` | Initialization ordering, rejection, shutdown, and EOF contracts. |
| `crates/volicord-mcp/src/tests/batching.rs` | JSON-RPC batch ordering, notification, and response contracts. |
| `crates/volicord-mcp/src/tests/protocol_projection.rs` | Registry/profile wire projection and schema compatibility contracts. |
| `crates/volicord-mcp/src/tests/tool_calls.rs` | Tool dispatch, result, error, and storage-capability contracts. |
| `crates/volicord-mcp/src/tests/managed_host_observation.rs` | Managed launch, routing, session binding, and host-observation contracts. |
| `crates/volicord-mcp/src/tests/diagnostics.rs` | Diagnostic persistence and workflow-metric contracts. |
| `crates/volicord-mcp/src/tests/conformance.rs` | Module-level registry-driven protocol conformance assertions. |
| `crates/volicord-mcp/src/tests/support.rs` | Shared MCP test fixture and protocol-message construction only. |
| `crates/volicord-mcp/tests/protocol_conformance.rs` | One registry-driven wire conformance case for every production profile, including pinned-schema validation, required tools, the designated round trip, profile-specific projection and batching, lifecycle rejection, and EOF. |
| `tests/conformance/` | Cross-method conformance scenarios. |
| `tests/conformance/mcp-spec/` | Versioned official MCP schemas, release and handshake-family metadata, reviewed `production_supported` and `pre_release_only` facts, immutable upstream pins, license attribution, and checksums used as offline conformance inputs. |
| `tests/release-integrity/` | Generic five-target, version, canonical-byte, package, checksum, and release-workflow integrity tests. |
| `crates/volicord-test-support/` | Reusable fixtures only: disposable Runtime Home, repository, Store-facing setup and inspection, intentional corruption/malformed-storage setup, and request helpers. Product-behavior assertions stay in owner-specific tests, and implementation-test modules do not embed storage SQL. |

## Repository Maintenance Tooling

| Path | Responsibility |
|---|---|
| `xtask/Cargo.toml` | Lightweight maintenance dependency boundary: `volicord-mcp-protocol` supplies production profiles without pulling in `volicord-mcp`, Core, Store, or platform crates. |
| `xtask/src/mcp_spec/mod.rs` | MCP specification maintenance facade and command entry points. |
| `xtask/src/mcp_spec/manifest.rs` | Strict pinned manifest model, parsing, and deterministic rendering. |
| `xtask/src/mcp_spec/validation.rs` | Offline metadata, immutable-pin, checksum, artifact, schema, ordering, and registry-parity validation. |
| `xtask/src/mcp_spec/report.rs` | Deterministic check and synchronization report types. |
| `xtask/src/mcp_spec/sync.rs` | The sole networked MCP specification path, using a verified temporary candidate before replacement. |
| `xtask/tests/mcp_spec.rs` | Strict manifest parsing, classification, parity failures, immutable-pin, checksum, required-artifact, ordering, reporting, and offline-success coverage. |

Update this map when a durable responsibility moves. Do not list removed,
generated, or private scratch paths.
