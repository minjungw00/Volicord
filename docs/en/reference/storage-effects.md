# Storage effects

This document owns method-to-storage effect semantics for the baseline scope source design.

## Owns / Does not own

This document owns:

- read-only, dry-run, rejected, staging-created, Core-committed, and committed-blocked storage-effect distinctions
- whether a method branch creates replay rows, `task_events`, record changes, state-version increments, staged-handle creation or consumption, artifact promotion, or `Write Authorization` changes
- the persistence boundary for blocker-like response data
- no-effect guarantees for rejected branches and valid dry-run preview branches

This document does not own:

- record-family overview; see [Storage Records](storage-records.md)
- baseline SQLite DDL, constraints, indexes, foreign keys, or migration table shape; see [Storage DDL](storage-ddl.md)
- artifact lifecycle details; see [Artifact Storage](storage-artifacts.md)
- idempotency, locks, state-version clocks, event ordering, or migrations; see [Storage Versioning](storage-versioning.md)
- public response branches or schemas; see [API Schema Core](api/schema-core.md)
- API method behavior; see the [API Methods](api/methods.md) and method owner documents
- public error code precedence; see [API error precedence](api/error-precedence.md)

## Shape versus effect

Response data shape and storage effect are separate.

API data shapes belong to API schema owners, including [API State Schemas](api/schema-state.md) for blocker-like state shapes and [API Artifact Schemas](api/schema-artifacts.md) for artifact shapes. Examples include:
- `CloseReadinessBlocker`
- `WriteDecisionReason`
- `PlannedBlocker`
- `ArtifactRef`
- `StagedArtifactHandle`

Non-claim: their presence in a response does not by itself prove persistence, artifact promotion, staged-handle consumption, replay storage, close-state mutation, or `project_state.state_version` increment.

Effects come from the selected method behavior and response branch. The table summarizes each branch; the detail blocks separate allowed effects from forbidden effects.

| Branch | Summary | Details |
|---|---|---|
| Read-only `MethodResult` | Response only | [Read-only result](#read-only-result) |
| `ToolRejectedResponse` | No storage effect | [`ToolRejectedResponse`](#toolrejectedresponse-effect) |
| Valid `ToolDryRunResponse` | Preview only | [Valid dry-run preview](#valid-dry-run-preview) |
| `StageArtifactResult` with `effect_kind=staging_created` | transient staging only | [Staging-created artifact result](#staging-created-artifact-result) |
| Core committed `MethodResult` | Method-owned committed effects | [Core committed result](#core-committed-result) |
| Committed blocked `MethodResult` | Explicitly allowed blocked effects only | [Committed blocked result](#committed-blocked-result) |

<a id="read-only-result"></a>
### Read-only result

Storage effect:

- Response only.

Disallowed effects:

- replay row
- event
- current-row mutation
- artifact effect
- `Write Authorization` effect
- `project_state.state_version` increment

<a id="toolrejectedresponse-effect"></a>
### `ToolRejectedResponse`

Storage effect:

- None.

Disallowed effects:

- current row
- replay row
- event
- artifact effect
- `Write Authorization` creation or consumption
- `project_state.state_version` increment

<a id="valid-dry-run-preview"></a>
### Valid dry-run preview

Storage effect:

- Response preview only.

Disallowed effects:

- current row
- generated persistent ref
- replay row
- event
- staged-handle creation
- artifact promotion or link
- `project_state.state_version` increment

<a id="staging-created-artifact-result"></a>
### Staging-created artifact result

Allowed effect:

- storage-owned transient staging

Disallowed effects:

- Core current row
- replay row
- event
- persistent `ArtifactRef`
- `project_state.state_version` increment

<a id="core-committed-result"></a>
### Core committed result

Condition:

- The method owner allows the committed effect.

Allowed effects:

- current-row mutation
- `task_events` append
- replay row creation
- exactly one `project_state.state_version` increment

<a id="committed-blocked-result"></a>
### Committed blocked result

Condition:

- The method owner allows the blocked commit.

Allowed effects:

- explicitly allowed blocker-state effect
- explicitly allowed event effect
- explicitly allowed replay-row effect
- explicitly allowed `project_state.state_version` effect

Disallowed effects:

- creating the missing authority that the branch reports

## No-effect branches

These failures return no-effect branches:

- malformed requests
- validation failures before commit
- local access failures before a protected operation can proceed
- capability failures
- stale `expected_state_version`
- stale `WriteAuthorization.basis_state_version`
- idempotency request-hash conflicts
- rejected artifact inputs

No-effect branches must not:

- create current rows
- append `task_events`
- write `tool_invocations.response_json`
- create replay rows
- update evidence summaries
- mutate close state
- create or consume `Write Authorization`
- change `artifact_staging.status`
- set `consumed_by_run_id` or `promoted_artifact_id`
- promote or link artifacts
- increment `project_state.state_version`

When preflight returns `ToolRejectedResponse`, the requested committed operation does not proceed. This principle applies to `dry_run` requests too. `dry_run` does not bypass validation, access, capability, or stale-state rejection.

## Dry-run preview effects

Valid dry-run previews may include `DryRunSummary.would_blockers: PlannedBlocker[]` or planned effects. Those preview entries do not create:

- `task_event` or `task_events` append
- replay row or `tool_invocations.response_json`
- generated persistent ref
- `close_state` mutation
- `Write Authorization` change
- staged-handle creation or consumption
- artifact effect
- evidence update
- `CloseReadinessBlocker` storage
- `project_state.state_version` increment

## Read-only effects

Read-only results are response-only and not replay rows.

For response computation, `volicord.status` and `volicord.close_task intent=check` may compute `CurrentCloseBasis`, close state, risk acceptance coverage, blockers, `CloseReadinessBlocker[]`, evidence summaries, artifact refs, diagnostics, and next actions for the response when the method owner selects those projections.

Storage must not persist those computed values merely because the read occurred.

Read-time projections must distinguish uncomputed, unavailable, empty, and verified state. Storage must not write empty arrays, empty hashes, zero sizes, invented content types, or stronger guarantee displays merely because a read path could not compute the underlying facts.

Read-time artifact checks may compute an effective missing, unavailable, or integrity-failed state for evidence, close, or status output when the current body cannot be verified against stored facts. That response computation does not mutate `artifacts.status`, `artifacts.integrity_status`, artifact links, or stored lifecycle rows unless a separate owner-defined mutation occurs.

`volicord.status` with `close_blockers: CloseReadinessBlocker[]` is a read-only observation. It does not create:

- `task_event` or `task_events` append
- replay row or `tool_invocations.response_json`
- `close_state` mutation
- `Write Authorization` change
- staged-handle consumption
- artifact effect
- evidence update
- `project_state.state_version` increment

For `volicord.close_task intent=check`, the response branch is owned by [`volicord.close_task`](api/method-close-task.md). This storage page only asserts that the check remains read-only, including with `dry_run=true` and with `blockers: CloseReadinessBlocker[]`.

## Committed blocked effects

Committed blocked outcomes are distinct from rejected responses.

Condition: a committed blocked `volicord.prepare_write` or `volicord.close_task` outcome is a `MethodResult` only when the relevant method owner allows the blocked commit.

Owner links:
- [Prepare-write method](api/method-prepare-write.md)
- [Close-task method](api/method-close-task.md)

<a id="volicordprepare_write-committed-non-allow-decision"></a>
### `volicord.prepare_write` committed non-allow decision

Conditions:

- The call is committed with `dry_run=false`.
- The result is `decision=blocked`, `decision=approval_required`, or `decision=decision_required`.

Allowed effects:

- append exactly one `task_events` event containing the structured `write_decision_reasons: WriteDecisionReason[]`
- create a replay row when an idempotency key is present
- increment `project_state.state_version` exactly once
- record the method-owned decision and `write_decision_reasons` in the response and replay payload

Disallowed effects:

- creating consumable `Write Authorization`
- creating a separate public history method
- adding a new public response field for historical non-allow decisions
- requiring `volicord.status` to expose historical non-allow decisions
- changing `close_state`
- evaluating close readiness
- storing `CloseReadinessBlocker`
- updating evidence
- changing artifacts
- consuming staged handles
- applying `close_task` effects

Persistence boundary:

- Request-side `volicord.prepare_write` payload fields belong to the [`volicord.prepare_write` reference](api/method-prepare-write.md).
- Stored `write_decision_reasons` remain `volicord.prepare_write` decision reasons.
- The durable audit location for a valid committed non-allow decision is the committed task event and, when keyed, the replay row.

Those stored reasons are not:

- close-readiness blockers
- `CloseReadinessBlocker[]`
- close-readiness blocker records

<a id="volicordclose_task-committed-blocked-result"></a>
### `volicord.close_task` committed blocked result

Conditions:

- Close readiness evaluation has run.
- The `volicord.close_task` method contract permits committing the blocked result.

Allowed effects:

- blocker state
- `task_events`
- replay row
- `project_state.state_version`

The Task remains open.

Disallowed uses:

- using this branch for `STATE_VERSION_CONFLICT`
- storing `STATE_VERSION_CONFLICT` as replay

`STATE_VERSION_CONFLICT` belongs to the preflight `ToolRejectedResponse` branch.

<a id="method-effects"></a>
## Method effect summary

This table summarizes persistence effects. Method behavior and response unions remain owned by method owner documents routed from the [API Methods](api/methods.md).

| Method | Primary storage effect | Details |
|---|---|---|
| `volicord.intake` | creates task and shaping records | See [`volicord.intake`](#volicordintake) |
| `volicord.update_scope` | updates current scope records | See [`volicord.update_scope`](#volicordupdate_scope) |
| `volicord.status` | read-only response | See [`volicord.status`](#volicordstatus) |
| `volicord.prepare_write` | records write decision effects | See [`volicord.prepare_write`](#volicordprepare_write) |
| `volicord.stage_artifact` | creates transient staging only | See [`volicord.stage_artifact`](#volicordstage_artifact) |
| `volicord.record_run` | records run, current close-basis, and evidence effects | See [`volicord.record_run`](#volicordrecord_run) |
| `volicord.request_user_judgment` | creates pending judgment request | See [`volicord.request_user_judgment`](#volicordrequest_user_judgment) |
| `volicord.record_user_judgment` | resolves user judgment | See [`volicord.record_user_judgment`](#volicordrecord_user_judgment) |
| `volicord.close_task intent=check` | read-only close-readiness check | See [`volicord.close_task intent=check`](#volicordclose_task-intentcheck) |
| `volicord.close_task intent=complete` | persists method-selected `complete` terminal or blocked effect | See [`volicord.close_task intent=complete`](#volicordclose_task-intentcomplete) |
| `volicord.close_task intent=cancel` | persists method-selected cancellation terminal or blocked effect | See [`volicord.close_task intent=cancel`](#volicordclose_task-intentcancel) |
| `volicord.close_task intent=supersede` | persists method-selected supersession terminal or blocked effect | See [`volicord.close_task intent=supersede`](#volicordclose_task-intentsupersede) |

<a id="volicordintake"></a>
### `volicord.intake`

Committed `dry_run=false` may:

- create the Task
- create an optional Change Unit
- create shaping records
- append events
- create a replay row
- increment `project_state.state_version` once

No-effect branches:

- valid `dry_run=true`
- rejected attempts

Those branches create no Task, refs, event, replay row, or state-version increment.

Owner links:

- [`volicord.intake` method](api/method-intake.md)
- [Storage Records](storage-records.md)
- [Storage Versioning](storage-versioning.md)

<a id="volicordupdate_scope"></a>
### `volicord.update_scope`

Committed `dry_run=false` may:

- update current-scope Task fields
- create or replace current `change_units`
- increment `tasks.scope_revision` for material current-scope or current Change Unit changes
- invalidate `tasks.close_basis_json` and increment `tasks.close_basis_revision` for material scope changes
- mark incompatible judgment basis rows stale or superseded as owner-defined compatibility requires
- update blockers or stale `Write Authorization` refs as the method owner allows
- append events
- create a replay row
- increment `project_state.state_version` once

No-effect branches:

- valid dry-run previews
- rejected attempts

Valid dry-run previews only describe scope, Change Unit, blocker, and stale authorization effects.

Semantically identical normalized updates do not increment `tasks.scope_revision` or invalidate the current close basis.

Owner links:

- [`volicord.update_scope` method](api/method-update-scope.md)
- [Storage Records](storage-records.md)
- [Storage Versioning](storage-versioning.md)

<a id="volicordstatus"></a>
### `volicord.status`

Read-only calls:

- return response data only
- do not create replay rows
- do not mutate storage
- do not increment `project_state.state_version`

`dry_run=true` remains `StatusResult` with `effect_kind=read_only`, not `ToolDryRunResponse`.

No-effect branches:

- rejected attempts

Owner links:

- [`volicord.status` method](api/method-status.md)

<a id="volicordprepare_write"></a>
### `volicord.prepare_write`

An original committed `dry_run=false` call with `decision=allowed` may:

- create a compatible `status=active` `Write Authorization`
- append events
- create a replay row
- increment `project_state.state_version` once

Idempotent replay returns the stored original response under [Storage Versioning](storage-versioning.md) and does not repeat these effects.

Committed non-allowed decisions:

- See [`volicord.prepare_write` committed non-allow decision](#volicordprepare_write-committed-non-allow-decision).
- They append exactly one task event, create a replay row when keyed, and increment `project_state.state_version` exactly once.
- They do not create consumable `Write Authorization`, a separate public history method, or a new public response field.
- `volicord.status` is not required to expose historical non-allow decisions.

No-effect branches:

- rejected attempts
- valid dry-run previews

Those branches do not create:

- replay row
- `Write Authorization`
- event
- `close_state` mutation
- artifact or evidence effect
- `project_state.state_version` increment

Owner links:

- [`volicord.prepare_write` method](api/method-prepare-write.md)
- [Storage Records](storage-records.md)
- [Storage Versioning](storage-versioning.md)

<a id="volicordstage_artifact"></a>
### `volicord.stage_artifact`

Successful staging may:

- create `artifact_staging` or an equivalent storage-owned staging record
- store transient safe bytes or notices under `artifacts/tmp/`

This branch creates only transient storage-owned staging.

It does not create:

- Core current row
- persistent `ArtifactRef`
- replay row
- `project_state.state_version` increment

No-effect branches:

- valid `dry_run=true`
- invalid staging requests

Valid `dry_run=true` does not create:

- bytes
- staging record
- `StagedArtifactHandle`
- replay row
- `project_state.state_version` increment

Owner links:

- [`volicord.stage_artifact` method](api/method-stage-artifact.md)
- [Artifact Storage](storage-artifacts.md)

<a id="volicordrecord_run"></a>
### `volicord.record_run`

Committed `dry_run=false` may:

- create `runs`
- consume compatible `write_authorizations`
- consume eligible `artifact_staging`
- promote or link `artifacts`
- update `evidence_summaries` or allowed blockers
- update `tasks.close_basis_revision` and `tasks.close_basis_json` according to `close_assessment`
- append events
- create a replay row
- increment `project_state.state_version` once

No-effect branches:

- valid dry-run previews
- rejected attempts
- invalid staged handles before commit

Valid dry-run previews do not create:

- `run_summary`
- current close basis
- persistent residual-risk IDs
- persistent artifact
- artifact link
- evidence update
- blocker update
- event
- replay row
- staged-handle consumption
- `Write Authorization` consumption
- `project_state.state_version` increment

Rejected attempts do not change:

- staging rows
- artifacts

Product file write persistence boundary:

- When the method owner allows a committed run that records a product file write, storage may consume a compatible `write_authorizations` row in the same commit.
- Test evidence persistence can promote staged artifacts and update evidence without implying a product file write observation.
- Exact run classification belongs to the [`volicord.record_run` method](api/method-record-run.md).

Current close-basis persistence boundary:

- A committed `volicord.record_run` increments `tasks.close_basis_revision` exactly once.
- A non-null `close_assessment` writes a new current `CurrentCloseBasis` in `tasks.close_basis_json` and stores Core-generated opaque residual-risk IDs.
- Sensitive action requirements stored in that `CurrentCloseBasis` are derived by Core from the committed Run and any consumed `Write Authorization`, preserving operation, normalized paths, sensitive categories, baseline, Change Unit, source Run ref, and source `Write Authorization` ref through close.
- Category-only caller input cannot establish, satisfy, or erase a sensitive action requirement.
- `close_assessment=null` records that the committed Run does not establish a current close basis; any existing current basis becomes stale or absent.
- Run, current close basis, evidence, artifact, authorization, replay, event, and revision effects commit atomically.

Owner links:

- [`volicord.record_run` method](api/method-record-run.md)
- [Artifact Storage](storage-artifacts.md)
- [Storage Records](storage-records.md)

<a id="volicordrequest_user_judgment"></a>
### `volicord.request_user_judgment`

Committed `dry_run=false` may:

- create a pending `user_judgments` row
- store `basis_json` and `basis_status='current'` for the Core-derived judgment basis
- update affected blockers
- append events
- create a replay row
- increment `project_state.state_version` once

No-effect branches:

- valid dry-run previews
- rejected attempts

Valid dry-run previews do not create:

- real `user_judgment_ref`
- pending judgment
- blocker update
- event
- replay row
- `project_state.state_version` increment

Owner links:

- [`volicord.request_user_judgment` method](api/method-request-user-judgment.md#volicordrequest_user_judgment)
- [Storage Records](storage-records.md)

<a id="volicordrecord_user_judgment"></a>
### `volicord.record_user_judgment`

Committed `dry_run=false` may:

- set a `user_judgments` row to `status='resolved'`
- store the selected option, `resolution_machine_action`, `resolution_outcome`, derived resolution actor provenance, answer payload, and basis status as allowed by the method owner
- update dependent blockers or next actions
- append events
- create a replay row
- increment `project_state.state_version` once

No-effect branches:

- valid dry-run previews
- rejected attempts

Valid dry-run previews do not create:

- judgment resolution
- blocker update
- event
- replay row
- `project_state.state_version` increment

Recording a user judgment does not increment `tasks.scope_revision` or `tasks.close_basis_revision`.

`status='resolved'` records that an answer was recorded; it is not acceptance by itself. Current resolved rows require complete basis, selected action, `resolution_outcome`, resolution payload, resolution timestamp, resolved surface identity, verification basis, assurance level, and required actor provenance. Missing required resolution authority is invalid stored state, not a readable historical audit judgment.

Owner links:

- [`volicord.record_user_judgment` method](api/method-record-user-judgment.md#volicordrecord_user_judgment)
- [Storage Records](storage-records.md)

<a id="volicordclose_task-intentcheck"></a>
### `volicord.close_task intent=check`

Read-only calls:

- return computed close readiness
- use the same close-readiness calculation as `volicord.status include.close=true`
- do not create replay rows
- do not append events
- do not create blocker rows
- do not mutate close state
- do not touch artifacts or evidence
- do not increment `project_state.state_version`

`dry_run=true` remains `CloseTaskResult` with `effect_kind=read_only`.

No-effect branches:

- rejected attempts

Owner links:

- [`volicord.close_task` method](api/method-close-task.md)

<a id="volicordclose_task-intentcomplete"></a>
### `volicord.close_task intent=complete`

Committed `dry_run=false` may:

- persist the method-selected terminal completion effect
- persist a terminal close summary distinct from `tasks.close_basis_json` when the method-selected completion effect succeeds
- persist an owner-allowed blocked `complete` effect while the Task remains open
- append events
- create a replay row
- increment `project_state.state_version` once

No-effect branches:

- valid `dry_run=true`
- preflight failures

Valid `dry_run=true` returns `ToolDryRunResponse`. Preflight failures are no-effect `ToolRejectedResponse`.

Owner links:

- [`volicord.close_task` method](api/method-close-task.md)
- [Storage Versioning](storage-versioning.md)

<a id="volicordclose_task-intentcancel"></a>
### `volicord.close_task intent=cancel`

Committed `dry_run=false` may:

- persist the method-selected cancellation effect
- persist an owner-allowed blocked cancellation effect while the Task remains open
- append events
- create a replay row
- increment `project_state.state_version` once

No-effect branches:

- valid `dry_run=true`
- preflight failures

Valid `dry_run=true` returns `ToolDryRunResponse`.

Cancellation effects require the method-owned current cancellation judgment with `machine_action=accept`, `resolution_outcome=accepted`, compatible basis, and verified `user_interaction` actor provenance. Missing or incompatible cancellation authority may produce an owner-allowed blocked cancellation effect, but must not fabricate acceptance or completion-only close evidence.

Owner links:

- [`volicord.close_task` method](api/method-close-task.md)
- [Storage Versioning](storage-versioning.md)

<a id="volicordclose_task-intentsupersede"></a>
### `volicord.close_task intent=supersede`

Committed `dry_run=false` may:

- persist the method-selected supersession effect
- update `project_state.active_task_id` in the same mutation when the method-selected effect requires it
- persist an owner-allowed blocked supersession effect
- append events
- create a replay row
- increment `project_state.state_version` once

No-effect branches:

- valid `dry_run=true`
- preflight failures

Valid `dry_run=true` returns `ToolDryRunResponse`.

Owner links:

- [`volicord.close_task` method](api/method-close-task.md)
- [Storage Versioning](storage-versioning.md)

## Related owners

- [API Methods](api/methods.md) and method owner documents for selected method behavior and response unions.
- [API error routing](api/error-routing.md) and [API error codes](api/error-codes.md) for rejected-response public errors.
- [Storage Records](storage-records.md) for records that effects may touch.
- [Artifact Storage](storage-artifacts.md) for staged-handle and artifact lifecycle details.
- [Storage Versioning](storage-versioning.md) for state clocks and replay/idempotency semantics.
