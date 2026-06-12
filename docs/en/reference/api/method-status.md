<a id="harnessstatus"></a>

# `harness.status` reference

## What this document owns

This document owns baseline method behavior for `harness.status`:

- method-specific required inputs, access requirements, state-version behavior, result branches, and dry-run behavior
- the minimal request and representative response for the shared account data export confirmation scenario
- method-level storage-effect expectations before storage owners define record-level details

## What this document does not own

This document does not own:

- common `ToolEnvelope`, `ToolResultBase`, `ToolRejectedResponse`, or `ToolDryRunResponse` schema bodies
- nested state, artifact, judgment, value-set, or error schema definitions
- storage DDL, storage record layouts, artifact lifecycle, security guarantees, or Core product meaning

## Purpose

Return a read-only current-position view over Core state: active Task summary, blockers, pending user judgments, Write Authorization summary, evidence summary, close state, close-readiness findings, guarantee display, and next safe actions.

## Required inputs

- `ToolEnvelope` with `project_id`, `surface_id`, `request_id`, and `dry_run`; `idempotency_key` and `expected_state_version` may be `null`.
- `include` flags selecting which summaries the caller needs.

## Access requirements

Condition: protected Core detail is returned.

Requires:

- same-project active local surface
- `VerifiedSurfaceContext.access_class=read_status`

Non-claim: a stale projection, chat summary, generated Markdown file, or cached text is not state authority.

## State version behavior

No state change occurs and `project_state.state_version` never increments.

The result may report the current observed state version.

The method creates no:

- event
- replay row
- close mutation
- artifact effect
- staged-handle consumption
- evidence update
- Write Authorization change

## Success result

Returns `StatusResult` with:

- `base.response_kind=result`
- `base.effect_kind=read_only`

When `include.close=true`, `StatusResult.close_blockers` are read-only `CloseReadinessBlocker[]` observations.

Non-claim: `StatusResult.close_blockers` are not stored close results.

## Blocked result

There is no committed blocked branch.

Blockers and close blockers in a `StatusResult` are computed response fields only.

## Rejected result

Returns `ToolRejectedResponse` only when the read cannot be safely served, such as:

- unavailable Core
- local access mismatch
- insufficient capability for the requested protected detail
- missing active Task for a Task-scoped read
- stale or unavailable projection when such a view was requested

Public error code meaning and precedence are owned by [API Errors](errors.md).

## Dry-run behavior

`dry_run=true` does not create a `ToolDryRunResponse` branch for this read-only method.

A valid request returns the same `StatusResult` shape with:

- `base.dry_run=true`
- `base.effect_kind=read_only`

Branch rules are owned by [API Schema Core](schema-core.md).

## Storage effect

This is a read-only method. Exact no-effect persistence semantics are owned by [Storage Effects](../storage-effects.md).

## Minimal valid request

```yaml
method: harness.status
params:
  envelope:
    project_id: proj_123
    task_id: task_456
    actor_kind: agent
    surface_id: surface_local
    request_id: req_status_001
    idempotency_key: null
    expected_state_version: null
    dry_run: false
    locale: en-US
  include:
    task: true
    pending_user_judgments: true
    write_authority: true
    evidence: true
    close: true
    guarantees: true
```

## Representative response

Result branch (`StatusResult`, read-only). This status snapshot is observed after `harness.record_run` has created `run_account_export_tests_001` and promoted `artifact_account_export_test_log_001` as evidence:

```yaml
base:
  response_kind: result
  effect_kind: read_only
  dry_run: false
  state_version: 21
  events: []
active_task:
  project_id: proj_123
  state_version: 21
  task_ref:
    record_kind: task
    record_id: task_456
    project_id: proj_123
    task_id: task_456
    state_version: 21
  mode: work
  lifecycle:
    lifecycle_phase: ready
    close_reason: none
    result: none
    closed_at: null
  goal_summary: "Add explicit confirmation before account data export."
  scope_summary: "Account data export flow and account data export confirmation tests."
  active_change_unit_ref:
    record_kind: change_unit
    record_id: cu_001
    project_id: proj_123
    task_id: task_456
    state_version: 21
status_summary: "Account data export confirmation tests are recorded. User acceptance of the account data export confirmation copy is still pending."
next_actions:
  - action: harness.request_user_judgment
    reason: "Ask the user to accept the account data export confirmation copy before close."
pending_user_judgments: []
write_authority_summary:
  status: stale
  write_authorization_ref:
    record_kind: write_authorization
    record_id: wa_001
    project_id: proj_123
    task_id: task_456
    state_version: 20
  basis_state_version: 19
  intended_paths:
    - src/account/export.ts
    - src/account/export-confirmation.ts
    - tests/account-export.test.ts
  guarantee_display:
    level: cooperative
    notes:
      - "Write Authorization is a Harness compatibility record, not OS permission."
evidence_summary:
  status: sufficient
  coverage_items:
    - claim: "Account data export confirmation tests passed."
      required_for_close: true
      coverage_state: supported
      supporting_refs:
        - record_kind: run
          record_id: run_account_export_tests_001
          project_id: proj_123
          task_id: task_456
          state_version: 21
      supporting_artifact_refs:
        - artifact_id: artifact_account_export_test_log_001
          project_id: proj_123
          task_id: task_456
          display_name: "account_export_confirmation_test.log"
          content_type: text/plain
          sha256: sha256:example
          size_bytes: 65
          redaction_state: none
          availability: available
          created_by_run_ref:
            record_kind: run
            record_id: run_account_export_tests_001
            project_id: proj_123
            task_id: task_456
            state_version: 21
          created_by_surface_id: surface_local
          created_by_surface_instance_id: surface_instance_01
          storage_ref: artifact://artifact_account_export_test_log_001
      gap_refs: []
  artifact_refs:
    - artifact_id: artifact_account_export_test_log_001
      project_id: proj_123
      task_id: task_456
      display_name: "account_export_confirmation_test.log"
      content_type: text/plain
      sha256: sha256:example
      size_bytes: 65
      redaction_state: none
      availability: available
      created_by_run_ref:
        record_kind: run
        record_id: run_account_export_tests_001
        project_id: proj_123
        task_id: task_456
        state_version: 21
      created_by_surface_id: surface_local
      created_by_surface_instance_id: surface_instance_01
      storage_ref: artifact://artifact_account_export_test_log_001
blocker_refs: []
close_readiness:
  ready: false
  blockers:
    - code: missing_user_judgment
      message: "The user has not accepted the account data export confirmation copy."
guarantee_display:
  level: cooperative
  notes:
    - "No stronger local guarantee is active."
```

## Owner links

- Request envelope and response branches: [API Schema Core](schema-core.md).
- Status state, close-readiness shapes, evidence summaries, and guarantee display: [API State Schemas](schema-state.md).
- Active values and access classes: [API Value Sets](schema-value-sets.md).
- Public errors and close blocker routing: [API Errors](errors.md) and [`close_task` blocker mapping](errors.md#harnessclose_task-close-blockers).
- Persistence effects: [Storage Effects](../storage-effects.md).
