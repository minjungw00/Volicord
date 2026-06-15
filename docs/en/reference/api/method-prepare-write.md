<a id="harnessprepare_write"></a>

# `harness.prepare_write` reference

## What this document owns

This document owns baseline method behavior for `harness.prepare_write`:

- method-specific required inputs, access requirements, state version behavior, result branches, and `dry_run` behavior
- `PrepareWriteResult` decision behavior
- method-specific handling for creating one consumable `Write Authorization`
- prepare-write examples

## What this document does not own

This document does not own:

- common request envelope, response branch, dry-run, or rejected-response schema bodies
- nested state, judgment, value-set, or error schema definitions
- Core meaning of `Write Authorization`, ordinary write approval, sensitive-action approval, final acceptance, residual-risk acceptance, or user-owned judgment
- storage DDL, storage record layouts, exact storage effects, artifact lifecycle, or security guarantees
- public error code meaning, public error precedence, or shared response-branch routing

## Purpose

`harness.prepare_write` checks one proposed product-file write against:

- current Task
- currently applied Change Unit
- current scope
- baseline
- required separate sensitive-action approval
- verified local surface capability

When the check is allowed, the method creates a consumable single-use `Write Authorization`. When the check is not allowed, the method denies or defers that `Write Authorization` path.

Security non-claims belong to [Security](../security.md).

## Required inputs

- A valid `ToolEnvelope`; committed non-dry-run requests require non-null `idempotency_key` and current `expected_state_version`.
- `task_id` and `change_unit_id`, or `null` only when owner resolution can unambiguously use the current Task and currently applied Change Unit.
- `intended_operation`, `intended_paths`, `product_file_write_intended`, `sensitive_categories`, and `baseline_ref`.

## Access requirements

Requires:

- `VerifiedSurfaceContext.access_class=write_authorization`
- `verified=true`
- compatible current scope
- compatible baseline
- required user-owned judgments
- any separate sensitive-action approval (`sensitive_approval`)
- local surface capability needed for the intended product-file write check

## State version behavior

| Result | State-version effect | `Write Authorization` effect |
|---|---|---|
| Committed `decision=allowed` | Increments `project_state.state_version` exactly once. | Creates one `status=active` `Write Authorization`. |
| Committed non-allow decision | May increment only for method-owned write-decision reason state. | Creates no consumable `Write Authorization`. |
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
- No consumable `Write Authorization` is created.

## Rejected result

Returns `ToolRejectedResponse` for failures before decision evaluation or commit, including:

- stale `expected_state_version`
- idempotency request-hash conflict
- request validation failure
- missing current Task or currently applied Change Unit
- local access failure
- Core unavailability
- stale baseline
- invalid requested guarantee
- capability failure

Non-claim: `STATE_VERSION_CONFLICT` is always a rejected response error, never a write decision reason.

Public error code meaning, precedence, and rejected-response routing are owned by the error documents linked below.

## Dry-run behavior

For `dry_run=true`, a valid preview:

- returns `ToolDryRunResponse`
- creates no `Write Authorization`
- persists no write-decision state

## Storage effect

On commit, the method may persist `Write Authorization` or write-decision state according to the method result. Exact storage effects are owned by the storage documents linked below.

## Minimal valid request

This example uses `account_preference_update` as a sample `sensitive_categories` string. It does not define the sensitive-category value set.

```yaml
method: harness.prepare_write
params:
  envelope:
    project_id: proj_pref_001
    task_id: task_pref_001
    actor_kind: agent
    surface_id: surface_write
    request_id: req_prepare_pref_001
    idempotency_key: idem_prepare_pref_001
    expected_state_version: 19
    dry_run: false
    locale: en-US
  task_id: task_pref_001
  change_unit_id: cu_pref_001
  intended_operation: "update profile preference save flow"
  intended_paths:
    - src/preferences/profile-save.ts
    - src/preferences/profile-save.test.ts
  product_file_write_intended: true
  sensitive_categories:
    - account_preference_update
  baseline_ref: baseline_pref_001
```

## Representative response

### Allowed branch

This branch applies after the separate sensitive-action approval is already present.

`uj_sensitive_pref_001` represents an existing resolved `judgment_kind=sensitive_approval` whose `SensitiveActionScope` matches the profile preference update. It is not ordinary write approval, final acceptance, residual-risk acceptance, or `Write Authorization`.

```yaml
base:
  response_kind: result
  effect_kind: core_committed
  dry_run: false
  state_version: 20
  events:
    - event_id: evt_pref_001
      event_kind: write_authorization_created
decision: allowed
state:
  project_id: proj_pref_001
  state_version: 20
  task_ref:
    record_kind: task
    record_id: task_pref_001
    project_id: proj_pref_001
    task_id: task_pref_001
    state_version: 20
write_authorization_ref:
  record_kind: write_authorization
  record_id: wa_pref_001
  project_id: proj_pref_001
  task_id: task_pref_001
  state_version: 20
write_authorization:
  authorization_id: wa_pref_001
  status: active
  basis_state_version: 19
  authorized_paths:
    - src/preferences/profile-save.ts
    - src/preferences/profile-save.test.ts
authorization_effect: created
active_user_judgment_refs:
  - record_kind: user_judgment
    record_id: uj_sensitive_pref_001
    project_id: proj_pref_001
    task_id: task_pref_001
    state_version: 19
write_decision_reasons: []
user_judgment_candidate: null
guarantee_display:
  level: cooperative
  basis: "Write Authorization is a Harness compatibility record, not OS permission."
  capability_refs: []
```

### Approval-required branch excerpt

This branch applies when the matching sensitive-action approval is missing.

```yaml
decision: approval_required
write_authorization_ref: null
write_authorization: null
authorization_effect: none
write_decision_reasons:
  - code: sensitive_account_preference
    message: "Profile preference updates require separate sensitive-action approval before Write Authorization."
```

## Owner links

- Request envelope, common result branches, and dry-run summaries: [API Schema Core](schema-core.md).
- `WriteAuthorizationSummary`, state summaries, and refs: [API State Schemas](schema-state.md).
- `SensitiveActionScope` and user-owned approval shapes: [API Judgment Schemas](schema-judgment.md).
- `Write Authorization`, write approval, sensitive-action approval, final-acceptance, and residual-risk boundaries: [Core Model](../core-model.md).
- Supported values and access classes: [API Value Sets](schema-value-sets.md).
- Public errors, `STATE_VERSION_CONFLICT`, branch routing, and blocked/dry-run behavior: [API error codes](error-codes.md), [API error precedence](error-precedence.md), and [API error routing](error-routing.md).
- Persistence effects and state clocks: [Storage Effects](../storage-effects.md) and [Storage Versioning](../storage-versioning.md).
