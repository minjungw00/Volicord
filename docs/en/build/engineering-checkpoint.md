# Build: Engineering Checkpoint

## What this document helps you do

Use this page to plan the first internal Harness Server implementation slice: Engineering Checkpoint. It is a smoke of the local Core authority loop. It is not the product MVP, not MVP-1 User Work Loop, and not evidence that a runtime exists today.

This is planning documentation only. Runtime/server implementation may start only after documentation acceptance and a separate implementation-planning readiness decision in [Implementation Overview](implementation-overview.md#documentation-acceptance-status).

## Read this when

- You need the smallest future runnable slice.
- You are checking that the first batch has not become user-value MVP scope.
- You need owner links for the checkpoint without copying API, DDL, or fixture definitions.

## Main idea

Engineering Checkpoint proves that Harness can keep one local authority record alive through Core:

1. One local project is known.
2. One active Task exists.
3. One active Change Unit or equivalent owner-approved scope boundary exists for an intended write.
4. `harness.prepare_write` refuses incompatible work and allows compatible work.
5. One durable, single-use Write Authorization is created.
6. `harness.record_run` records one compatible Run and consumes that authorization once.
7. One artifact/evidence ref is registered and linked through an owner path.
8. Status/blocker output reads current Core state without mutating it.
9. A narrow `harness.close_task` blocker check can show close is blocked when required support is missing.

That is all. The checkpoint exists to prove the authority loop before user-facing value is added.

## Not product MVP

Engineering Checkpoint explicitly does not include:

- ordinary-language intake or full requirements clarification
- full user judgment presentation
- detailed Evidence Manifest behavior
- detached verification, Eval, Manual QA, work acceptance, residual-risk acceptance, or full close semantics
- projection renderer, detailed templates, dashboards, hosted UI, reports, export, or recover
- conformance runner or executable fixture catalog
- broad connector ecosystem, team workflow, orchestration, metrics, hook expansion, preventive guard expansion, or Roadmap automation

If a proposed first slice needs those capabilities to pass, it is no longer Engineering Checkpoint.

## Build order

Use this as an implementation planning order after readiness is accepted. It names capabilities, not command names or schema details.

| Step | Implementer goal | Done when | Owner docs |
|---|---|---|---|
| 1. Runtime home and project registration | Resolve one local product repository through the future Harness runtime home. | Status can distinguish unregistered, registered-idle, and active-work states. | [Runtime Architecture Reference](../reference/runtime-architecture.md), [Storage](../reference/storage.md), [Security Reference](../reference/security.md). |
| 2. One Task record | Create or seed one active Task through an owner-valid path. | Status can show the active Task and state version; stale state-changing calls are rejected where required. | [Core Model Reference](../reference/core-model.md), [API Errors](../reference/api/errors.md). |
| 3. One active Change Unit/scope boundary | Attach the smallest active Change Unit or owner-approved scope boundary that can constrain one intended product write. | Product writes without compatible scope cannot receive write authority. | [Core Model Reference](../reference/core-model.md). |
| 4. `prepare_write` allow/block | Route the intended write through the owner pre-write scope check. | Missing or out-of-scope work blocks; compatible work returns a Write Authorization ref. | [Core Model Reference](../reference/core-model.md#prepare_write), [`harness.prepare_write`](../reference/api/mvp-api.md#harnessprepare_write), [API Errors](../reference/api/errors.md). |
| 5. `record_run` | Record one compatible Run and consume the authorization. | A compatible Run succeeds once; reuse of the consumed authorization fails. | [Core Model Reference](../reference/core-model.md#record_run), [`harness.record_run`](../reference/api/mvp-api.md#harnessrecord_run). |
| 6. Artifact/evidence ref | Register one durable artifact or evidence ref through the owner path. | A Run or minimal owner relation can cite that registered ref. | [API Schema Core](../reference/api/schema-core.md#artifactref), [Storage](../reference/storage.md). |
| 7. Status and blockers | Expose current state and blockers without mutation. | Repeated reads do not change state, and blockers are structured enough for future smoke checks. | [`harness.status`](../reference/api/mvp-api.md#harnessstatus), [Core Model Reference](../reference/core-model.md), [API Schema Core](../reference/api/schema-core.md). |
| 8. Narrow close blocker check | Check whether close is blocked by the missing active support in this authority loop. | A blocked close returns a structured blocker without creating work acceptance, residual-risk acceptance, full assurance close semantics, or generated reports. | [Core Model Reference](../reference/core-model.md#close_task), [`harness.close_task`](../reference/api/mvp-api.md#harnessclose_task), [API Errors](../reference/api/errors.md). |

For API staging, use the [Stage Profile Manifest](../reference/api/schema-core.md#stage-profile-manifest). For storage planning, use [Storage](../reference/storage.md) and apply only the owner-approved minimal subset needed by this checkpoint.

## Doc-level acceptance checks

A future Engineering Checkpoint plan is acceptable when:

- It is local, single-project, and focused on one Task authority loop.
- It remains planning-only until [Documentation acceptance status](implementation-overview.md#documentation-acceptance-status) accepts implementation planning readiness.
- It proves one scoped write path through `prepare_write`, Write Authorization, `record_run`, artifact/evidence ref, structured status/blocker output, and a narrow close-blocker check.
- It refuses missing scope, out-of-scope intended work, missing Write Authorization for product-write Runs, reuse of a consumed Write Authorization, and missing artifact/evidence support where the active path requires support.
- It treats all status text, generated prose, and projection-like output as downstream reads from Core records.
- It does not require full projection rendering, multiple projection kinds, detailed templates, operations, conformance runner, or later-profile storage to pass.
- It links strict fixture format and assertions to [Conformance Fixtures Reference](../reference/conformance-fixtures.md) instead of defining them here.

## Future smoke checks

Kernel Smoke is only the narrow future authoring label for Engineering Checkpoint checks. It is not a stage name, not a full suite, and not a current executable fixture set.

When runtime implementation exists, future smoke checks should assert owner records, state transitions, artifact/evidence refs, structured blockers, and errors. They should not prove success by matching rendered prose, generated Markdown, or polished templates.

Use [Conformance Fixtures Reference: Kernel Smoke Authoring Queue](../reference/conformance-fixtures.md#kernel-smoke-authoring-queue) for future authoring order and [Conformance Fixture Format](../reference/conformance-fixtures.md#conformance-fixture-format) for exact future fixture shape.

## What this proves

Engineering Checkpoint proves:

- Core can own one local state transition path.
- Scope is required before product-write authority.
- Write Authorization is durable and single-use.
- `record_run` consumes write authority and records observed work.
- At least one registered artifact/evidence ref can support the recorded Run.
- Status/blocker reads can explain missing authority without becoming authority.
- A close-blocker check can report missing support without full close semantics or generated close reports.

## What remains for MVP-1

MVP-1 User Work Loop starts after this checkpoint. It adds ordinary-language start/resume, work-shape classification, scope/non-goals/success criteria, minimal user judgment, evidence summary, user-facing close result/blocker display, next safe action, residual-risk visibility, and separate display of sensitive approval, work acceptance, and risk acceptance.

Use [MVP-1 User Work Loop](mvp-user-work-loop.md) for that plan.
