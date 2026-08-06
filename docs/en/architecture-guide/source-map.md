# Source Map

The Store-derived current effective shaping authority graph, separate
immutable history, exact stale retirement or reauthorization, explicit
checkpoint succession and application carry-forward, decision-owner
application, advisor finalization, and Task-wide shaping progression are implemented in
`volicord-core::methods::{record_shaping,update_scope,advance_task}`,
`volicord-core::workflow_projection`,
`volicord-store::core_pipeline::shaping`, the canonical project schema, shared
public types, and MCP wire/adapter/registry modules.

This map routes maintainers to current implementation owners. It is not a
product contract; use the focused Reference document for exact behavior.

## Shared Types

| Path | Responsibility |
|---|---|
| `crates/volicord-types/src/lib.rs` | Public owner-module routing. Shared definitions are public through their owning modules. |
| `crates/volicord-types/src/schema.rs` | Shared request, response, and stored-record shapes. |
| `crates/volicord-types/src/methods.rs` | Public method request and result schemas, method-operation mappings, and exact typed accessors for method-owned `ChangeUnitUpdate` object members. |
| `crates/volicord-types/src/product_path.rs` | Platform-neutral Product Repository relative-path value, lexical validation, pure component-aware containment relationships, and immutable `WriteTicketPathScope` uniqueness and disjointness; no filesystem observation. |
| `crates/volicord-types/src/values.rs` | Closed product value sets. |
| `crates/volicord-types/src/managed_guidance.rs` | Closed semantic facts and distinct canonical digests for generated managed-host guidance, the Core workflow contract, MCP action forms, and the MCP semantic schema. The facts include current transition and variant admission, exact method-and-variant forms and fixed authority arguments, type-owned schema semantics, Store-derived current authority, non-actionable immutable history, exact stale retirement or fresh-identity reauthorization, implementation-preserving rejection, current checkpoint/UserAction preservation, explicit compatible-application carry-forward, decision ownership, User Channel resolution, tagged rejection/recovery surfacing, and close-review boundaries. All four digests participate in the project integration revision. |
| `crates/volicord-types/src/ids.rs` | Opaque identifiers. |
| `crates/volicord-types/src/canonical.rs` | Canonical serialization and hashing. |
| `crates/volicord-types/src/canonical_scalar.rs` | The exact type-owned `BaselineRef` byte grammar, cross-layer predicate and invalid-corpus generation, and its canonical scalar-contract digest. |
| `crates/volicord-types/src/diagnostics.rs` | Lifecycle-specific occurrence/current finding types, opaque `DiagnosticSubjectIdentity`, `CurrentDiagnosticKey` canonical identity and fixed digest ID derivation, lifecycle-aware `StoredDiagnosticFinding` and `StoredDiagnosticGraph`, separate `DiagnosticLookupReport`, shared read-only `DiagnosticFinding` and selected-Connection `DiagnosticReport` types, stable namespaced-code validation, bounded redacting projection of typed owner facts, cause-graph validation, and unexpected-failure fallback. |
| `crates/volicord-types/src/platform.rs` | Shared platform-environment and platform-path types. |
| `crates/volicord-types/src/host_configuration.rs` | Shared connection-intent and host-scope configuration types. |
| `crates/volicord-types/src/connection_verification.rs` | Canonical `ConnectionStatus`, `IntegrationActivationState`, `HookActivationState`, checks, single hierarchical `IntegrationActivationPlan`, stable actor/channel/step metadata, topological validation, nested agent sequence, session-role evidence, and verification-report types. |
| `crates/volicord-types/src/integration_revision.rs` | Typed Connection/project integration revision bases and derivation, including distinct Store-manifest, managed-guidance, workflow-contract, action-form-contract, MCP-semantic-schema, and scalar-contract digest inputs. |
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
| `crates/volicord-platform-fs/src/repository_observation/mod.rs` | Public repository-observer module routing and facade exports. |
| `crates/volicord-platform-fs/src/repository_observation/model.rs` | Closed repository baseline/outcome snapshot, path-state, transition, delta, observation-unavailable reason, invocation-path, canonical serialization, and semantic observer-contract digest types. |
| `crates/volicord-platform-fs/src/repository_observation/coordinates.rs` | Canonical repository and Git-layout identity, HEAD/tree and status coordinates, and exact dirty/untracked status-path parsing. |
| `crates/volicord-platform-fs/src/repository_observation/path_state.rs` | Contained worktree and immutable-tree path-state observation, streamed content hashing, executable and symbolic-link identity, clean Gitlink observation, and aggregate hash accounting. |
| `crates/volicord-platform-fs/src/repository_observation/snapshot.rs` | Complete status/invocation candidate union, bounded stable double observation, surrounding coordinate rechecks, and repository baseline/outcome snapshot construction. |
| `crates/volicord-platform-fs/src/repository_observation/delta.rs` | Repository baseline/outcome observed-path, status-path, invocation-path, and changed-tree candidate union plus deterministic net path-transition calculation. |
| `crates/volicord-platform-fs/src/repository_observation/bounded.rs` | Typed observer limits and bounded Git process input, output, duration, termination, capture, and streamed-blob handling. |
| `crates/volicord-platform-fs/src/repository_observation/tests.rs` | Disposable-Git-repository coverage for net worktree and tree transitions, unchanged pre-existing states, Gitlink and platform path states, instability, containment, resource bounds, and canonical digest determinism. |
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
| `crates/volicord-store/src/workflow_records.rs` | Private workflow-policy row projection, strict schema, closed-value, canonical-byte, fingerprint, source, and timestamp decoding into typed policy records; typed mutation input, application, and effects. Policy mutation evaluates Write Ticket compatibility only through the focused typed authority view supplied by the Write Ticket aggregate. |
| `crates/volicord-store/src/core_pipeline/mod.rs` | Public Core Store type routing, commit and mutation inputs, and transaction-level Store tests. |
| `crates/volicord-store/src/core_pipeline/facade.rs` | `CoreProjectStore` connection and project identity, retained mutation authority, facade accessors, and shared read-snapshot primitive. |
| `crates/volicord-store/src/core_pipeline/open.rs` | Explicit read-only opening and mutation opening that retains the context's typed canonical Runtime Home identity. |
| `crates/volicord-store/src/core_pipeline/project_state.rs` | Project-state column projection, row decoding, timestamp validation, and facade reads. |
| `crates/volicord-store/src/core_pipeline/enforcement_profile.rs` | Project enforcement-profile projection, strict JSON decoding, validation, and facade read. |
| `crates/volicord-store/src/core_pipeline/clock.rs` | Store-handle clock samples, project UTC-floor reads, and transactional floor advancement. |
| `crates/volicord-store/src/core_pipeline/tasks.rs` | Task and acceptance mutation inputs, storage validation and SQL application; Task, acceptance-criterion, evidence-claim, and Task-revision projections; facade reads and focused tests. |
| `crates/volicord-store/src/core_pipeline/change_units.rs` | Change Unit mutation inputs, storage validation and SQL application; projections, strict row and JSON decoding, facade reads, and focused tests. |
| `crates/volicord-store/src/core_pipeline/shaping.rs` | Store-derived current effective shaping authority graph, separate immutable application and reauthorization history reads, atomic shaping-checkpoint/gap/link/UserAction and exact stale-authority mutations, and persisted-invariant validation. |
| `crates/volicord-store/src/core_pipeline/write_tickets.rs` | Sole owner of the physical Write Ticket table and columns; fully typed insertion serialization from one `WriteTicketPathScope`; one private row projection and canonical normal/transaction decoder that reconstructs the invariant-bearing scope on the physical-to-opaque-`StoredWriteTicket` path; private fields exposed through semantic accessors; closed-value, structured-field, and typed cross-field persisted-invariant validation; focused authority views, all-candidate facade reads, deterministic diagnostic ordering that grants no semantic authority, and aggregate tests. |
| `crates/volicord-store/src/core_pipeline/runs.rs` | Run mutation inputs, storage validation and SQL application; Run and observed-change projections, strict decoding, facade reads, and focused tests. |
| `crates/volicord-store/src/core_pipeline/evidence.rs` | Evidence mutation inputs, storage validation and SQL application; evidence-summary and observation projections, strict row decoding, record-reference projection, facade reads, and focused tests. |
| `crates/volicord-store/src/core_pipeline/artifacts.rs` | Artifact mutation inputs, storage validation and SQL application; staging and durable-artifact projections, strict decoding, link reads, persistent-body verification, facade reads, and focused tests. |
| `crates/volicord-store/src/core_pipeline/user_actions.rs` | User-action mutation inputs, storage validation and SQL application; strict decoding from physical JSON and stored scalars into opaque `StoredUserActionRequest`, `StoredUserActionResolution`, and paired `StoredUserActionRecordSet` values; effective-status derivation, facade reads, and focused consistency tests. |
| `crates/volicord-store/src/core_pipeline/continuity.rs` | Continuity mutation inputs, storage validation and SQL application; project-continuity projection, bounded snapshot pages, facade reads, and focused tests. |
| `crates/volicord-store/src/core_pipeline/replay.rs` | Private tool-invocation rows, SQL, strict typed invocation-identity and replay-context decoding, immutable operation-result projection, exact method-response bytes for Core-owned semantic replay, and facade reads. |
| `crates/volicord-store/src/core_pipeline/reconciliation.rs` | Strictly decoded typed expected-write and Unrecorded Change product-write metric candidate projections, including Product Repository paths, plus current-handle unresolved-change reads used by close-readiness fact acquisition. |
| `crates/volicord-store/src/core_pipeline/blockers.rs` | Active blocker-reference query and facade read. |
| `crates/volicord-store/src/core_pipeline/events.rs` | Project authority-event identity lookup. |
| `crates/volicord-store/src/core_pipeline/agent_sessions.rs` | Project-local Agent Session facade entry point over the Guard-owned strict row reader. |
| `crates/volicord-store/src/core_pipeline/record_refs.rs` | Shared stored-record reference representation used by aggregate reads. |
| `crates/volicord-store/src/core_pipeline/inspection.rs` | No-effect project storage counters used by verification paths. |
| `crates/volicord-store/src/core_pipeline/mutations.rs` | Grouped `CoreStorageMutation` routing, static aggregate dispatch, transaction-scoped mutation context, and typed aggregate application results. |
| `crates/volicord-store/src/core_pipeline/commit.rs` | Replay and freshness gates, ordered aggregate delegation, one state-version advance and canonical commit timestamp, atomic event/replay/response persistence, rollback, and final commit outcome. |
| `crates/volicord-store/src/core_pipeline/validation.rs` | Persisted-value and mutation-input validation shared by current Store owners. |
| `crates/volicord-store/src/guards.rs` and `crates/volicord-store/src/guards/repository_observation.rs` | Typed exact host-correlation normalization, MCP-only project anchors, storage-manifest and current semantic-digest-bound project integration revision derivation, prompt captures, atomic invocation-scoped repository observations, exact expected-write matching, and unmatched-delta Unrecorded Change materialization. |
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
| `crates/volicord-user-action-service/src/service.rs` | Store-backed acquisition of typed construction, artifact, target, pending-authority, and resolved request facts, including exact machine action and outcome, without Core request orchestration. |
| `crates/volicord-user-action-service/src/materialization.rs` | Application of caller-supplied operation identity and construction of canonical public requests and immutable resolutions. |
| `crates/volicord-user-action-service/src/persistence.rs` | Exact typed mapping from canonical request or resolution values to Store mutation inputs. |
| `crates/volicord-user-action-service/src/authority.rs` | Normalized authority and public request projection from Store-validated typed records without physical persisted-row validation. |
| `crates/volicord-user-action-service/src/lifecycle.rs` | Pure projected Task lifecycle interpretation from current pending authority facts. |
| `crates/volicord-user-action-service/src/resolution.rs` | Current-basis validation, canonical typed resolution construction, and replay-input comparison. |
| `crates/volicord-user-action-service/src/continuity.rs` | Semantic derivation of continuity drafts for accepted authority-bearing resolutions. |
| `crates/volicord-user-action-service/src/projection.rs`, `summary.rs` | Adapter-neutral pending, resolution, instruction, and safe-summary facts, with semantic typed-fact mismatches reported as service invariants. |
| `crates/volicord-user-action-service/src/tests/` | Responsibility-local validation, body, identity, authority, lifecycle, materialization, persistence, resolution, continuity, and projection tests. |

## Core

| Path | Responsibility |
|---|---|
| `crates/volicord-core/src/pipeline.rs` | Separate read-only path and admitted-context `CoreService` construction, typed Core/Store Runtime Home authorization, common preflight, replay, plan selection, response, commit orchestration, Store-error detail projection, and neutral typed operational projection of platform-owned Product Repository observation failures. |
| `crates/volicord-core/src/operation_plan.rs` | Method-neutral committed-operation inputs: Task and optional Change Unit identity, ordered Store mutations, and event payload. It contains no public method result or response types. |
| `crates/volicord-core/src/method_execution.rs` | Method-generic request preparation, mutation policy selection, replay decoding, storage scalar conversion, and the generic method-plan error carrier. It owns no aggregate policy or source-specific error mapping. |
| `crates/volicord-core/src/method_rejection.rs` | Shared validation and rejection response construction plus method-neutral dry-run summaries. |
| `crates/volicord-core/src/error_boundary/` | Focused Store, UserAction, close-readiness, artifact-policy, and Product Repository path error mappings at the boundary between each source error model and public method planning. |
| `crates/volicord-core/src/product_path.rs` | Coordination of shared lexical parsing for caller-supplied paths with platform-owned live Product Repository observations. Store-returned paths arrive as typed values and do not pass through this module. |
| `crates/volicord-core/src/identity.rs` | Generic bounded durable-ID allocation and record-family collision checks over an injected `DurableIdGenerator`; callers do not expose `CoreService` for identity allocation. |
| `crates/volicord-core/src/artifact.rs` | Artifact verification, integrity and availability facts, and source-reference normalization over typed Store records. |
| `crates/volicord-core/src/continuity/` | Core continuity planning, projection, Store fact acquisition, durable identity allocation, and UserAction-derived continuity materialization. |
| `crates/volicord-core/src/acceptance_facts.rs`, `task_facts.rs`, `enforcement_facts.rs`, `evidence_facts.rs` | Focused typed Store reads and semantic consistency checks for acceptance criteria, Task blocker and close-basis facts, project enforcement, and evidence. They do not repeat physical row decoding or own view projection. |
| `crates/volicord-core/src/change_unit_planning.rs` | Change Unit record projection and Store mutation planning over exact schema-owned typed field accessors. |
| `crates/volicord-core/src/state_summary.rs`, `evidence_projection.rs`, `guarantee_projection.rs` | Store-independent typed-fact projection into public state summaries and evidence or guarantee display values. |
| `crates/volicord-core/src/guidance.rs` | Adapter-neutral typed blocker remediation, display-action normalization, primary display selection, and operation-category projection; it does not own workflow authority. |
| `crates/volicord-core/src/workflow_projection.rs` | Strict normalized `WorkflowSnapshot` construction from the current effective graph and the pure `WorkflowMachine` that solely selects workflow kind, next actor, typed blocker, required refs, separate close readiness, and the deterministic fixed-coordinate `WorkflowTransitionCatalog`. |
| `crates/volicord-core/src/methods/tests/workflow_state_space.rs` | Deterministic reachable-state exploration that checks catalog uniqueness, required-member integrity, transition-owned admission, nonterminal liveness, exact typed recovery membership, and Core/MCP action-form parity. |
| `crates/volicord-core/src/summary_text.rs` | Adapter-neutral public `SummaryCard` text projection without CLI syntax, transport framing, Markdown, or terminal rendering. |
| `crates/volicord-core/src/record_refs.rs`, `task_state.rs` | Focused state-record reference conversion and typed Task-state interpretation. |
| `crates/volicord-core/src/task_policy.rs` | Reusable Task policy, including typed lifecycle interpretation and lifecycle mutation planning. |
| `crates/volicord-core/src/write_ticket/` | Canonical Write Ticket ownership. `planning.rs` evaluates focused `PrepareWriteInput` facts, classifies every active reuse candidate separately from stale mutation facts, and returns common facts, mutation facts, and one closed `PrepareWriteTicketPlan` issue/reuse/no-ticket branch; it also materializes the issue branch into a validated `PlannedWriteTicket`, while `MaterializedPrepareWriteTicket` keeps issued/reused/none identities and effects closed. Planned and stored tickets expose one immutable `WriteTicketPathScope`; only no-ticket owns separate decision paths. `semantic.rs` supplies only the immutable meaning shared by planned and stored forms. `read_model.rs` acquires typed stored-ticket, Task, workflow-policy, observation-time, UserAction-resolution, and evidence facts; raw UserAction authority values go directly to `approval.rs` and are not retained in `WriteTicketCurrentFacts`. `approval.rs` privately constructs the current sensitive-approval set and returns a typed approval basis or `WriteTicketApprovalAssessment`. `current_validity.rs` pre-evaluates terminal stored states and evaluates active candidates from non-approval current facts plus that assessment into reusable or invalidated states while keeping stored identity mandatory. `selection.rs` owns distinct pure policies for historical/display stored-evaluation precedence and authority-bearing compatibility cardinality; the latter returns `None`, `One`, or invariant-bearing `Ambiguous` with sorted diagnostic identities. `summary.rs` owns separate planned and stored projections. `service.rs` partitions terminal and active records before current-fact loading, hands raw approval authorities only to `approval.rs`, selects a display stored evaluation, and loads evidence after that selection. `policy.rs` owns typed prepare-write and attempt-scope decisions. `admission.rs` combines `ReusableStoredWriteTicket` with the matching exact-attempt compatibility proof and returns `AdmissibleStoredWriteTicket` or an admission error. Public request metadata and response composition remain in the calling method. |
| `crates/volicord-core/src/methods/` | Public-method entry points and request-specific orchestration. `record_shaping.rs` exposes separate `record_shaping_checkpoint` and `finalize_advice` entry points and plan wrappers while sharing only their internal shaping validation helpers. The checkpoint plan owns explicit succession, exact stale retirement or fresh-request reauthorization, and live-decision preservation; the finalization plan owns advisor decision application and close-basis creation. `update_scope.rs` applies only exact scope-owned decisions and rejects implementation-phase authority invalidation before mutation; `advance_task.rs` validates Task-wide current authority and atomically applies advance-owned decisions with the explicit phase transition. Production modules import shared responsibilities from their explicit owners; `methods/mod.rs` provides only module wiring, while method-specific plan wrappers stay in their owning method modules. |
| `crates/volicord-core/src/recording/mod.rs`, `context.rs`, `model.rs` | The narrow semantic `RecordRunInput` and `RecordRunOperationPlan` boundary, input normalization, Store-aware typed fact acquisition, and the closed fact, authority, observation, artifact, and mutation-plan models shared only within the recording package. |
| `crates/volicord-core/src/recording/authority.rs` | Store-backed capture-intent and receipt resolution into typed capture authority, reusing evidence-fact, artifact, relevance, and UserAction owners without constructing public responses. |
| `crates/volicord-core/src/recording/evidence.rs`, `artifact.rs` | Record Run-specific coordination that turns shared evidence and artifact policy outputs into typed observation, producer, artifact-promotion, and link plans. |
| `crates/volicord-core/src/recording/plan.rs` | Record Run policy-service coordination, typed `RecordRunMutationPlan` assembly, and final `RecordRunEffect` plus `RecordRunResultFacts` projection; domain mutations remain distinct until final Store-plan conversion. |
| `crates/volicord-core/src/recording/state.rs` | Store-aware acquisition of the projected post-operation `StateSummary` semantic fact. |
| `crates/volicord-core/src/recording/tests/` | Responsibility-local Record Run context, capture-authority, evidence, artifact, typed mutation-plan, semantic error, and result-fact coverage. |
| `crates/volicord-core/src/close_readiness/mod.rs` | Narrow package surface for close-readiness services, projections, and blocker helpers consumed by method planners. |
| `crates/volicord-core/src/close_readiness/facts.rs` | Typed current-fact acquisition and projected-fact assembly, including one acceptance-criteria snapshot, one workflow-policy snapshot, and current-handle unresolved-change reads; owns no readiness decision. |
| `crates/volicord-core/src/close_readiness/change_control.rs` | Task, Change Unit, close-basis, baseline, recovery, unresolved-change, and Write Ticket condition evaluation. |
| `crates/volicord-core/src/close_readiness/evidence.rs` | Close evidence and artifact availability evaluation through the focused evidence fact and pure policy owners. |
| `crates/volicord-core/src/close_readiness/acceptance.rs` | Pending close authority, cancellation, sensitive approval, final acceptance, and residual-risk acceptance evaluation. |
| `crates/volicord-core/src/close_readiness/policy.rs` | Store-independent effective-control resolution and ordered pure combination of typed readiness evaluations into the close state. |
| `crates/volicord-core/src/close_readiness/blockers.rs` | Canonical typed close blocker construction, Write Ticket blocker projection, and cross-blocker action normalization. |
| `crates/volicord-core/src/close_readiness/guidance.rs` | Adapter-neutral semantic continuation selection with typed owner methods and operation categories; owns no CLI syntax, capture path, Markdown, rendering, or credentials. |
| `crates/volicord-core/src/close_readiness/summary.rs` | Full close-operation assessment and deliberate smaller method-neutral readiness projection. |
| `crates/volicord-core/src/close_readiness/service.rs` | Narrow coordination of fact acquisition, responsibility-owned evaluation, pure policy combination, full close assessment, and method-neutral summary projection. |
| `crates/volicord-core/src/close_readiness/recording.rs` | Record Run close-basis reference resolution, current sensitive-action basis construction, residual-risk validation, and typed `CurrentCloseBasis` planning through shared close-readiness and evidence policy. |
| `crates/volicord-core/src/close_readiness/tests/` | Responsibility-local fact, change-control, evidence, acceptance, policy, blocker, and guidance tests plus close-readiness service integration coverage. |
| `crates/volicord-core/src/methods/prepare_evidence_capture.rs` | Evidence-capture request validation and planning; consumes target policy for acceptance-criterion and supplemental-claim matching. |
| `crates/volicord-core/src/methods/prepare_write.rs` | Public Prepare Write request-to-semantic-input adaptation; typed planning, Store, and UserAction error mapping; dry-run projection from the closed planning branch; durable ticket-ID allocation for issue only; closed ticket materialization; branch-specific public ticket projection; Store insertion from the same issued plan; response assembly; and commit-plan submission. |
| `crates/volicord-core/src/methods/record_run.rs` | Public Record Run request-to-semantic-input adaptation, semantic error-to-response mapping, `RecordRunOperationPlan` conversion to neutral execution inputs and public result fields, Core plan submission, and method-specific metric recording. |
| `crates/volicord-core/src/methods/tests/record_run.rs` | Representative public Record Run orchestration, commit, rejection, artifact/evidence, rollback, and replay integration coverage; domain policy matrices stay under their owner paths. |
| `crates/volicord-core/src/methods/close_task.rs` | Request-specific close orchestration: request validation, close-readiness service invocation, terminal mutation planning, and typed result assembly. |
| `crates/volicord-core/src/methods/update_scope.rs` | Scope-update planning and projected evidence-summary completion through the close-readiness evidence policy owner. |
| `crates/volicord-core/src/methods/status.rs` | Read-only status projection, including consumption of shared close-readiness evidence policy through Core projection paths. |
| `crates/volicord-core/src/methods/user_action.rs` | Direct request and resolution method orchestration; consumes shared typed UserAction services and maps their results into method plans and responses. |
| `crates/volicord-core/src/methods/user_action_read.rs` | User Channel authorization, coherent Store snapshots, originating-result replay, and public method-result projection. |
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
| `crates/volicord-cli/src/version_command.rs` | Concise product-version output plus shared human-readable and typed JSON presentation of embedded build provenance. |
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
| `crates/volicord-cli/src/guard_integration/plan.rs` and `hosts/codex.rs` | Source templates for managed AGENTS and Codex rule guidance, including current workflow-catalog method-group and variant admission, exact method-and-variant forms and fixed authority arguments, type-owned MCP schemas, truthful pre-Core rejection reporting, current-authority/history separation, exact stale recovery, implementation-preserving rejection, rejection/recovery surfacing, and the nested integration-verification sequence with its stop and diagnostic boundaries. |
| `crates/volicord-cli/src/guard_command/` | Explicit `codex-command-hooks` event decoding, semantic Guard-probe filtering, and bounded source-specific observations without routed MCP payload retention. |
| `crates/volicord-cli/src/user_command.rs` | CLI inbox and local-user resolution, with pre-admission syntax and repository targeting followed by admitted Registry/project selection, neutral Core fact consumption, shared UserAction presentation, one-snapshot candidate planning, diagnostics, Core effect, and terminal response rendering under the same mutation context. |
| `crates/volicord-cli/src/doctor_command.rs` | Doctor diagnostic fact collection, pure report finalization, and compact, verbose, or JSON projection from one finalized report. |
| `crates/volicord-cli/src/doctor_command/remediation.rs` | Typed Doctor action candidates, deterministic remediation-plan merge and conflict validation, urgency and priority policy, provenance, and primary-action ordering. |

## MCP Protocol Profiles

| Path | Responsibility |
|---|---|
| `crates/volicord-mcp-protocol/src/lib.rs` | Closed typed MCP revision parsing, exact production profile lookup, the single revision-to-semantic-capability map, deterministic supported-revision iteration, tracked pre-release classification, and explicit unknown or unsupported rejection. |
| `crates/volicord-mcp-protocol/tests/protocol_registry.rs` | Pinned manifest parity, complete semantic/schema capability parity, registry uniqueness, deterministic iteration, exact parsing and selection, preferred-revision membership, and pre-release exclusion. |

## MCP Wire

| Path | Responsibility |
|---|---|
| `crates/volicord-mcp-wire/src/methods.rs` | Exact MCP argument, structured result, workflow action form, authoritative argument context, retry contract, operational error, compact mutation, UserAction projection, serialization, and generated request/result schema ownership. |
| `crates/volicord-mcp-wire/src/action_form.rs` | The canonical method-and-semantic-variant action-form projection descriptors, exact submitted-variant selectors, fixed and Agent-authored path ownership, and descriptor integrity checks. |
| `crates/volicord-mcp-wire/src/semantic_schema.rs` | Closed type-owned semantic schema nodes, generic required-nullable semantics, explicit discriminators, branch-local semantic validation metadata, bounded annotation-preserving runtime projection, deterministic JSON Schema and descriptor digests, canonical examples, and descriptor integrity checks. |
| `crates/volicord-mcp-wire/src/tool_contracts.rs` | The single `AgentToolId`-keyed MCP contract entries for input/output descriptors, descriptions, typed canonical examples, exact request decoding, and catalog integrity. |
| `crates/volicord-mcp-wire/src/tools.rs` | Exact capability-field names, tool annotations, and capability-selected tool-definition and tool-result envelopes. |
| `crates/volicord-mcp-wire/src/json_rpc.rs` | JSON syntax decoding, JSON-RPC envelope classification, request-ID and object-parameter validation, and success/error response construction without Core access. |
| `crates/volicord-mcp-wire/src/contracts.rs` | `mcp.wire` identifier derivation from canonical MCP schemas, profile field vocabularies, and envelopes. |
| `crates/volicord-mcp-wire/tests/wire_contract.rs` | Exact serialization and JSON-RPC round trips, generated MCP schema ownership, and neutral public-schema separation. |

## MCP Adapter

| Path | Responsibility |
|---|---|
| `crates/volicord-mcp/src/lib.rs` | Adapter-owned public entry points and canonical adapter result composition. Neutral method and tool identities remain at their `volicord-types` owner-module routes; MCP wire values are consumed directly from `volicord-mcp-wire`. |
| `crates/volicord-mcp/src/build_info.rs` | Embedded build-provenance facts, deterministic build-correlation identity, and pure typed provenance assessment shared by administrative adapters. |
| `crates/volicord-mcp/src/managed_launch.rs` | Canonical typed personal/shared hidden-launcher command and arguments, Runtime Home environment binding, strict launch-shape validation, public manual probe materialization, projection, and fingerprint inputs. |
| `crates/volicord-mcp/src/mutation_admission.rs` | Per-message and per-tool `SharedWriter` acquisition, Store context construction, typed setup-busy propagation, and bounded lease lifetime across complete MCP effects. |
| `crates/volicord-mcp/src/stdio.rs` | Public manual and in-memory lease-bound managed stdio facade. It selects the entry-path binding and delegates the connected stream without retaining protocol, lifecycle, or tool-dispatch implementations. |
| `crates/volicord-mcp/src/transport.rs` | Bounded newline-delimited stdio reads and writes, UTF-8 and frame-limit enforcement, transport-loop termination, and delegation of decoded JSON values to lifecycle handling. |
| `crates/volicord-mcp/src/json_rpc.rs` | Adapter-error and diagnostic mapping around wire-owned JSON-RPC envelope decoding and response construction. |
| `crates/volicord-mcp/src/lifecycle.rs` | Exact initialize profile selection, initialized-notification admission, capability-driven batch and per-method lifecycle validity, runtime-session start and close, and the closed `SessionState` variants `AwaitingInitialization`, `AwaitingInitializedNotification`, `InitializedAndReady`, and `Closed`. Initialization selection exists only in initialized variants, and termination data exists only in `Closed`. |
| `crates/volicord-mcp/src/binding.rs` | Runtime Home resolution, repository discovery, Connection/project preflight and binding, and managed Codex session/thread/turn correlation. |
| `crates/volicord-mcp/src/tool_dispatch.rs` | `tools/list` and `tools/call` parameter decoding, canonical tool selection, adapter/Core invocation, and shared canonical tool-result carrier assembly. It does not frame transport messages or own mutation, recovery, UserAction, or metric projection. |
| `crates/volicord-mcp/src/mutation_projection.rs` | Mutation detail selection, effect anchoring, compact method-result projection, fresh-authority composition, and capability-driven normal result-budget enforcement. |
| `crates/volicord-mcp/src/authority_refresh.rs` | Post-mutation Agent Session binding, current authority reread, coordinate validation, and extraction of the fresh compact authority receipt plus tagged workflow authority. |
| `crates/volicord-mcp/src/action_form.rs` | Pure descriptor-bound projection from neutral Agent `TransitionDescriptor` values to deterministic method-and-variant MCP form catalogs and variant-aware authority-only retry contracts; it does not recalculate transition availability or maintain a second method/variant selector. |
| `crates/volicord-mcp/src/committed_result_recovery.rs` | Capability-selected, authority-first bounded recovery after committed mutation projection, refresh, or post-effect failures without mutation retry. |
| `crates/volicord-mcp/src/user_action_projection.rs` | Committed UserAction coordinate extraction, neutral current-fact reread, adapter-owned safe MCP result construction, neutral failure mapping, and shared CLI inbox fallback attachment. |
| `crates/volicord-mcp/src/telemetry.rs` | Runtime-session finding and diagnostic-event persistence plus bounded best-effort handling for diagnostic-carrier failures where the contract permits it. |
| `crates/volicord-mcp/src/session_metrics.rs` | Diagnostic-session establishment and session-scoped tools-list, method-call, and status-reread workflow metrics. |
| `crates/volicord-mcp/src/diagnostics.rs` | Closed MCP diagnostic mapping, shared finding construction, and preservation of platform-owned diagnostic codes and action classes in bootstrap and persisted terminal projections. |
| `crates/volicord-mcp/src/adapter.rs` | Retained pre-operation routing identity, bounded read-only invalid-argument authority bootstrap, exact method, semantic-variant, and action-form-ref admission before mutation Core entry, live mutation-context correlation, context-bound Core invocation APIs, plus managed in-chat begin/probe/get integration-verification orchestration outside Core. |
| `crates/volicord-mcp/src/constants.rs` | MCP initialize instructions for the user-level verification request and current managed workflow: current method-group and variant catalog admission, exact method-and-variant forms and fixed authority arguments, type-owned schemas and retry contracts, truthful pre-Core rejection reporting, Store-derived current authority, immutable-history exclusion, exact stale recovery, fresh UserAction identity, explicit shaping and advance, implementation-preserving rejection, rejection/recovery surfacing, close review, stop rules, unavailable boundary, and optional active diagnostics. |
| `crates/volicord-mcp/src/tool_registry.rs` | Descriptor-consuming assembly of `AgentToolId` definitions, protocol-profile schema compaction, annotations, visibility, and the explicit-server collision-checked Codex callable catalog. |
| `crates/volicord-mcp/src/schema_validation.rs` | Known-tool argument and output validation through wire-owned semantic validator trees and metadata. |
| `crates/volicord-mcp/src/routing.rs` | Bound Product Repository discovery, current Connection/project routing, and preflight diagnostic projection of server/raw/callable identities from the canonical catalog. |

## Tests

| Path | Responsibility |
|---|---|
| `crates/*/tests/` and module-local `tests` | Crate boundary and unit tests. |
| `crates/volicord-command-model/src/lib.rs` module tests | Clap structural assertions, complete public traversal, hidden-subtree exclusion, canonical-invocation self-parsing, typed inbox-resolution invocation round trips, and current required-argument, conflict, and value-set behavior. |
| `crates/volicord-mcp/src/transport.rs` and `binding.rs` module tests, plus `crates/volicord-mcp-wire/tests/wire_contract.rs` | Frame limits and draining, delimiter and UTF-8 behavior, wire-owned request identifiers and notification classification, exact managed-call metadata, and Runtime Home binding failures at their implementation owners. |
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
| `tests/conformance/` | Cross-method scenarios and direct conformance consumption of the canonical MCP descriptor schemas and typed examples. |
| `tests/conformance/mcp-spec/` | Versioned official MCP schemas, release and handshake-family metadata, reviewed `production_supported` and `pre_release_only` facts, immutable upstream pins, license attribution, and checksums used as offline conformance inputs. |
| `tests/release-integrity/` | Generic five-target, version, canonical-byte, package, checksum, source-bundle command, and semantic CI/release-workflow integrity tests, including build/smoke/staging order, matrix binary inputs, exactly-once action use, path filters, and dependency direction. |
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
| `xtask/Cargo.toml` | Lightweight maintenance dependency boundary. `volicord-command-model` supplies public command grammar, `volicord-types` supplies neutral public contract identifiers, `volicord-mcp-protocol` supplies production profiles, and `volicord-mcp-wire` supplies wire contract descriptors, without pulling in the MCP runtime adapter, Core, Store, CLI, platform, or test-process crates. |
| `xtask/src/lib.rs` | Thin repository-check orchestration and public report re-exports. |
| `xtask/src/diagnostics.rs` | Shared path, category, optional line, and message representation for validation issues. |
| `xtask/src/doc_index.rs` | Current documentation-index schema, applicability and exact semantic-contract routing, owner routing, indexed paths, and maintained-document coverage. |
| `xtask/src/markdown.rs` | Shared Markdown event parsing, heading meaning units, and supported contract-literal constructs. |
| `xtask/src/links.rs` | Local Markdown target resolution, links, fragments, and anchors. |
| `xtask/src/parity.rs` | English/Korean heading-structure parity. |
| `xtask/src/terminology.rs` | Terminology-map paths and identity-sensitive role validation. |
| `xtask/src/cli_docs.rs` | `docs-sync` composition, generated Administrative CLI regions, and documented invocation validation through `volicord-command-model`; shell tokenization is not a second command grammar. |
| `xtask/src/contract_docs.rs` | Generated neutral API contract regions and the MCP semantic-descriptor catalog consumed directly from `volicord-mcp-wire`. |
| `xtask/src/document_structure.rs` | Current architecture-design section and surface-stability structure. |
| `xtask/src/contract_identifiers.rs` | Current neutral public-schema, command-model, typed-diagnostic, semantic protocol-profile, and MCP wire-owner identifier derivation; paired meaning-unit validation; and operation-category table parity. |
| `xtask/src/workspace_manifests.rs` | Shared workspace-manifest parsing and current package and Rust applicability values. |
| `xtask/src/architecture.rs` | Cargo-metadata-derived package manifests, target source roots, dependency edges, package-level architecture validation, bilingual generated responsibility and dependency regions, generated-region drift checks, and informational maintainability reporting. |
| `xtask/src/release_metadata.rs` | Workspace release-version inheritance and release-tag validation. |
| `xtask/src/source_bundle.rs` | Canonical Git-commit and tree resolution, tracked-state checks for default `HEAD`, deterministic ZIP generation from Git tree and blob metadata, canonical path and Unix mode encoding, and complete archive-to-tree validation. |
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
| `xtask/tests/source_bundle.rs` | Disposable-Git-repository and complete-current-tree coverage for untracked exclusion, regular, executable, and symlink modes, blob content, tracked-change rejection, deterministic bytes, command execution, extraction, and validation. |

Update this map when a durable responsibility moves. Do not list removed,
generated, or private scratch paths.
