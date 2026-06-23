<a id="volicordprepare_write"></a>

# `volicord.prepare_write` reference

## What this document owns

This document owns baseline method behavior for `volicord.prepare_write`:

- method-specific required inputs, access requirements, state version behavior, result branches, and `dry_run` behavior
- `PrepareWriteResult` decision behavior
- method-specific handling for creating one consumable `Write Authorization`
- method-specific `WriteDecisionReason.code` production behavior
- prepare-write examples

## What this document does not own

This document does not own:

- common request envelope, response branch, dry-run, or rejected-response schema bodies
- nested state, judgment, value-set, or error schema definitions
- Core meaning of `Write Authorization`, ordinary write approval, sensitive-action approval, final acceptance, residual-risk acceptance, or user-owned judgment
- storage DDL, storage record layouts, exact storage effects, artifact lifecycle, or security guarantees
- public error code meaning, public error precedence, or shared response-branch routing

## Purpose

`volicord.prepare_write` checks one proposed product-file write against:

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

## Request schema

This method owns the top-level `params` request shape below. `envelope` is the shared [`ToolEnvelope`](schema-core.md#tool-envelope); this block does not redefine `ToolEnvelope` fields.

All fields shown in this method-owned request block are required members of `params` unless a field note explicitly marks a member optional; `T | null` means the member must be present and may contain JSON `null`.

```yaml
PrepareWriteRequest:
  envelope: ToolEnvelope
  task_id: string | null
  change_unit_id: string | null
  intended_operation: string
  intended_paths: string[]
  product_file_write_intended: boolean
  sensitive_categories: string[]
  baseline_ref: string
```

Field notes:
- `intended_paths` entries are `Product Repository` API product paths. Product Repository path normalization is owned by [Runtime Boundaries](../runtime-boundaries.md#product-repository-api-path-normalization); this method uses normalized repo-relative paths when forming and comparing the path-level `AuthorizedAttemptScope`.
- `sensitive_categories` entries are opaque sensitive-category classification strings unless this method or a profile owner publishes a narrower local list.

## Access requirements

Requires:

- server-derived `VerifiedSurfaceContext` with `access_class=write_authorization`
- compatible current scope
- compatible baseline
- required user-owned judgments
- any separate accepted sensitive-action approval (`sensitive_approval`)
- local surface capability needed for the intended product-file write check

A separate sensitive-action approval satisfies this method only when the judgment is current, resolved by `actor_kind=user`, selected an option with `resolution_outcome=accepted`, and its `JudgmentBasis` remains compatible with the current `scope_revision`, current Change Unit, intended operation, normalized `intended_paths`, sensitive categories, and `baseline_ref`. A judgment cannot satisfy sensitive-action approval if it has invalid basis state or is stale, superseded, expired, rejected, deferred, missing required resolution authority, or incompatible. Callers do not submit revision fields to make an approval compatible.

## State version behavior

| Result | State-version effect | `Write Authorization` effect |
|---|---|---|
| Committed `decision=allowed` | Increments `project_state.state_version` exactly once. | Creates one `status=active` `Write Authorization`. |
| Committed non-allow decision | Increments `project_state.state_version` exactly once. | Creates no consumable `Write Authorization`. |
| Pre-commit rejection or dry run | Increments nothing. | Creates nothing. |

## Write Authorization lifetime and ID allocation

Newly created `Write Authorization` records have a default lifetime of 15 minutes. `expires_at` is an enforced authority condition, not display-only metadata. The effective expiration is the earlier of stored `expires_at` and `created_at + 15 minutes`; this same effective rule limits historical rows with far-future expiration timestamps. Expiration is calculated using parsed UTC timestamps, not lexical string comparison.

A newly allowed committed authorization receives its durable `write_authorization_id` only when the allowed mutation is committed. Blocked, approval-required, decision-required, rejected, and `dry_run` paths do not allocate a durable `Write Authorization` ID.

## Method result fields

`PrepareWriteResult` is the method-specific result branch for committed write-preparation decisions. It carries `base: ToolResultBase` and these method-owned top-level fields:

| Field | Result-field meaning |
|---|---|
| `base` | Common result metadata. The `ToolResultBase` shape, including `events`, is owned by [API Schema Core](schema-core.md#common-response). Committed `PrepareWriteResult` branches use `base.response_kind=result` and `base.effect_kind=core_committed`. `base.events[].event_kind`, when present, is an opaque illustrative classification string. |
| `decision` | The method decision for this write-preparation attempt. Supported values are owned by [API Value Sets](schema-value-sets.md#method-local-values). |
| `state` | Current `StateSummary` when this result includes a state snapshot. Nested state fields, including `write_authority_summary`, are owned by [API State Schemas](schema-state.md). |
| `write_authorization_ref` | `StateRecordRef | null` for the consumable `Write Authorization` in an allowed decision result. A new allowed commit creates it; idempotent replay returns the stored original result without changing this field. It is `null` for non-allow decisions. |
| `write_authorization` | `WriteAuthorizationSummary | null` for the `Write Authorization` in an allowed decision result. A new allowed commit creates it; idempotent replay returns the stored original result without changing this field. It is `null` for non-allow decisions. |
| `authorization_effect` | Method result effect for the `Write Authorization` path. Supported values are owned by [API Value Sets](schema-value-sets.md#method-local-values). |
| `active_user_judgment_refs` | `StateRecordRef[]` for current accepted user-owned judgments applied to the write-preparation decision, including matching `sensitive_approval` judgments when present. |
| `write_decision_reasons` | `WriteDecisionReason[]` explaining non-allow decisions. The shape is owned by [API State Schemas](schema-state.md#current-position-display-shapes). |
| `user_judgment_candidate` | `UserJudgmentCandidate | null` when the method proposes a focused user-owned judgment instead of creating `Write Authorization`; otherwise `null`. The shape is owned by [API Judgment Schemas](schema-judgment.md#userjudgmentcandidate). |
| `guarantee_display` | `GuaranteeDisplay | null` for the method's compatibility display. The display shape is owned by [API State Schemas](schema-state.md#close-readiness-and-validation-shapes); security guarantee meaning is owned by [Security](../security.md). |

Nested `StateRecordRef`, `StateSummary`, `WriteAuthorizationSummary`, `WriteDecisionReason`, `UserJudgmentCandidate`, and `GuaranteeDisplay` field bodies stay with the schema owners linked above.

## Success result

Returns `PrepareWriteResult` with:

- `base.response_kind=result`
- `base.effect_kind=core_committed`

For `decision=allowed`:

- `write_authorization_ref` is non-null
- `write_authorization` is non-null
- `authorization_effect` is `created` for a new committed `decision=allowed` response
- idempotent replay returns the stored original committed `PrepareWriteResult` exactly; it does not recompute or reclassify `authorization_effect`, `base.state_version`, `base.events`, or any other response field, and it does not create another `Write Authorization` or repeat the storage effect
- the authorization is scoped to the path-level `AuthorizedAttemptScope` using normalized repo-relative `intended_paths`
- `active_user_judgment_refs` may cite current accepted user-owned judgments that satisfy write preconditions, including a separate `sensitive_approval`

## Blocked result

Committed blocked decisions are `PrepareWriteResult` values with one of these decision values:

- `decision=blocked`
- `decision=approval_required`
- `decision=decision_required`

Result data:

- `write_authorization_ref` is `null`.
- `write_authorization` is `null`.
- `authorization_effect` is `none`.
- `write_decision_reasons` must be non-empty.
- A valid committed `dry_run=false` non-allow result appends one task event containing the structured `write_decision_reasons`, creates a replay row when an idempotency key is present, and increments `project_state.state_version` exactly once.
- It creates no consumable `Write Authorization`, no separate public history method, and no new public response field.
- `volicord.status` is not required to expose historical non-allow decisions.
- Each entry is a `WriteDecisionReason`.
- `category` uses the controlled `WriteDecisionReason.category` value set.
- `code` uses this method's local v1 code list below.
- `message` is a free-form display string.
- `related_refs` uses `StateRecordRef[]`; use `[]` when no related refs apply.

Method-local `WriteDecisionReason.code` list:

The production meanings below apply only when this method reaches a committed non-allow `PrepareWriteResult`. Pre-commit failures still return `ToolRejectedResponse` according to the error owners.

| Code | Category | Local production meaning |
|---|---|---|
| `scope_not_current` | `scope` | Current scope is not compatible with the addressed Task, Change Unit, or intended write basis. |
| `path_out_of_scope` | `scope` | One or more `intended_paths` are outside current scope. |
| `sensitive_approval_missing` | `sensitive_approval` | A required separate `sensitive_approval` user judgment is absent. |
| `user_judgment_unresolved` | `user_judgment` | A user-owned judgment required for the write preconditions remains unresolved. |
| `baseline_mismatch` | `baseline` | `baseline_ref` does not match the write-compatibility basis. |
| `surface_access_class_mismatch` | `surface_capability` | The verified surface `access_class` is incompatible with the `Write Authorization` path. |
| `surface_capability_insufficient` | `surface_capability` | The verified surface lacks a required capability for the intended product-file write check. |
| `product_write_flag_mismatch` | `write_compatibility` | `product_file_write_intended` does not match the intended operation or paths. |
| `no_current_change_unit` | `scope` | No current Change Unit can be resolved for the write-preparation decision. |

Non-claims:

- These codes are method-local `WriteDecisionReason.code` values. They are not public `ErrorCode` values, not `CloseReadinessBlocker.code` values, and not global value-set entries.
- `STATE_VERSION_CONFLICT` is a rejected-response `ErrorCode`; it must not be represented as a method-local write decision reason.
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

Non-claim: `STATE_VERSION_CONFLICT` is always a rejected response error, never a method-local write decision reason.

Public error code meaning, precedence, and rejected-response routing are owned by the error documents linked below.

## Dry-run behavior

For `dry_run=true`, a valid preview:

- returns `ToolDryRunResponse`
- creates no `Write Authorization`
- persists no write-decision state

## Storage effect

On commit, the method may persist `Write Authorization` or write-decision state according to the method result. Exact storage effects are owned by the storage documents linked below.

The examples are intentionally compact and method-local. Representative responses show fields needed for the relevant `PrepareWriteResult` branch; nested schema bodies are illustrated only where they clarify the method result.

## Minimal valid request

This example uses `account_preference_update` as a sample `sensitive_categories` string. It does not define the sensitive-category value set.

```yaml
method: volicord.prepare_write
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

`uj_sensitive_pref_001` represents an existing current `judgment_kind=sensitive_approval` resolved by the user with `resolution_outcome=accepted` and a `SensitiveActionScope` that matches the profile preference update. It is not ordinary write approval, final acceptance, residual-risk acceptance, or `Write Authorization`.

In this example, the request carries `expected_state_version: 19`; the allowed commit advances the project to `state_version: 20` and creates an active `Write Authorization` with `basis_state_version: 20`.

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
  mode: work
  lifecycle:
    lifecycle_phase: ready
    close_reason: none
    result: none
    closed_at: null
  goal_summary: "Update profile preference save flow."
  scope_summary: "Profile preference save flow update."
  non_goals:
    - "Changing account deletion."
  acceptance_criteria:
    - "Profile preferences save successfully with related tests."
  autonomy_boundary: "Stay within the profile preference save flow."
  active_change_unit_ref:
    record_kind: change_unit
    record_id: cu_pref_001
    project_id: proj_pref_001
    task_id: task_pref_001
    state_version: 19
  baseline_ref: baseline_pref_001
  shaping_readiness: null
  pending_user_judgment_refs: []
  blocker_refs: []
  write_authority_summary:
    status: active
    write_authorization_ref:
      record_kind: write_authorization
      record_id: wa_pref_001
      project_id: proj_pref_001
      task_id: task_pref_001
      state_version: 20
    basis_state_version: 20
    intended_paths:
      - src/preferences/profile-save.ts
      - src/preferences/profile-save.test.ts
    guarantee_display:
      level: cooperative
      basis: "Write Authorization is a Volicord compatibility record, not OS permission."
      capability_refs: []
  evidence_summary: null
  close_state: null
  close_blockers: []
  guarantee_display:
    level: cooperative
    basis: "Write Authorization is a Volicord compatibility record, not OS permission."
    capability_refs: []
write_authorization_ref:
  record_kind: write_authorization
  record_id: wa_pref_001
  project_id: proj_pref_001
  task_id: task_pref_001
  state_version: 20
write_authorization:
  write_authorization_ref:
    record_kind: write_authorization
    record_id: wa_pref_001
    project_id: proj_pref_001
    task_id: task_pref_001
    state_version: 20
  status: active
  authorized_attempt_scope:
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
  basis_state_version: 20
  expires_at: "<future-expiration-timestamp>"
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
  basis: "Write Authorization is a Volicord compatibility record, not OS permission."
  capability_refs: []
```

### Approval-required branch

This branch applies when the matching sensitive-action approval is missing.

The `code: sensitive_approval_missing` value below is one of this method's local reason codes. It is not a public `ErrorCode` value.

```yaml
base:
  response_kind: result
  effect_kind: core_committed
  dry_run: false
  state_version: 20
  events: []
decision: approval_required
write_authorization_ref: null
write_authorization: null
authorization_effect: none
write_decision_reasons:
  - category: sensitive_approval
    code: sensitive_approval_missing
    message: "Profile preference updates require separate sensitive-action approval before Write Authorization."
    related_refs: []
active_user_judgment_refs: []
user_judgment_candidate: null
guarantee_display:
  level: cooperative
  basis: "Write Authorization is a Volicord compatibility record, not OS permission."
  capability_refs: []
```

## Owner links

- Request envelope, common result branches, and dry-run summaries: [API Schema Core](schema-core.md).
- `WriteAuthorizationSummary`, state summaries, and refs: [API State Schemas](schema-state.md).
- `SensitiveActionScope` and user-owned approval shapes: [API Judgment Schemas](schema-judgment.md).
- `Write Authorization`, write approval, sensitive-action approval, final-acceptance, and residual-risk boundaries: [Core Model](../core-model.md).
- Product Repository path normalization: [Runtime Boundaries](../runtime-boundaries.md#product-repository-api-path-normalization).
- Supported values and access classes: [API Value Sets](schema-value-sets.md).
- Public errors, `STATE_VERSION_CONFLICT`, branch routing, and blocked/dry-run behavior: [API error codes](error-codes.md), [API error precedence](error-precedence.md), and [API error routing](error-routing.md).
- Persistence effects and state clocks: [Storage Effects](../storage-effects.md) and [Storage Versioning](../storage-versioning.md).
