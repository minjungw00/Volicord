# Build: First Runnable Slice

## What this document helps you do

This document turns the Build overview into the smallest runnable product MVP an implementer should plan first.

This is planning documentation. It does not authorize runtime/server implementation, generated operational files, executable fixtures, or runtime data before the documentation set is accepted for implementation planning. The first product MVP target is v0.1 Kernel MVP, exercised by the Kernel Smoke conformance profile: one local process with modules proving one authority loop. v0.2 through v0.4 are staged packs toward the Agency-Hardened MVP reference conformance target. v1+ Expansion remains roadmap scope unless owner docs promote and prove it.

## Read this when

- You are planning v0.1 Kernel MVP.
- You need a checklist for the first end-to-end authority path.
- You want to review whether a proposed first slice proves enough without becoming the full MVP.

## Before you read

Read [Implementation Overview](implementation-overview.md) first, including its [Documentation Acceptance Status](implementation-overview.md#documentation-acceptance-status). That handoff table is the Build entry gate; until maintainers accept first runtime-batch planning, this slice remains planning-only. For storage and DDL details, use [Storage And DDL](../reference/storage-and-ddl.md). For v1+ Expansion candidates, use the [Roadmap](../roadmap.md).

## Main idea

Prove one Task can move through the Core state, `task_events`, and artifact path for scoped write authority, Run recording, artifact-backed evidence, basic Decision Packet blocking, status/next, minimal projection freshness or enqueueing, and close blockers before building the wider packs.

The loop stays intentionally small: `prepare_write` decides product-write authority, the returned Write Authorization is durable and single-use, `record_run` consumes it for one compatible direct Run or implementation Run while recording observed changes and artifacts, and `close_task` decides completion with structured blockers. Use [Kernel Reference](../reference/kernel.md#prepare_write) and [MCP API And Schemas](../reference/mcp-api-and-schemas.md#public-tools) for the exact contracts.

## Goal

Plan v0.1 Kernel MVP: the smallest Harness path that can prove authority over one local Task. The slice should create one project, one Task with direct/work/advisor mode basics, one active Change Unit, one basic Decision Packet lifecycle, one allowed `prepare_write` decision, one single-use Write Authorization, one compatible recorded Run that consumes it, one registered artifact, one minimal Evidence Manifest, status/next reads, one minimal `TASK` projection or projection enqueue, and one structured close blocker.

This is a command-independent implementation guide. It describes capabilities and observable behavior, not CLI syntax.

Do not include or duplicate full DDL here. Storage details and DDL are owned by [Storage And DDL](../reference/storage-and-ddl.md).

The first slice is deliberately not Agency-Hardened MVP as a whole, a projection-template-polish milestone, dashboard or hosted-workflow-UI milestone, broad connector ecosystem or marketplace milestone, multi-surface connector expansion, Context Index, Browser QA Capture system, Cross-Surface Verification path, hook expansion, preventive guard expansion, Advanced Sidecar Watcher, Local Derived Metrics surface, team workflow, or parallel automation path. It still includes the one reference surface and minimal MCP reachability needed for v0.1 Kernel MVP. The excluded items can only read from, display, provide artifact candidates for existing owner paths, or wrap the authority loop after the Core records and transitions are real. Any durable artifact registration or attachment still follows existing Core/MCP owner paths or a future promoted owner contract under the [Roadmap promotion rule](../roadmap.md#promotion-rule).

## Success story

An implementer can run a local Harness process against a temporary product repository and observe this story:

1. Harness registers the project and reference surface.
2. A Task is created with current state, initial gates, and direct/work/advisor mode basics.
3. A Change Unit scopes the intended product write.
4. A basic Decision Packet can be requested, recorded, shown in status/next, and used as a blocker when unresolved.
5. A write outside scope is blocked.
6. A write inside scope receives a durable single-use Write Authorization from `prepare_write`.
7. A direct Run or implementation Run for a work task records the write and consumes that authorization once.
8. A diff or log artifact is registered and linked to the Run.
9. A minimal Evidence Manifest references the Run and artifact.
10. Status and next reads show the current Task, gates, write authority, evidence state, Decision Packet refs, blockers, and projection freshness without mutating state.
11. A `TASK` projection is current or durably queued for rendering.
12. `close_task` is blocked with structured blockers when evidence or decision requirements are still missing.

Passing this story means v0.1 Kernel MVP works. It does not mean the MVP is agency-hardened, and it does not pull later automation into the MVP.

The observable result can be plain. A user or operator should be able to see the current Task, mode basics, why a write is blocked or allowed, which Decision Packet is unresolved or resolved, which Write Authorization was consumed, which artifact backs the Run, whether the Evidence Manifest is sufficient, whether the `TASK` projection is fresh or queued, what next action is safe, and why close still blocks.

## Doc-level acceptance checks

Use these checks to review the planned v0.1 Kernel MVP slice before executable fixtures exist, and again when mapping the slice to the [Kernel Smoke Authoring Queue](../reference/conformance-fixtures.md#kernel-smoke-authoring-queue). They are planning checks, not fixture body fields, schema additions, DDL, or runtime authorization.

A proposed first runnable slice is acceptable when:

- It remains local, single-project, single-reference-surface, and focused on one Task authority loop.
- It stays planning-only until the [Documentation Acceptance Status](implementation-overview.md#documentation-acceptance-status) explicitly allows first runtime-batch planning.
- It proves exactly one scoped write path: active Task, direct/work/advisor mode basics, active Change Unit, basic Decision Packet lifecycle, `prepare_write` allow/block, durable single-use Write Authorization, `record_run` consumption, artifact registration, minimal Evidence Manifest, status/next, minimal `TASK` projection freshness or enqueueing, and structured close blockers.
- It blocks or refuses missing authority: no active Change Unit, out-of-scope intended path, missing Write Authorization for product-write Runs, reuse of a consumed Write Authorization, missing required evidence, and unresolved blocking Decision Packet.
- It keeps status reads, projections, reports, and generated prose downstream from Core records; none of them authorize writes, satisfy evidence, close work, or repair state by being read.
- It links strict fixture body shape, assertion modes, primary errors, artifact refs, projection assertions, and seed validation to [Conformance Fixtures Reference](../reference/conformance-fixtures.md#conformance-fixture-format) instead of copying those contracts here.
- It names excluded capabilities as not yet proven by v0.1 Kernel MVP, not as failed v0.1 requirements.

The build order below is a post-acceptance planning sequence. The headings use implementation verbs so the future runtime batch is easy to execute, but this document still does not authorize runtime/server implementation, generated operational files, executable fixtures, or runtime data before documentation acceptance.

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
- Initialize gates conservatively for direct, work, and advisor mode basics.
- Append a task event when the Task is created or changed.
- Expose active Task reads through status.

Done when:

- The system can show one active Task and its state version.
- Advisor mode can stay read-only, while direct/work modes can participate in the scoped authority loop when a compatible Change Unit exists.
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

### 5. Basic Decision Packet Lifecycle

Add only enough Decision Packet behavior to prove that user-owned or material technical judgment can be represented and block the selected loop.

Checklist:

- Create or seed a canonical Decision Packet record, or exercise the public request/record route that creates and resolves one. A Decision Packet candidate may feed that route, but a candidate alone does not satisfy `decision_gate`, remove a blocker, or pass a runtime fixture.
- Record whether the decision is unresolved, resolved, deferred, or blocking according to the owner contract.
- Surface active Decision Packet refs through status and next-action reads.
- Let `prepare_write` or `close_task` block when the required decision is missing or unresolved.
- Keep full Decision Packet quality, option completeness, residual-risk impact, and judgment separation for the later Agency Pack.

Done when:

- Status and next can show an active required decision without mutating state.
- A required unresolved Decision Packet can block write authority or close.
- A compatible recorded decision can remove that specific blocker without granting write authority by itself.

### 6. `prepare_write` Allow/Block

Implement the first meaningful gate.

Checklist:

- Validate the request envelope, idempotency key, project id, Task id, and expected state version.
- Resolve the active Task and active Change Unit.
- Check intended paths, tools, commands, network targets, secrets, and sensitive categories against the active Change Unit.
- Check the intended operation against the active Change Unit Autonomy Boundary.
- Check baseline freshness at the level needed for the first slice.
- Check Decision Packet requirements and, when an existing owner rule or seeded owner record says a sensitive-action Approval is missing or incompatible, block or hold the selected path without proving the full Approval lifecycle.
- Check design-policy preconditions that apply before writing.
- Check surface capability honestly and report cooperative or detective limits.
- Return a blocker when scope, state version, decision, baseline, capability, or an applicable seeded/owner-defined sensitive-action Approval requirement is incompatible.
- When allowed, create a durable single-use Write Authorization compatible with one later direct Run or implementation Run.
- On idempotent replay of the same committed request, return the committed response rather than creating a second authorization.

Done when:

- Missing active Change Unit blocks.
- Out-of-scope intended paths block.
- A compatible scoped write returns a Write Authorization ref.
- No product write can be recorded by a direct Run or implementation Run without that ref.

### 7. `record_run`

Record one direct Run or implementation Run and consume the Write Authorization.

Checklist:

- Require a compatible, unexpired, unconsumed Write Authorization for direct or implementation Runs that record product writes.
- Mark the Write Authorization consumed exactly once on successful commit.
- Record actor, surface, kind, intended operation, observed changes, command results, artifact refs, summary, and Run status.
- Validate observed changed paths, created/deleted paths, artifact inputs and refs, command results, and Run summary against the authorization and active Change Unit.
- Detect observations outside the authorization and route them to a violation, blocker, stale evidence, or Decision Packet path.
- Append `task_events` in the same transaction as current record updates.

Done when:

- `record_run` without write authority is blocked.
- `record_run` with compatible authority succeeds once.
- Replaying the same committed Run request is idempotent.
- A second distinct Run cannot reuse the consumed authorization.

### 8. Artifact Registration

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

### 9. Minimal Evidence Manifest

Create the first evidence summary from records and artifact refs.

Checklist:

- Map at least one completion condition or acceptance criterion to Run refs and artifact refs.
- Distinguish supported, unsupported, not applicable, partial, sufficient, stale, and blocked evidence at the level needed for close blockers.
- Avoid treating chat text or projection prose as evidence.
- Update the evidence gate from the manifest and related records.

Done when:

- A completed Run can produce partial or sufficient evidence state.
- Missing required evidence causes close to block.

### 10. Minimal Status And Next Resources

Expose the current work state and next safe action without mutation.

Checklist:

- Read project, active Task, current gates, active Change Unit, write authority summary, active Decision Packet refs, evidence status, close blockers, projection freshness, and next safe action.
- Include enough Journey Card-style context for a user or agent to resume.
- Do not append events, enqueue projections, create artifacts, satisfy gates, authorize writes, or close the Task from a read.

Done when:

- Repeated status reads return the same state version unless another action changed state.
- Repeated next reads do not create authority or satisfy blockers.
- A stale projection or missing evidence is reported as status, not silently repaired.

### 11. Minimal `TASK` Projection Or Projection Enqueue

Implement the smallest projection behavior that proves state and readable output are separated.

`APR`, `RUN-SUMMARY`, `EVIDENCE-MANIFEST`, `EVAL`, and `DIRECT-RESULT` rendering are not v0.1 Kernel MVP completion criteria. They enter v0.2+ evidence/projection work and Agency-Hardened/reference MVP support when their source records exist or change.

Do this after the Task, gate, Run, artifact, and evidence records exist. Do not let the projection template shape the state model, and do not add template polish or additional renderer-first work just to make the first slice look complete.

Checklist:

- Enqueue a `TASK` projection job when Task state changes, or render a minimal managed `TASK` projection after commit.
- Track source state version and projection freshness.
- Treat projection render failure as projection failure, not Core state failure.
- Preserve the rule that Markdown projections are derived views, not source of truth.

Done when:

- A Task-changing action returns or records projection freshness.
- A projection failure can be represented without rolling back the Task mutation.

### 12. Close Blocker Smoke

Make close refuse to finish work when required authority or evidence is missing.

Checklist:

- Implement enough `close_task` state logic to inspect gates, evidence, Decision Packets, and any applicable seeded/owner-defined sensitive-action Approval blocker at a minimal level.
- Keep residual-risk accepted close, Manual QA, detached verification, acceptance, and full Approval lifecycle or drift handling for later packs.
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
- Direct, work, and advisor mode basics are represented without claiming full policy coverage.
- A scoped Change Unit is required for product writes.
- Basic Decision Packet lifecycle can expose and block on a required decision.
- `prepare_write` is the only product-write authorization decision point.
- Write Authorization is durable and single-use.
- `record_run` consumes write authority once and records observed work, artifacts, and summary.
- Artifacts and evidence can be registered without relying on chat.
- Status and next are read-only.
- Projections are derived and failure-isolated.
- `close_task` can block with structured blockers when required evidence or decisions are missing.

## What this does not prove yet

This slice does not prove the items below yet. These are not failed v0.1 Kernel MVP requirements; they are not-yet-proven capabilities for later packs or the v1+ Expansion roadmap:

- v0.2 Evidence & Projection Pack: full projection and reconcile behavior, projection template completeness, and evidence/projection coverage beyond the minimal loop.
- v0.3 Agency Pack: full Decision Packet quality, full Approval lifecycle and Approval drift handling, detached verification independence, Manual QA policy matrix, residual-risk visibility and accepted-close semantics, feedback-loop policy, TDD trace, codebase stewardship, stewardship validators, and context-hygiene coverage.
- v0.4 Operations Pack: release handoff, recover, export, artifact integrity, broad operator smoke, and large fixture suite coverage.
- v1+ Expansion: dashboard, hosted workflow UI, Context Index, connector marketplace, Browser QA Capture, Cross-Surface Verification, native hook expansion, Advanced Sidecar Watcher, Local Derived Metrics, preventive guard expansion, parallel orchestration, and team workflow.

Those belong either to later packs in [MVP Plan](mvp-plan.md) or to the v1+ Expansion [Roadmap](../roadmap.md), depending on the item.

## Fixtures to write

Write fixtures that drive Core behavior and assert state, events, artifacts, projections, and errors. Do not assert success by matching rendered prose.

Each runtime fixture should execute in an isolated runtime home and temporary Product Repository, seed its own starting records and files, run one Core or operator action, and compare the captured executable result. Fixture body fields, assertion modes such as `partial_deep` and `contains_ordered`, JSON `TEXT` validation, and owner-bound status value validation are owned by [Conformance Fixtures Reference](../reference/conformance-fixtures.md#conformance-fixture-format).

The list below is the v0.1 behavior checklist. Use the [Kernel Smoke Authoring Queue](../reference/conformance-fixtures.md#kernel-smoke-authoring-queue) for the practical order, seed guidance, stable event targets, artifact/projection assertions, and primary-error expectations.

Minimum first-slice fixtures:

- no-active-task status read returns idle state and appends no events
- project bootstrap creates project state and reference surface
- intake or seeded Task creates one active Task and initial gates
- direct/work/advisor mode basics are visible without creating write authority from mode labels alone
- active Change Unit scopes one intended path
- basic Decision Packet request, record, status/next visibility, and unresolved-decision blocker behavior works
- `prepare_write` blocks when no active Change Unit exists
- `prepare_write` blocks an out-of-scope path
- `prepare_write` allows a compatible scoped write and creates one Write Authorization
- idempotent `prepare_write` replay returns the committed authorization response
- `record_run` blocks when write authority is missing
- `record_run` consumes a compatible Write Authorization and records observed changes plus artifact-backed summary
- second distinct `record_run` cannot reuse a consumed authorization
- artifact registration stores hash, redaction state, and owner relation
- Evidence Manifest records partial and sufficient evidence states
- status and next reads report gates, evidence, write authority, Decision Packet refs, next safe action, and projection freshness without mutation
- Task mutation enqueues or renders a `TASK` projection
- projection failure does not roll back committed state
- `close_task` blocks evidence-insufficient close with a structured blocker
- `close_task` blocks unresolved decision close with a structured blocker

Use the fixture shape and comparison rules in [Conformance Fixtures Reference](../reference/conformance-fixtures.md#conformance-fixture-format). Do not add fields to the fixture body to express suite stage, authoring order, or docs-maintenance results.

## Reference docs to consult

- [Kernel Reference](../reference/kernel.md): Task, Change Unit, Decision Packet, gates, `prepare_write`, Write Authorization, `record_run` semantics, and `close_task`.
- [Runtime Architecture Reference](../reference/runtime-architecture.md): three spaces, Core process model, transaction flow, artifact store, projection/reconcile, guarantee levels, and failure handling.
- [MCP API And Schemas](../reference/mcp-api-and-schemas.md): public resources, tool envelopes, request/response schemas, error taxonomy, artifact refs, and `ProjectionKind`.
- [Storage And DDL](../reference/storage-and-ddl.md): runtime layout, DDL, migrations, locks, artifacts, baselines, projection jobs, and validator-run storage.
- [Operations And Conformance Reference](../reference/operations-and-conformance.md): operator semantics and conformance staging.
- [Conformance Fixtures Reference](../reference/conformance-fixtures.md): fixture format, execution, assertion rules, suite catalogs, and examples.
