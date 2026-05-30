# Build: Implementation Overview

## What this document helps you do

This document tells implementers what to build before they dive into the full reference specs. It is the bridge between the reader-centered docs and the detailed contracts in the kernel, runtime, MCP, storage, projection, and conformance references.

This is planning documentation for documentation redesign / feedback incorporation and handoff review. The repository is documentation-only today and is intended to become the Harness Server source repository after documentation acceptance; no Harness Server/runtime implementation exists here yet. The first product MVP target is v0.1 Kernel MVP, exercised by the Kernel Smoke conformance profile: one local process with modules proving one authority loop. v0.2 through v0.4 are staged packs toward the Agency-Hardened MVP reference conformance target. v1+ Expansion remains roadmap scope unless owner docs promote and prove it.

Use it to answer three questions:

- What are the runtime pieces that must exist first?
- What proof should the first runnable slice produce?
- What staged proof is required before the agency-hardened reference target can be called complete?

This document does not define SQLite DDL, public MCP schemas, projection template bodies, or command syntax. Those details stay in the reference docs.

## Read this when

- You are planning the first implementation shape after maintainer handoff explicitly accepts the docs for first runtime-batch planning.
- You need to review whether a proposed MVP build keeps the right scope.
- You want the short map before reading the strict reference specs.

## Before you read

You should already understand the basic Harness concepts from the Learn path. For exact behavior, use the Reference docs linked at the end of this page. For v1+ Expansion candidates and promotion criteria, use the [Roadmap](../roadmap.md).

## Main idea

Harness is a local work ledger and judgment router for AI-assisted product work. It records what may change, who must decide, what evidence exists, what risk remains, and whether the work can close. The first implementation path should prove that local ledger and judgment route through the smallest Core authority loop before adding evidence depth, agency hardening, operations, or automation.

Build v0.1 Kernel MVP first: the smallest local Core authority path, with Kernel Smoke as its first conformance profile. Core alone changes canonical operational state. Then harden that path through v0.2 Evidence & Projection Pack, v0.3 Agency Pack, and v0.4 Operations Pack.

All implementation verbs in this Build path describe future runtime-batch planning after the maintainer handoff explicitly accepts the docs for that planning. While [Documentation Acceptance Status](#documentation-acceptance-status) says first runtime-batch planning is not accepted, use this document only to review scope and handoff readiness.

When that handoff changes, implementation is expected to happen in this repository as the Harness Server / Installation source code. This repository is still not the user's Product Repository and not the Harness Runtime Home; runtime state, artifacts, projection output, and logs belong in a Harness Runtime Home.

The local kernel is a coordination and authority record, not a replacement for the product repository, source control, tests, code review, conversation, or user-owned product and material technical judgment. Build the first path so status and close output explain what changed, what was checked, what remains risky, and what decision is needed.

The first authority loop is narrow: `prepare_write` is the only product-write authorization decision point, a returned Write Authorization is durable and single-use, `record_run` consumes it for one compatible direct Run or implementation Run while recording observed changes and artifacts, and `close_task` is the only completion decision point. Exact state logic lives in [Kernel Reference](../reference/kernel.md#prepare_write) and public request/response details live in [MCP API And Schemas](../reference/mcp-api-and-schemas.md#public-tools).

Start with canonical state, `task_events`, artifact refs, Core tool behavior, and the minimal reference surface and MCP reachability needed to exercise that path. The initial implementation assumption is one local process with modules, not a distributed platform. Treat projection-template polish, dashboards or hosted workflow UI, indexes, broad connector ecosystems or marketplaces, team workflow, surface-specific connector automation, hook expansion, Browser QA automation, derived metrics, parallel orchestration, and broad automation as non-authoritative things that read from or wrap that authority loop after it exists.

If a proposed implementation starts with Agency-Hardened MVP as one large first batch, projection template polish, a dashboard or hosted workflow UI, a Context Index, a connector marketplace, hook expansion, metrics, parallel orchestration, or broad automation lanes, it is starting in the wrong place.

## Documentation acceptance status

This is a maintainer-updated documentation handoff marker. It is not a Reference contract, conformance result, generated operational record, or runtime implementation authorization. Do not infer acceptance from the checklist below; maintainers must change this table deliberately.

Current revision status: documentation acceptance remains No unless maintainers deliberately change it. This status marker is not runtime/server implementation, runtime conformance, or implementation readiness.

| Question | Current status |
|---|---|
| Is documentation redesign / feedback incorporation still active? | Yes. Documentation redesign / feedback incorporation remains active, and implementation handoff still requires a deliberate maintainer update. |
| Are docs accepted for first runtime-batch planning? | No. First runtime-batch planning may not begin until maintainers change this row to Yes after the checkpoint below is satisfied. |
| Has runtime/server implementation started? | No. This repository still contains documentation, not Harness runtime/server implementation. |
| Are there open follow-up docs issues? | Known redesign issues are tracked in the [Authoring Guide](../maintain/authoring-guide.md#known-redesign-issues-tracker). They are documentation redesign inputs, not runtime conformance, implementation readiness, or authorization to start server/runtime implementation. Docs accepted for implementation planning remains No unless maintainers deliberately change the handoff status. Harness Server/runtime implementation remains not started. |

Build readers should treat this table as the entry gate. Until maintainer handoff changes the second row to Yes, even v0.1 Kernel MVP remains planning-only in this repository and runtime/server implementation must not start.

## Implementation handoff checkpoint

Use this checkpoint to decide what must be true before maintainers can switch the documentation acceptance status from documentation maintenance to first runtime-batch planning. It is a planning handoff only: it does not authorize runtime or server implementation by itself, and it does not define exact schemas, DDL, fixture semantics, or runtime contracts.

First implementation planning means v0.1 Kernel MVP planning first, not Agency-Hardened MVP or roadmap automation. It may start only when all of these are true:

- The final docs-maintenance drift pass is complete, or remaining known gaps are recorded as `TODO_DECISION` or `TODO_IMPLEMENT` in the relevant owner docs. Docs-maintenance remains a read-only documentation check; see [Authoring Guide](../maintain/authoring-guide.md#docs-maintenance-checks) and [Operations And Conformance Reference](../reference/operations-and-conformance.md#docs-maintenance-profile).
- The local-only MCP exposure baseline is accepted for v0.1 Kernel MVP. Remote, shared, tunneled, or non-loopback exposure remains outside the v0.1 baseline unless owner docs promote and prove a connector profile; see [Runtime Architecture](../reference/runtime-architecture.md#local-access-expectations), [Security Threat Model Reference](../reference/security-threat-model.md#mcp-local-access-and-caller-boundaries), and [MCP API And Schemas](../reference/mcp-api-and-schemas.md#mcp-boundary-and-caller-trust).
- The reference surface capability profile is accepted as a concrete declaration for the actual host/profile/configuration in use, with refresh triggers for version, MCP config, hooks, permissions, workspace policy, generated files, conformance result, capture method, QA capture method, redaction policy, and artifact retention behavior. Exact connector profile and surface recipe details stay in [Agent Integration Reference](../reference/agent-integration.md#capability-profiles) and [Surface Cookbook](../reference/surface-cookbook.md).
- The Core-only mutation model is accepted: Core alone changes canonical operational state, while resources, projections, reports, diagnostics, MCP callers, and operator entrypoints remain read-only or derived unless they enter a Core state-changing path. See [Core process model](../reference/runtime-architecture.md#core-process-model), [State transaction flow](../reference/runtime-architecture.md#state-transaction-flow), and the MCP [Idempotency](../reference/mcp-api-and-schemas.md#idempotency) and [State conflict behavior](../reference/mcp-api-and-schemas.md#state-conflict-behavior) sections.
- The Kernel Smoke fixture queue is identified as the v0.1 Kernel MVP conformance authoring order. Exact fixture format, assertions, and catalog semantics stay in [Conformance Fixtures Reference](../reference/conformance-fixtures.md#kernel-smoke-authoring-queue).
- The first runnable slice remains local, single-project, single-reference-surface, and fixture-proven. Use [First Runnable Slice](first-runnable-slice.md) for the planning checklist.
- v1+ Expansion features remain outside v0.1 Kernel MVP, the v0.2 through v0.4 staged packs, and Agency-Hardened MVP unless promoted by owner docs through the [Roadmap promotion rule](../roadmap.md#promotion-rule).

This handoff does not promote roadmap items, dashboards or hosted workflow UI, Browser QA Capture automation, Context Index, broad connector ecosystems or marketplaces, team workflow, remote MCP exposure, preventive guard expansion, Local Derived Metrics or long-term metrics, or parallel orchestration into v0.1 Kernel MVP, the v0.2 through v0.4 staged packs, or Agency-Hardened MVP. Keep exact contracts in Reference docs and use this section only as the short readiness checkpoint.

## Proof boundaries

| Boundary | What it proves | What the user or operator can observe |
|---|---|---|
| v0.1 Kernel MVP | One local Task can go through the first Core authority loop: project registration, Task, direct/work/advisor mode basics, Change Unit, basic Decision Packet lifecycle, `prepare_write`, single-use Write Authorization, `record_run`, minimal `ArtifactRef`, minimal Evidence Manifest, status/next, minimal `TASK` projection or enqueue, and structured close blockers. | Status and next show the active Task, gates, Change Unit, Decision Packet refs, evidence, blockers, and projection freshness. `prepare_write` refuses out-of-scope write authorization, compatible scoped work is authorized and consumed once, and `close_task` refuses missing evidence or required decisions with structured blockers. |
| v0.2 Evidence & Projection Pack | Evidence and projection behavior broadens after the first loop exists while projections remain derived. | Evidence state, artifact-backed support, projection freshness, projection failure isolation, and reconcile items are visible from owner records. |
| v0.3 Agency Pack | The local reference path handles user judgment, sensitive-action Approval separation, detached verification, Manual QA, residual-risk visibility and accepted-close semantics, stewardship, TDD, and feedback-loop policy with honest boundaries. | Fixtures show why work can or cannot proceed, verify, require QA, accept, or close through the same Core records and errors. |
| v0.4 Operations Pack | Operator readiness, recover/export, artifact integrity, release handoff, large fixture suite coverage, and later-boundary checks complete the Agency-Hardened MVP reference conformance target. | Operator entrypoints diagnose, recover, export, check artifacts, and run conformance over the same Core state without creating a second authority model. |
| Roadmap boundary: v1+ Expansion | Later surfaces or automation can be considered only after the local kernel and agency proof are stable. | Optional capabilities remain read-only, display-only, metadata-only, or artifact-candidate-only until an owner promotes them through the [Roadmap promotion rule](../roadmap.md#promotion-rule) with exact contracts and fixtures. |

## What you are building

After maintainer handoff explicitly accepts the docs for first runtime-batch planning, Harness implementation starts in this repository with v0.1 Kernel MVP as a local work ledger and judgment router for AI-assisted product work. It keeps durable local state, artifact refs, and readable projections around the work journey, while leaving product history, executable checking, review, and user judgment with the existing engineering process. The agency-preserving local authority kernel principle remains the implementation center: Core owns canonical local state, and user-owned judgment stays with the user. The initial implementation assumption is one local system with clear internal modules, not a distributed platform.

The sections below describe future responsibilities for that runtime batch. They are not work orders for the current documentation-acceptance phase.

### Local Server / Process

Build one local Harness server or process that exposes the MCP boundary, owns Core transitions, reads and writes the runtime home, and runs validators, projection enqueueing, reconcile, recovery, export, and conformance entrypoints through the same Core rules.

v0.1 Kernel MVP can be one process with modules. It does not need separate services for Core, projection, validation, and operator tools.

### Core

Core is the only path that mutates canonical operational state. Implement the transaction order owned by [Runtime Architecture](../reference/runtime-architecture.md#state-transaction-flow): envelope and state-version validation, lock acquisition, current-state read, validators, record update, `task_events` append, projection job enqueue, and commit. At this Build level, that means Core must:

- validate tool envelopes, idempotency keys, and expected state versions before a new mutation
- acquire the relevant project or task lock
- read current records
- run Core checks and validators
- update current records, append `task_events`, and enqueue projection work in the Core transaction
- return blockers and refs that explain the result

Agents, MCP tools, operator commands, projectors, and recovery flows must either enter through Core or preserve the same Core compatibility rules. None of them may maintain a second canonical state model.

### State Store

The state store keeps canonical operational state: project state, Tasks, gates, Change Units, Decision Packets, Approval records, Write Authorizations, Runs, evidence manifests, Eval records, Manual QA records, residual risks, projection jobs, reconcile items, and `task_events`.

Do not design this from scratch in the Build layer. Storage details and DDL are owned by [Storage And DDL](../reference/storage-and-ddl.md).

### Artifact Store

The artifact store keeps durable evidence files and integrity metadata. Raw artifacts may include diffs, logs, screenshots, bundles, manifests, checkpoints, export components, or other evidence files.

The artifact store is not a loose file dump. Every artifact that supports Harness state needs a registered artifact ref, hash, size, redaction state, and relation to the Task or owner record that uses it.

### MCP API

The MCP server exposes read resources and public tools. MCP resources are read-only. State-changing work goes through public tools and Core.

If the MCP server cannot be reached, no authoritative Core response is available from that call path. The first implementation should report that as MCP unavailable, hold write-capable work by the reference surface's actual guarantee level, and avoid inventing state from cached projections, generated files, or chat text.

For the first build path, prioritize:

- status and active Task reads
- intake or Task creation
- next-action guidance
- direct, work, and advisor mode basics
- basic Decision Packet lifecycle and blocker visibility
- `prepare_write` as the only product-write authorization decision point
- `record_run` consumption of one compatible Write Authorization for one implementation or direct product-write Run
- artifact registration through the tool flows that need it
- evidence manifest updates
- `close_task` blocker behavior as the only completion decision point

The public request and response contracts belong to [MCP API And Schemas](../reference/mcp-api-and-schemas.md).

State conflict and idempotency replay behavior are part of that public tool contract. Build code should use the owner sections for [Idempotency](../reference/mcp-api-and-schemas.md#idempotency) and [State conflict behavior](../reference/mcp-api-and-schemas.md#state-conflict-behavior), with durable storage details left to [Storage And DDL](../reference/storage-and-ddl.md).

### Projections

Projections are human-readable views derived from state records and artifact refs. `TASK`, `APR`, `RUN-SUMMARY`, `EVIDENCE-MANIFEST`, `EVAL`, `DIRECT-RESULT`, and other report projections are not canonical state.

Build projection output from the Core source records it depends on, such as Task, gate, Run, artifact, evidence, Eval, QA, and other owner records after those records exist. A minimal `TASK` projection freshness or enqueueing path can be part of v0.1 Kernel MVP, but projection templates cannot create authority, satisfy evidence, replace state, shape the state model, or become the first proof.

The v0.1 slice may enqueue a minimal `TASK` projection job or render a minimal `TASK` projection. Later packs must support the Reference-required `ProjectionKind` values when their source records exist or change. `ProjectionKind` values and API-owned tiering belong to [MCP API And Schemas](../reference/mcp-api-and-schemas.md#shared-schemas). [Document Projection Reference](../reference/document-projection.md#template-tiers) owns projection authority boundaries, source-record rules, freshness rules, and template tier presentation; [Template Reference](../reference/templates/README.md) owns rendered template bodies and display cards.

Projection failure must not roll back committed Core state. It should mark projection freshness or job status and leave recovery or reconcile to a later action. `source_state_version` and freshness are display/readiness facts: close/readiness output should show when a readable view is stale or failed, but stale Markdown cannot authorize work, satisfy close, replace current Core state, replace source control, replace tests, or replace review.

Human-editable projection sections are proposal surfaces. The implementation path should route proposal -> reconcile item -> accepted Core state-changing action and `task_events` row, or reject, defer, or note. Direct managed-block edits are drift, not state changes.

### Operator Commands

Operator entrypoints are surfaces over Core behavior, not a second state model. Build them as command-independent capabilities first:

- connect or register a project
- report doctor/readiness status across runtime home, project state, artifact store, reference surface, MCP availability, projections, reconcile, validators/checks, and agency/stewardship/context
- serve or expose the MCP boundary
- refresh projections
- reconcile human edits, generated-file drift, or managed-block drift without silently overwriting the existing file or treating it as state
- recover interrupted or stale operational state, including baseline drift, approval drift, evaluator repo drift, artifact missing or hash mismatch, projection failure, managed Markdown direct edits, MCP unavailable, and surface capability mismatch, without treating recovery artifacts as successful completion proof
- export state snapshots, report projection snapshots, artifact refs, redaction status, omitted-secret notes, and retained, expired, or unavailable artifact status
- check artifact integrity
- run conformance fixtures

Exact command names and flags can come later. The important part is the command-independent behavior contract: operator behavior uses the same Core state, `task_events`, artifacts, projections, and existing errors or diagnostics as MCP tools. State-changing operator outcomes must enter Core or a documented recovery path that preserves Core ordering; operator output must not become a parallel source of state truth.

## What you are not building yet

Keep the first implementation narrow. Do not build these as prerequisites for v0.1 Kernel MVP, the v0.2 through v0.4 staged packs, Agency-Hardened MVP, or any authority path unless owner docs promote them:

- dashboard, hosted workflow UI, or rich UI as an authority path or close-readiness source
- broad connector ecosystem or marketplace beyond one reference surface
- Context Index as authority or read/write prerequisite
- Browser QA Capture as required automation or acceptance replacement
- Cross-Surface Verification as a required assurance path
- native hook expansion beyond a concrete reference-surface capability
- Advanced Sidecar Watcher as required enforcement
- Local Derived Metrics or long-term metrics as staged-delivery-critical state, authority, or readiness
- team workflow, shared workspaces, permissions, or profile import/export
- parallel orchestration automation, concurrent lane scheduling, or multi-agent scheduling
- preventive guard expansion unless the reference surface proves a concrete pre-tool blocking path for the covered operation

v0.1 Kernel MVP may display cooperative or detective guard/freeze status and may hold or narrow work through existing Change Unit, Autonomy Boundary, and `prepare_write` behavior. Surface labels do not upgrade the stored guarantee level.

Useful later capabilities can appear only as read-only displays, metadata, artifact candidates for existing owner paths, or fixture candidates until their owner docs define capability profile, redaction/secret/PII policy, retention or test-environment rules when needed, fixture coverage, fallback behavior, and no projection-as-canonical dependency. They must not be required to run v0.1 Kernel MVP or to claim staged-delivery close readiness.

## The first proof

The first product MVP target is v0.1 Kernel MVP: the smallest runnable path that proves Harness can make and enforce one authority decision. Kernel Smoke is the conformance profile for this target.

v0.1 proves the authority loop, not Agency-Hardened MVP, not template completeness, and not broad automation.

It should show:

- one registered project and reference surface
- one Task with current state and gates
- direct, work, and advisor mode basics
- one active scoped Change Unit
- basic Decision Packet lifecycle and blocker visibility
- `prepare_write` refuses write authorization without authority and allows a compatible scoped write
- allowed `prepare_write` creates a durable single-use Write Authorization
- `record_run` consumes that authorization for one direct Run or implementation Run and records observed changes plus artifacts
- artifacts can be registered and linked to the run or evidence
- a minimal Evidence Manifest records support or insufficiency
- status and next reads are non-mutating
- a minimal `TASK` projection is current or at least durably enqueued
- `close_task` blocks with structured blockers when evidence or decision requirements are missing
- the same behavior is executable through basic Core fixtures

v0.1 Kernel MVP is not Agency-Hardened MVP. It proves the write authority path is alive. Use [First Runnable Slice](first-runnable-slice.md#doc-level-acceptance-checks) for doc-level acceptance checks, and use [Conformance Fixtures Reference](../reference/conformance-fixtures.md#conformance-fixture-format) for exact fixture semantics.

## The agency-hardened proof

The later reference conformance target is Agency-Hardened MVP. It is reached through the v0.2, v0.3, and v0.4 packs after v0.1 Kernel MVP, not as the first implementation batch. It adds the remaining conformance needed for an agent to act with honest boundaries:

- Decision Packet quality and user-judgment routing
- separation between sensitive-action Approval, Decision Packets, and Write Authorizations
- residual-risk visibility before acceptance and close
- detached verification independence
- Manual QA records and QA blockers
- feedback-loop, TDD, stewardship, and context-hygiene validators
- projection and reconcile completeness
- recovery, export, and artifact integrity behavior
- release handoff report/export behavior where owner docs define it
- later-boundary checks that keep broad automation in v1+ Expansion
- fixture coverage for required agency conformance

Agency-Hardened MVP is complete only when conformance proves behavior through Core state, events, artifacts, projections, and errors rather than rendered prose alone.

## Build reading path

Read the Build layer in this order:

1. [Implementation Overview](implementation-overview.md) for the system you are building.
2. [First Runnable Slice](first-runnable-slice.md) for the smallest proof to implement first.
3. [MVP Plan](mvp-plan.md) for v0.1 through v0.4 staged delivery and the boundary to v1+ Expansion.

Use [Roadmap](../roadmap.md) for v1+ Expansion candidates and promotion rules.

Then use the reference docs and current owners for exact behavior:

- [Kernel Reference](../reference/kernel.md) for entities, gates, state logic, `prepare_write`, and `close_task`.
- [Runtime Architecture Reference](../reference/runtime-architecture.md) for runtime spaces, Core flow, artifacts, projection/reconcile, and guarantee levels.
- [MCP API And Schemas](../reference/mcp-api-and-schemas.md) for public resources, tools, schemas, errors, artifact refs, idempotency, and state conflict behavior.
- [Storage And DDL](../reference/storage-and-ddl.md) for runtime layout, DDL, migrations, locks, artifacts, baselines, projection jobs, and validator-run storage.
- [Operations And Conformance Reference](../reference/operations-and-conformance.md) for operator semantics and conformance run overview.
- [Conformance Fixtures Reference](../reference/conformance-fixtures.md) for fixture body shape, assertion semantics, suite catalogs, and examples.
