# Storage effects

This document owns method-to-storage effect semantics for the current MVP source design. It is documentation source material only and does not execute or simulate Harness runtime procedures.

## Owns / Does not own

This document owns:

- read-only, dry-run, rejected, staging-created, Core-committed, and committed-blocked storage-effect distinctions
- whether a method branch creates replay rows, `task_events`, record changes, state-version increments, staged-handle consumption, artifact promotion, or Write Authorization changes
- the persistence boundary for blocker-like response data
- no-effect guarantees for rejected branches and valid dry-run preview branches

This document does not own:

- record layout or DDL; see [Storage Records](storage-records.md)
- artifact lifecycle details; see [Artifact Storage](storage-artifacts.md)
- idempotency, locks, state-version clocks, event ordering, or migrations; see [Storage Versioning](storage-versioning.md)
- public response branches or schemas; see [API Schema Core](api/schema-core.md)
- API method behavior; see [MVP API](api/mvp-api.md)
- public error code precedence; see [API Errors](api/errors.md)

## Shape versus effect

Response data shape and storage effect are separate. `CloseReadinessBlocker`, `WriteDecisionReason`, `PlannedBlocker`, `ArtifactRef`, and `StagedArtifactHandle` are API data shapes. Their presence in a response does not by itself prove persistence, artifact promotion, staged-handle consumption, replay storage, close-state mutation, or `project_state.state_version` increment.

Effects come from the selected method behavior and response branch:

| Branch | Storage effect |
|---|---|
| Read-only `MethodResult` | Response only. No replay row, event, current-row mutation, artifact effect, Write Authorization effect, or state-version increment. |
| `ToolRejectedResponse` | No effect. No current row, no replay row, no event, no artifact effect, no Write Authorization creation/consumption, no state-version increment. |
| Valid `ToolDryRunResponse` | Preview only. No current row, no generated persistent ref, no replay row, no event, no staged handle, no artifact promotion/link, no state-version increment. |
| `StageArtifactResult` with `effect_kind=staging_created` | Temporary storage-owned staging only. No Core current row, replay row, event, persistent `ArtifactRef`, or state-version increment. |
| Core committed `MethodResult` | May mutate current rows, append `task_events`, create replay rows, and increment `project_state.state_version` exactly once as allowed by the method owner. |
| Committed blocked `MethodResult` | May persist only the blocker-state, event, replay-row, and state-version effects explicitly allowed by the method owner. It must not create the missing authority it reports. |

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
- create or consume Write Authorizations
- change `artifact_staging.status`
- set `consumed_by_run_id` or `promoted_artifact_id`
- promote or link artifacts
- increment `project_state.state_version`

Valid dry-run previews may include `DryRunSummary.would_blockers: PlannedBlocker[]` or planned effects. Those preview entries do not create:

- `task_event` or `task_events` append
- replay row or `tool_invocations.response_json`
- `close_state` mutation
- Write Authorization change
- staged-handle creation or consumption
- artifact effect
- evidence update
- `CloseReadinessBlocker` storage
- `project_state.state_version` increment

## Read-only effects

Read-only results are response-only and not replay rows. `harness.status` and `harness.close_task intent=check` may compute blockers, `CloseReadinessBlocker[]`, evidence summaries, artifact refs, diagnostics, and next actions for the response.

Storage must not persist those computed values merely because the read occurred.

`harness.status` with `close_blockers: CloseReadinessBlocker[]` is a read-only observation. It does not create:

- `task_event` or `task_events` append
- replay row or `tool_invocations.response_json`
- `close_state` mutation
- Write Authorization change
- staged-handle consumption
- artifact effect
- evidence update
- `project_state.state_version` increment

For `harness.close_task intent=check`, the response branch is owned by [`harness.close_task`](api/mvp-api.md#harnessclose_task). This storage page only asserts that the check remains read-only, including with `dry_run=true` and with `blockers: CloseReadinessBlocker[]`.

## Committed blocked effects

Committed blocked outcomes are distinct from rejected responses. A committed blocked `harness.prepare_write` or `harness.close_task` outcome is a `MethodResult` only when [MVP API](api/mvp-api.md) allows the blocked commit.

A committed non-dry-run `PrepareWriteResult` with `decision=blocked`, `decision=approval_required`, or `decision=decision_required` may include `write_decision_reasons: WriteDecisionReason[]` in the response and replay payload when the method state-effect contract permits committing that decision.

Those reasons are prepare-write decision reasons. They are not:

- close-readiness blockers
- `CloseReadinessBlocker[]`
- close-readiness blocker records

This branch must not:

- create a consumable Write Authorization
- mutate `close_state`
- run close-readiness evaluation
- create `CloseReadinessBlocker` storage
- update evidence
- touch artifacts
- consume staged handles
- perform `close_task` effects

`CloseTaskResult(close_state=blocked)` is storage-effective only when close readiness evaluation has run and the `harness.close_task` method contract permits committing the blocked result. It may include `blockers: CloseReadinessBlocker[]` and may create only the effects explicitly allowed by the API/storage contract:

- blocker state
- `task_events`
- replay row
- `project_state.state_version`

The Task remains open. This branch must not be used for `STATE_VERSION_CONFLICT`; that code belongs to the preflight `ToolRejectedResponse` branch and is not stored as replay.

<a id="method-effects"></a>
## Method effect summary

This table summarizes persistence effects. Method behavior and response unions remain owned by [MVP API](api/mvp-api.md).

| Method | Primary storage effect | Details |
|---|---|---|
| `harness.intake` | creates task and shaping records | See [`harness.intake`](#harnessintake) |
| `harness.update_scope` | updates active scope records | See [`harness.update_scope`](#harnessupdate_scope) |
| `harness.status` | read-only response | See [`harness.status`](#harnessstatus) |
| `harness.prepare_write` | records write decision effects | See [`harness.prepare_write`](#harnessprepare_write) |
| `harness.stage_artifact` | creates temporary staging only | See [`harness.stage_artifact`](#harnessstage_artifact) |
| `harness.record_run` | records run/evidence effects | See [`harness.record_run`](#harnessrecord_run) |
| `harness.request_user_judgment` | creates pending judgment request | See [`harness.request_user_judgment`](#harnessrequest_user_judgment) |
| `harness.record_user_judgment` | resolves user judgment | See [`harness.record_user_judgment`](#harnessrecord_user_judgment) |
| `harness.close_task intent=check` | read-only close-readiness check | See [`harness.close_task intent=check`](#harnessclose_task-intentcheck) |
| `harness.close_task intent=complete` | closes or records blocked complete outcome | See [`harness.close_task intent=complete`](#harnessclose_task-intentcomplete) |
| `harness.close_task intent=cancel` | cancels or records blocked cancellation | See [`harness.close_task intent=cancel`](#harnessclose_task-intentcancel) |
| `harness.close_task intent=supersede` | supersedes or records blocked supersession | See [`harness.close_task intent=supersede`](#harnessclose_task-intentsupersede) |

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

- [MVP API](api/mvp-api.md#harnessintake)
- [Storage Records](storage-records.md)
- [Storage Versioning](storage-versioning.md)

### `harness.update_scope`

Committed `dry_run=false` may:

- update active Task scope fields
- create or replace active `change_units`
- update blockers or stale Write Authorization refs as the method owner allows
- append events
- create a replay row
- increment `project_state.state_version` once

No-effect branches:

- valid dry-run previews
- rejected attempts

Valid dry-run previews only describe scope, Change Unit, blocker, and stale authorization effects.

Owner links:

- [MVP API](api/mvp-api.md#harnessupdate_scope)
- [Storage Records](storage-records.md)
- [Storage Versioning](storage-versioning.md)

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

- [MVP API](api/mvp-api.md#harnessstatus)

### `harness.prepare_write`

Committed `dry_run=false` with `decision=allowed` may:

- create or return a compatible active Write Authorization
- append events
- create a replay row
- increment `project_state.state_version` once

Committed non-allowed decisions may persist only allowed decision-state and replay effects.

No-effect branches:

- rejected attempts
- valid dry-run previews

Those branches create no replay row, Write Authorization, event, close-state mutation, artifact/evidence effect, or state-version increment.

Owner links:

- [MVP API](api/mvp-api.md#harnessprepare_write)
- [Storage Records](storage-records.md)
- [Storage Versioning](storage-versioning.md)

### `harness.stage_artifact`

Successful staging may:

- create `artifact_staging` or an equivalent storage-owned staging manifest
- store temporary safe bytes or notices under `artifacts/tmp/`

This branch creates only temporary storage-owned staging. It creates no Core current row, persistent `ArtifactRef`, replay row, or state-version increment.

No-effect branches:

- valid `dry_run=true`
- invalid staging requests

Valid `dry_run=true` creates no bytes, staging manifest, `StagedArtifactHandle`, replay row, or state-version increment.

Owner links:

- [MVP API](api/mvp-api.md#harnessstage_artifact)
- [Artifact Storage](storage-artifacts.md)

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

Valid dry-run previews create no `run_summary`, persistent artifact, artifact link, evidence update, blocker update, event, replay row, staged-handle consumption, Write Authorization consumption, or state-version increment. Rejected attempts do not change staging rows or artifacts.

Owner links:

- [MVP API](api/mvp-api.md#harnessrecord_run)
- [Artifact Storage](storage-artifacts.md)
- [Storage Records](storage-records.md)

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

Valid dry-run previews create no real `user_judgment_ref`, pending judgment, blocker update, event, replay row, or state-version increment.

Owner links:

- [MVP API](api/mvp-api.md#harnessrequest_user_judgment)
- [Storage Records](storage-records.md)

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

Valid dry-run previews create no judgment resolution, blocker update, event, replay row, or state-version increment.

Owner links:

- [MVP API](api/mvp-api.md#harnessrecord_user_judgment)
- [Storage Records](storage-records.md)

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

- [MVP API](api/mvp-api.md#harnessclose_task)

### `harness.close_task intent=complete`

Committed `dry_run=false` may:

- close the Task when blockers allow it
- commit allowed blocked `complete` effects while the Task remains open
- append events
- create a replay row
- increment `project_state.state_version` once

No-effect branches:

- valid `dry_run=true`
- preflight failures

Valid `dry_run=true` returns `ToolDryRunResponse`. Preflight failures are no-effect `ToolRejectedResponse`.

Owner links:

- [MVP API](api/mvp-api.md#harnessclose_task)
- [Storage Versioning](storage-versioning.md)

### `harness.close_task intent=cancel`

Committed `dry_run=false` may:

- cancel the Task
- commit blockers that invalidate cancellation itself while the Task remains open
- append events
- create a replay row
- increment `project_state.state_version` once

Cancellation is not evidence sufficiency.

No-effect branches:

- valid `dry_run=true`
- preflight failures

Valid `dry_run=true` returns `ToolDryRunResponse`.

Owner links:

- [MVP API](api/mvp-api.md#harnessclose_task)
- [Storage Versioning](storage-versioning.md)

### `harness.close_task intent=supersede`

Committed `dry_run=false` may:

- supersede the Task
- update `project_state.active_task_id` in the same mutation
- commit blockers that invalidate supersession itself
- append events
- create a replay row
- increment `project_state.state_version` once

Supersession is not evidence sufficiency.

No-effect branches:

- valid `dry_run=true`
- preflight failures

Valid `dry_run=true` returns `ToolDryRunResponse`.

Owner links:

- [MVP API](api/mvp-api.md#harnessclose_task)
- [Storage Versioning](storage-versioning.md)

## Related owners

- [MVP API](api/mvp-api.md) for selected method behavior and response unions.
- [API Errors](api/errors.md) for rejected-response public errors.
- [Storage Records](storage-records.md) for records that effects may touch.
- [Artifact Storage](storage-artifacts.md) for staged-handle and artifact lifecycle details.
- [Storage Versioning](storage-versioning.md) for state clocks and replay/idempotency semantics.
