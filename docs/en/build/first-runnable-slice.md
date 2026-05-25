# Build: First Runnable Slice

## What this document helps you do

This document turns the Build overview into the smallest runnable proof an implementer should plan first.

This is planning documentation; it does not authorize runtime or server implementation before the redesigned docs are accepted.

## Read this when

- You are planning Kernel Smoke.
- You need a checklist for the first end-to-end authority path.
- You want to review whether a proposed first slice proves enough without becoming the full MVP.

## Before you read

Read [Implementation Overview](implementation-overview.md) first. For storage and DDL details, use [Storage And DDL](../reference/storage-and-ddl.md).

## Main idea

Prove one Task can move through the Core state, `task_events`, and artifact path for scoped write authority, Run recording, artifact-backed evidence, status, minimal projection freshness, and close blockers before building the wider MVP.

## Goal

Build the smallest Harness slice that can prove authority over one local Task. The slice should create one project, one Task, one active Change Unit, one allowed write decision, one recorded Run, one registered artifact, one minimal Evidence Manifest, and one close blocker.

This is a command-independent implementation guide. It describes capabilities and observable behavior, not CLI syntax.

Do not include or duplicate full DDL here. Storage details and DDL are owned by [Storage And DDL](../reference/storage-and-ddl.md).

The first slice is deliberately not a projection-template-polish milestone, dashboard milestone, broad connector ecosystem or marketplace milestone, multi-surface connector expansion, Context Index, Browser QA Capture system, hook expansion, or parallel automation path. It still includes the one reference surface and minimal MCP reachability needed for Kernel Smoke. The excluded items can only wrap or extend the authority loop after the Core records and transitions are real.

## Success story

An implementer can run a local Harness process against a temporary product repository and observe this story:

1. Harness registers the project and reference surface.
2. A Task is created with current state and initial gates.
3. A Change Unit scopes the intended product write.
4. A write outside scope is blocked.
5. A write inside scope receives a durable Write Authorization from `prepare_write`.
6. A direct or implementation Run records the write and consumes the authorization.
7. A diff or log artifact is registered and linked to the Run.
8. A minimal Evidence Manifest references the Run and artifact.
9. Status reads show the current Task, gates, write authority, evidence state, blockers, and projection freshness without mutating state.
10. A `TASK` projection is current or durably queued for rendering.
11. Close is blocked when evidence or decision requirements are still missing.

Passing this story means the kernel authority path works. It does not mean the MVP is agency-hardened.

The observable result can be plain. A user or operator should be able to see the current Task, why a write is blocked or allowed, which Write Authorization was consumed, which artifact backs the Run, whether the Evidence Manifest is sufficient, whether the `TASK` projection is fresh or queued, and why close still blocks.

## Build order

### 1. Runtime Home Bootstrap

Build enough runtime home support to create local Harness authority outside chat history and outside generated Markdown.

Checklist:

- Create or select a configurable runtime home.
- Initialize the registry store, one project runtime area, one project state store, and an artifact store.
- Record a project-level state version before project-scoped mutations depend on it.
- Register one reference surface with an honest cooperative or detective guarantee level.
- Provide a readiness read that can report whether the runtime home, project state, and artifact store exist.

Done when:

- A fresh environment can be initialized repeatedly without creating duplicate authority records.
- A read-only status call can report "no active Task" from Core state.

### 2. Project Registration

Register exactly one local product repository before implementing multi-project concerns.

Checklist:

- Store the project id, display name, repo root, runtime path, and static project configuration.
- Connect the project to the reference surface.
- Keep static project configuration separate from current Task state.
- Make registration idempotent for the same project identity.

Done when:

- Core can resolve the current project for all later Task-scoped actions.
- Doctor/readiness can distinguish an unregistered repo from a registered but idle repo.

### 3. One Task Record

Create the first Task through Core or a fixture seed path that uses the same validation rules.

Checklist:

- Store mode, lifecycle phase, result, close reason, assurance level, state version, current summary, and gate state.
- Initialize gates conservatively for the selected mode.
- Append a task event when the Task is created or changed.
- Expose active Task reads through status.

Done when:

- The system can show one active Task and its state version.
- A state-changing request with a stale expected state version is rejected or returns a state conflict.

### 4. One Change Unit

Add one active Change Unit to scope product writes.

Checklist:

- Record the intended operation, allowed paths, allowed tools or command classes, sensitive categories, completion conditions, and evidence expectations.
- Record a minimal Autonomy Boundary: what the agent may do, what requires user judgment, and stop conditions.
- Attach the Change Unit to the active Task and make it the active write scope.
- Keep dependency metadata optional unless the first slice needs it for ordering, visibility, or close blockers.

Done when:

- Status can explain what may change and what still requires user judgment.
- Product writes without an active compatible Change Unit cannot receive write authority.

### 5. `prepare_write` Allow/Block

Implement the first meaningful gate.

Checklist:

- Validate the request envelope, idempotency key, project id, Task id, and expected state version.
- Resolve the active Task and active Change Unit.
- Check intended paths, tools, commands, network targets, secrets, and sensitive categories against the Change Unit.
- Check baseline freshness at the level needed for the first slice.
- Check approval and Decision Packet requirements enough to block missing authority.
- Check surface capability honestly and report cooperative or detective limits.
- Return a blocker when scope, state version, approval, decision, baseline, or capability is incompatible.
- When allowed, create a durable single-use Write Authorization.
- On idempotent replay of the same committed request, return the committed response rather than creating a second authorization.

Done when:

- Missing active Change Unit blocks.
- Out-of-scope intended paths block.
- A compatible scoped write returns a Write Authorization ref.
- No product write can be recorded as implementation/direct work without that ref.

### 6. `record_run`

Record one direct or implementation Run and consume the Write Authorization.

Checklist:

- Require a compatible, unexpired, unconsumed Write Authorization for direct or implementation Runs that record product writes.
- Mark the Write Authorization consumed exactly once on successful commit.
- Record actor, surface, kind, intended operation, observed changes, command results, artifact refs, summary, and Run status.
- Detect observed changes outside the authorization and route them to a violation, blocker, stale evidence, or Decision Packet path.
- Append `task_events` in the same transaction as current record updates.

Done when:

- `record_run` without write authority is blocked.
- `record_run` with compatible authority succeeds once.
- Replaying the same committed Run request is idempotent.
- A second distinct Run cannot reuse the consumed authorization.

### 7. Artifact Registration

Register the first durable evidence file.

Checklist:

- Accept either an approved staged file or an existing committed artifact ref.
- Verify hash and size when provided.
- Apply redaction or secret omission before final storage.
- Store the artifact bytes in the artifact store.
- Store artifact metadata and relation to the Task, Run, evidence manifest, or other owner record.
- Return an `ArtifactRef` that uses the public shape from the API docs.

Done when:

- A Run can cite a registered artifact.
- Artifact integrity can be checked from state plus stored bytes.
- Raw secrets are omitted or blocked rather than stored as evidence.

### 8. Minimal Evidence Manifest

Create the first evidence summary from records and artifact refs.

Checklist:

- Map at least one completion condition or acceptance criterion to Run refs and artifact refs.
- Distinguish supported, unsupported, not applicable, partial, sufficient, stale, and blocked evidence at the level needed for close blockers.
- Avoid treating chat text or projection prose as evidence.
- Update the evidence gate from the manifest and related records.

Done when:

- A completed Run can produce partial or sufficient evidence state.
- Missing required evidence causes close to block.

### 9. Minimal Status Resource

Expose the current work state without mutation.

Checklist:

- Read project, active Task, current gates, active Change Unit, write authority summary, active Decision Packet refs, evidence status, close blockers, and projection freshness.
- Include enough Journey Card-style context for a user or agent to resume.
- Do not append events, enqueue projections, create artifacts, satisfy gates, authorize writes, or close the Task from a read.

Done when:

- Repeated status reads return the same state version unless another action changed state.
- A stale projection or missing evidence is reported as status, not silently repaired.

### 10. Minimal `TASK` Projection Or Projection Enqueue

Implement the smallest projection behavior that proves state and readable output are separated.

Do this after the Task, gate, Run, artifact, and evidence records exist. Do not let the projection template shape the state model, and do not add template polish or additional renderer-first work just to make the first slice look complete.

Checklist:

- Enqueue a `TASK` projection job when Task state changes, or render a minimal managed `TASK` projection after commit.
- Track source state version and projection freshness.
- Treat projection render failure as projection failure, not Core state failure.
- Preserve the rule that Markdown projections are derived views, not source of truth.

Done when:

- A Task-changing action returns or records projection freshness.
- A projection failure can be represented without rolling back the Task mutation.

### 11. Close Blocker Smoke

Make close refuse to finish work when required authority or evidence is missing.

Checklist:

- Implement enough `close_task` state logic to inspect gates, evidence, Decision Packets, approval state, residual-risk visibility, QA, verification, and acceptance at a minimal level.
- Return structured blockers rather than only prose.
- Prove at least evidence-insufficient and decision-required close blockers.
- Allow a clean self-checked direct close only when the direct path has sufficient state and no required blocker remains.

Done when:

- A Task with missing required evidence cannot close successfully.
- A Task with an unresolved blocking Decision Packet cannot close successfully.
- Close results are based on canonical records, not rendered reports.

## What this proves

The first runnable slice proves:

- Core can own state transitions.
- The state store and `task_events` are usable.
- A scoped Change Unit is required for product writes.
- `prepare_write` is the authority decision for a write attempt.
- Write Authorization is durable and single-use.
- `record_run` consumes write authority and records observed work.
- Artifacts and evidence can be registered without relying on chat.
- Status is read-only.
- Projections are derived and failure-isolated.
- Close can block when required evidence or decisions are missing.

## What this does not prove yet

This slice does not prove:

- full Decision Packet quality
- full approval lifecycle and approval drift handling
- detached verification independence
- Manual QA policy coverage
- residual-risk visibility before acceptance and close
- feedback-loop and TDD conformance
- codebase stewardship and context-hygiene coverage
- full projection and reconcile behavior
- projection template completeness
- recover, export, artifact integrity, and broad operator smoke
- dashboard, Context Index, connector marketplace, or Browser QA Capture behavior
- preventive guard behavior
- parallel orchestration or team workflow

Those belong to the Agency-Hardened MVP path in [MVP Plan](mvp-plan.md).

## Fixtures to write

Write fixtures that drive Core behavior and assert state, events, artifacts, projections, and errors. Do not assert success by matching rendered prose.

Minimum first-slice fixtures:

- no-active-task status read returns idle state and appends no events
- project bootstrap creates project state and reference surface
- intake or seeded Task creates one active Task and initial gates
- active Change Unit scopes one intended path
- `prepare_write` blocks when no active Change Unit exists
- `prepare_write` blocks an out-of-scope path
- `prepare_write` allows a compatible scoped write and creates one Write Authorization
- idempotent `prepare_write` replay returns the committed authorization response
- `record_run` blocks when write authority is missing
- `record_run` consumes a compatible Write Authorization and records observed changes
- second distinct `record_run` cannot reuse a consumed authorization
- artifact registration stores hash, redaction state, and owner relation
- Evidence Manifest records partial and sufficient evidence states
- status read reports gates, evidence, write authority, and projection freshness without mutation
- Task mutation enqueues or renders a `TASK` projection
- projection failure does not roll back committed state
- `close_task` blocks evidence-insufficient close
- `close_task` blocks unresolved decision close

Use the fixture shape and comparison rules in [Operations And Conformance Reference](../reference/operations-and-conformance.md#conformance-fixture-format).

## Reference docs to consult

- [Kernel Reference](../reference/kernel.md): Task, Change Unit, Decision Packet, gates, `prepare_write`, Write Authorization, `record_run` semantics, and `close_task`.
- [Runtime Architecture Reference](../reference/runtime-architecture.md): three spaces, Core process model, transaction flow, artifact store, projection/reconcile, guarantee levels, and failure handling.
- [MCP API And Schemas](../reference/mcp-api-and-schemas.md): public resources, tool envelopes, request/response schemas, error taxonomy, artifact refs, and `ProjectionKind`.
- [Storage And DDL](../reference/storage-and-ddl.md): runtime layout, DDL, migrations, locks, artifacts, baselines, projection jobs, and validator-run storage.
- [Operations And Conformance Reference](../reference/operations-and-conformance.md): operator semantics, conformance staging, fixture format, execution, and assertion rules.
