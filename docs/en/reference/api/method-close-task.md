<a id="harnessclose_task"></a>

# `harness.close_task` reference

## What this document owns

This document owns baseline method behavior for `harness.close_task`:

- method-specific required inputs, access requirements, state-version behavior, result branches, and dry-run behavior
- the minimal request and representative response for the shared account data export confirmation scenario
- method-level storage-effect summary and links to storage owners

## What this document does not own

This document does not own:

- common `ToolEnvelope`, `ToolResultBase`, `ToolRejectedResponse`, or `ToolDryRunResponse` schema bodies
- nested state, artifact, judgment, value-set, or error schema definitions
- storage DDL, storage record layouts, artifact lifecycle, security guarantees, or Core product meaning

## Purpose

Evaluate close readiness for an active Task.

Terminal mutation is allowed only when the selected intent permits mutation and blockers are absent. The method may commit `intent=complete`, `intent=cancel`, or `intent=supersede`, and it may return close blockers.

Close is a Core state transition, not a report. Close is not inferred from chat, status text, acceptance alone, residual-risk acceptance alone, evidence alone, or a rendered view.

## Required inputs

- `ToolEnvelope` with `project_id`, `surface_id`, `request_id`, and `dry_run`.
- `task_id`, `intent`, `close_reason`, `superseding_task_id`, and `user_note`.
- For `intent=complete`, `intent=cancel`, or `intent=supersede` with `dry_run=false`, non-null `idempotency_key` and current `expected_state_version`.
- For `intent=check`, `idempotency_key` and `expected_state_version` may be `null`, and `close_reason` must be `null`.

## Access requirements

| `intent` kind | Conditions |
|---|---|
| `intent=check` | Requires `VerifiedSurfaceContext.access_class=read_status` for protected close-readiness detail. |
| Mutating intents | Require `core_mutation`, verified surface context, compatible Task state, and close-relevant owner records. |

## State version behavior

| Case | State-version effect |
|---|---|
| `intent=check` | Always read-only and never increments state, including when `dry_run=true`. |
| Committed terminal close or committed blocked close for mutating intents | Increments `project_state.state_version` exactly once. |
| Pre-commit failure or dry-run preview | Increments nothing. |

Pre-commit failure includes close preflight rejection, stale `expected_state_version`, stale close-relevant `WriteAuthorization.basis_state_version`, and idempotency request-hash conflict.

## Success result

Returns `CloseTaskResult` with `base.response_kind=result`.

| Case | Effect | `close_state` |
|---|---|---|
| `intent=check` | `base.effect_kind=read_only` | Computed current close state. |
| Successful terminal mutation | `base.effect_kind=core_committed` | `closed`, `cancelled`, or `superseded`. |

## Blocked result

Conditions:

- close preflight succeeds
- `intent=complete`

The method may return `CloseTaskResult(close_state=blocked)` with `blockers: CloseReadinessBlocker[]`. Mutating intents may persist blocker-state effects only when the method state-effect table allows that committed blocked result.

Non-claims:

- The presence of `CloseReadinessBlocker` alone does not imply persistence.
- `STATE_VERSION_CONFLICT` is never a `CloseReadinessBlocker.code`.

## Rejected result

Returns `ToolRejectedResponse` for close preflight failures before close-readiness evaluation, such as:

- validation failure
- local access failure
- stale `expected_state_version`
- stale close-relevant Write Authorization basis
- idempotency request-hash conflict
- wrong-project or unreadable Task identity
- unavailable Core
- insufficient capability

Non-claims:

- Rejected responses return no `CloseTaskResult.blockers`.
- Rejected responses create no close effect.

## Dry-run behavior

`intent=check` with `dry_run=true` remains the read-only `CloseTaskResult` branch.

Mutating intents with `dry_run=true` use the common preview branch when valid.

Branch shape and planned-blocker representation are owned by [API Schema Core](schema-core.md) and [API Errors](errors.md).

## Storage effect

`intent=check` has no storage effect. Mutating close intents may persist close or blocker outcomes according to the method result. Exact storage effects are owned by [Storage Effects](../storage-effects.md).

Close-readiness scenario data:

The literal `intent=complete` selects the completion intent. It is not shorthand for the full close-readiness evaluation order.

Successful close-readiness observation for the account data export confirmation scenario. The evidence relies on existing run ref `run_account_export_tests_001`, promoted artifact `artifact_account_export_test_log_001`, and resolved user judgment `uj_001`:

```yaml
close_readiness:
  ready: true
  evidence:
    - "Account data export confirmation tests passed."
    - "User accepted the account data export confirmation copy."
```

Blocked close-readiness observation for the same scenario. This is the version-21 variant used by the representative response below: the test evidence is recorded in existing run ref `run_account_export_tests_001` with promoted artifact `artifact_account_export_test_log_001`, and no resolved user judgment is available:

```yaml
close_readiness:
  ready: false
  blockers:
    - code: missing_user_judgment
      message: "User acceptance is missing for the account data export confirmation copy."
```

## Minimal valid request

```yaml
method: harness.close_task
params:
  envelope:
    project_id: proj_123
    task_id: task_456
    actor_kind: agent
    surface_id: surface_local
    request_id: req_close_check_001
    idempotency_key: null
    expected_state_version: null
    dry_run: false
    locale: en-US
  task_id: task_456
  intent: check
  close_reason: null
  superseding_task_id: null
  user_note: null
```

## Representative response

Blocked read-only result branch (`CloseTaskResult`, `intent=check`):

```yaml
base:
  response_kind: result
  effect_kind: read_only
  dry_run: false
  state_version: 21
  events: []
close_state: blocked
state:
  project_id: proj_123
  state_version: 21
  task_ref:
    record_kind: task
    record_id: task_456
    project_id: proj_123
    task_id: task_456
    state_version: 21
blockers:
  - category: user_judgment
    code: missing_user_judgment
    message: "User acceptance is missing for the account data export confirmation copy."
    related_refs: []
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
next_actions:
  - action: harness.request_user_judgment
    reason: "Ask the user to accept the account data export confirmation copy before attempting close."
```

## Owner links

- Request envelope, common response branches, and dry-run summaries: [API Schema Core](schema-core.md).
- Close-readiness shapes, `CloseReadinessBlocker`, `EvidenceSummary`, and `StateSummary`: [API State Schemas](schema-state.md).
- Close state, lifecycle, close reason, and blocker values: [API Value Sets](schema-value-sets.md).
- Close-readiness meaning and close honesty: [Core Model close readiness](../core-model.md#close_task).
- Public errors and close blocker routing: [API Errors](errors.md) and [`close_task` blocker mapping](errors.md#harnessclose_task-close-blockers).
- Persistence effects and state-version behavior: [Storage Effects](../storage-effects.md) and [Storage Versioning](../storage-versioning.md).
