# Build: Implementation Overview

## What this document helps you do

This document tells implementers what to build before they dive into the full reference specs. It is the bridge between the reader-centered docs and the detailed contracts in the kernel, runtime, MCP, storage, projection, and conformance references.

This is planning documentation. It does not authorize runtime/server implementation, generated operational files, executable fixtures, or runtime data before the redesigned docs are accepted. The first implementation/proof target is Kernel Smoke: one local process with modules proving one authority loop. Agency-Hardened MVP is a later hardening and conformance target after Kernel Smoke, and roadmap automation stays outside MVP unless owner docs promote and prove it.

Use it to answer three questions:

- What are the runtime pieces that must exist first?
- What proof should the first runnable slice produce?
- What proof is required before the MVP can be called complete?

This document does not define SQLite DDL, public MCP schemas, projection template bodies, or command syntax. Those details stay in the reference docs.

## Read this when

- You are planning the first implementation shape after the documentation redesign is accepted.
- You need to review whether a proposed MVP build keeps the right scope.
- You want the short map before reading the strict reference specs.

## Before you read

You should already understand the basic Harness concepts from the Learn path. For exact behavior, use the Reference docs linked at the end of this page. For post-MVP candidates and promotion criteria, use the [Roadmap](../roadmap.md).

## Main idea

Build Kernel Smoke first: the smallest local Core authority path. Core alone changes canonical operational state. Then harden that path through evidence, projections, conformance, and operator recovery.

The first authority loop is narrow: `prepare_write` is the only product-write authorization decision point, a returned Write Authorization is durable and single-use, `record_run` consumes it for one compatible implementation or direct Run while recording observed changes and artifacts, and `close_task` is the only completion decision point. Exact state logic lives in [Kernel Reference](../reference/kernel.md#prepare_write) and public request/response details live in [MCP API And Schemas](../reference/mcp-api-and-schemas.md#public-tools).

Start with canonical state, `task_events`, artifact refs, Core tool behavior, and the minimal reference surface and MCP reachability needed to exercise that path. The initial implementation assumption is one local process with modules, not a distributed platform. Treat projection-template polish, dashboards or hosted workflow UI, indexes, broad connector ecosystems or marketplaces, team workflow, surface-specific connector automation, hook expansion, Browser QA automation, derived metrics, parallel orchestration, and broad automation as non-authoritative things that read from or wrap that authority loop after it exists.

If a proposed implementation starts with Agency-Hardened MVP as one large first batch, projection template polish, a dashboard or hosted workflow UI, a Context Index, a connector marketplace, hook expansion, metrics, parallel orchestration, or broad automation lanes, it is starting in the wrong place.

## Documentation acceptance status

This is a maintainer-updated documentation handoff marker. It is not a Reference contract, conformance result, generated operational record, or runtime implementation authorization. Do not infer acceptance from the checklist below; maintainers must change this table deliberately.

| Question | Current status |
|---|---|
| Is documentation maintenance still active? | Yes. The redesigned docs are ready for human review, and implementation handoff has not been accepted yet. |
| Are docs accepted for first runtime-batch planning? | No. First runtime-batch planning may not begin until maintainers change this row to Yes after the checkpoint below is satisfied. |
| Has runtime/server implementation started? | No. This repository still contains documentation, not Harness runtime/server implementation. |
| Are there open follow-up docs issues? | No. The known follow-up docs issues below are resolved; maintainers still must change the acceptance status deliberately. |

Build readers should treat this table as the entry gate. Until maintainers change the second row to Yes, even Kernel Smoke remains planning-only in this repository.

### Known follow-up docs issues

Maintainers update this list. These entries route documentation maintenance only; they do not define Reference contracts, conformance results, generated operational records, or runtime/server authorization.

- Resolved in this batch - repo-level `.agents` / `.codex` instruction audit: inspected the actual repository surfaces. `.agents` is an empty directory and `.codex` is an empty file; neither contains active instructions, generated instructions, managed blocks, connector manifests, or stale Harness guidance. They are left untouched as inert placeholders, and `AGENTS.md` remains the only repo-level always-on instruction surface.
- Resolved in this batch - User Guide opening convention alignment: verified that [User Guide](../use/user-guide.md) already follows the [Authoring Guide standard opening pattern](../maintain/authoring-guide.md#standard-opening-pattern) and keeps the no-startup-phrase convention aligned with [Agent Session Flow](../use/agent-session-flow.md).

## Implementation handoff checkpoint

Use this checkpoint to decide what must be true before maintainers can switch the documentation acceptance status from documentation maintenance to first runtime-batch planning. It is a planning handoff only: it does not authorize runtime or server implementation by itself, and it does not define exact schemas, DDL, fixture semantics, or runtime contracts.

First implementation planning means Kernel Smoke planning first, not Agency-Hardened MVP or roadmap automation. It may start only when all of these are true:

- The final docs-maintenance drift pass is complete, or remaining known gaps are recorded as `TODO_DECISION` or `TODO_IMPLEMENT` in the relevant owner docs. Docs-maintenance remains a read-only documentation check; see [Authoring Guide](../maintain/authoring-guide.md#docs-maintenance-checks) and [Operations And Conformance Reference](../reference/operations-and-conformance.md#docs-maintenance-profile).
- The local-only MCP exposure baseline is accepted for MVP. Remote, shared, tunneled, or non-loopback exposure remains outside the MVP baseline unless owner docs promote and prove a connector profile; see [Runtime Architecture](../reference/runtime-architecture.md#local-access-expectations) and [MCP API And Schemas](../reference/mcp-api-and-schemas.md#mcp-boundary-and-caller-trust).
- The reference surface capability profile is accepted as a concrete declaration for the actual host/profile/configuration in use, with refresh triggers for version, MCP config, hooks, permissions, workspace policy, generated files, conformance result, capture method, QA capture method, redaction policy, and artifact retention behavior. Exact connector profile and surface recipe details stay in [Agent Integration Reference](../reference/agent-integration.md#capability-profiles) and [Surface Cookbook](../reference/surface-cookbook.md).
- The Core-only mutation model is accepted: Core alone changes canonical operational state, while resources, projections, reports, diagnostics, MCP callers, and operator entrypoints remain read-only or derived unless they enter a Core state-changing path. See [Core process model](../reference/runtime-architecture.md#core-process-model), [State transaction flow](../reference/runtime-architecture.md#state-transaction-flow), and the MCP [Idempotency](../reference/mcp-api-and-schemas.md#idempotency) and [State conflict behavior](../reference/mcp-api-and-schemas.md#state-conflict-behavior) sections.
- The Kernel Smoke fixture queue is identified as the first runtime conformance authoring order. Exact fixture format, assertions, and catalog semantics stay in [Operations And Conformance Reference](../reference/operations-and-conformance.md#kernel-smoke-authoring-queue).
- The first runnable slice remains local, single-project, single-reference-surface, and fixture-proven. Use [First Runnable Slice](first-runnable-slice.md) for the planning checklist.
- Post-MVP features remain outside MVP unless promoted by owner docs through the [Roadmap promotion rule](../roadmap.md#promotion-rule).

This handoff does not promote roadmap items, dashboards or hosted workflow UI, Browser QA Capture automation, Context Index, broad connector ecosystems or marketplaces, team workflow, remote MCP exposure, preventive guard expansion, Local Derived Metrics or long-term metrics, or parallel orchestration into MVP. Keep exact contracts in Reference docs and use this section only as the short readiness checkpoint.

## Proof boundaries

| Boundary | What it proves | What the user or operator can observe |
|---|---|---|
| Kernel Smoke | One local Task can go through the Core authority loop: `prepare_write` as the write authorization point, single-use Write Authorization, `record_run` consumption with observed changes and artifacts, artifact-backed evidence, status, minimal projection freshness, and structured close blockers. | Status shows the active Task, gates, Change Unit, evidence, blockers, and projection freshness. Out-of-scope work is blocked, compatible scoped work is authorized and consumed once, and `close_task` refuses missing evidence or required decisions with structured blockers. |
| Agency-Hardened MVP | Later hardening after Kernel Smoke: the local reference MVP handles user judgment, approvals, detached verification, Manual QA, residual risk, reconcile, recovery, export, and conformance with honest boundaries. | Fixtures and operator entrypoints show why work can or cannot continue, verify, accept, export, recover, or close through the same Core records and errors. |
| Post-MVP roadmap | Later surfaces or automation can be considered only after the local kernel and agency proof are stable. | Optional capabilities remain read-only, display-only, metadata-only, or artifact-candidate-only until an owner promotes them through the [Roadmap promotion rule](../roadmap.md#promotion-rule) with exact contracts and fixtures. |

## What you are building

Harness MVP is a local authority kernel for AI-assisted product work. The first implementation target is Kernel Smoke. The initial implementation assumption is one local system with clear internal modules, not a distributed platform.

### Local Server / Process

Build one local Harness server or process that exposes the MCP boundary, owns Core transitions, reads and writes the runtime home, and runs validators, projection enqueueing, reconcile, recovery, export, and conformance entrypoints through the same Core rules.

The MVP can be one process with modules. It does not need separate services for Core, projection, validation, and operator tools.

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

The state store keeps canonical operational state: project state, Tasks, gates, Change Units, Decision Packets, approvals, Write Authorizations, Runs, evidence manifests, Eval records, Manual QA records, residual risks, projection jobs, reconcile items, and `task_events`.

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
- `prepare_write` as the only product-write authorization decision point
- `record_run` consumption of one compatible Write Authorization for one implementation or direct product-write Run
- artifact registration through the tool flows that need it
- evidence manifest updates
- `close_task` blocker behavior as the only completion decision point

The public request and response contracts belong to [MCP API And Schemas](../reference/mcp-api-and-schemas.md).

State conflict and idempotency replay behavior are part of that public tool contract. Build code should use the owner sections for [Idempotency](../reference/mcp-api-and-schemas.md#idempotency) and [State conflict behavior](../reference/mcp-api-and-schemas.md#state-conflict-behavior), with durable storage details left to [Storage And DDL](../reference/storage-and-ddl.md).

### Projections

Projections are human-readable views derived from state records and artifact refs. `TASK`, `APR`, `RUN-SUMMARY`, `EVIDENCE-MANIFEST`, `EVAL`, `DIRECT-RESULT`, and other report projections are not canonical state.

Build projection output from the Core source records it depends on, such as Task, gate, Run, artifact, evidence, Eval, QA, and other owner records after those records exist. A minimal `TASK` projection freshness or enqueueing path can be part of Kernel Smoke, but projection templates cannot create authority, satisfy evidence, replace state, shape the state model, or become the first proof.

The first runnable slice may enqueue a minimal `TASK` projection job or render a minimal `TASK` projection. The final MVP must support enqueueing and rendering MVP-required `ProjectionKind` values when their source records exist or change: `TASK`, `APR`, `RUN-SUMMARY`, `EVIDENCE-MANIFEST`, `EVAL`, and `DIRECT-RESULT`.

Projection failure must not roll back committed Core state. It should mark projection freshness or job status and leave recovery or reconcile to a later action. `source_state_version` and freshness are display/readiness facts: close/readiness output should show when a readable view is stale or failed, but stale Markdown cannot authorize work, satisfy close, or replace current Core state.

Human-editable projection sections are proposal surfaces. The implementation path should route proposal -> reconcile item -> accepted Core state-changing action and `task_events` row, or reject, defer, or note. Direct managed-block edits are drift, not state changes.

### Operator Commands

Operator entrypoints are surfaces over Core behavior, not a second state model. Build them as command-independent capabilities first:

- connect or register a project
- report doctor/readiness status
- serve or expose the MCP boundary
- refresh projections
- reconcile human edits, generated-file drift, or managed-block drift without silently overwriting the existing file or treating it as state
- recover interrupted or stale operational state
- export state, projections, and artifact refs
- check artifact integrity
- run conformance fixtures

Exact command names and flags can come later. The important part is that operator behavior uses the same Core state, events, artifacts, projections, and errors as MCP tools. State-changing operator outcomes must enter Core or a documented recovery path that preserves Core ordering; operator output must not become a parallel source of state truth.

## What you are not building yet

Keep the first implementation narrow. Do not build these as MVP prerequisites:

- dashboard, hosted workflow UI, or rich UI as an authority path or close-readiness source
- broad connector ecosystem or marketplace beyond one reference surface
- Context Index as authority or read/write prerequisite
- Browser QA Capture as required automation or acceptance replacement
- Cross-Surface Verification as a required assurance path
- native hook expansion beyond a concrete reference-surface capability
- Advanced Sidecar Watcher as required enforcement
- Local Derived Metrics or long-term metrics as MVP-critical state, authority, or readiness
- team workflow, shared workspaces, permissions, or profile import/export
- parallel orchestration automation, concurrent lane scheduling, or multi-agent scheduling
- preventive guard expansion unless the reference surface proves a concrete pre-tool blocking path for the covered operation

MVP may display cooperative or detective guard/freeze status and may hold or narrow work through existing Change Unit, Autonomy Boundary, and `prepare_write` behavior. Surface labels do not upgrade the stored guarantee level.

Useful later capabilities can appear only as read-only displays, metadata, artifact candidates for existing owner paths, or fixture candidates until their owner docs define capability profile, redaction/secret/PII policy, retention or test-environment rules when needed, fixture coverage, fallback behavior, and no projection-as-canonical dependency. They must not be required to run Kernel Smoke or to claim MVP close readiness.

## The first proof

The first implementation/proof target is Kernel Smoke: the smallest runnable path that proves Harness can make and enforce one authority decision.

Kernel Smoke proves the authority loop, not the full MVP, not template completeness, and not broad automation.

It should show:

- one registered project and reference surface
- one Task with current state and gates
- one active scoped Change Unit
- `prepare_write` blocks writes without authority and allows a compatible scoped write
- allowed `prepare_write` creates a durable single-use Write Authorization
- `record_run` consumes that authorization for one implementation or direct Run and records observed changes plus artifacts
- artifacts can be registered and linked to the run or evidence
- a minimal Evidence Manifest records support or insufficiency
- a minimal `TASK` projection is current or at least durably enqueued
- `close_task` blocks with structured blockers when evidence or decision requirements are missing
- the same behavior is executable through basic Core fixtures

Kernel Smoke is not the final MVP. It proves the write authority path is alive.

## The final MVP proof

The final proof is Agency-Hardened MVP. It is a later hardening and conformance target after Kernel Smoke, not the first implementation batch. It adds the remaining conformance needed for an agent to act with honest boundaries:

- Decision Packet quality and user-judgment routing
- separation between approvals, Decision Packets, and Write Authorizations
- residual-risk visibility before acceptance and close
- detached verification independence
- Manual QA records and QA blockers
- feedback-loop, TDD, stewardship, and context-hygiene validators
- projection and reconcile completeness
- recovery, export, and artifact integrity behavior
- later-boundary checks that keep broad automation out of MVP
- fixture coverage for required agency conformance

Agency-Hardened MVP is complete only when conformance proves behavior through Core state, events, artifacts, projections, and errors rather than rendered prose alone.

## Build reading path

Read the Build layer in this order:

1. [Implementation Overview](implementation-overview.md) for the system you are building.
2. [First Runnable Slice](first-runnable-slice.md) for the smallest proof to implement first.
3. [MVP Plan](mvp-plan.md) for staged delivery from MVP-0 through MVP-5.

Then use the reference docs and current owners for exact behavior:

- [Kernel Reference](../reference/kernel.md) for entities, gates, state logic, `prepare_write`, and `close_task`.
- [Runtime Architecture Reference](../reference/runtime-architecture.md) for runtime spaces, Core flow, artifacts, projection/reconcile, and guarantee levels.
- [MCP API And Schemas](../reference/mcp-api-and-schemas.md) for public resources, tools, schemas, errors, artifact refs, idempotency, and state conflict behavior.
- [Storage And DDL](../reference/storage-and-ddl.md) for runtime layout, DDL, migrations, locks, artifacts, baselines, projection jobs, and validator-run storage.
- [Operations And Conformance Reference](../reference/operations-and-conformance.md) for operator semantics and fixture expectations.
