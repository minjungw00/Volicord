# Build: Implementation Overview

## What this document helps you do

This document tells implementers what to build before they dive into the full reference specs. It is the bridge between the reader-centered docs and the detailed contracts in the kernel, runtime, MCP, storage, projection, and conformance references.

This is planning documentation; it does not authorize runtime or server implementation before the redesigned docs are accepted.

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

## Documentation acceptance status

This is a maintainer-updated documentation handoff marker. It is not a Reference contract, conformance result, generated operational record, or runtime implementation authorization. Do not infer acceptance from the checklist below; maintainers must change this table deliberately.

| Question | Current status |
|---|---|
| Is documentation maintenance still active? | Yes. The redesigned docs are ready for human review, and implementation handoff has not been accepted yet. |
| Are docs accepted for first runtime-batch planning? | No. First runtime-batch planning may not begin until maintainers change this row to Yes after the checkpoint below is satisfied. |
| Has runtime/server implementation started? | No. This repository still contains documentation, not Harness runtime/server implementation. |
| Are there open follow-up docs issues? | Yes. See the maintainer-updated follow-up list below before changing acceptance status. |

### Known follow-up docs issues

Maintainers update this list. These entries route documentation maintenance only; they do not define Reference contracts, conformance results, generated operational records, or runtime/server authorization.

- Open - repo-level `.agents` / `.codex` instruction audit: decide whether repo-level `.agents` and `.codex` placeholders are intended instruction surfaces, generated connector artifacts, or removable placeholders. Owner docs: [Authoring Guide entrypoint rule](../maintain/authoring-guide.md#entrypoint-rule) and [Surface Cookbook: Codex](../reference/surface-cookbook.md#codex). `TODO_DECISION`: choose the owner and expected handling before docs acceptance. `TODO_IMPLEMENT`: update or remove stale repo-level instruction placeholders after that decision.
- Resolved in this batch - User Guide opening convention alignment: verified that [User Guide](../use/user-guide.md) already follows the [Authoring Guide standard opening pattern](../maintain/authoring-guide.md#standard-opening-pattern) and keeps the no-startup-phrase convention aligned with [Agent Session Flow](../use/agent-session-flow.md).

## Implementation handoff checkpoint

Use this checkpoint to decide what must be true before maintainers can switch the documentation acceptance status from documentation maintenance to first runtime-batch planning. It is a planning handoff only: it does not authorize runtime or server implementation by itself, and it does not define exact schemas, DDL, fixture semantics, or runtime contracts.

First implementation planning may start only when all of these are true:

- The final docs-maintenance drift pass is complete, or remaining known gaps are recorded as `TODO_DECISION` or `TODO_IMPLEMENT` in the relevant owner docs. Docs-maintenance remains a read-only documentation check; see [Authoring Guide](../maintain/authoring-guide.md#docs-maintenance-checks) and [Operations And Conformance Reference](../reference/operations-and-conformance.md#docs-maintenance-profile).
- The local-only MCP exposure baseline is accepted for MVP. Remote, shared, tunneled, or non-loopback exposure remains outside the MVP baseline unless owner docs promote and prove a connector profile; see [Runtime Architecture](../reference/runtime-architecture.md#local-access-expectations) and [MCP API And Schemas](../reference/mcp-api-and-schemas.md#mcp-boundary-and-caller-trust).
- The Core-only mutation model is accepted: state-changing work goes through Core, while resources, projections, reports, and diagnostics remain read-only or derived unless a Core path commits state. See [Core process model](../reference/runtime-architecture.md#core-process-model) and [State transaction flow](../reference/runtime-architecture.md#state-transaction-flow).
- The Kernel Smoke fixture queue is identified as the first runtime conformance authoring order. Exact fixture format, assertions, and catalog semantics stay in [Operations And Conformance Reference](../reference/operations-and-conformance.md#kernel-smoke-authoring-queue).
- The first runnable slice remains local, single-project, single-reference-surface, and fixture-proven. Use [First Runnable Slice](first-runnable-slice.md) for the planning checklist.
- Post-MVP features remain outside MVP unless promoted by owner docs through the [Roadmap promotion rule](../roadmap.md#promotion-rule).

This handoff does not promote roadmap items, dashboards, Browser QA Capture automation, Context Index, a broad connector marketplace, remote MCP exposure, preventive guard expansion, or parallel orchestration into MVP. Keep exact contracts in Reference docs and use this section only as the short readiness checkpoint.

## Main idea

Build the smallest local Core authority path first, then harden it through evidence, projections, conformance, and operator recovery.

Start with canonical state, `task_events`, artifact refs, Core tool behavior, and the minimal reference surface and MCP reachability needed to exercise that path. Treat projection-template polish, dashboards, indexes, broad connector ecosystems or marketplaces, surface-specific connector automation, hook expansion, Browser QA automation, and broad automation as non-authoritative things that read from or wrap that authority loop after it exists.

If a proposed implementation starts with projection template polish, a dashboard, a Context Index, a connector marketplace, hook expansion, or broad automation lanes, it is starting in the wrong place.

## Proof boundaries

| Boundary | What it proves | What the user or operator can observe |
|---|---|---|
| Kernel Smoke | One local Task can go through the Core authority loop: scoped write decision, Write Authorization, `record_run`, artifact-backed evidence, status, minimal projection freshness, and close blockers. | Status shows the active Task, gates, Change Unit, evidence, blockers, and projection freshness. Out-of-scope work is blocked, compatible scoped work is authorized and consumed once, and close refuses missing evidence or required decisions. |
| Agency-Hardened MVP | The local reference MVP handles user judgment, approvals, detached verification, Manual QA, residual risk, reconcile, recovery, export, and conformance with honest boundaries. | Fixtures and operator entrypoints show why work can or cannot continue, verify, accept, export, recover, or close through the same Core records and errors. |
| Post-MVP roadmap | Later surfaces or automation can be considered only after the local kernel and agency proof are stable. | Optional capabilities remain read-only, display-only, metadata-only, or artifact-candidate-only until an owner promotes them through the [Roadmap promotion rule](../roadmap.md#promotion-rule) with exact contracts and fixtures. |

## What you are building

Harness MVP is a local authority kernel for AI-assisted product work. The first implementation should be one local system with clear internal modules, not a distributed platform.

### Local Server / Process

Build one local Harness server or process that exposes the MCP boundary, owns Core transitions, reads and writes the runtime home, and runs validators, projection enqueueing, reconcile, recovery, export, and conformance entrypoints through the same Core rules.

The MVP can be one process with modules. It does not need separate services for Core, projection, validation, and operator tools.

### Core

Core is the only path that mutates canonical operational state. It must:

- validate tool envelopes, idempotency keys, and expected state versions
- acquire the relevant project or task lock
- read current records
- run Core checks and validators
- update current records and append `task_events` in one transaction
- enqueue projection work after state changes
- return blockers and refs that explain the result

Agents, operator commands, projectors, and recovery flows must either enter through Core or preserve the same Core compatibility rules.

### State Store

The state store keeps canonical operational state: project state, Tasks, gates, Change Units, Decision Packets, approvals, Write Authorizations, Runs, evidence manifests, Eval records, Manual QA records, residual risks, projection jobs, reconcile items, and `task_events`.

Do not design this from scratch in the Build layer. Storage details and DDL are owned by [Storage And DDL](../reference/storage-and-ddl.md).

### Artifact Store

The artifact store keeps durable evidence files and integrity metadata. Raw artifacts may include diffs, logs, screenshots, bundles, manifests, checkpoints, export components, or other evidence files.

The artifact store is not a loose file dump. Every artifact that supports Harness state needs a registered artifact ref, hash, size, redaction state, and relation to the Task or owner record that uses it.

### MCP API

The MCP server exposes read resources and public tools. MCP resources are read-only. State-changing work goes through public tools and Core.

For the first build path, prioritize:

- status and active Task reads
- intake or Task creation
- next-action guidance
- `prepare_write`
- `record_run`
- artifact registration through the tool flows that need it
- evidence manifest updates
- `close_task` blocker behavior

The public request and response contracts belong to [MCP API And Schemas](../reference/mcp-api-and-schemas.md).

### Projections

Projections are human-readable views derived from state records and artifact refs. They are not canonical state.

Build projection output from the Core source records it depends on, such as Task, gate, Run, artifact, evidence, Eval, QA, and other owner records after those records exist. A minimal `TASK` projection freshness or enqueueing path can be part of Kernel Smoke, but projection templates cannot create authority, satisfy evidence, replace state, shape the state model, or become the first proof.

The first runnable slice may enqueue a minimal `TASK` projection job or render a minimal `TASK` projection. The final MVP must support MVP-required `ProjectionKind` values when their source records exist: `TASK`, `APR`, `RUN-SUMMARY`, `EVIDENCE-MANIFEST`, `EVAL`, and `DIRECT-RESULT`.

Projection failure must not roll back committed Core state. It should mark projection freshness or job status and leave recovery or reconcile to a later action.

### Operator Commands

Operator entrypoints are surfaces over Core behavior, not a second state model. Build them as command-independent capabilities first:

- connect or register a project
- report doctor/readiness status
- serve or expose the MCP boundary
- refresh projections
- reconcile human edits or managed-block drift
- recover interrupted or stale operational state
- export state, projections, and artifact refs
- check artifact integrity
- run conformance fixtures

Exact command names and flags can come later. The important part is that operator behavior uses the same Core state, events, artifacts, projections, and errors as MCP tools.

## What you are not building yet

Keep the first implementation narrow. Do not build these as MVP prerequisites:

- dashboard or rich hosted UI as an authority path
- broad connector ecosystem
- Context Index as authority or read/write prerequisite
- Browser QA Capture as required automation or acceptance replacement
- Cross-Surface Verification as a required assurance path
- native hook expansion beyond a concrete reference-surface capability
- Advanced Sidecar Watcher as required enforcement
- Local Derived Metrics as MVP-critical state
- team workflow, shared workspaces, permissions, or profile import/export
- parallel orchestration automation
- preventive guard expansion unless the reference surface proves a concrete pre-tool blocking path

MVP may display cooperative or detective guard/freeze status and may hold or narrow work through existing Change Unit, Autonomy Boundary, and `prepare_write` behavior. Surface labels do not upgrade the stored guarantee level.

Useful later capabilities can appear only as read-only displays, metadata, artifact candidates for existing owner paths, or fixture candidates until their owner docs define capability profile, redaction/secret/PII policy, retention or test-environment rules when needed, fixture coverage, fallback behavior, and no projection-as-canonical dependency.

## The first proof

The first proof is Kernel Smoke: the smallest runnable path that proves Harness can make and enforce one authority decision.

Kernel Smoke proves the authority loop, not the full MVP, not template completeness, and not broad automation.

It should show:

- one registered project and reference surface
- one Task with current state and gates
- one active scoped Change Unit
- `prepare_write` blocks writes without authority and allows a compatible scoped write
- allowed `prepare_write` creates a durable Write Authorization
- `record_run` consumes that authorization for one implementation or direct Run
- artifacts can be registered and linked to the run or evidence
- a minimal Evidence Manifest records support or insufficiency
- a minimal `TASK` projection is current or at least durably enqueued
- `close_task` blocks when evidence or decision requirements are missing
- the same behavior is executable through basic Core fixtures

Kernel Smoke is not the final MVP. It proves the write authority path is alive.

## The final MVP proof

The final proof is Agency-Hardened MVP. It adds the remaining conformance needed for an agent to act with honest boundaries:

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
- [MCP API And Schemas](../reference/mcp-api-and-schemas.md) for public resources, tools, schemas, errors, and artifact refs.
- [Storage And DDL](../reference/storage-and-ddl.md) for runtime layout, DDL, migrations, locks, artifacts, baselines, projection jobs, and validator-run storage.
- [Operations And Conformance Reference](../reference/operations-and-conformance.md) for operator semantics and fixture expectations.
