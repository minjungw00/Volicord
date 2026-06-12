# Storage effects

This document owns method-to-storage effect semantics for the current MVP source design. It is documentation source material only and does not execute or simulate Harness runtime procedures.

## Owns / Does not own

This document owns:

- read-only, dry-run, rejected, staging-created, Core-committed, and committed-blocked storage-effect distinctions
- whether a method branch creates replay rows, `task_events`, record changes, state-version increments, staged-handle creation or consumption, artifact promotion, or `Write Authorization` changes
- the persistence boundary for blocker-like response data
- no-effect guarantees for rejected branches and valid dry-run preview branches

This document does not own:

- record layout or DDL; see [Storage Records](storage-records.md)
- artifact lifecycle details; see [Artifact Storage](storage-artifacts.md)
- idempotency, locks, state-version clocks, event ordering, or migrations; see [Storage Versioning](storage-versioning.md)
- public response branches or schemas; see [API Schema Core](api/schema-core.md)
- API method behavior; see the [MVP API router](api/mvp-api.md) and method owner documents
- public error code precedence; see [API Errors](api/errors.md)

## Shape versus effect

Response data shape and storage effect are separate.

API data shapes include:
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
| `StageArtifactResult` with `effect_kind=staging_created` | Temporary staging only | [Staging-created artifact result](#staging-created-artifact-result) |
| Core committed `MethodResult` | Method-owned committed effects | [Core committed result](#core-committed-result) |
| Committed blocked `MethodResult` | Explicitly allowed blocked effects only | [Committed blocked result](#committed-blocked-result) |

<a id="read-only-result"></a>
### Read-only result

Storage effect:

- Response only.

Not allowed:

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

Not allowed:

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

Not allowed:

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

- storage-owned temporary staging

Not allowed:

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

Not allowed:

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

Allowed response computation: `harness.status` and `harness.close_task intent=check` may compute blockers, `CloseReadinessBlocker[]`, evidence summaries, artifact refs, diagnostics, and next actions for the response.

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

- The response and replay payload may record `write_decision_reasons: WriteDecisionReason[]`.
- This is allowed only when the method contract permits the committed decision record.

Not allowed:

- creating consumable `Write Authorization`
- changing `close_state`
- evaluating close readiness
- storing `CloseReadinessBlocker`
- updating evidence
- changing artifacts
- consuming staged handles
- applying `close_task` effects

Example account data export write-decision data:

For the request-side `harness.prepare_write` payload fields, see [`method-prepare-write.md`](api/method-prepare-write.md). This section only describes the storage effect of recording the write decision and its reasons.

```yaml
intended_operation: "update account data export confirmation flow"
intended_paths:
  - src/account/export.ts
  - src/account/export-confirmation.ts
  - tests/account-export.test.ts
sensitive_categories:
  - personal_data_export
decision: approval_required
write_decision_reasons:
  - code: sensitive_export_flow
    message: "Account data export may include personal data and requires separate sensitive-action approval before Write Authorization."
```

Those reasons are prepare-write decision reasons. They are not:

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

Result:

- The Task remains open.

Not allowed:

- using this branch for `STATE_VERSION_CONFLICT`
- storing `STATE_VERSION_CONFLICT` as replay

`STATE_VERSION_CONFLICT` belongs to the preflight `ToolRejectedResponse` branch.

<a id="method-effects"></a>
## Method effect summary

This table summarizes persistence effects. Method behavior and response unions remain owned by method owner documents routed from the [MVP API router](api/mvp-api.md).

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
| `harness.close_task intent=complete` | closes or records blocked `complete` outcome | See [`harness.close_task intent=complete`](#harnessclose_task-intentcomplete) |
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

- [`harness.intake` method](api/method-intake.md)
- [Storage Records](storage-records.md)
- [Storage Versioning](storage-versioning.md)

### `harness.update_scope`

Committed `dry_run=false` may:

- update active Task scope fields
- create or replace active `change_units`
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

### `harness.prepare_write`

Committed `dry_run=false` with `decision=allowed` may:

- create or return a compatible active `Write Authorization`
- append events
- create a replay row
- increment `project_state.state_version` once

Committed non-allowed decisions:

- See [`harness.prepare_write` committed non-allow decision](#harnessprepare_write-committed-non-allow-decision).

For account data export that may include personal data, a persisted write decision may record only the separate sensitive-action approval requirement before `Write Authorization`:

```yaml
decision: approval_required
write_decision_reasons:
  - code: sensitive_export_flow
    message: "Account data export may include personal data and requires separate sensitive-action approval before Write Authorization."
```

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

### `harness.stage_artifact`

Successful staging may:

- create `artifact_staging` or an equivalent storage-owned staging manifest
- store temporary safe bytes or notices under `artifacts/tmp/`

This branch creates only temporary storage-owned staging.

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
- staging manifest
- `StagedArtifactHandle`
- replay row
- `project_state.state_version` increment

Owner links:

- [`harness.stage_artifact` method](api/method-stage-artifact.md)
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

For an account export confirmation test run, a committed `harness.record_run` may record the run, promote the staged test log, and update evidence:

```yaml
command: "npm test -- account-export"
summary: "Account export confirmation tests passed."
artifacts:
  - staged_artifact_account_export_test_log_001
run_ref: run_account_export_tests_001
state_version: 21
```

Owner links:

- [`harness.record_run` method](api/method-record-run.md)
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

Valid dry-run previews do not create:

- real `user_judgment_ref`
- pending judgment
- blocker update
- event
- replay row
- `project_state.state_version` increment

Owner links:

- [`harness.request_user_judgment` method](api/method-user-judgment.md#harnessrequest_user_judgment)
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

Valid dry-run previews do not create:

- judgment resolution
- blocker update
- event
- replay row
- `project_state.state_version` increment

Owner links:

- [`harness.record_user_judgment` method](api/method-user-judgment.md#harnessrecord_user_judgment)
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

- [`harness.close_task` method](api/method-close-task.md)

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

- [`harness.close_task` method](api/method-close-task.md)
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

- [`harness.close_task` method](api/method-close-task.md)
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

- [`harness.close_task` method](api/method-close-task.md)
- [Storage Versioning](storage-versioning.md)

## Related owners

- [MVP API router](api/mvp-api.md) and method owner documents for selected method behavior and response unions.
- [API Errors](api/errors.md) for rejected-response public errors.
- [Storage Records](storage-records.md) for records that effects may touch.
- [Artifact Storage](storage-artifacts.md) for staged-handle and artifact lifecycle details.
- [Storage Versioning](storage-versioning.md) for state clocks and replay/idempotency semantics.
