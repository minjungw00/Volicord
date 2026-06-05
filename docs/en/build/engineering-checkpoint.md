# Build: Engineering Checkpoint

## What this document helps you do

Use this page to plan the first internal Harness Server implementation slice: Engineering Checkpoint. It is a smoke of the local Core authority loop. It is not the product MVP, not MVP-1 User Work Loop, and not evidence that a runtime exists today.

This is planning documentation only. Runtime/server implementation may start only after documentation acceptance and a separate implementation-planning readiness decision in [Implementation Overview](implementation-overview.md#documentation-acceptance-status).

## Read this when

- You need the smallest future runnable slice.
- You are checking that the first batch has not become user-value MVP scope.
- You need owner links for the checkpoint without copying API, DDL, or fixture definitions.

## Main idea

Engineering Checkpoint is designed to prove that future Harness can keep one local authority record alive through Core:

1. Status can report no active Task without mutating state.
2. An owner-valid setup/intake path can create exactly one active Task.
3. One reference `capability_profile` is registered for `surface_id=reference-local-mcp`.
4. One active Change Unit or equivalent owner-approved scope boundary is required before a compatible product-write check can succeed.
5. `harness.prepare_write` returns structured blockers for missing or out-of-scope work, creates one durable active Write Authorization only for a compatible non-dry-run decision, creates no authorization for dry-run, replays without duplicating authorization, and reports same-key/different-hash idempotency conflicts without side effects.
6. `harness.record_run` records one compatible Run and consumes that authorization once.
7. Consumed, missing, stale, or observed-outside-authorized-scope attempts block `record_run` or route to an owner violation/audit path without creating completion evidence.
8. Artifact/evidence refs record hash and redaction metadata through an owner path, and raw secret artifact storage is blocked or represented only with safe omission/block metadata.
9. Evidence summary can show partial or sufficient state from registered refs.
10. Status/blocker output reads current Core state without mutating it and shows the reference surface guarantee limit.
11. Narrow `harness.close_task` blocker checks can show close is blocked by missing evidence or unresolved user judgment, and can show residual risk before acceptance without implementing full close semantics.

That is all. The checkpoint exists to prove the authority loop before user-facing value is added.

## Not product MVP

Engineering Checkpoint explicitly does not include:

- ordinary-language intake or full requirements clarification
- full user judgment presentation
- detailed Evidence Manifest behavior
- detached verification, Eval, Manual QA, final acceptance, residual-risk acceptance, or full close semantics
- projection renderer, detailed templates, dashboards, hosted UI, reports, export, or recover
- conformance runner or executable fixture catalog
- broad connector ecosystem, hosted connector registry, cross-surface orchestration, team workflow, metrics, hook expansion, preventive guard expansion, or Roadmap automation

If a proposed first slice needs those capabilities to pass, it is no longer Engineering Checkpoint.

## Build order

Use this as an implementation planning order after readiness is accepted. It names capabilities, not command names or schema details.

| Step | Implementer goal | Done when | Owner docs |
|---|---|---|---|
| 1. Runtime home, project registration, and reference surface profile | Resolve one local product repository through the future Harness runtime home and register the reference `capability_profile`. | Status can distinguish unregistered, registered-idle, and active-work states, and can display that the reference profile is cooperative/detective with no pre-tool blocking or isolation claim. | [Runtime Architecture Reference](../reference/runtime-architecture.md), [Storage](../reference/storage.md), [Security Reference](../reference/security.md), [Agent Integration Reference](../reference/agent-integration.md#capability-profiles). |
| 2. One Task record | Create or seed one active Task through an owner-valid path. | Status can show the active Task and state version; stale state-changing calls are rejected where required. | [Core Model Reference](../reference/core-model.md), [API Errors](../reference/api/errors.md). |
| 3. One active Change Unit/scope boundary | Attach the smallest active Change Unit or owner-approved scope boundary that can constrain one intended product write. | Product writes without compatible scope cannot receive a Write Authorization. | [Core Model Reference](../reference/core-model.md). |
| 4. `prepare_write` decision | Route the intended write through the owner pre-write scope check. | Missing or out-of-scope work returns a structured Harness blocker or non-`allowed` response; compatible work returns a Write Authorization ref with honest guarantee display. This is not OS permission or physical pre-tool blocking. | [Core Model Reference](../reference/core-model.md#prepare_write), [`harness.prepare_write`](../reference/api/mvp-api.md#harnessprepare_write), [API Errors](../reference/api/errors.md). |
| 5. `record_run` | Record one compatible Run and consume the authorization. | A compatible Run succeeds once; reuse of the consumed authorization fails. | [Core Model Reference](../reference/core-model.md#record_run), [`harness.record_run`](../reference/api/mvp-api.md#harnessrecord_run). |
| 6. Artifact/evidence ref | Register one durable artifact or evidence ref through the owner path. | A Run or minimal owner relation can cite that registered ref, including hash, size, content type, redaction, owner, and availability metadata where the owner path requires it. | [API Schema Core](../reference/api/schema-core.md#artifactref), [Storage](../reference/storage.md). |
| 7. Status and blockers | Expose current state and blockers without mutation. | Repeated reads do not change state, and blockers are structured enough for future smoke checks. | [`harness.status`](../reference/api/mvp-api.md#harnessstatus), [Core Model Reference](../reference/core-model.md), [API Schema Core](../reference/api/schema-core.md). |
| 8. Narrow close blocker check | Check whether close is blocked by missing evidence, unresolved user judgment, or visible residual risk in this authority loop. | A blocked close returns structured blockers without creating final acceptance, residual-risk acceptance, full assurance close semantics, or generated reports. | [Core Model Reference](../reference/core-model.md#close_task), [`harness.close_task`](../reference/api/mvp-api.md#harnessclose_task), [API Errors](../reference/api/errors.md). |

For API staging, use the [Stage Profile Manifest](../reference/api/schema-core.md#stage-profile-manifest). For storage planning, use [Storage](../reference/storage.md) and apply only the owner-approved minimal subset needed by this checkpoint.

## Doc-level planning review checks

A future Engineering Checkpoint plan is ready for maintainer planning review when:

- It is local, single-project, and focused on one Task authority loop.
- It uses one registered reference `capability_profile`, not a connector platform or registry.
- It remains planning-only until [Documentation acceptance status](implementation-overview.md#documentation-acceptance-status) accepts implementation planning readiness.
- It demonstrates one scoped Harness authority path through `prepare_write`, Write Authorization, `record_run`, artifact/evidence ref, structured status/blocker output, and a narrow close-blocker check.
- It returns structured blockers, non-`allowed` responses, or API-owned errors for missing scope, out-of-scope intended work, same-key/different-hash idempotency conflicts, missing Write Authorization for product-write Runs, reuse of a consumed Write Authorization, observed attempts outside authorized scope, raw secret artifact input, and missing artifact/evidence support where the active path requires support.
- It treats all status text, generated prose, and projection-like output as downstream reads from Core records, not fixture proof.
- Its future smoke checks use the structured draft fields owned by [Conformance Fixtures Reference](../reference/conformance-fixtures.md): `expected_response`, `expected_state_changes`, `expected_storage_rows`, `expected_events`, `expected_artifacts`, `expected_blockers`, `expected_errors`, and `forbidden_side_effects`.
- It does not require full projection rendering, multiple projection kinds, detailed templates, operations, conformance runner, broad connector ecosystem, hosted connector registry, cross-surface orchestration, or later-profile storage to pass.
- It links strict fixture format and assertions to [Conformance Fixtures Reference](../reference/conformance-fixtures.md) instead of defining them here.

These are documentation planning checks only. They do not create acceptance state, manual acceptance, close readiness, runtime conformance results, generated artifacts, projection refreshes, or implementation readiness.

## Future smoke checks

Kernel Smoke is only the narrow future authoring label for Engineering Checkpoint checks. It is not a stage name, not a full suite, and not a current executable fixture set.

When runtime implementation exists, future smoke checks should assert owner records, state transitions, storage rows, `task_events` when stable events exist, artifact/evidence refs, structured blockers, primary errors, guarantee display facts, and forbidden side effects. They should not prove success by matching rendered prose, generated Markdown, or polished templates.

Use [Conformance Fixtures Reference: Kernel Smoke Authoring Queue](../reference/conformance-fixtures.md#kernel-smoke-authoring-queue) for future authoring order and [Conformance Fixture Format](../reference/conformance-fixtures.md#conformance-fixture-format) for the structured non-executable fixture draft shape.

## What this proves

Engineering Checkpoint proves:

- Core can own one local state transition path.
- Scope is required before a Harness-compatible Write Authorization can be created.
- Write Authorization is durable and single-use.
- `record_run` consumes the Write Authorization and records observed work.
- At least one registered artifact/evidence ref can support the recorded Run.
- Status/blocker reads can explain missing authority without becoming authority.
- A close-blocker check can report missing support without full close semantics or generated close reports.

## What remains for MVP-1

MVP-1 User Work Loop starts after this checkpoint. It adds ordinary-language start/resume, work-shape classification, scope/non-goals/success criteria, minimal user judgment, evidence summary, user-facing close result/blocker display, next safe action, residual-risk visibility, final-acceptance blockers, accepted-risk close when a compatible residual-risk acceptance judgment exists, and display-label proof that localized labels are not canonical state.

Use [MVP-1 User Work Loop](mvp-user-work-loop.md) for that plan.
