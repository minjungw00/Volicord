# Build: First Runnable Slice

## What this document helps you do

This document turns the Build overview into the smallest runnable kernel slice an implementer should plan first.

This is planning documentation. It does not authorize runtime/server implementation, generated operational files, executable fixtures, or runtime data before the documentation set is accepted for implementation planning. The first runnable target is v0.1 Core Authority Slice, with Kernel Smoke as its narrow conformance authoring profile. It is an internal implementation milestone, not the user-facing MVP. The first product MVP target is v0.2 User-Facing Harness MVP.

## Read this when

- You are planning v0.1 Core Authority Slice.
- You need a checklist for the first end-to-end authority path.
- You want to review whether a proposed first slice is small enough to run without becoming the product MVP.

## Before you read

Read [Implementation Overview](implementation-overview.md) first, including its [Documentation Acceptance Status](implementation-overview.md#documentation-acceptance-status). That handoff table is the Build entry gate; until maintainers accept first runtime-batch planning, this slice remains planning-only. For storage and DDL details, use [Storage And DDL](../reference/storage-and-ddl.md). For staged delivery after this slice, use [MVP Plan](mvp-plan.md). For v1+ Expansion candidates, use the [Roadmap](../roadmap.md).

## Main idea

Prove one Task can move through the smallest Core authority record: project registration, one scope, one write authorization decision, one authorized Run, one evidence link, and one structured blocker/status response.

The first slice should show that Harness state is local, durable, and authoritative without trying to prove the whole user-facing product. It keeps `prepare_write` as the product-write authorization decision point, Write Authorization as durable and single-use, `record_run` as the place where one compatible Run consumes authority, and `close_task` or status as the place where missing evidence or required judgment can be reported as structured blockers.

Use [Kernel Reference](../reference/kernel.md#prepare_write) and [MCP API And Schemas](../reference/mcp-api-and-schemas.md#public-tools) for the exact contracts.

## Goal

Plan v0.1 Core Authority Slice: the smallest Harness path that can prove local authority over one Task.

The slice should create or seed:

- one project and one reference surface
- one Task
- one basic scope for the intended change
- one allowed `prepare_write` decision and at least one blocked decision
- one durable single-use Write Authorization
- one compatible recorded Run that consumes the authorization
- one artifact or evidence ref linked to the Run or evidence relation
- one minimal evidence state that can support or fail the selected claim
- one read-only status/next response
- one structured blocker/status response when scope, evidence, or required seeded user judgment is missing

This is a command-independent implementation guide. It describes capabilities and observable behavior, not CLI syntax. Do not include or duplicate full DDL here. Storage details and DDL are owned by [Storage And DDL](../reference/storage-and-ddl.md).

The first slice is deliberately not the User-Facing Harness MVP, the hardened local reference target as a whole, a projection-template-polish milestone, dashboard or hosted-workflow-UI milestone, broad connector ecosystem or marketplace milestone, multi-surface connector expansion, Context Index, Browser QA Capture system, Cross-Surface Verification path, hook expansion, preventive guard expansion, Advanced Sidecar Watcher, Local Derived Metrics surface, team workflow, export/recover path, release handoff path, or parallel automation path.

## Success story

An implementer can run a local Harness process against a temporary product repository and observe this story:

1. Harness registers one project and one reference surface.
2. A Task exists in Core state and changes append `task_events`.
3. A basic scope names the intended product change.
4. `prepare_write` blocks a missing or incompatible scope.
5. `prepare_write` allows one compatible scoped write and creates a durable single-use Write Authorization.
6. `record_run` records one compatible Run and consumes that authorization once.
7. One artifact or evidence ref is registered and linked to the Run or evidence relation.
8. Status and next reads show current Task, scope, write authority, evidence state, and blockers without mutating state.
9. Close or status output returns a structured blocker when required evidence, scope, or required seeded user judgment is missing.

Passing this story means v0.1 Core Authority Slice works. It does not mean users have experienced the Harness MVP yet. The user-facing MVP begins when ordinary requests are clarified into scope, judgment, evidence, close-readiness, acceptance, and residual-risk language.

## Doc-level acceptance checks

Use these checks to review the planned v0.1 Core Authority Slice before executable fixtures exist, and again when mapping the slice to the [Kernel Smoke Authoring Queue](../reference/conformance-fixtures.md#kernel-smoke-authoring-queue). They are planning checks, not fixture body fields, schema additions, DDL, or runtime authorization.

A proposed first runnable slice is acceptable when:

- It remains local, single-project, single-reference-surface, and focused on one Task authority loop.
- It stays planning-only until the [Documentation Acceptance Status](implementation-overview.md#documentation-acceptance-status) explicitly allows first runtime-batch planning.
- It proves exactly one scoped write path: active Task, one basic scope, `prepare_write` allow/block, durable single-use Write Authorization, `record_run` consumption, artifact/evidence link, read-only status/next, and structured blocker/status response.
- It blocks or refuses missing authority: missing scope, out-of-scope intended path, missing Write Authorization for product-write Runs, reuse of a consumed Write Authorization, missing required evidence, or missing required seeded user judgment.
- It keeps status reads, generated prose, and any projection output downstream from Core records; none of them authorize writes, satisfy evidence, close work, or repair state by being read.
- It links strict fixture body shape, assertion modes, primary errors, artifact refs, projection assertions, and seed validation to [Conformance Fixtures Reference](../reference/conformance-fixtures.md#conformance-fixture-format) instead of copying those contracts here.
- It names excluded capabilities as not yet proven by v0.1 Core Authority Slice, not as failed first-slice requirements.

The build order below is a post-acceptance planning sequence. The headings use implementation verbs so the future runtime batch is easy to execute, but this document still does not authorize runtime/server implementation, generated operational files, executable fixtures, or runtime data before documentation acceptance.

## Build order

### 1. Runtime Home And Project Registration

Plan enough runtime home support to create local Harness authority outside chat history and generated Markdown, then register exactly one local product repository.

Checklist:

- Create or select a configurable runtime home.
- Initialize the registry store, one project runtime area, one project state store, and an artifact store.
- Record a project-level state version before project-scoped mutations depend on it.
- Store the project id, display name, repo root, runtime path, and static project configuration.
- Register one reference surface with an honest cooperative or detective guarantee level.
- Provide a read-only status that can report "no active Task."

Done when:

- A fresh environment can be initialized repeatedly without creating duplicate authority records.
- Core can resolve the current project for all later Task-scoped actions.
- Status can distinguish an unregistered or idle project from an active Task.

### 2. One Task Record

Create the first Task through Core or a fixture seed path that uses the same validation rules.

Checklist:

- Store the Task id, lifecycle phase, state version, current summary, and minimal gate/status state needed by the slice.
- Append a task event when the Task is created or changed.
- Expose active Task reads through status.
- Keep mode policy depth, intake quality, and procedural budget routing for v0.2 User-Facing Harness MVP.

Done when:

- The system can show one active Task and its state version.
- A state-changing request with a stale expected state version is rejected or returns a state conflict where the owner contract requires it.

### 3. One Basic Scope

Add the smallest scope record that can constrain one intended product write. A Change Unit may be the owner shape, but the first slice should not expand into dependency graphs, full Autonomy Boundary policy, or multi-lane orchestration.

Checklist:

- Record the intended operation and allowed paths or command/tool class needed by the selected write.
- Attach the scope to the active Task.
- Record only the minimum evidence expectation needed by the selected claim.
- Keep full Discovery and user-facing procedural budget routing for v0.2.

Done when:

- Status can explain what may change.
- Product writes without an active compatible scope cannot receive write authority.

### 4. `prepare_write` Allow/Block

Implement the first meaningful write gate.

Checklist:

- Validate the request envelope, idempotency key, project id, Task id, and expected state version where required.
- Resolve the active Task and active scope.
- Check intended paths, tools, commands, network targets, secrets, and sensitive categories at the minimal level needed for the selected write.
- Check baseline freshness at the level needed for the first slice.
- Return a structured blocker when scope, state version, baseline, capability, or seeded required judgment is incompatible.
- When allowed, create a durable single-use Write Authorization compatible with one later direct Run or implementation Run.

Done when:

- Missing scope blocks.
- Out-of-scope intended paths block.
- A compatible scoped write returns a Write Authorization ref.
- No product write can be recorded by a product-write Run without that ref.

### 5. `record_run`

Record one direct Run or implementation Run and consume the Write Authorization.

Checklist:

- Require a compatible, unexpired, unconsumed Write Authorization for the selected product-write Run.
- Mark the Write Authorization consumed exactly once on successful commit.
- Record actor, surface, kind, intended operation, observed changes, artifact refs or evidence inputs, summary, and Run status at the minimal level needed for the slice.
- Validate observed changed paths and artifact refs against the authorization and scope.
- Append `task_events` in the same transaction as current record updates.

Done when:

- `record_run` without write authority is blocked.
- `record_run` with compatible authority succeeds once.
- A second distinct Run cannot reuse the consumed authorization.

### 6. Artifact Or Evidence Link

Register one durable evidence file or equivalent evidence ref through the owner path.

Checklist:

- Accept an approved staged file or existing committed artifact ref.
- Verify hash and size when provided.
- Apply redaction or secret omission before final storage when relevant.
- Store artifact metadata and relation to the Task, Run, evidence relation, or other owner record.
- Return an `ArtifactRef` or owner-defined evidence ref that uses the public shape from the API docs.

Done when:

- A Run can cite a registered artifact or evidence ref.
- Raw secrets are omitted or blocked rather than stored as evidence.

### 7. Minimal Evidence State

Create the smallest evidence relation needed to explain whether the selected claim is supported.

Checklist:

- Map one completion condition or acceptance criterion to the Run and artifact/evidence ref.
- Distinguish supported, partial, and insufficient evidence at the level needed for a close/status blocker.
- Avoid treating chat text, status prose, or projection prose as evidence.

Done when:

- A completed Run can produce a supported or partial evidence state.
- Missing required evidence causes close/status output to block.

### 8. Status, Next, And Structured Blockers

Expose current work state and safe next action without mutation, and return structured blockers when the first slice cannot close or proceed.

Checklist:

- Read project, active Task, current scope, write authority summary, evidence status, close/status blockers, and next safe action.
- Report missing scope, missing evidence, missing Write Authorization, reused authorization, or seeded required user judgment as structured blockers.
- Do not append events, enqueue projections, create artifacts, satisfy gates, authorize writes, or close the Task from a read.

Done when:

- Repeated status/next reads return the same state version unless another action changed state.
- The structured blocker can be compared by fixtures without matching prose.
- Close/status results are based on canonical records, not rendered reports.

## What this proves

The first runnable slice proves:

- Core can own state transitions.
- The state store and `task_events` are usable.
- A scoped record is required for product writes.
- `prepare_write` is the product-write authorization decision point.
- Write Authorization is durable and single-use.
- `record_run` consumes write authority once and records observed work.
- One artifact/evidence link can support the recorded Run.
- Evidence can be insufficient without relying on chat.
- Status and next are read-only.
- Structured blockers can report missing scope, evidence, authorization, or seeded required user judgment.

## What this does not prove yet

This slice does not prove the items below. They are stage boundaries, not failed v0.1 requirements.

| Later stage | Not yet proven by v0.1 Core Authority Slice |
|---|---|
| v0.2 User-Facing Harness MVP | Natural-language intake quality, Discovery, product/UX versus architecture judgment presentation, small-change versus tracked-work budgets, residual-risk display, final acceptance separation, user-facing projection/card sufficiency. |
| v0.3 Assurance & Stewardship Pack | Full Decision Packet quality, full Approval lifecycle and drift handling, detached verification independence, Manual QA policy matrix, residual-risk accepted close, feedback-loop policy, TDD trace, codebase stewardship, stewardship validators, context hygiene. |
| v0.4 Operations & Handoff Pack | Release handoff, recover, export, artifact integrity operations, broad operator smoke, broader fixture suite coverage. |
| v1+ Expansion | Dashboard, hosted workflow UI, Context Index, connector marketplace, Browser QA Capture, Cross-Surface Verification automation, native hook expansion, Advanced Sidecar Watcher, Local Derived Metrics, preventive guard expansion, parallel orchestration, team workflow. |

## Fixtures to write

Write fixtures that drive Core behavior and assert state, events, artifacts, projections or freshness when applicable, and errors. Do not assert success by matching rendered prose. These rows are future authoring candidates; they do not imply executable fixture files exist now.

Each runtime fixture should execute in an isolated runtime home and temporary Product Repository, seed its own starting records and files, run one Core or operator action, and compare the captured executable result. Fixture body fields, assertion modes such as `partial_deep` and `contains_ordered`, JSON `TEXT` validation, and owner-bound status value validation are owned by [Conformance Fixtures Reference](../reference/conformance-fixtures.md#conformance-fixture-format).

Minimum first-slice fixture candidates:

- no-active-task status read returns idle state and appends no events
- project registration creates project state and reference surface
- Task creation or seed produces one active Task and task event behavior
- basic scope allows one intended path and does not create write authority by itself
- `prepare_write` blocks when scope is missing
- `prepare_write` blocks an out-of-scope path
- `prepare_write` allows one compatible scoped write and creates one Write Authorization
- `record_run` blocks when write authority is missing
- `record_run` consumes a compatible Write Authorization once
- second distinct `record_run` cannot reuse a consumed authorization
- artifact or evidence ref registration stores integrity/redaction metadata and owner relation
- minimal evidence state reports supported, partial, or insufficient support
- status and next reads report current Task, scope, write authority, evidence, blockers, and next safe action without mutation
- close/status output blocks missing evidence with a structured blocker
- close/status output blocks a seeded required user judgment with a structured blocker

Use the [Kernel Smoke Authoring Queue](../reference/conformance-fixtures.md#kernel-smoke-authoring-queue) for practical order, seed guidance, stable event targets, artifact/projection assertions, and primary-error expectations. Do not add fields to the fixture body to express suite stage, authoring order, or docs-maintenance results.

## Reference docs to consult

- [Kernel Reference](../reference/kernel.md): Task, Change Unit, Decision Packet, gates, `prepare_write`, Write Authorization, `record_run` semantics, and `close_task`.
- [Runtime Architecture Reference](../reference/runtime-architecture.md): three spaces, Core process model, transaction flow, artifact store, projection/reconcile, guarantee levels, and failure handling.
- [MCP API And Schemas](../reference/mcp-api-and-schemas.md): public resources, tool envelopes, request/response schemas, error taxonomy, artifact refs, and `ProjectionKind`.
- [Storage And DDL](../reference/storage-and-ddl.md): runtime layout, DDL, migrations, locks, artifacts, baselines, projection jobs, and validator-run storage.
- [Operations And Conformance Reference](../reference/operations-and-conformance.md): operator semantics and conformance staging.
- [Conformance Fixtures Reference](../reference/conformance-fixtures.md): fixture format, execution, assertion rules, suite catalogs, and examples.
