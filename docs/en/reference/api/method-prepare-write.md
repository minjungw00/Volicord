<a id="harnessprepare_write"></a>

# `harness.prepare_write` reference

## What this document owns

This document owns baseline method behavior for `harness.prepare_write`:

- method-specific required inputs, access requirements, state-version behavior, result branches, and dry-run behavior
- the minimal request and representative response for the shared account data export confirmation scenario
- method-level storage-effect summary and links to storage owners

## What this document does not own

This document does not own:

- common `ToolEnvelope`, `ToolResultBase`, `ToolRejectedResponse`, or `ToolDryRunResponse` schema bodies
- nested state, artifact, judgment, value-set, or error schema definitions
- storage DDL, storage record layouts, artifact lifecycle, security guarantees, or Core product meaning

## Purpose

Check one proposed product-file write against:

- current Task
- active Change Unit
- scope
- baseline
- required separate sensitive-action approval
- verified local surface capability

When the check is allowed, the method creates a consumable single-use Write Authorization. When the check is not allowed, the method denies or defers that Write Authorization path.

Security non-claims belong to [Security](../security.md).

## Required inputs

- `ToolEnvelope` with non-null `idempotency_key` and current `expected_state_version` for non-dry-run commits.
- `task_id` and `change_unit_id`, or `null` only when owner resolution can unambiguously use the active Task and active Change Unit.
- `intended_operation`, `intended_paths`, `product_file_write_intended`, `sensitive_categories`, and `baseline_ref`.

## Access requirements

Requires:

- `VerifiedSurfaceContext.access_class=write_authorization`
- `verified=true`
- compatible active scope
- compatible baseline
- required user-owned judgments
- any separate sensitive-action approval (`sensitive_approval`)
- local surface capability needed for the intended product-file write check

## State version behavior

| Result | State-version effect | Write Authorization effect |
|---|---|---|
| Committed `decision=allowed` | Increments `project_state.state_version` exactly once. | Creates one active Write Authorization. |
| Committed non-allow decision | May increment only for method-owned write-decision reason state. | Creates no consumable Write Authorization. |
| Pre-commit rejection or dry run | Increments nothing. | Creates nothing. |

## Success result

Returns `PrepareWriteResult` with:

- `base.response_kind=result`
- `base.effect_kind=core_committed`

For `decision=allowed`:

- `write_authorization_ref` is non-null
- `write_authorization` is non-null
- `authorization_effect` is `created` for a new commit or `returned` for an idempotent replay
- the authorization is scoped to the path-level `AuthorizedAttemptScope`
- `active_user_judgment_refs` may cite resolved user-owned judgments that satisfy write preconditions, including a separate `sensitive_approval`

## Blocked result

Committed blocked decisions are `PrepareWriteResult` values with one of these decision values:

- `decision=blocked`
- `decision=approval_required`
- `decision=decision_required`

Result data:

- `write_decision_reasons` must be non-empty.

Non-claims:

- `write_decision_reasons` are not `CloseReadinessBlocker` values.
- `write_decision_reasons` do not evaluate close readiness.
- No consumable Write Authorization is created.

## Rejected result

Returns `ToolRejectedResponse` for failures before decision evaluation or commit, including:

- stale `expected_state_version`
- idempotency request-hash conflict
- request validation failure
- missing active Task or Change Unit
- local access failure
- Core unavailability
- stale baseline
- invalid requested guarantee
- capability failure

Non-claim: `STATE_VERSION_CONFLICT` is always a rejected response error, never a write decision reason.

## Dry-run behavior

For `dry_run=true`, a valid preview returns `ToolDryRunResponse`. Branch shape is owned by [API Schema Core](schema-core.md); no-effect persistence semantics are owned by [Storage Effects](../storage-effects.md).

## Storage effect

On commit, the method may persist Write Authorization or write-decision state according to the method result. Exact storage effects are owned by [Storage Effects](../storage-effects.md).

## Minimal valid request

The sample uses `personal_data_export` as an example `sensitive_categories` value for account data export that may include personal data. This method example does not define `personal_data_export` as a new supported value or define the full sensitive-category value set.

```yaml
method: harness.prepare_write
params:
  envelope:
    project_id: proj_123
    task_id: task_456
    actor_kind: agent
    surface_id: surface_local
    request_id: req_prepare_001
    idempotency_key: idem_prepare_001
    expected_state_version: 19
    dry_run: false
    locale: en-US
  task_id: task_456
  change_unit_id: cu_001
  intended_operation: "update account data export flow to require explicit confirmation"
  intended_paths:
    - src/account/export.ts
    - src/account/export-confirmation.ts
    - tests/account-export.test.ts
  product_file_write_intended: true
  sensitive_categories:
    - personal_data_export
  baseline_ref: baseline_account_export_001
```

## Representative response

### Allowed branch

This branch applies after the separate sensitive-action approval is already present.

The existing sensitive-action approval is represented by `active_user_judgment_refs` at `state_version: 19`.
`uj_sensitive_export_001` represents an existing resolved `judgment_kind=sensitive_approval` whose `SensitiveActionScope` matches the account data export step and is needed before `Write Authorization`.
The account data export confirmation copy judgment is handled separately by the user-judgment methods and is not the same approval.

```yaml
base:
  response_kind: result
  effect_kind: core_committed
  dry_run: false
  state_version: 20
  events:
    - event_id: evt_1003
      event_kind: write_authorization_created
decision: allowed
state:
  project_id: proj_123
  state_version: 20
  task_ref:
    record_kind: task
    record_id: task_456
    project_id: proj_123
    task_id: task_456
    state_version: 20
write_authorization_ref:
  record_kind: write_authorization
  record_id: wa_001
  project_id: proj_123
  task_id: task_456
  state_version: 20
write_authorization:
  authorization_id: wa_001
  status: active
  basis_state_version: 19
  authorized_paths:
    - src/account/export.ts
    - src/account/export-confirmation.ts
    - tests/account-export.test.ts
authorization_effect: created
active_user_judgment_refs:
  - record_kind: user_judgment
    record_id: uj_sensitive_export_001
    project_id: proj_123
    task_id: task_456
    state_version: 19
write_decision_reasons: []
user_judgment_candidate: null
guarantee_display:
  level: cooperative
  notes:
    - "Write Authorization is a Harness compatibility record, not OS permission."
```

### Approval-required branch excerpt

This branch applies when the matching sensitive-action approval is missing.

The approval-required reason corresponds to request `sensitive_categories: [personal_data_export]`.

```yaml
decision: approval_required
write_authorization_ref: null
write_authorization: null
authorization_effect: none
write_decision_reasons:
  - code: sensitive_export_flow
    message: "Account data export may include personal data and requires separate sensitive-action approval before Write Authorization."
```

## Owner links

- Request envelope, common result branches, and dry-run summaries: [API Schema Core](schema-core.md).
- `WriteAuthorizationSummary`, state summaries, and refs: [API State Schemas](schema-state.md).
- `SensitiveActionScope` and user-owned approval boundaries: [API Judgment Schemas](schema-judgment.md).
- Supported values and access classes: [API Value Sets](schema-value-sets.md).
- Public errors, `STATE_VERSION_CONFLICT`, and blocked/dry-run behavior: [API error codes](error-codes.md), [state version conflict](error-precedence.md#state-conflict-behavior), and [API error routing](error-routing.md).
- Persistence effects and state clocks: [Storage Effects](../storage-effects.md) and [Storage Versioning](../storage-versioning.md).
