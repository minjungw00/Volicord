# Source Map

This map routes maintainers to current implementation owners. It is not a
product contract; use the focused Reference document for exact behavior.

## Shared Types

| Path | Responsibility |
|---|---|
| `crates/volicord-types/src/lib.rs` | Public owner-module routing. Shared definitions are public through their owning modules. |
| `crates/volicord-types/src/schema.rs` | Shared request, response, and stored-record shapes. |
| `crates/volicord-types/src/product_path.rs` | Platform-neutral Product Repository relative-path value, lexical validation, and pure component-aware containment relationships; no filesystem observation. |
| `crates/volicord-types/src/values.rs` | Closed product value sets. |
| `crates/volicord-types/src/ids.rs` | Opaque identifiers. |
| `crates/volicord-types/src/canonical.rs` | Canonical serialization and hashing. |
| `crates/volicord-types/src/diagnostics.rs` | Lifecycle-specific occurrence/current finding types, opaque `DiagnosticSubjectIdentity`, `CurrentDiagnosticKey` canonical identity and fixed digest ID derivation, lifecycle-aware `StoredDiagnosticFinding` and `StoredDiagnosticGraph`, separate `DiagnosticLookupReport`, shared read-only `DiagnosticFinding` and selected-Connection `DiagnosticReport` types, stable namespaced-code validation, bounded redacting projection of typed owner facts, cause-graph validation, and unexpected-failure fallback. |
| `crates/volicord-types/src/platform.rs` | Shared platform-environment and platform-path types. |
| `crates/volicord-types/src/host_configuration.rs` | Shared connection-intent and host-scope configuration types. |
| `crates/volicord-types/src/connection_verification.rs` | Canonical `ConnectionStatus`, `IntegrationActivationState`, `HookActivationState`, checks, single hierarchical `IntegrationActivationPlan`, stable actor/channel/step metadata, topological validation, nested agent sequence, session-role evidence, and verification-report types. |
| `crates/volicord-types/src/integration_revision.rs` | Typed Connection/project integration revision bases and derivation. |
| `crates/volicord-types/src/guard_manifest.rs` | Canonical Guard manifest, managed-artifact, hook-phase, and typed command contracts. |
| `crates/volicord-types/src/tool_names.rs` | Closed `AgentToolId` catalog, `MethodName` reuse for Core-owned tools, category and mode metadata, compile-time verification-role binding, catalog-owned `IntegrationVerificationToolRole`, and stable MCP wire-name projection. |
| `crates/volicord-types/src/integration_verification.rs` | Shared closed tagged integration-verification workflow state, fixed canonical `AgentToolId`-backed tool-reference types, typed routed-event relevance, Guard-probe acquisition stages with their terminal-reason mapping, restart reasons, and begin/probe/get public result shapes. |

## Host Wire Contracts

| Path | Responsibility |
|---|---|
| `crates/volicord-host-contract/src/lib.rs` | Semantic `CodexMcpTurnMetadata`, `CodexCommandHooks`, and `CodexMcpCallableNames` contracts; typed host-tool and server-namespace/catalog-derived-exact hook routing; MCP-only routing classification; deterministic profile digests; bounded values and errors; source-specific correlation; explicit `McpServerKey`, `McpRawToolName`, and `McpToolIdentity`; collision- and role-consistency-checked projection to `HostCallableIdentity`; and exact `McpToolCatalog` reverse lookup. |
| `crates/volicord-host-contract/tests/host_contracts.rs` | Contract parsing, source-type separation, required-field and bound enforcement, typed matcher routing/reconstruction, MCP consistency, and pinned-fixture manifest/checksum/profile parity. |
| `tests/conformance/codex-host/` | Reviewed offline Codex command-hook, MCP turn-metadata, and MCP callable-name fixtures plus their semantic-profile coverage manifest and checksums. |

## Platform Filesystem Boundary

| Path | Responsibility |
|---|---|
| `crates/volicord-platform-fs/src/lib.rs` | Current process target and platform observation, native Linux/WSL2 kernel classification, WSL2 `/etc/os-release` distribution validation, path-filesystem observation, closed typed platform diagnostic kinds with unique canonical codes and bounded detail, shared platform-finding projection, effect-aware exact directory-tree removal, typed atomic no-replace regular-file publication and parent-entry durability, platform-native namespace operations, safe Runtime Home mutation-lease and permit exports, canonical read-only Git layout discovery, and Product Repository observation exports. |
| `crates/volicord-platform-fs/src/product_path.rs` | Exclusive live Product Repository root/candidate observation, private canonical identities, nearest-existing-ancestor handling for missing candidates, link-aware containment, and opaque typed observations. |
| `crates/volicord-platform-fs/src/mutation_lease.rs` | Canonical Runtime Home identity, domain-separated full-digest external coordination-file derivation, shared-writer and exclusive-setup modes on one OS lock region, immediate and bounded typed acquisition, borrowed mutation permits, and Unix/macOS or native Windows handle-lifetime release. |
| `crates/volicord-platform-fs/tests/mutation_lease_process.rs` | Cross-process shared/exclusive mutation-lease contention and process-termination release regression. |
| `crates/volicord-cli/src/host_integration/process.rs` | Process-target validation, target-path filesystem enforcement from platform-boundary observations, and canonical platform diagnostic display projection. |

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
| `crates/volicord-store/src/mutation.rs` | Non-cloneable, permit-borrowing, exact-target `RuntimeHomeMutationContext`; retained `CanonicalRuntimeHomePath`; typed identity comparison without post-admission recanonicalization; shared/exclusive mode checks; and stable setup-in-progress condition projection. |
| `crates/volicord-store/src/sqlite.rs` | Separate read-only opens and crate-private context-gated writable Registry/project database opens with exact Runtime Home ownership validation. |
| `crates/volicord-store/src/bootstrap.rs` | Context-owned Runtime Home staging and project lookup, opaque publication provenance, atomic no-replace publication outcomes, typed publication identity, token-backed terminal rollback states, composite confirmation failures, and Store bootstrap. |
| `crates/volicord-store/src/diagnostics.rs` | Non-authority diagnostics schema and manifest, same-directory staged carrier publication, concurrent-winner validation, exact reads and writes, and retention. |
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
| `crates/volicord-store/src/integration_verification/begin.rs` | Verification creation, exact-coordinate resume, coordinate-change terminalization, typed retry eligibility, current prompt selection, and durable ID allocation in one immediate Registry transaction. |
| `crates/volicord-store/src/integration_verification/probe.rs` | First-write probe acknowledgement, exact active and terminal replay, and concurrent-call convergence in one immediate Registry transaction. |
| `crates/volicord-store/src/integration_verification/observation.rs` | Typed hook acquisition, semantic callable filtering through the Connection server's `McpToolCatalog`, distinct correlation-mismatch stages, and bounded payload-free observation persistence. |
| `crates/volicord-store/src/integration_verification/correlation.rs` | Prompt and acquired pre/post event matching, hook-contract and tool-use correlation, timestamp ordering, and atomic completion refresh. |
| `crates/volicord-store/src/integration_verification/status.rs` | Effective lifecycle status, latest and exact reads, public result and tagged workflow projection, and stale-owner handling. |
| `crates/volicord-store/src/integration_verification/coordinate.rs` | Typed caller, current, and stored verification coordinates plus caller and run-owner validation. |
| `crates/volicord-store/src/integration_verification/row.rs` | Private verification SQL, row decoding, status and timestamp parsing, database representation conversion, and focused row-decoder tests. |
| `crates/volicord-store/src/integration_verification/tests/` | Lifecycle-owner tests for begin, probe, typed acquisition, correlation, status, and concurrent first acknowledgement, with shared fixture construction isolated from assertions. |
| `crates/volicord-store/src/workflow_records.rs` | Project workflow-policy record reads, workflow-policy mutation input and application, and typed policy mutation effects. |
| `crates/volicord-store/src/core_pipeline/mod.rs` | Public Core Store type routing, commit and mutation inputs, and transaction-level Store tests. |
| `crates/volicord-store/src/core_pipeline/facade.rs` | `CoreProjectStore` connection and project identity, retained mutation authority, facade accessors, and shared read-snapshot primitive. |
| `crates/volicord-store/src/core_pipeline/open.rs` | Explicit read-only opening and mutation opening that retains the context's typed canonical Runtime Home identity. |
| `crates/volicord-store/src/core_pipeline/project_state.rs` | Project-state column projection, row decoding, timestamp validation, and facade reads. |
| `crates/volicord-store/src/core_pipeline/enforcement_profile.rs` | Project enforcement-profile projection, strict JSON decoding, validation, and facade read. |
| `crates/volicord-store/src/core_pipeline/clock.rs` | Store-handle clock samples, project UTC-floor reads, and transactional floor advancement. |
| `crates/volicord-store/src/core_pipeline/tasks.rs` | Task and acceptance mutation inputs, storage validation and SQL application; Task, acceptance-criterion, evidence-claim, and Task-revision projections; facade reads and focused tests. |
| `crates/volicord-store/src/core_pipeline/change_units.rs` | Change Unit mutation inputs, storage validation and SQL application; projections, strict row and JSON decoding, facade reads, and focused tests. |
| `crates/volicord-store/src/core_pipeline/write_tickets.rs` | Write Ticket mutation inputs, storage validation and SQL application; projections, strict row and JSON decoding, facade reads, and focused tests. |
| `crates/volicord-store/src/core_pipeline/runs.rs` | Run mutation inputs, storage validation and SQL application; Run and observed-change projections, strict decoding, facade reads, and focused tests. |
| `crates/volicord-store/src/core_pipeline/evidence.rs` | Evidence mutation inputs, storage validation and SQL application; evidence-summary and observation projections, strict row decoding, record-reference projection, facade reads, and focused tests. |
| `crates/volicord-store/src/core_pipeline/artifacts.rs` | Artifact mutation inputs, storage validation and SQL application; staging and durable-artifact projections, strict decoding, link reads, persistent-body verification, facade reads, and focused tests. |
| `crates/volicord-store/src/core_pipeline/user_actions.rs` | User-action mutation inputs, storage validation and SQL application; strict decoding from physical JSON and stored scalars into typed request and resolution records; effective-status derivation, facade reads, and focused tests. |
| `crates/volicord-store/src/core_pipeline/continuity.rs` | Continuity mutation inputs, storage validation and SQL application; project-continuity projection, bounded snapshot pages, facade reads, and focused tests. |
| `crates/volicord-store/src/core_pipeline/replay.rs` | Tool-invocation projection, SQL, strict replay-context decoding, immutable operation-result projection, and facade reads. |
| `crates/volicord-store/src/core_pipeline/reconciliation.rs` | Confirmed expected-write and unrecorded-change observation candidate projections, plus current-handle unresolved-change reads used by close-readiness fact acquisition. |
| `crates/volicord-store/src/core_pipeline/blockers.rs` | Active blocker-reference query and facade read. |
| `crates/volicord-store/src/core_pipeline/events.rs` | Project authority-event identity lookup. |
| `crates/volicord-store/src/core_pipeline/agent_sessions.rs` | Project-local Agent Session facade entry point over the Guard-owned strict row reader. |
| `crates/volicord-store/src/core_pipeline/record_refs.rs` | Shared stored-record reference representation used by aggregate reads. |
| `crates/volicord-store/src/core_pipeline/inspection.rs` | No-effect project storage counters used by verification paths. |
| `crates/volicord-store/src/core_pipeline/mutations.rs` | Grouped `CoreStorageMutation` routing, static aggregate dispatch, transaction-scoped mutation context, and typed aggregate application results. |
| `crates/volicord-store/src/core_pipeline/commit.rs` | Replay and freshness gates, ordered aggregate delegation, one state-version advance and canonical commit timestamp, atomic event/replay/response persistence, rollback, and final commit outcome. |
| `crates/volicord-store/src/core_pipeline/validation.rs` | Persisted-value and mutation-input validation shared by current Store owners. |
| `crates/volicord-store/src/guards.rs` | Typed host-correlation normalization, MCP-only project anchors, phase-specific Guard observations, prompt captures, expected writes, and suppression inputs. |
| `crates/volicord-store/src/evidence_capture.rs` | Evidence-capture intent and producer records. |
| `crates/volicord-store/src/artifacts.rs` | Artifact staging and durable body validation. |
| `crates/volicord-store/src/runtime_home.rs` | Runtime Home selection and path-boundary validation, including propagation of typed platform diagnostics across runtime-path failures. |
| `crates/volicord-store/src/operational_diagnostics.rs` | Typed Runtime Home and Store finding projection, with direct preservation of platform-owned finding identity and action policy. |
| `crates/volicord-store/src/error.rs` | Store failure classification and typed platform diagnostic retention. |

## UserAction Service

| Path | Responsibility |
|---|---|
| `crates/volicord-user-action-service/src/lib.rs` | Narrow public routing for typed contexts, intents, facts, service errors, and responsibility-owner functions; no Core or adapter facade. |
| `crates/volicord-user-action-service/src/model.rs` | Semantic intent, validated construction values, explicit construction and persistence contexts, and adapter-neutral pending/current/resolution facts. |
| `crates/volicord-user-action-service/src/validation.rs` | Filesystem-free validation and normalization of action kinds, coordinates, authority-bearing combinations, semantic Product Repository path values, operation targets, and expiration semantics. |
| `crates/volicord-user-action-service/src/relevance.rs` | Pure current-authority and operation-relevance decisions over typed semantic facts, including component-aware sensitive-path relationships without repository access. |
| `crates/volicord-user-action-service/src/body.rs` | Pure construction of canonical typed `UserActionRequestBody` and `UserActionBasis` values from validated intent and acquired facts. |
| `crates/volicord-user-action-service/src/identity.rs` | Stable source identity, deduplication metadata, and focused request-identity availability checks. |
| `crates/volicord-user-action-service/src/service.rs` | Store-backed acquisition of typed construction, artifact, target, pending, and resolved authority facts without Core request orchestration. |
| `crates/volicord-user-action-service/src/materialization.rs` | Application of caller-supplied operation identity and construction of canonical public requests and immutable resolutions. |
| `crates/volicord-user-action-service/src/persistence.rs` | Exact typed mapping from canonical request or resolution values to Store mutation inputs. |
| `crates/volicord-user-action-service/src/authority.rs` | Normalized authority and public request projection from Store-decoded typed records. |
| `crates/volicord-user-action-service/src/lifecycle.rs` | Pure projected Task lifecycle interpretation from current pending authority facts. |
| `crates/volicord-user-action-service/src/resolution.rs` | Current-basis validation, canonical typed resolution construction, and replay-input comparison. |
| `crates/volicord-user-action-service/src/continuity.rs` | Semantic derivation of continuity drafts for accepted authority-bearing resolutions. |
| `crates/volicord-user-action-service/src/projection.rs`, `summary.rs` | Adapter-neutral pending, resolution, instruction, and safe-summary facts. |
| `crates/volicord-user-action-service/src/tests/` | Responsibility-local validation, body, identity, authority, lifecycle, materialization, persistence, resolution, continuity, and projection tests. |

## Core

| Path | Responsibility |
|---|---|
| `crates/volicord-core/src/pipeline.rs` | Separate read-only path and admitted-context `CoreService` construction, typed Core/Store Runtime Home authorization, common preflight, replay, plan selection, response, commit orchestration, Store-error detail projection, and neutral typed operational projection of platform-owned Product Repository observation failures. |
| `crates/volicord-core/src/product_path.rs` | Coordination of shared lexical parsing with platform-owned live Product Repository observations, plus filesystem-free parsing of already stored semantic identities. |
| `crates/volicord-core/src/methods/` | Method-specific structural validation and planning. Production method modules import shared helpers, pipeline and policy functions, Store services, and shared types from their explicit owners; the parent module is not an import prelude. |
| `crates/volicord-core/src/methods/evidence_facts.rs` | Shared Store reads and strict decoding that acquire typed facts for stored and projected evidence without owning evidence-policy classification. |
| `crates/volicord-core/src/methods/close_readiness/mod.rs` | Narrow package surface for close-readiness services, projections, and blocker helpers consumed by method planners. |
| `crates/volicord-core/src/methods/close_readiness/facts.rs` | Typed current-fact acquisition and projected-fact assembly, including one acceptance-criteria snapshot, one workflow-policy snapshot, and current-handle unresolved-change reads; owns no readiness decision. |
| `crates/volicord-core/src/methods/close_readiness/change_control.rs` | Task, Change Unit, close-basis, baseline, recovery, unresolved-change, and Write Ticket condition evaluation. |
| `crates/volicord-core/src/methods/close_readiness/evidence.rs` | Close evidence and artifact availability evaluation through the focused evidence fact and pure policy owners. |
| `crates/volicord-core/src/methods/close_readiness/acceptance.rs` | Pending close authority, cancellation, sensitive approval, final acceptance, and residual-risk acceptance evaluation. |
| `crates/volicord-core/src/methods/close_readiness/policy.rs` | Store-independent effective-control resolution and ordered pure combination of typed readiness evaluations into the close state. |
| `crates/volicord-core/src/methods/close_readiness/blockers.rs` | Canonical typed close blocker construction, Write Ticket blocker projection, and cross-blocker action normalization. |
| `crates/volicord-core/src/methods/close_readiness/guidance.rs` | Adapter-neutral semantic continuation selection with typed owner methods and operation categories; owns no CLI syntax, capture path, Markdown, rendering, or credentials. |
| `crates/volicord-core/src/methods/close_readiness/summary.rs` | Full close-operation assessment and deliberate smaller method-neutral readiness projection. |
| `crates/volicord-core/src/methods/close_readiness/service.rs` | Narrow coordination of fact acquisition, responsibility-owned evaluation, pure policy combination, full close assessment, and method-neutral summary projection. |
| `crates/volicord-core/src/methods/close_readiness/tests/` | Responsibility-local fact, change-control, evidence, acceptance, policy, blocker, and guidance tests plus close-readiness service integration coverage. |
| `crates/volicord-core/src/methods/prepare_evidence_capture.rs` | Evidence-capture request validation and planning; consumes target policy for acceptance-criterion and supplemental-claim matching. |
| `crates/volicord-core/src/methods/record_run.rs` | Run and evidence-update validation and planning; consumes provenance, relevance, target, binding, and close-readiness evidence policy. |
| `crates/volicord-core/src/methods/close_task.rs` | Request-specific close orchestration: request validation, close-readiness service invocation, terminal mutation planning, and typed result assembly. |
| `crates/volicord-core/src/methods/update_scope.rs` | Scope-update planning and projected evidence-summary completion through the close-readiness evidence policy owner. |
| `crates/volicord-core/src/methods/status.rs` | Read-only status projection, including consumption of shared close-readiness evidence policy through Core projection paths. |
| `crates/volicord-core/src/methods/user_action.rs` | Direct request and resolution method orchestration; consumes shared typed UserAction services and maps their results into method plans and responses. |
| `crates/volicord-core/src/methods/user_action_read.rs` | User Channel authorization, coherent Store snapshots, originating-result replay, and public method-result projection. |
| `crates/volicord-core/src/methods/user_action_continuity.rs` | Store fact acquisition, Core-owned continuity identifiers and timestamps, service draft consumption, and persistence sequencing. |
| `crates/volicord-core/src/methods/reconcile_changes.rs` | Reconciliation-specific planning, including direct consumption of the UserAction service when unresolved changes require typed pending actions. |
| `crates/volicord-core/src/policy/` | Responsibility-owned reusable policy. Method implementations consume these owners directly rather than obtaining shared policy from sibling method modules. |
| `crates/volicord-core/src/policy/evidence_provenance.rs` | Pure evidence provenance and assurance classification over typed facts. |
| `crates/volicord-core/src/policy/evidence_relevance.rs` | Pure evidence relevance and support classification. |
| `crates/volicord-core/src/policy/evidence_target.rs` | Evidence target, observation basis, and `CurrentCloseBasis` matching. |
| `crates/volicord-core/src/policy/evidence_binding.rs` | Producer-reference, producer-output, and exact artifact binding policy. |
| `crates/volicord-core/src/policy/close_readiness_evidence.rs` | Close-readiness evidence interpretation, required-criterion summary completion, and evidence-gate evaluation. |
| `crates/volicord-core/src/agent_session.rs` | Current Connection, project membership, mode, and managed runtime/project-session validation. |
| `crates/volicord-core/src/authority_status.rs` | Typed status and authority-receipt correspondence. |

## Command Model

| Path | Responsibility |
|---|---|
| `crates/volicord-command-model/src/lib.rs` | Complete Clap command declaration for the `volicord` binary; root parser; public and hidden subcommand tree; command and argument DTOs; command-surface value enums and syntax validators; root `clap::Command` construction; actual-model visibility classification; command-path traversal; canonical synopsis rendering; public-invocation validation; generation of canonical parseable public invocations; and typed inbox-resolution invocation builders that derive paths and option spellings from the same declaration and parse-check their output. |

## UserAction Presentation

| Path | Responsibility |
|---|---|
| `crates/volicord-user-action-presentation/src/lib.rs` | Typed CLI projection from adapter-neutral UserAction facts into `CliUserActionInboxResponse`, `CliUserActionInboxItem`, closed channel/capture-path states, CLI JSON Schemas, and recovery instructions. It gets command syntax only from typed `volicord-command-model` invocations and owns no Core policy, Store read, command execution, terminal rendering, or MCP envelope. |

## CLI And Codex Adapter

| Path | Responsibility |
|---|---|
| `crates/volicord-cli/src/main.rs` | Process entry, parsing through `volicord-command-model`, and administrative command dispatch. |
| `crates/volicord-cli/src/mutation_admission.rs` | Exact Runtime Home resolution, per-operation `SharedWriter` acquisition, Store mutation-context construction, stable typed busy mapping, and lease retention for mutating CLI and Guard operations. |
| `crates/volicord-cli/src/host_launch.rs` | Hidden same-process host launcher, exact current Codex entry revalidation, launch-lease issue/cleanup, and in-memory transition into managed stdio. |
| `crates/volicord-cli/src/connection_command/` | Connection add, list, status, verify, mode, and remove orchestration. |
| `crates/volicord-cli/src/connection_command/service.rs` | Locked setup service boundary that acquires `ExclusiveSetup` mutation admission before planning and retains the canonical Runtime Home lease through dry-run reporting, commit, cleanup, or rollback for init and Connection add. |
| `crates/volicord-cli/src/connection_command/setup_transaction.rs` | Typed `SetupPlan`, explicit Runtime Home publication ownership and removal-effect states, same-directory atomic file mutations, freshness validation, deterministic commit, effect-aware Project Home cleanup, and guarded bounded rollback for `volicord init`. |
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
| `crates/volicord-cli/src/connection_command/output/` | Canonical selected-Connection diagnostic report construction, aggregate status and roots, typed Runtime Home rollback effect/durability output, and concise, verbose, and lossless JSON presentation of the same required and optional activation plan without a second renderer-owned step list. |
| `crates/volicord-cli/tests/init_record_regression.rs` | Init plan/read-only, replay, exact owner record, post-rename and all-stage setup fault injection, deterministic exclusive mutation-admission success/rollback contention in both invocation orders, busy and dry-run non-mutation, unexpected external publication abort, concurrent file modification, full rollback, and partial-rollback reporting regressions. |
| `crates/volicord-cli/src/diagnostics_command.rs` | Finding-ID and runtime-session detail commands, bounded lifecycle-aware cause traversal, lookup-specific JSON and human projection, and lookup-status exit outcomes independent of finding severity. |
| `crates/volicord-cli/src/host_integration/codex/` | Codex configuration parsing and serialization, canonical managed-entry validation, preservation of the allowed tool-approval overlay, managed configuration mutation, diagnostic executable observations, and connection verification. |
| `crates/volicord-cli/src/host_integration/contracts.rs` | Explicit semantic Codex host-contract selection, typed Guard routing-strategy projection, and strict configuration reconstruction from the registered `McpServerKey`. |
| `crates/volicord-cli/src/guard_integration/manifest.rs` | Guard manifest, exact host-contract profile/digest, and canonical managed-artifact expectation generation. |
| `crates/volicord-cli/src/guard_integration/audit.rs` | Current Guard owner, artifact, command, marker, and executable-behavior audit. |
| `crates/volicord-cli/src/guard_integration/plan.rs` and `hosts/codex.rs` | Source templates for managed AGENTS and Codex rule guidance, including the nested integration-verification sequence and its stop and diagnostic boundaries. |
| `crates/volicord-cli/src/guard_command/` | Explicit `codex-command-hooks` event decoding, semantic Guard-probe filtering, and bounded source-specific observations without routed MCP payload retention. |
| `crates/volicord-cli/src/user_command.rs` | CLI inbox and local-user resolution, with pre-admission syntax and repository targeting followed by admitted Registry/project selection, neutral Core fact consumption, shared UserAction presentation, one-snapshot candidate planning, diagnostics, Core effect, and terminal response rendering under the same mutation context. |
| `crates/volicord-cli/src/doctor_command.rs` | Diagnostic fact collection and rendering. |

## MCP Protocol Profiles

| Path | Responsibility |
|---|---|
| `crates/volicord-mcp-protocol/src/lib.rs` | Closed typed MCP revision parsing, exact production profile lookup, the single revision-to-semantic-capability map, deterministic supported-revision iteration, tracked pre-release classification, and explicit unknown or unsupported rejection. |
| `crates/volicord-mcp-protocol/tests/protocol_registry.rs` | Pinned manifest parity, complete semantic/schema capability parity, registry uniqueness, deterministic iteration, exact parsing and selection, preferred-revision membership, and pre-release exclusion. |

## MCP Adapter

| Path | Responsibility |
|---|---|
| `crates/volicord-mcp/src/lib.rs` | Adapter-owned public entry points and boundary types. Shared type and tool identities remain at their `volicord-types` owner-module routes. |
| `crates/volicord-mcp/src/managed_launch.rs` | Canonical typed personal/shared hidden-launcher command and arguments, Runtime Home environment binding, strict launch-shape validation, public manual probe materialization, projection, and fingerprint inputs. |
| `crates/volicord-mcp/src/mutation_admission.rs` | Per-message and per-tool `SharedWriter` acquisition, Store context construction, typed setup-busy propagation, and bounded lease lifetime across complete MCP effects. |
| `crates/volicord-mcp/src/stdio.rs` | Public manual and in-memory lease-bound managed stdio facade. It selects the entry-path binding and delegates the connected stream without retaining protocol, lifecycle, or tool-dispatch implementations. |
| `crates/volicord-mcp/src/transport.rs` | Bounded newline-delimited stdio reads and writes, UTF-8 and frame-limit enforcement, transport-loop termination, and delegation of decoded JSON values to lifecycle handling. |
| `crates/volicord-mcp/src/json_rpc.rs` | JSON syntax decoding, JSON-RPC envelope classification, string/integer request-ID validation, object-parameter validation, and success/error response construction without Core access. |
| `crates/volicord-mcp/src/lifecycle.rs` | Exact initialize profile selection, initialized-notification admission, capability-driven batch and per-method lifecycle validity, runtime-session start and close, and the closed `SessionState` variants `AwaitingInitialization`, `AwaitingInitializedNotification`, `InitializedAndReady`, and `Closed`. Initialization selection exists only in initialized variants, and termination data exists only in `Closed`. |
| `crates/volicord-mcp/src/binding.rs` | Runtime Home resolution, repository discovery, Connection/project preflight and binding, and managed Codex session/thread/turn correlation. |
| `crates/volicord-mcp/src/tool_dispatch.rs` | `tools/list` and `tools/call` parameter decoding, canonical tool selection, adapter/Core invocation, and shared canonical tool-result carrier assembly. It does not frame transport messages or own mutation, recovery, UserAction, or metric projection. |
| `crates/volicord-mcp/src/mutation_projection.rs` | Mutation detail selection, effect anchoring, compact method-result projection, fresh-authority composition, and capability-driven normal result-budget enforcement. |
| `crates/volicord-mcp/src/authority_refresh.rs` | Post-mutation Agent Session binding, current authority reread, coordinate validation, and extraction of the fresh authority receipt and next actions. |
| `crates/volicord-mcp/src/committed_result_recovery.rs` | Capability-selected, authority-first bounded recovery after committed mutation projection, refresh, or post-effect failures without mutation retry. |
| `crates/volicord-mcp/src/user_action_projection.rs` | Committed UserAction coordinate extraction, neutral current-fact reread, adapter-owned safe MCP result construction, neutral failure mapping, and shared CLI inbox fallback attachment. |
| `crates/volicord-mcp/src/telemetry.rs` | Runtime-session finding and diagnostic-event persistence plus bounded best-effort handling for diagnostic-carrier failures where the contract permits it. |
| `crates/volicord-mcp/src/session_metrics.rs` | Diagnostic-session establishment and session-scoped tools-list, method-call, and status-reread workflow metrics. |
| `crates/volicord-mcp/src/diagnostics.rs` | Closed MCP diagnostic mapping, shared finding construction, and preservation of platform-owned diagnostic codes and action classes in bootstrap and persisted terminal projections. |
| `crates/volicord-mcp/src/adapter.rs` | Retained pre-operation routing identity, live mutation-context correlation, context-bound Core invocation APIs, plus managed in-chat begin/probe/get integration-verification orchestration outside Core that serializes the Store-owned workflow projection without adapter-local state derivation. |
| `crates/volicord-mcp/src/constants.rs` | MCP initialize instructions for the user-level verification request, nested workflow-directed sequence, stop rules, unavailable boundary, and optional active diagnostics. |
| `crates/volicord-mcp/src/tool_registry.rs` | Assembly of `AgentToolId`-keyed schemas, annotations, effects descriptions, metadata, and method lookup into canonical tool definitions/results, including the three Connection-integration tools; semantic-capability-only wire projection; and construction of the explicit-server, collision-checked Codex callable catalog. |
| `crates/volicord-mcp/src/schema_validation.rs` | Public schema validation. |
| `crates/volicord-mcp/src/routing.rs` | Bound Product Repository discovery, current Connection/project routing, and preflight diagnostic projection of server/raw/callable identities from the canonical catalog. |

## Tests

| Path | Responsibility |
|---|---|
| `crates/*/tests/` and module-local `tests` | Crate boundary and unit tests. |
| `crates/volicord-command-model/src/lib.rs` module tests | Clap structural assertions, complete public traversal, hidden-subtree exclusion, canonical-invocation self-parsing, typed inbox-resolution invocation round trips, and current required-argument, conflict, and value-set behavior. |
| `crates/volicord-mcp/src/transport.rs`, `json_rpc.rs`, and `binding.rs` module tests | Frame limits and draining, delimiter and UTF-8 behavior, request identifiers and notification classification, exact managed-call metadata, and Runtime Home binding failures at their implementation owners. |
| `crates/volicord-mcp/src/tests/lifecycle.rs` | Initialization ordering, rejection, shutdown, and EOF contracts. |
| `crates/volicord-mcp/src/tests/batching.rs` | JSON-RPC batch ordering, notification, and response contracts. |
| `crates/volicord-mcp/src/tests/protocol_projection.rs` | Registry/profile wire projection and schema compatibility contracts. |
| `crates/volicord-mcp/tests/protocol_conformance.rs` | The single executable production-profile harness for common initialize, lifecycle, schema, discovery, result-carrier, rejection, batching, and shutdown scenarios. |
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
| `xtask/Cargo.toml` | Lightweight maintenance dependency boundary. `volicord-command-model` supplies the public command grammar for documentation examples, `volicord-types` supplies runtime-owned contract identifiers, and `volicord-mcp-protocol` supplies production profiles for pinned specification parity, without pulling in `volicord-mcp`, Core, Store, CLI, platform, or test-process crates. |
| `xtask/src/lib.rs` | Thin repository-check orchestration and public report re-exports. |
| `xtask/src/diagnostics.rs` | Shared path, category, optional line, and message representation for validation issues. |
| `xtask/src/doc_index.rs` | Current documentation-index schema, applicability and exact semantic-contract routing, owner routing, indexed paths, and maintained-document coverage. |
| `xtask/src/markdown.rs` | Shared Markdown event parsing, heading meaning units, and supported contract-literal constructs. |
| `xtask/src/links.rs` | Local Markdown target resolution, links, fragments, and anchors. |
| `xtask/src/parity.rs` | English/Korean heading-structure parity. |
| `xtask/src/terminology.rs` | Terminology-map paths and identity-sensitive role validation. |
| `xtask/src/cli_docs.rs` | `docs-sync` composition, generated Administrative CLI regions, and documented invocation validation through `volicord-command-model`; shell tokenization is not a second command grammar. |
| `xtask/src/document_structure.rs` | Current architecture-design section and surface-stability structure. |
| `xtask/src/contract_identifiers.rs` | Current public-schema, command-model, typed-diagnostic, and protocol-registry identifier derivation; paired meaning-unit validation; and operation-category table parity. |
| `xtask/src/workspace_manifests.rs` | Shared workspace-manifest parsing and current package and Rust applicability values. |
| `xtask/src/architecture.rs` | Cargo-metadata-derived package manifests, target source roots, dependency edges, package-level architecture validation, bilingual generated responsibility and dependency regions, generated-region drift checks, and informational maintainability reporting. |
| `xtask/src/release_metadata.rs` | Workspace release-version inheritance and release-tag validation. |
| `xtask/src/storage.rs` | Canonical Storage DDL documentation validation. |
| `xtask/src/artifact_hygiene.rs` | Git-index validation against repository artifact-exclusion rules owned by `.gitignore`. |
| `xtask/src/repository.rs` | Shared repository path normalization used by focused validators. |
| `xtask/src/mcp_spec/mod.rs` | MCP specification maintenance facade and command entry points. |
| `xtask/src/mcp_spec/manifest.rs` | Strict pinned manifest model, parsing, and deterministic rendering. |
| `xtask/src/mcp_spec/validation.rs` | Offline metadata, immutable-pin, checksum, artifact, schema, ordering, and registry-parity validation. |
| `xtask/src/mcp_spec/report.rs` | Deterministic check and synchronization report types. |
| `xtask/src/mcp_spec/sync.rs` | The sole networked MCP specification path, using a verified temporary candidate before replacement. |
| `xtask/tests/docs_check.rs` | Shared neutral fixture construction and current documentation-check test composition. |
| `xtask/tests/docs_check/*.rs` | Focused current-schema, link, structure, contract-identifier, terminology, artifact, CLI, and architecture tests grouped with their owning validators. |
| `xtask/tests/mcp_spec.rs` | Strict manifest parsing, classification, parity failures, immutable-pin, checksum, required-artifact, ordering, reporting, and offline-success coverage. |

Update this map when a durable responsibility moves. Do not list removed,
generated, or private scratch paths.
