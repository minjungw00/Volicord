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

For response computation, `harness.status` and `harness.close_task intent=check` may compute blockers, `CloseReadinessBlocker[]`, evidence summaries, artifact refs, diagnostics, and next actions for the response.

Storage must not persist those computed values merely because the read occurred.

`harness.status` with `close_blockers: CloseReadinessBlocker[]` is a read-only observation. It does not create:

- `task_event` or `task_events` append
- replay row or `tool_invocations.response_json`
- `close_state` mutation
- `Write Authorization` change
- staged-handle consumption
- artifact effect
- evidence update
- `project_state.state_version` increment

For `harness.close_task intent=check`, the response branch is owned by [`harness.close_task`](api/method-close-task.md). This storage page only asserts that the check remains read-only, including with `dry_run=true` and with `blockers: CloseReadinessBlocker[]`.

## Committed blocked effects

Committed blocked outcomes are distinct from rejected responses.

Condition: a committed blocked `harness.prepare_write` or `harness.close_task` outcome is a `MethodResult` only when the relevant method owner allows the blocked commit.

Owner links:
- [Prepare-write method](api/method-prepare-write.md)
- [Close-task method](api/method-close-task.md)

<a id="harnessprepare_write-committed-non-allow-decision"></a>
### `harness.prepare_write` committed non-allow decision

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
- requiring `harness.status` to expose historical non-allow decisions
- changing `close_state`
- evaluating close readiness
- storing `CloseReadinessBlocker`
- updating evidence
- changing artifacts
- consuming staged handles
- applying `close_task` effects

Persistence boundary:

- Request-side `harness.prepare_write` payload fields belong to the [`harness.prepare_write` reference](api/method-prepare-write.md).
- Stored `write_decision_reasons` remain `harness.prepare_write` decision reasons.
- The durable audit location for a valid committed non-allow decision is the committed task event and, when keyed, the replay row.

Those stored reasons are not:

- close-readiness blockers
- `CloseReadinessBlocker[]`
- close-readiness blocker records

<a id="harnessclose_task-committed-blocked-result"></a>
### `harness.close_task` committed blocked result

Conditions:

- Close readiness evaluation has run.
- The `harness.close_task` method contract permits committing the blocked result.

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
| `harness.intake` | creates task and shaping records | See [`harness.intake`](#harnessintake) |
| `harness.update_scope` | updates current scope records | See [`harness.update_scope`](#harnessupdate_scope) |
| `harness.status` | read-only response | See [`harness.status`](#harnessstatus) |
| `harness.prepare_write` | records write decision effects | See [`harness.prepare_write`](#harnessprepare_write) |
| `harness.stage_artifact` | creates transient staging only | See [`harness.stage_artifact`](#harnessstage_artifact) |
| `harness.record_run` | records run and evidence effects | See [`harness.record_run`](#harnessrecord_run) |
| `harness.request_user_judgment` | creates pending judgment request | See [`harness.request_user_judgment`](#harnessrequest_user_judgment) |
| `harness.record_user_judgment` | resolves user judgment | See [`harness.record_user_judgment`](#harnessrecord_user_judgment) |
| `harness.close_task intent=check` | read-only close-readiness check | See [`harness.close_task intent=check`](#harnessclose_task-intentcheck) |
| `harness.close_task intent=complete` | persists method-selected `complete` terminal or blocked effect | See [`harness.close_task intent=complete`](#harnessclose_task-intentcomplete) |
| `harness.close_task intent=cancel` | persists method-selected cancellation terminal or blocked effect | See [`harness.close_task intent=cancel`](#harnessclose_task-intentcancel) |
| `harness.close_task intent=supersede` | persists method-selected supersession terminal or blocked effect | See [`harness.close_task intent=supersede`](#harnessclose_task-intentsupersede) |

<a id="harnessintake"></a>
### `harness.intake`

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

- [`harness.intake` method](api/method-intake.md)
- [Storage Records](storage-records.md)
- [Storage Versioning](storage-versioning.md)

<a id="harnessupdate_scope"></a>
### `harness.update_scope`

Committed `dry_run=false` may:

- update current-scope Task fields
- create or replace current `change_units`
- update blockers or stale `Write Authorization` refs as the method owner allows
- append events
- create a replay row
- increment `project_state.state_version` once

No-effect branches:

- valid dry-run previews
- rejected attempts

Valid dry-run previews only describe scope, Change Unit, blocker, and stale authorization effects.

Owner links:

- [`harness.update_scope` method](api/method-update-scope.md)
- [Storage Records](storage-records.md)
- [Storage Versioning](storage-versioning.md)

<a id="harnessstatus"></a>
### `harness.status`

Read-only calls:

- return response data only
- do not create replay rows
- do not mutate storage
- do not increment `project_state.state_version`

`dry_run=true` remains `StatusResult` with `effect_kind=read_only`, not `ToolDryRunResponse`.

No-effect branches:

- rejected attempts

Owner links:

- [`harness.status` method](api/method-status.md)

<a id="harnessprepare_write"></a>
### `harness.prepare_write`

An original committed `dry_run=false` call with `decision=allowed` may:

- create a compatible `status=active` `Write Authorization`
- append events
- create a replay row
- increment `project_state.state_version` once

Idempotent replay returns the stored original response under [Storage Versioning](storage-versioning.md) and does not repeat these effects.

Committed non-allowed decisions:

- See [`harness.prepare_write` committed non-allow decision](#harnessprepare_write-committed-non-allow-decision).
- They append exactly one task event, create a replay row when keyed, and increment `project_state.state_version` exactly once.
- They do not create consumable `Write Authorization`, a separate public history method, or a new public response field.
- `harness.status` is not required to expose historical non-allow decisions.

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

- [`harness.prepare_write` method](api/method-prepare-write.md)
- [Storage Records](storage-records.md)
- [Storage Versioning](storage-versioning.md)

<a id="harnessstage_artifact"></a>
### `harness.stage_artifact`

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

- [`harness.stage_artifact` method](api/method-stage-artifact.md)
- [Artifact Storage](storage-artifacts.md)

<a id="harnessrecord_run"></a>
### `harness.record_run`

Committed `dry_run=false` may:

- create `runs`
- consume compatible `write_authorizations`
- consume eligible `artifact_staging`
- promote or link `artifacts`
- update `evidence_summaries` or allowed blockers
- append events
- create a replay row
- increment `project_state.state_version` once

No-effect branches:

- valid dry-run previews
- rejected attempts
- invalid staged handles before commit

Valid dry-run previews do not create:

- `run_summary`
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
- Exact run classification belongs to the [`harness.record_run` method](api/method-record-run.md).

Owner links:

- [`harness.record_run` method](api/method-record-run.md)
- [Artifact Storage](storage-artifacts.md)
- [Storage Records](storage-records.md)

<a id="harnessrequest_user_judgment"></a>
### `harness.request_user_judgment`

Committed `dry_run=false` may:

- create a pending `user_judgments` row
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

- [`harness.request_user_judgment` method](api/method-request-user-judgment.md#harnessrequest_user_judgment)
- [Storage Records](storage-records.md)

<a id="harnessrecord_user_judgment"></a>
### `harness.record_user_judgment`

Committed `dry_run=false` may:

- resolve a `user_judgments` row
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

Owner links:

- [`harness.record_user_judgment` method](api/method-record-user-judgment.md#harnessrecord_user_judgment)
- [Storage Records](storage-records.md)

<a id="harnessclose_task-intentcheck"></a>
### `harness.close_task intent=check`

Read-only calls:

- return computed close readiness
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

- [`harness.close_task` method](api/method-close-task.md)

<a id="harnessclose_task-intentcomplete"></a>
### `harness.close_task intent=complete`

Committed `dry_run=false` may:

- persist the method-selected terminal completion effect
- persist an owner-allowed blocked `complete` effect while the Task remains open
- append events
- create a replay row
- increment `project_state.state_version` once

No-effect branches:

- valid `dry_run=true`
- preflight failures

Valid `dry_run=true` returns `ToolDryRunResponse`. Preflight failures are no-effect `ToolRejectedResponse`.

Owner links:

- [`harness.close_task` method](api/method-close-task.md)
- [Storage Versioning](storage-versioning.md)

<a id="harnessclose_task-intentcancel"></a>
### `harness.close_task intent=cancel`

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

Owner links:

- [`harness.close_task` method](api/method-close-task.md)
- [Storage Versioning](storage-versioning.md)

<a id="harnessclose_task-intentsupersede"></a>
### `harness.close_task intent=supersede`

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

- [`harness.close_task` method](api/method-close-task.md)
- [Storage Versioning](storage-versioning.md)

## Related owners

- [API Methods](api/methods.md) and method owner documents for selected method behavior and response unions.
- [API error routing](api/error-routing.md) and [API error codes](api/error-codes.md) for rejected-response public errors.
- [Storage Records](storage-records.md) for records that effects may touch.
- [Artifact Storage](storage-artifacts.md) for staged-handle and artifact lifecycle details.
- [Storage Versioning](storage-versioning.md) for state clocks and replay/idempotency semantics.
