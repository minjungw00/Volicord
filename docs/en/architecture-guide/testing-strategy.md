# Testing Strategy

Tests protect owner-defined current behavior. They do not create product
contracts, preserve removed surfaces, or justify broader support claims.

## Choose The Narrowest Layer

| Test layer | Use it for |
|---|---|
| Unit test | Pure parsing, canonical encoding, closed values, and policy decisions. |
| Crate integration test | Adapter boundaries, Store reads/writes, process behavior, and strict persisted-record rejection. |
| Conformance test | Public cross-method outcomes, error categories, replay, effects, and projections. |
| Release-integrity test | Volicord target, version, package, checksum, committed-tree source bundle, workflow, and actual-binary smoke invariants. |
| Architecture check | Workspace package declarations, dependency kinds and directions, production/test-support separation, and Core dependency-layer eligibility. |
| Documentation check | Owner routing, links, terminology, parity, examples, and generated-source drift. |

Within `volicord-store`, aggregate-local unit tests stay beside the mutation
inputs, storage validation and application, read projection, and strict
decoder they protect. Transaction, replay ordering, rollback, durability, and
cross-aggregate storage-effect tests stay at the
`CoreProjectStore` commit boundary. Assertions prefer typed results and
observable storage effects; complete SQL text is asserted only where a
canonical SQL owner makes those bytes part of the current contract.

Store tests own physical persistence, transactions, and strict row decoding for
every public Store-to-Core or Store-to-service record boundary. They inject
malformed physical enums, JSON, timestamps, Product Repository paths, missing
values, and contradictory duplicate columns, then assert Store-owned
persisted-data corruption. Core and service tests consume valid
Store-constructed typed records and cover semantic policy and invariant
failures; they do not repeat physical decoders or construct Store corruption.
This split applies to workflow policy, Write Ticket, replay identity,
reconciliation observations, UserAction, and the other Core-facing record
families. Write Ticket boundary tests compile successfully when using
`StoredWriteTicket` accessors and fail to compile when external code attempts
a struct literal, private-field access, or destructuring. They assert the
compile outcome rather than compiler-message text.

Within `volicord-core`, reusable semantic-owner tests stay beside
`identity.rs`, `artifact.rs`, the focused fact, projection, guidance, summary
text, Change Unit planning, and Task policy modules, `continuity/`,
`write_ticket/`, `close_readiness/`, and the focused `error_boundary/` module
they protect. Pure projection tests use typed facts and no Store handle.
Write Ticket read-model tests cover typed ticket, Task, workflow-policy,
UserAction-resolution, and evidence acquisition plus Store-error propagation
without asserting policy. Their shared current-facts fixtures contain only the
non-approval Task and workflow-control inputs used by active
current-validity evaluation. Current-validity tests receive those facts plus a
typed `WriteTicketApprovalAssessment` and cover active, invalidated, consumed,
revoked, and effective-expiry transitions; they also prove that terminal
records complete before current-fact loading and every evaluated stored state
retains a mandatory ticket ID. Historical/display selection tests
accept only `StoredWriteTicketEvaluation` values and own stored-state
precedence and tie-breaking. Prepare Write compatibility-selection tests
consume complete classified candidate sets and distinguish no active
candidates, active but incompatible candidates, exactly one compatible ticket,
two or more compatible tickets, and mixed compatible/incompatible sets. They
verify deterministic ambiguity identity ordering without treating that order
as authority. Approval-owner tests pass raw UserAction authority facts only to
the canonical approval owner and prove that its private current set yields a
typed issuance basis or typed persisted-basis assessment. These policy unit
tests cover requirement construction, current sensitive-approval construction,
Store-valid persisted-basis assessment, typed semantic change reasons, multiple
approval references, and current and stale full resolution identities. They do
not invoke or claim coverage of a consumer path.

The Store-backed cross-consumer wiring conformance suite lives in
`crates/volicord-core/src/write_ticket/tests/approval_consumer_conformance.rs`
and is attached to the crate-private method integration harness. Its shared
scenario table contains only source facts for current approval, approval not
required, newly required approval, a stale resolution, changed approval scope,
ticket expiry, consumption, revocation, exactly one compatible reusable
ticket, and multiple compatible reusable tickets. Each scenario persists valid
Task, Change Unit, UserAction request and resolution, and Write Ticket records
as applicable. It invokes the actual `CoreService::status`,
`CoreService::check_close`, `CoreService::prepare_write`, and
`CoreService::record_run` paths and asserts their projected status, Store
effects, admission result, and Write Ticket blocker identities. Record Run uses
a freshly materialized copy of the same source facts so a preceding Prepare
Write invalidation cannot replace its own admission evaluation. Fixture helpers
do not compare approval references, construct resolution-ID sets, determine
currentness, select compatible tickets, or reproduce invalidation reasons.
With multiple compatible tickets, status projects one display-selected active
summary while the persisted candidate set remains inspectable, close readiness
projects one blocker for each current ticket, Prepare Write blocks without
reuse, and Record Run admits only the ticket explicitly named for consumption.

Summary tests separately map
`PlannedWriteTicket` and evaluated stored state without a Store fixture or
policy reevaluation. Focused
service tests verify terminal pre-evaluation, active-only current fact
acquisition, post-selection evidence loading, and representative persisted,
invalidated, approval-dependent, dry-run, and failure paths. Ordinary
compilation and function signatures are the primary proof that reuse requires
`ReusableStoredWriteTicket`, admission returns
`AdmissibleStoredWriteTicket`, and terminal states cannot enter those paths.
The production Write Ticket module lint rejects panic-based domain narrowing.
Mutation-planning tests assert typed plans and exact schema-owned field
accessors. The other owner tests assert typed facts, policy decisions, retry
behavior, or one precise boundary mapping. Public method orchestration,
response-family, replay, and committed-effect matrices remain under
`methods/tests/`. A method integration test does not become the sole coverage
for reusable owner logic. Write Ticket planning tests cover typed semantic
validation errors without response metadata, the closed issue/reuse/no-ticket
planning family, identity-free issuance drafts, invariant-bearing
`WriteTicketPathScope`, closed materialization, and derivation of a fully typed
`WriteTicketInsert` from the issued plan. They prove that reuse and no-ticket
cannot produce an insertion. They do not construct public responses or treat
dry-run intent as a semantic planning fact. Prepare Write method tests own
public error metadata, state-versioned reference projection, durable-ID
allocation, dry-run issue and reuse outcomes, and no-ticket results. They
compare the issued or reused nested ticket, top-level result facts, typed
planned or stored source, Store mutation input, and reloaded record for the
currently exposed path, validity, expiry, Task, Change Unit, and approval-basis
facts. No-ticket tests verify null ticket identity and reference fields and no
insertion. The approval-dependent method scenarios also prove that Prepare
Write selection receives the canonical typed assessment rather than raw
UserAction authority facts. Store-backed Prepare Write ambiguity coverage
persists multiple compatible active tickets, invokes the actual method path,
verifies the method-owned blocked outcome and sorted candidate refs, and
confirms no candidate was reused, consumed, invalidated, or selected. Replay
retains exact response coverage.

Record Run follows this split by source responsibility. Request and fact
acquisition, capture authority, evidence observation and reuse, artifact
verification and promotion, typed mutation planning, semantic error variants,
and `RecordRunResultFacts` projection live under
`crates/volicord-core/src/recording/tests/`. Close-basis and residual-risk
coverage lives in `close_readiness/tests/recording.rs`; ticket compatibility,
admission, consumption, and no-effect rejection coverage lives in
`write_ticket/tests/record_run_admission.rs`. The focused typed mutation plan is
exercised through these owner scenarios and the Store commit boundary. These
tests enter admission with a typed reusable ticket and matching exact-attempt
compatibility proof, then carry only an admissible ticket into mutation
planning and consumption. The committed product-write scenario also proves
that the Run row, Run ticket-effect payload, and consumed ticket use the same
admitted ticket identity. Approval-dependent admission scenarios pass the
locally acquired raw authority directly to the canonical approval owner and
exercise current-validity only with the returned typed assessment. The
small `methods/tests/record_run.rs` suite retains representative request
orchestration, conversion to the neutral execution carrier and public result
fields, semantic-error routing with dry-run and state-version metadata,
committed/no-effect alternatives, evidence and artifact paths, ticket and
stale-state rejection, rollback propagation, and replay consistency. Neutral
`OperationPlan` tests assert its method-independent execution inputs. Complete
domain policy matrices do not live in the public method suite.

Close-readiness Write Ticket tests accept only
`StoredWriteTicketEvaluation` values and verify blocker projection from active
and terminal evaluated states. They do not construct raw UserAction authority
facts or repeat approval-policy evaluation.

Within `volicord-user-action-service`, unit tests own semantic validation,
canonical body and identity construction, authority, lifecycle,
materialization, persistence mapping, resolution, continuity, and neutral
projection behavior. Core tests own request orchestration, generated
identifiers and timestamps, replay, transaction sequencing, and service-error
mapping. UserAction duplicated representations, missing physical values, and
request-resolution identity or action-kind disagreement remain Store tests.

Product Repository path tests follow the same ownership split.
`volicord-types` tests lexical values and pure relationships without temporary
directories. `volicord-platform-fs` tests live existing and missing paths,
nearest existing ancestors, inaccessible paths, and link escape with real
disposable directories. Core tests consume typed platform results and verify
neutral operational routing without reimplementing platform observation.
UserAction service tests remain filesystem-free. Adapter tests verify the
stable projected operation and resource identities.

`volicord-mcp-wire` tests own exact MCP serialization, JSON-RPC envelopes,
semantic descriptors, discriminator-first nested-union selection,
required-nullable behavior, branch-local issue context, deterministic issue
ordering and bounds, typed canonical examples, deterministic input/output
schemas, and descriptor integrity. They prove that an invalid or missing
discriminator cannot expose fields from an unselected branch and that
same-named sibling fields cannot exchange metadata. `volicord-mcp` tests
consume the same validator trees in registry, response-level selected variants
and canonical examples, compact argument errors, exact decode parity, output,
and bounded discovery projections. A descriptor-valid value rejected by the
exact request decoder is an internal diagnostic failure before Core, not a
user-field issue.
`volicord-types` tests own only neutral public schemas. Cross-owner coverage
asserts that public method schemas contain no MCP-only structures and that the
MCP adapter maps neutral Core operational failures to the current wire error.
The conformance package consumes the same descriptors and examples directly;
it does not copy JSON fixtures or schema metadata. Its MCP semantic case
requires every canonical value to validate and decode exactly and mutates each
declared discriminator to prove branch-local rejection without branch guessing.

Use disposable Runtime Homes and Product Repositories. Keep fixtures minimal and
typed. A fixture proves parser or implementation behavior only; it does not
prove behavior of a real Codex installation or platform support.

`volicord-test-process` owns bounded child execution shared by repository tests
and smoke harnesses. It composes the process-group, Windows Job Object, and
nonblocking pipe primitives owned by `volicord-platform-process`; it does not
duplicate those OS implementations. Product MCP supervision policy, protocol
framing, lifecycle progress, and diagnostics remain owned by `volicord-cli`.

## Changed-Owner Routing

`cargo run -p xtask -- owner-route --changed` derives repository maintenance
scope from Git changed paths. An explicit `--base <revision>` includes committed
series changes and the current working tree. Package membership comes from
Cargo metadata, maintained document and language-pair identity comes from
`docs/doc-index.yaml`, and only the associations absent from those owners live
in the validated `docs/owner-routing.yaml` catalog.

Routing tests use disposable Git repositories. They cover Rust packages,
paired documents, repository guidance, workflow files, unknown paths, dirty
working trees, explicit base revisions, stable ordering, human/JSON parity, and
the read-only worktree boundary. Tests do not carry a second workspace package
inventory or discover owner routes by scanning prose.

`cargo run -p xtask -- validate focused --base <revision>` turns that route
into an intermediate command plan. It selects only changed workspace packages,
direct documentation, architecture, MCP specification, release/workflow, and
hygiene checks. It never schedules the exact workspace aggregate.

`cargo run -p xtask -- validate final --base <revision>` schedules the complete
repository policy once all series commits are present. Every child process
writes stdout and stderr directly to files under
`target/volicord-validation/<run-id>/` while it runs. The runner checkpoints a
machine-readable summary and per-command result with the exact invocation,
timestamps, and exit code so completed results survive loss of a terminal
handle.

Validation-runner tests use injected command outcomes rather than invoking a
second validation engine. They cover focused planning, changed-package
selection, documentation routing, durable logs and recovery, exit-code
preservation, skipped commands, human/JSON category parity, aggregate retry
limits, unchanged-package decomposition, changed-package failure, and truthful
overall summaries.

## Workspace Architecture Validation

`cargo run -p xtask -- architecture-check` compares Cargo's current workspace
packages and internal normal, development, and build dependency edges with the
package entries under `workspace.metadata.architecture.packages` in the root
`Cargo.toml`. The check rejects any workspace package missing from that owner,
any owner entry missing from Cargo, unresolved allowlist targets, and every
edge outside the source package's kind-specific allowlist. It independently
rejects normal or build production dependencies on test-support packages,
Core-facing dependencies on adapter or presentation packages, the required
UserAction service, Core, shared-types, and Store boundary violations, and
cycles in the normal/build dependency graph. A semantic wire-family rule also
rejects a dependency on a `*-wire` owner unless the source is its matching
adapter or validation tooling or tests.

Focused validator tests use neutral synthetic package and group names to cover
valid current metadata, kind-specific disallowed edges, unregistered packages,
invalid dependency kinds, production/test-support separation, Core/adapter
independence, matching-adapter wire access, unrelated adapter and foundational
wire rejection, and cycles. A separate test runs the same validator against
the current Cargo workspace and its owner. Tests do not carry a second copy of
the workspace package graph. Architecture rules apply directly to the current
graph and are not selected through package, schema, or protocol versions.

Current-workspace coverage also asserts that
`volicord-user-action-service` has its dedicated responsibility entry and
admits only `volicord-types` and `volicord-store` as normal dependencies and
`volicord-test-support` as a development dependency. Any dependency on Core,
CLI, MCP, or presentation fails the architecture gate.

Core's normal allowlist admits `volicord-platform-fs` for typed,
invocation-scoped Repository Observation. The shared-types and
UserAction-service groups do not admit that dependency, so live filesystem
observation cannot move into their semantic values or validation.

Core tests construct host-neutral requests with typed local-user or validated
Agent Connection authority. CLI, MCP, application, and host-contract owners
test their command syntax, installation and launch configuration, host-specific
value validation, paths, and rendering. Adapter tests compare equivalent
adapter and direct-Core operations at the typed domain-result boundary.
Architecture enforcement inspects the Cargo package graph; behavioral tests
exercise public typed boundaries and owner output.

## Pinned MCP Specification Inputs

`tests/conformance/mcp-spec/` owns the minimal versioned upstream schemas and
license attribution needed for deterministic MCP conformance work. Its manifest
keeps finalized initialization-based revisions separate from pre-release-only
inputs, pins full upstream commits, records the handshake family and release
classification, checksums every local artifact, and records the reviewed
`production_supported` and `pre_release_only` facts. Production support
requires a released, non-pre-release entry with pinned artifacts and an exact
matching profile in `ProtocolRegistry`. A tracked pre-release entry remains
outside production support.

`cargo run -p xtask -- mcp-spec-check` is the offline integrity gate. It parses
the manifest, validates classifications and immutable references, and verifies
schema presence, schema family, attribution, checksums, and exact set parity
between released manifest entries marked `production_supported=true` and the
compiled production profiles without network access. Its report gives
deterministic counts for all pinned revisions, production-supported revisions,
and tracked pre-release revisions. `cargo run -p xtask -- mcp-spec-sync` is an
explicit maintenance action: it resolves the recorded releases to their pinned
commits, downloads into a temporary directory, preserves the reviewed support
metadata, validates the complete candidate, and only then replaces the fixture.
Ordinary builds and tests never invoke the networked sync path.

Executable wire conformance is an independent gate:
`cargo test -p volicord-mcp --test protocol_conformance`. This is the single
all-profile harness. Its generic runner iterates
`ProtocolRegistry::production().oldest_to_newest()` directly and applies the
same applicable scenarios to every supported profile. Assertions derive result
carrier form, structured content, `isError`, output schema, annotation, title,
`_meta`, initialize fields, client-capability shape, and committed-result
recovery from the profile's semantic capabilities. Separate registry and
projection tests require unique complete capability data, reject unsupported
identifiers without substitution, and prove that projection does not select
behavior by revision ordering. The manifest records reviewed upstream and
support facts; it does not record whether executable tests ran. The runner owns
no separate conformance revision array or per-revision coverage boolean; direct
registry iteration defines the matrix.

## Required Boundary Coverage

Durable tests should cover, as applicable:

- unknown members, duplicate keys, malformed closed values, and corrupt stored
  owner records;
- structural rejection before policy, replay, ticket invalidation, or mutation;
- exact public response-family coverage derived from the canonical method
  declaration, including decoder, schema, descriptor, Core branch, and adapter
  rejection of undeclared preview branches;
- read-only branches with no authority event or state-version advance;
- one atomic successful mutation and exact replay behavior;
- current-contract mismatch routed through the owned corrupt-data failure;
- Runtime Home `Absent`, `Ready`, `Incompatible`, and `Corrupt` inspection;
  same-parent staged creation with singleton and installation metadata; exact
  manifest, opaque publication provenance, and relation facts; cleanup at each
  pre-publication failure; per-canonical-home shared/exclusive mutation
  admission with concurrent shared writers and exclusive conflict on the same
  lock region; lexical and symlink aliasing and distinct-home independence;
  immediate and bounded acquisition; persistent coordination-file
  non-ownership; shared and exclusive process-termination release; native Unix
  and Windows OS-lock behavior; borrowed permit target/mode binding; no-replace
  publication with exactly one owner; token-backed
  rollback revalidation, ownership-loss and managed-host-consumption
  preservation, recursive failure before effect, partial or unclassifiable
  removal, parent-sync failure after known removal, terminal retry behavior,
  composite confirmation error plus rollback facts, unrelated replacement
  safety; and unchanged bytes and
  timestamps for existing incompatible state;
- diagnostics first creation through a complete same-parent staging carrier;
  final-path absence until exact validation; deterministic concurrent
  `SharedWriter` publication with every distinct session retained; cleanup
  scoped to each invocation on losing and pre-publication faults; preservation
  of an externally created invalid final file; known publication retention
  across parent-synchronization failure; exact existing-invalid rejection;
  read-only staging ignorance; Unix final permissions; and native coverage of
  the platform publication primitive;
- missing or ineligible operation-result rows remaining
  `OPERATION_RESULT_UNAVAILABLE`;
- MCP rejection of hidden context and CLI-only UserAction resolution;
- every closed workflow-rejection code, typed current mode/phase and received
  request, allowed alternatives, authoritative workflow, retryability,
  exact recovery action key, unchanged effect counts/state version, pending User Channel
  presentation, exact command construction, and phase-transition no-write-
  ticket facts;
- agent-evaluation observation totals for workflow rejections and final-answer
  surfacing; any observed rejection omitted from the final answer makes the
  result incomplete even when the underlying task otherwise succeeded;
- authoritative MCP runtime-session source separation, milestone ordering,
  current revisions, project binding, and diagnostics non-authority;
- exact hidden-launcher configuration, current-entry drift rejection,
  deterministic cleanup of unused leases, atomic one-time lease consumption,
  replay/expiry/Connection/revision/fingerprint rejection, and proof that
  public stdio remains `manual_cli` regardless of process environment;
- exact revision-set parity between released `production_supported=true`
  manifest entries and production protocol profiles, with tracked pre-release
  generations excluded from production support;
- `AgentToolId` wire-name uniqueness and round-trip parsing, exact canonical
  registry identity, mode availability, and the compile-time
  `ManagedHostRoundTrip` binding used by the CLI, MCP runtime, and Store;
- for every production profile selected directly from `ProtocolRegistry`,
  standalone `initialize`, the initialized notification, `tools/list`,
  pinned-schema validation, required tools, the designated round-trip identity,
  revision-specific definition and result projection, profile-selected
  operation batching or rejection, invalid lifecycle behavior,
  initialization-batch rejection, and EOF/shutdown;
- exact supported-revision selection, explicit unsupported-identifier
  rejection, and capability-driven initialize, batching, `tools/list`, and
  `tools/call` wire projection;
- independently pinned Codex host fixtures that are not derived from the
  production protocol registry and do not substitute for revision conformance,
  with exact `CodexMcpTurnMetadata`, `CodexCommandHooks`, and
  `CodexMcpCallableNames` profile coverage; source-specific correlation;
  explicit server/raw/callable fixture parity; complete raw-name projection;
  normalization-collision and contradictory-role rejection; exact catalog
  reverse lookup; additive-field and bound checks; checksum parity; typed
  host-tool plus server-namespace/catalog-derived-exact routing; complete
  probe-target/workflow-control/unrelated-known role coverage; current
  Guard-probe pre/post fixture delivery; nonterminal begin/get/status
  self-observation; unknown same-server exact-ID claim handling; foreign-server
  exclusion; generated matcher/catalog parity; strict matcher-drift rejection;
  and CLI conformance evidence kept separate from actual `managed_host`
  observations;
- immutable MCP preflight evidence on readable read-only Registry and project
  databases, unchanged selected-database row counts and modification times,
  writeability always `not_checked`, active write evidence stored only under
  `last_active_verification`, unchanged preflight evidence after verification,
  active timestamp/source, disposable conformance state, concise/verbose/JSON
  parity, and strict rejection of a combined evidence shape;
- lifecycle-specific diagnostic construction and Store APIs, immutable
  occurrence insertion, complete-current-key digest and persisted-ID
  validation, current snapshot identity immutability, resolution and
  reactivation, active/reportable filtering, explicit report seeds and bounded
  lifecycle-aware exact cause chains, occurrence/active/resolved lookup
  projection, lookup-status process exits independent of severity, typed
  diagnostic codes and bounded/redacted facts, deterministic roots,
  dependency-driven `Blocked` checks, and equivalent human and JSON
  projections of their respective selected-Connection or exact-lookup report;
- Guard manifest exact-shape and owner binding, hash-free policy commands versus
  hash-bound runtime commands, wrapper/file drift, platform-independent script
  executable expectations, current-definition hook hashes, unchanged-manifest
  observation preservation, changed-definition invalidation, current-owned
  hook observations, older-event exclusion, distinct absence/malformed/unknown-
  callable/correlation-mismatch acquisition stages, non-probe same-server tool
  exclusion from verification, and bounded payload-free callable evidence;
- exact current Codex hook configuration, Git-root dispatch, every required
  phase wrapper, owner and managed-command bindings, policy hash, host output,
  hashes, and permissions as the positive basis for the typed Hook path-safety
  assessment; distinct verified, failed, not-recorded, not-checked, and
  not-applicable dimensions; exact failure reasons for policy, owner, and root-
  resolution violations; bounded deterministic evidence; failed and incomplete
  phase aggregation; and input-order-independent JSON;
- exact `HookActivationState` evidence precedence including unknown, setup
  review, current-definition observation, policy management, invocation bypass,
  and explicit disabled states, with no synthetic trusted state;
- `IntegrationActivationState` transitions through configured, reload, hook
  review/unknown, managed MCP observation, Guard verification, complete, and
  failed, with `project_trust` kept independent;
- selected-status/list human-label parity for complete, host reload, hook
  review/unknown, MCP observation, Guard verification, failed, hook disabled,
  policy-managed hook, and invocation-bypassed hook states, while both JSON
  projections retain stable underscore spellings;
- the single Connection count projection for all-passed, pending, blocked,
  failed, not-applicable, and mixed inputs; concise `Passed`, `Blocked`,
  `Pending`, `Failed` fields; matching list vocabulary and ordering; and the
  verbose not-applicable count;
- shared concise/verbose active-verification projection for no evidence, all
  passed, Registry failure, exact project-write failure IDs, initialize,
  designated-safe-tool and shutdown failures, and host-compatibility failure;
  five successful production revisions compactly ordered oldest to newest;
  failed rows expanded with complete lifecycle and diagnostic facts; compact
  successful host rows; exact persisted evidence time and humanized source;
  Store-writeability aggregation; strict malformed-evidence rejection;
  contradictory-evidence expansion; human/JSON fact parity with exhaustive JSON
  preservation; and internal IDs absent from concise output;
- separate ambient and correlated Guard checks: ambient passed with correlated
  failed, ambient pending with no attempt, correlated complete,
  repair-required never projected as pending, and an older proof retained
  without hiding a newer failed attempt;
- correlated Guard evidence time selected from typed persisted lifecycle facts
  for awaiting-probe, awaiting-observation, complete, repair-required, and no-run
  states; strict chronology and attempt/proof identity mismatch failure; stable
  evidence time and details across read-only status evaluations with changing
  report times and no Store writes; list evaluation-time separation; and
  verbose/JSON timestamp parity;
- latest active-verification evidence time, source, aggregate result, and Store
  writeability preserved across repeated read-only status evaluations whose
  report times change, with the same evidence time in concise, verbose, and
  JSON output and no Store or filesystem mutation;
- Guard report parity across concise, verbose, and JSON; top-level Guard
  runtime sessions and verification IDs; canonical deduplication of managed
  and Guard session roles; recoverable failed aggregation to
  `action_required`; and direct stable-code mapping for every typed repair
  reason and acquisition stage;
- the single `IntegrationActivationPlan`, fixed semantic step IDs, distinct
  initiator/executor, `codex_chat` request channel, completed checks,
  root-finding ordering, topological prerequisite order, and strict rejection
  of duplicates, cycles, unknown prerequisites, inconsistent metadata,
  top-level nested tools, and required diagnostic-only steps;
- init output count and ordering for reload, hook review, one user-level
  request, and status; one `Required next steps` block; accurate count and
  pluralization; current-status suffixes; typed repair-required plans; and
  optional active diagnostics kept separate;
- transactional init fault injection after Runtime Home preparation, Store
  recovery preparation, Runtime Home rename at parent-sync, publication
  read-back, and manifest-validation phases, every managed
  hook/rule/guidance replacement, before and after Codex configuration
  replacement, before integration-revision commit, and during rollback; exact
  fresh and existing-state restoration; deterministic competing full-init
  coverage for either first-acquirer order, success release, rollback-complete
  release, mutation-free typed busy and locked dry run, and a fresh retry after
  release; stale-plan abort when an external publisher creates the final path
  while the lease is held; external concurrent-byte preservation; every setup
  publication result and `planned`, `committed`,
  `preserved`, `rolled_back`, and `partially_rolled_back` report projection;
  typed JSON and human distinctions for synchronized removal,
  removed-but-unsynchronized, incomplete removal, policy preservation, and
  ownership loss; effect-aware Project Home cleanup;
  read-only dry-run parity; replay idempotence; and activation only after
  commit;
- generated AGENTS, Codex rule, and MCP server instructions preserving the
  canonical request, every tagged workflow kind, its canonical returned tool,
  the nested list/begin/probe/status order, the unavailable path, and the
  prohibition on shell sleep/poll loops, same-turn automatic restart, raw
  stdio, hand-authored `_meta`, and resource discovery as proof;
- immutable semantic-coordinate begin replay, terminal same-turn replay without
  a new ID, new-turn attempts, prompt ownership, first-write-wins probe
  acknowledgement, and duplicate-begin concurrency;
- the pinned current Codex semantic contract's synchronous one-read observation
  policy, no numeric-version branch, missing-event immediate
  repair without TTL waiting, distinct payload/callable/verification/session/
  turn/tool-use repair reasons, immutable complete and repair terminals, and
  retry-policy gating of genuinely new coordinates;
- generated guidance with the deterministic begin, one probe, one policy-owned
  status read, and stop sequence, including explicit absence of sleep,
  repeated polling, and automatic same-turn retry;
- all reachable tagged variants, begin/probe/get parity from one Store
  projection, rejection of contradictory state/tool combinations, and
  state-correct responses across every production MCP revision;
- `crates/volicord-cli/tests/operational_host_e2e.rs` covering the complete
  applied-setup, launch-lease, managed MCP milestone, same-turn Guard
  prompt/pre/post verification, complete begin replay, exact complete probe
  replay, activation-complete, and matching bounded get journey without
  non-managed source substitution;
- repeated Guard initialization with stable identities and preservation of
  unrelated repository content;
- invocation-scoped Guard repository observations with exact pre-tool
  baselines, post-tool outcomes, deterministic net deltas, exact host
  correlation, and `open`, `complete`, or `unavailable` states;
- complete-delta expected-write matching, pre-existing dirty-state attribution
  bounds, unmatched-delta Unrecorded Changes, and unavailable diagnostics kept
  separate from actual changes; and
- Codex configuration drift and behavior-probe failure reporting;
- reusable bounded test child execution across success, stdin delivery,
  nonzero exit, timeout, deterministic stdout/stderr truncation, simultaneous
  streams, descendant-held pipes, stdin-write failure cleanup, repeated
  cleanup, native Unix process groups, native Windows Job Objects, paths and
  arguments containing spaces, and explicit environment addition/removal.

## Runtime Home Mutation-Admission Regressions

Mutation-admission tests compose the behavior owned by
[Runtime Boundaries](../reference/runtime-boundaries.md), the focused Store
owners, and the applicable CLI, MCP, and Guard owners. This guide describes
coverage organization; it does not redefine their contracts.

The reusable child-process protocol runs both immediate and bounded acquisition
for the complete lock matrix:

| First process | Second process | Required observation |
|---|---|---|
| `SharedWriter` | `SharedWriter` | Both acquire admission. |
| `SharedWriter` | `ExclusiveSetup` | Setup is busy or exhausts its bounded wait. |
| `ExclusiveSetup` | `SharedWriter` | The writer is busy or exhausts its bounded wait. |
| `ExclusiveSetup` | `ExclusiveSetup` | The second setup is busy or exhausts its bounded wait. |

The same protocol proves OS-handle release after normal return, error return,
panic, and forced process termination for both modes. Native runners exercise
the platform OS locks; cross-compilation alone is reported separately and does
not count as lock execution. The maintained native matrix includes Linux,
Windows, and macOS. WSL2-sensitive Unix cases remove `WSL_DISTRO_NAME` when
validating the native-Linux branch.

Setup race coverage uses barriers, channels, acquisition signals, and setup
fault points rather than elapsed-time sleeps. The fresh-publication matrix
pauses after publication and at later Store, Product Repository, Codex
configuration, and rollback points. A real external writer must remain
mutation-free until setup reports or rolls back, and a post-release retry must
observe only the resulting current or absent state. The existing-home
checkpoint case pauses after a setup Store commit and before its checkpoint;
the external writer cannot enter that checkpoint, setup restores its own
snapshot, and the writer's accepted retry remains present afterward. Both
first-acquirer orders are covered where the race permits either process to win.

The representative writer-domain matrix combines actual project and Connection
commands, public Core commits, artifact staging, evidence capture, inbox
resolution, change reconciliation, policy application, managed launch and
runtime-session observations, `tools/list` and verification milestones,
integration-verification events, Guard hook ingestion, diagnostic persistence,
and operational findings. Each case checks the typed busy/no-effect projection
before the owner operation, exact unchanged rows, files, state versions,
timestamps, findings, events, and receipts as applicable, followed by a
successful retry after lease release. A generic dummy write cannot substitute
for an owner operation.

Focused inbox-resolution coverage holds `ExclusiveSetup` while the project
database is unavailable and proves that typed setup-busy is returned before
Registry lookup, project Store opening, candidate planning, or diagnostic
creation. After lease release, the same command retries successfully. Choice
and evidence-observation cases exercise canonical candidate validation from one
admitted snapshot, no-effect invalid selections, text and JSON projection,
exact immutable replay, concurrent Core revalidation, and best-effort
diagnostics. Native Windows execution of this case additionally proves that no
pre-admission SQLite handle can block setup replacement or rollback.

Alias coverage runs pending choice, evidence observation, immutable replay,
change reconciliation, and another admitted Core operation through lexical
Runtime Home aliases. Unix also runs a symlink alias. These cases assert one
Registry project, the same typed Store/Core identity and coherent UserAction
snapshot, one committed resolution, and diagnostic correlation in the admitted
home. Separate negative cases keep another Runtime Home, project, or
verification basis unauthorized. Native Windows runners execute the supported
alias cases; compile-only validation is reported separately.

MCP lifecycle tests begin setup before runtime-session creation, reject
mutating calls before Core effects, and keep observation-persisting reads
no-effect when admission is unavailable. An idle server must not retain
`SharedWriter`; admission is acquired per operation. Guard record-profile tests
preserve cooperative host continuation while proving that the rejected hook is
not counted as an observed phase and that a later hook records normally.
Owner-defined read-only commands, including Connection list and status, diagnostic
lookup, project list/current, authority export, and MCP preflight, remain
writer-lease-free and must preserve Runtime Home bytes, rows, state versions,
and modification times.

Binary-level contextual-output coverage also runs `volicord status`,
compact/verbose/JSON doctor, and human/JSON privacy footprint repeatedly against
one disposable Runtime Home and Product Repository. It verifies the explicit
no-active-Task state, applicability-based human fields, complete structured
reports, sectioned privacy claims, exact terminal hygiene, and unchanged Store
effect counters and authority snapshots. Focused privacy coverage captures
stdout directly as bytes in both modes. It requires valid UTF-8 and JSON,
compares every category string and the output-scope scalar with the canonical
typed definition and human section, protects the complete diagnostics sentence
including `when diagnostics are present`, enforces one claim occurrence and one
trailing human newline, and rejects tabs or other control corruption. The same
fixture compares all persisted entries, file bytes, and modification times
before and after each command, covering the Registry, diagnostics and project
Stores, Product Repository, managed configuration, installation profile, Hook
files, and persisted verification reports. Pure renderer tests cover ready,
warning, action-required or failed Doctor states; intentional titles for every
current check; deterministic semantic group and check order; healthy command
aggregation; missing CLI and MCP commands; configured-versus-resolved PATH
mismatch; strict inconsistent-fact rejection; optional host detection;
structured non-success details; long paths; empty collections; terminal
hygiene; and exhaustive JSON check parity. Doctor remediation coverage supplies
explicit findings and direct candidates to the pure report-finalization
boundary. It verifies
finding-action inclusion, direct-action inclusion, command enrichment,
required-over-recommended urgency, deterministic priority and code ordering,
conflict rejection, strict required/recommended partitioning, one primary
action across JSON and human projections, warning-without-action wording, and
no Store or Product Repository mutation during rendering.

Build-presentation coverage constructs exact-profile, class-only, and missing
profile metadata and compares Version and Doctor human wording from the shared
typed owner. It also proves that JSON retains the exact `class_only` machine
value without a parallel presentation-label field. Focused Doctor renderer
coverage distinguishes `not recorded`, `not applicable`, `not checked`, and an
actually empty collection rendered as `none`. It proves that the concise build
assessment does not repeat complete provenance, six verified current Hook
artifacts collapse to the three safety dimensions plus one count, and a failed
Hook artifact expands with its path, phase, source, reason, and installation ID.
Connection verbose coverage separately proves source counts for verified
artifacts and failure-first expansion for failed, not-recorded, not-checked,
mixed, and bounded-at-limit evidence with explicit omission labelling. JSON
remains byte-for-byte equivalent to the typed report, every human branch is
tab-free with exactly one trailing newline, and rendering does not mutate Store,
filesystem, or terminal state.

The Doctor binary fixture initializes the canonical current artifacts once,
then invokes compact, verbose, and JSON Doctor modes without rerunning setup.
It requires a passed `guard_files` check only with a verified path-safety
assessment, one strict state object with no parallel fields, explicit verbose
semantic labels, no successful compact detail, and unchanged Registry, project
and diagnostics Stores, managed configuration, installation profile, Hook
files, and Product Repository bytes and modification times. Registry coverage
requires `storage_profile` to be the structured current `StorageManifest` in
JSON, checks its verbose fields and capability list, and injects malformed
persisted manifest JSON to prove strict failure without raw-string fallback.
Every branch preserves exactly one trailing newline and the read-only state of
the disposable Runtime Home after its fixture setup.

Connection-list lifecycle coverage compares each available membership with
selected status at setup, managed-session, and complete stages; retains current
`complete` when persisted active verification remains `action_required`; and
proves independent complete and waiting memberships. Corruption and
unavailability cases cover registration metadata, persisted active evidence,
managed configuration, and project Store failures without hiding valid rows.
Filter cases prove unselected memberships are not evaluated.
JSON and human projections share the typed summary, one invocation timestamp,
tab-free structured paths, compact primary action, and verbose-only IDs,
revision, not-applicable counts, all steps, and bounded issue detail. Repeated
status and list reads also preserve Registry, project and diagnostic Stores,
managed configuration, Product Repository content, Runtime Home reports, and
timestamps.

Operational interoperability coverage accepts arbitrary bounded version
strings, exercises initialize and tool-list milestones, checks required tools
and safe read-only calls, audits Guard artifacts and required-phase
observations, and isolates session ownership and integration revisions.

## Release Integrity And Optional Host Smoke

The durable release test package is `tests/release-integrity`. It verifies all
five published Volicord targets, version consistency, canonical text bytes,
package and archive shape, packaged-binary identity, checksum output, and the
ordinary build and package structure in the release workflow.

`cargo run -p xtask -- source-bundle --output <path>` is the one source-bundle
implementation. It selects `HEAD` by default and rejects tracked index or
working-tree changes. `--commit <commit>` selects another exact commit when a
release or CI check needs it. The command reads the selected tree and blobs
through Git, writes a deterministically ordered ZIP with normalized metadata,
and validates every entry before publishing the output. Regular files,
executables, directories, and symbolic links use modes derived from the Git
tree. Because the filesystem is never walked for inclusion, Git metadata,
untracked files, runtime output, and existing untracked archives are outside
the bundle.

`cargo run -p xtask -- source-bundle-validate --input <path>` independently
reopens the ZIP and compares its paths, file types, modes, link targets, and
contents with the selected Git tree. Focused tests use disposable Git
repositories for dirty tracked state, untracked content, regular files,
executables, symbolic links, unsafe or duplicate ZIP paths, extraction, and
byte-for-byte repeated generation. A complete-current-tree test exercises the
same implementation against this repository. Ordinary CI and tagged release
publication invoke the canonical creation command.

`cargo run -p volicord-release-smoke -- --bin <path>` invokes the dedicated
publish-disabled cross-platform actual-binary smoke package. It creates a
disposable Git Product Repository, Runtime Home, Codex home, and a stable
test-owned Codex fixture executable; runs public `volicord init`; decodes its
JSON with Serde; and starts public
`volicord mcp serve --connection <connection-id>`. It requests the protocol
registry's preferred server revision, completes initialization and
`tools/list`, and checks representative public tools through canonical
`AgentToolId` identities while proving that the user-only resolution operation
is absent. The Codex fixture is a copy of the smoke executable under the
platform Codex filename; only `--version` succeeds and reports the bounded
semantic fixture version `codex-fixture 0.145.0-test`.

The package owns release-specific orchestration, transcript validation, fixture
setup, and result reporting. It supplies lifecycle and capture limits to
`volicord-test-process`, which owns reusable bounded child execution, process
tree cleanup, and direct-child reaping.

`.github/actions/volicord-release-smoke` is the reusable workflow invocation
boundary. Ordinary CI builds the local debug `volicord` binary and invokes the
action exactly once. Every native release matrix entry invokes the same action
exactly once with the exact Linux, macOS, or Windows binary it already built,
before artifact staging. Release-integrity tests validate build, smoke, and
staging order, matrix target and binary references, and exactly-once counts as
YAML semantics rather than complete shell-command formatting. These processes
are public manual transport and remain `manual_cli`; they do not call the
hidden managed-host launcher or provide managed-host evidence.

Generic release-integrity tests cover Volicord platform build and package
artifacts plus source-bundle workflow routing. Operational Codex
interoperability tests separately cover managed configuration, MCP
initialization, required tools, safe tool round trips, Guard observations,
session ownership, and revision isolation as defined by
[Agent Connection](../reference/agent-connection.md).

A real-Codex run is optional operational smoke. It may report the bounded host
version as a diagnostic and repeat the observation when that version changes.
Its result applies only to the behavior observed in that configuration and
environment; it does not establish future host behavior, human identity, or
runtime authority. Missing smoke infrastructure does not block the ordinary
Volicord release checks.

## Documentation Validation

Meaning-changing paired documents require English/Korean semantic parity.
Generated contract projections must match their sources. The focused profile
selects the documentation and diff checks from the owner route. Then inspect
the diff for owner routing, exact identifiers, paths, anchors, and repository
hygiene.

## Rust Validation

Use the focused profile for intermediate Rust changes and the final profile once
after the complete commit series:

```sh
cargo run -p xtask -- validate focused --base <revision>
cargo run -p xtask -- validate final --base <revision>
```

The durable summary records narrower commands, the exact aggregate, any bounded
retry or decomposition, and every skipped command with its reason.
