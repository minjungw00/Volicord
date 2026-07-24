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
| `crates/volicord-types/src/connection_verification.rs` | Canonical `ConnectionStatus`, `IntegrationActivationState`, `HookActivationState`, checks, single hierarchical `IntegrationActivationPlan`, stable actor/channel/step metadata, topological validation, nested agent sequence, session-role evidence, and verification-report types. |
| `crates/volicord-types/src/integration_revision.rs` | Typed Connection/project integration revision bases and derivation. |
| `crates/volicord-types/src/guard_manifest.rs` | Canonical Guard manifest, managed-artifact, hook-phase, and typed command contracts. |
| `crates/volicord-types/src/tool_names.rs` | Closed `AgentToolId` catalog, `MethodName` reuse for Core-owned tools, category and mode metadata, compile-time verification-role binding, and stable MCP wire-name projection. |
| `crates/volicord-types/src/integration_verification.rs` | Shared closed tagged integration-verification workflow state, fixed canonical `AgentToolId`-backed tool-reference types, Guard-probe acquisition stages, restart reasons, and begin/probe/get public result shapes. |

## Host Wire Contracts

| Path | Responsibility |
|---|---|
| `crates/volicord-host-contract/src/lib.rs` | Semantic `CodexMcpTurnMetadata`, `CodexCommandHooks`, and `CodexMcpCallableNames` contracts; typed host-tool and server-namespace/catalog-derived-exact hook routing; deterministic profile digests; bounded values and errors; source-specific correlation; explicit `McpServerKey`, `McpRawToolName`, and `McpToolIdentity`; collision-checked projection to `HostCallableIdentity`; and exact `McpToolCatalog` reverse lookup. |
| `crates/volicord-host-contract/tests/host_contracts.rs` | Contract parsing, source-type separation, required-field and bound enforcement, typed matcher routing/reconstruction, MCP consistency, and pinned-fixture manifest/checksum/profile parity. |
| `tests/conformance/codex-host/` | Reviewed offline Codex command-hook, MCP turn-metadata, and MCP callable-name fixtures plus their semantic-profile coverage manifest and checksums. |

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
| `crates/volicord-test-process/src/lib.rs` | Safe `BoundedCommand`, `ProcessDeadline`, bounded capture/output, classified failure, single-supervisor stdio pumping, process-tree termination, direct-child reaping, and bounded cleanup for repository tests and smoke harnesses. |

## Store

| Path | Responsibility |
|---|---|
| `crates/volicord-store/src/schema/registry.sql` | Canonical Runtime Home registry DDL source. |
| `crates/volicord-store/src/schema/project.sql` | Canonical project Store DDL source. |
| `crates/volicord-store/src/bootstrap.rs` | Runtime Home and Store bootstrap. |
| `crates/volicord-store/src/setup_transaction.rs` | Explicit prepare, input validation, mutation checkpoint, commit, and guarded rollback boundary for the existing Store files touched by setup. |
| `crates/volicord-store/src/agent_connections.rs` | Agent Connection records, project allowlists, managed fingerprints, and persisted verification-report boundary. |
| `crates/volicord-store/src/diagnostic_findings/mod.rs` | Lifecycle-specific diagnostic persistence facade and public Store API exports. |
| `crates/volicord-store/src/diagnostic_findings/occurrence.rs` | Insert-only occurrence persistence and atomic runtime terminal-finding links. |
| `crates/volicord-store/src/diagnostic_findings/current_state.rs` | Current snapshot activation, replacement, resolution, and reactivation. |
| `crates/volicord-store/src/diagnostic_findings/graph.rs` | Cause-graph validation, current-report root selection, and bounded deterministic lifecycle-aware exact traversal. |
| `crates/volicord-store/src/diagnostic_findings/queries.rs` | Lifecycle-aware exact identifier, current-report projection, runtime-session occurrence, and active-current-scope queries. |
| `crates/volicord-store/src/diagnostic_findings/row.rs` | Internal finding row encoding, decoding, and lifecycle identity validation. |
| `crates/volicord-store/src/managed_launch_leases.rs` | Short-lived one-time managed MCP launch leases, current-Connection revalidation, deterministic cancellation/expiry cleanup, and atomic lease-consumption/runtime creation. |
| `crates/volicord-store/src/operational_sessions.rs` | Runtime-session source decoding, protocol milestones, revision-scoped managed MCP project sessions, exact cross-database bindings, and rejection of direct `managed_host` creation outside lease consumption. |
| `crates/volicord-store/src/integration_verification/mod.rs` | Public Store facade, stable integration-verification inputs and records, and bounded exports for the lifecycle implementation. |
| `crates/volicord-store/src/integration_verification/begin.rs` | Verification creation, exact-coordinate resume, active-run expiry before begin, current prompt selection, and durable ID allocation in one immediate Registry transaction. |
| `crates/volicord-store/src/integration_verification/probe.rs` | First-write probe acknowledgement, exact active and terminal replay, and concurrent-call convergence in one immediate Registry transaction. |
| `crates/volicord-store/src/integration_verification/observation.rs` | Typed hook acquisition, semantic callable filtering through the Connection server's `McpToolCatalog`, distinct correlation-mismatch stages, and bounded payload-free observation persistence. |
| `crates/volicord-store/src/integration_verification/correlation.rs` | Prompt and acquired pre/post event matching, hook-contract and tool-use correlation, timestamp ordering, and atomic completion refresh. |
| `crates/volicord-store/src/integration_verification/status.rs` | Effective lifecycle status, latest and exact reads, public result and tagged workflow projection, and stale-owner handling. |
| `crates/volicord-store/src/integration_verification/coordinate.rs` | Typed caller, current, and stored verification coordinates plus caller and run-owner validation. |
| `crates/volicord-store/src/integration_verification/row.rs` | Private verification SQL, row decoding, status and timestamp parsing, database representation conversion, and focused row-decoder tests. |
| `crates/volicord-store/src/integration_verification/tests/` | Lifecycle-owner tests for begin, probe, typed acquisition, correlation, status, and concurrent first acknowledgement, with shared fixture construction isolated from assertions. |
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
| `crates/volicord-cli/src/host_launch.rs` | Hidden same-process host launcher, exact current Codex entry revalidation, launch-lease issue/cleanup, and in-memory transition into managed stdio. |
| `crates/volicord-cli/src/connection_command/` | Connection add, list, status, verify, mode, and remove orchestration. |
| `crates/volicord-cli/src/connection_command/setup_transaction.rs` | Typed `SetupPlan`, Runtime Home preparation, same-directory atomic file mutations, freshness validation, deterministic commit, and bounded rollback for `volicord init`. |
| `crates/volicord-cli/src/connection_command/verification/mod.rs` | Connection verification coordinator, shared step/report types, and bounded package exports. |
| `crates/volicord-cli/src/connection_command/verification/host_checks.rs` | Managed configuration, host executable, project trust, and managed-host session checks. |
| `crates/volicord-cli/src/connection_command/verification/mcp_checks.rs` | MCP preflight/handshake check projection and MCP finding-ID inputs. |
| `crates/volicord-cli/src/connection_command/verification/guard_checks.rs` | Guard file, hook-execution, and observation check evaluation. |
| `crates/volicord-cli/src/connection_command/verification/dependency_graph.rs` | Cause attachment, `Blocked` propagation, graph finalization, current activation-plan suffix and typed repair selection, and canonical check construction. |
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
| `crates/volicord-cli/src/connection_command/output/` | Canonical selected-Connection diagnostic report construction, aggregate status and roots, and concise, verbose, and lossless JSON presentation of the same required and optional activation plan without a second renderer-owned step list. |
| `crates/volicord-cli/tests/init_record_regression.rs` | Init plan/read-only, replay, migration, exact owner record, all-stage setup fault injection, concurrent modification, full rollback, and partial-rollback reporting regressions. |
| `crates/volicord-cli/src/diagnostics_command.rs` | Finding-ID and runtime-session detail commands, bounded lifecycle-aware cause traversal, lookup-specific JSON and human projection, and lookup-status exit outcomes independent of finding severity. |
| `crates/volicord-cli/src/host_integration/codex/` | Codex configuration parsing and serialization, canonical managed-entry validation, preservation of the allowed tool-approval overlay, managed configuration mutation, diagnostic executable observations, and connection verification. |
| `crates/volicord-cli/src/host_integration/contracts.rs` | Explicit semantic Codex host-contract selection, typed Guard routing-strategy projection, and strict configuration reconstruction from the registered `McpServerKey`. |
| `crates/volicord-cli/src/guard_integration/manifest.rs` | Guard manifest, exact host-contract profile/digest, and canonical managed-artifact expectation generation. |
| `crates/volicord-cli/src/guard_integration/audit.rs` | Current Guard owner, artifact, command, marker, and executable-behavior audit. |
| `crates/volicord-cli/src/guard_integration/plan.rs` and `hosts/codex.rs` | Source templates for managed AGENTS and Codex rule guidance, including the nested integration-verification sequence and its stop and diagnostic boundaries. |
| `crates/volicord-cli/src/guard_command/` | Explicit `codex-command-hooks` event decoding, semantic Guard-probe filtering, and bounded source-specific observations without routed MCP payload retention. |
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
| `crates/volicord-mcp/src/managed_launch.rs` | Canonical typed personal/shared hidden-launcher command and arguments, Runtime Home environment binding, strict launch-shape validation, public manual probe materialization, projection, and fingerprint inputs. |
| `crates/volicord-mcp/src/stdio.rs` | Public manual stdio and in-memory lease-bound managed stdio entry paths, authoritative runtime-source selection, lifecycle and framing, typed initialization-profile selection, explicit `codex-mcp-turn-metadata` parsing, revision-aware message handling, and process preflight. |
| `crates/volicord-mcp/src/adapter.rs` | Public argument decoding, server-owned context, Core dispatch and wrapping, plus managed in-chat begin/probe/get integration-verification orchestration outside Core that serializes the Store-owned workflow projection without adapter-local state derivation. |
| `crates/volicord-mcp/src/constants.rs` | MCP initialize instructions for the user-level verification request, nested workflow-directed sequence, stop rules, unavailable boundary, and optional active diagnostics. |
| `crates/volicord-mcp/src/tool_registry.rs` | Assembly of `AgentToolId`-keyed schemas, annotations, effects descriptions, and metadata into canonical tool definitions/results, including the three Connection-integration tools; raw revision-specific wire-name projection through the selected protocol profile; and construction of the explicit-server, collision-checked Codex callable catalog. |
| `crates/volicord-mcp/src/schema_validation.rs` | Public schema validation. |
| `crates/volicord-mcp/src/routing.rs` | Bound Product Repository discovery, current Connection/project routing, and preflight diagnostic projection of server/raw/callable identities from the canonical catalog. |

## Tests

| Path | Responsibility |
|---|---|
| `crates/*/tests/` and module-local `tests` | Crate boundary and unit tests. |
| `crates/volicord-mcp/src/tests/lifecycle.rs` | Initialization ordering, rejection, shutdown, and EOF contracts. |
| `crates/volicord-mcp/src/tests/batching.rs` | JSON-RPC batch ordering, notification, and response contracts. |
| `crates/volicord-mcp/src/tests/protocol_projection.rs` | Registry/profile wire projection and schema compatibility contracts. |
| `crates/volicord-mcp/src/tests/tool_calls.rs` | Tool dispatch, result, error, and storage-capability contracts. |
| `crates/volicord-mcp/src/tests/managed_host_observation.rs` | Lease-bound managed launch, process-environment non-authority, runtime-source routing, session binding, and host-observation contracts. |
| `crates/volicord-mcp/src/tests/diagnostics.rs` | Diagnostic persistence and workflow-metric contracts. |
| `crates/volicord-mcp/src/tests/conformance.rs` | Module-level registry-driven protocol conformance assertions. |
| `crates/volicord-mcp/src/tests/support.rs` | Shared MCP test fixture and protocol-message construction only. |
| `crates/volicord-mcp/tests/protocol_conformance.rs` | One registry-driven wire conformance case for every production profile, including pinned-schema validation, required tools, the designated round trip, profile-specific projection and batching, lifecycle rejection, and EOF. |
| `crates/volicord-cli/tests/binary_admin.rs` | Actual-binary administrative CLI parser, help, output, and exit contracts. |
| `crates/volicord-cli/tests/operational_host_e2e.rs` | Full managed Codex activation journey from applied setup through lease-bound MCP and exact Guard prompt/pre/post verification to complete read-only status, plus operational failure and cleanup regressions. |
| `tests/conformance/` | Cross-method conformance scenarios. |
| `tests/conformance/mcp-spec/` | Versioned official MCP schemas, release and handshake-family metadata, reviewed `production_supported` and `pre_release_only` facts, immutable upstream pins, license attribution, and checksums used as offline conformance inputs. |
| `tests/release-integrity/` | Generic five-target, version, canonical-byte, package, checksum, and semantic CI/release-workflow integrity tests, including build/smoke/staging order, matrix binary inputs, exactly-once action use, path filters, and dependency direction. |
| `tests/release-smoke/Cargo.toml` | Publish-disabled dedicated smoke package boundary with protocol, canonical tool type, and shared bounded test-process dependencies but no CLI library, MCP implementation, Core, Store, or `xtask` dependency. |
| `tests/release-smoke/src/lib.rs` | Actual supplied-binary orchestration, disposable Git Product Repository and Runtime Home fixture, preferred-revision initialize and `tools/list` transcript validation, canonical representative tool assertions, release-specific process bounds, smoke result reporting, and focused transcript failure tests. |
| `tests/release-smoke/src/main.rs` | Package command entry and private stable Codex fixture behavior selected by the copied `codex` or `codex.exe` identity. |
| `tests/release-smoke/tests/` | Successful supplied-binary flow, stable and unsupported Codex fixture invocations, missing and unlaunchable binaries, test-owned Volicord process behavior, and bounded process timeout/cleanup coverage. |
| `.github/actions/volicord-release-smoke/action.yml` | Reusable workflow-level actual-binary smoke invocation with one binary-path input. |
| `crates/volicord-test-support/` | Reusable fixtures only: disposable Runtime Home, repository, Store-facing setup and inspection, intentional corruption/malformed-storage setup, and request helpers. Product-behavior assertions stay in owner-specific tests, and implementation-test modules do not embed storage SQL. |
| `crates/volicord-test-process/tests/` | Cross-platform bounded child execution, stdin, failure, timeout, truncation, concurrent stream, descendant-held pipe, cleanup, process containment, path, argument, and environment coverage. Native Unix and Windows cases exercise the platform containment selected by `volicord-platform-process`. |

## Repository Maintenance Tooling

| Path | Responsibility |
|---|---|
| `xtask/Cargo.toml` | Lightweight maintenance dependency boundary. `volicord-mcp-protocol` supplies production profiles for pinned specification parity without pulling in `volicord-mcp`, Core, Store, CLI, platform, or test-process crates. |
| `xtask/src/mcp_spec/mod.rs` | MCP specification maintenance facade and command entry points. |
| `xtask/src/mcp_spec/manifest.rs` | Strict pinned manifest model, parsing, and deterministic rendering. |
| `xtask/src/mcp_spec/validation.rs` | Offline metadata, immutable-pin, checksum, artifact, schema, ordering, and registry-parity validation. |
| `xtask/src/mcp_spec/report.rs` | Deterministic check and synchronization report types. |
| `xtask/src/mcp_spec/sync.rs` | The sole networked MCP specification path, using a verified temporary candidate before replacement. |
| `xtask/tests/mcp_spec.rs` | Strict manifest parsing, classification, parity failures, immutable-pin, checksum, required-artifact, ordering, reporting, and offline-success coverage. |

Update this map when a durable responsibility moves. Do not list removed,
generated, or private scratch paths.
