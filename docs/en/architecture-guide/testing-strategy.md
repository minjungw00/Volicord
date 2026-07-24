# Testing Strategy

Tests protect owner-defined current behavior. They do not create product
contracts, preserve removed surfaces, or justify broader support claims.

## Choose The Narrowest Layer

| Test layer | Use it for |
|---|---|
| Unit test | Pure parsing, canonical encoding, closed values, and policy decisions. |
| Crate integration test | Adapter boundaries, Store reads/writes, process behavior, and strict persisted-record rejection. |
| Conformance test | Public cross-method outcomes, error categories, replay, effects, and projections. |
| Release-integrity test | Volicord target, version, package, checksum, workflow, and actual-binary smoke invariants. |
| Documentation check | Owner routing, links, terminology, parity, examples, and generated-source drift. |

Use disposable Runtime Homes and Product Repositories. Keep fixtures minimal and
typed. A fixture proves parser or implementation behavior only; it does not
prove behavior of a real Codex installation or platform support.

`volicord-test-process` owns bounded child execution shared by repository tests
and smoke harnesses. It composes the process-group, Windows Job Object, and
nonblocking pipe primitives owned by `volicord-platform-process`; it does not
duplicate those OS implementations. Product MCP supervision policy, protocol
framing, lifecycle progress, and diagnostics remain owned by `volicord-cli`.

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
`cargo test -p volicord-mcp --test protocol_conformance`. Its generic runner
iterates `ProtocolRegistry::production().oldest_to_newest()` directly, so adding
a production profile automatically adds the same focused case to the matrix.
The manifest records reviewed upstream and support facts; it does not record
whether executable tests ran. The runner owns no separate conformance revision
array or per-revision coverage boolean; direct registry iteration defines the
matrix.

## Required Boundary Coverage

Durable tests should cover, as applicable:

- unknown members, duplicate keys, malformed closed values, and corrupt stored
  owner records;
- structural rejection before policy, replay, ticket invalidation, or mutation;
- read-only branches with no authority event or state-version advance;
- one atomic successful mutation and exact replay behavior;
- current-contract mismatch routed through the owned corrupt-data failure;
- Runtime Home `Absent`, `Ready`, `Incompatible`, and `Corrupt` inspection;
  same-parent staged creation with singleton and installation metadata; exact
  manifest and relation facts; cleanup at each pre-publication failure;
  no-replace concurrent publication on Unix and native Windows; and unchanged
  bytes and timestamps for existing incompatible state;
- missing or ineligible operation-result rows remaining
  `OPERATION_RESULT_UNAVAILABLE`;
- MCP rejection of hidden context and CLI-only UserAction resolution;
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
- exact-match and counter-offer negotiation plus profile-specific initialize
  capabilities, batching, `tools/list`, and `tools/call` wire projection;
- independently pinned Codex host fixtures that are not derived from the
  production protocol registry and do not substitute for revision conformance,
  with exact `CodexMcpTurnMetadata`, `CodexCommandHooks`, and
  `CodexMcpCallableNames` profile coverage; source-specific correlation;
  explicit server/raw/callable fixture parity; complete raw-name projection;
  normalization-collision rejection; exact catalog reverse lookup; additive-
  field and bound checks; checksum parity; typed host-tool plus server-
  namespace/catalog-derived-exact routing; current Guard-probe pre/post fixture
  delivery; foreign-server
  exclusion; generated matcher/catalog parity; strict matcher-drift rejection;
  and CLI conformance evidence kept separate from actual `managed_host`
  observations;
- immutable MCP preflight evidence on readable read-only Registry and project
  databases, unchanged selected-database row counts and modification times,
  writeability always `not_checked`, active write evidence stored only under
  `last_active_verification`, unchanged preflight evidence after verification,
  active timestamp/source, disposable conformance state, concise/verbose/JSON
  parity, and strict rejection of a combined legacy evidence shape;
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
- exact `HookActivationState` evidence precedence including unknown, setup
  review, current-definition observation, policy management, invocation bypass,
  and explicit disabled states, with no synthetic trusted state;
- `IntegrationActivationState` transitions through configured, reload, hook
  review/unknown, managed MCP observation, Guard verification, complete, and
  failed, with `project_trust` kept independent;
- separate ambient and correlated Guard checks: ambient passed with correlated
  failed, ambient pending with no attempt, correlated complete,
  repair-required never projected as pending, and an older proof retained
  without hiding a newer failed attempt;
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
  request, and status; one `Required next steps` block; no duplicate `Next`
  block or inaccurate “one more step”; current-status suffixes; typed
  repair-required plans; and optional active diagnostics kept separate;
- transactional init fault injection after Runtime Home preparation, Store
  recovery preparation, every managed hook/rule/guidance replacement, before
  and after Codex configuration replacement, before integration-revision
  commit, and during rollback; exact fresh and existing-state restoration;
  external concurrent-byte preservation; `planned`, `committed`,
  `preserved`, `rolled_back`, and `partially_rolled_back` report projection;
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
- the pinned current Codex semantic profile's synchronous one-read observation
  policy, no numeric-version or legacy-profile branch, missing-event immediate
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
- Guard observation and unrecorded-change suppression outcomes; and
- Codex configuration drift and behavior-probe failure reporting;
- reusable bounded test child execution across success, stdin delivery,
  nonzero exit, timeout, deterministic stdout/stderr truncation, simultaneous
  streams, descendant-held pipes, stdin-write failure cleanup, repeated
  cleanup, native Unix process groups, native Windows Job Objects, paths and
  arguments containing spaces, and explicit environment addition/removal.

Operational interoperability coverage accepts arbitrary bounded version
strings, exercises initialize and tool-list milestones, checks required tools
and safe read-only calls, audits Guard artifacts and required-phase
observations, and isolates session ownership and integration revisions.

## Release Integrity And Optional Host Smoke

The durable release test package is `tests/release-integrity`. It verifies all
five published Volicord targets, version consistency, canonical text bytes,
package and archive shape, packaged-binary identity, checksum output, and the
ordinary build and package structure in the release workflow.

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
artifacts. Operational Codex interoperability tests separately cover managed
configuration, MCP initialization, required tools, safe tool round trips,
Guard observations, session ownership, and revision isolation as defined by
[Agent Connection](../reference/agent-connection.md).

A real-Codex run is optional operational smoke. It may report the bounded host
version as a diagnostic and repeat the observation when that version changes.
Its result applies only to the behavior observed in that configuration and
environment; it does not establish future host behavior, human identity, or
runtime authority. Missing smoke infrastructure does not block the ordinary
Volicord release checks.

## Documentation Validation

Meaning-changing paired documents require English/Korean semantic parity.
Generated contract projections must match their sources. Run:

```sh
cargo run -p xtask -- docs-check
git diff --check
```

Then run the targeted stale-surface scan appropriate to the change and inspect
the diff for owner routing, exact identifiers, paths, anchors, and repository
hygiene.

## Rust Validation

The normal workspace gate is:

```sh
cargo fmt
cargo clippy --all-targets --all-features
cargo test --all-targets --all-features
```

If a narrower command is necessary, record why and identify the unexecuted
workspace checks in the handoff.
