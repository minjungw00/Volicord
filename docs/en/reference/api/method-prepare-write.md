<a id="volicordprepare_write"></a>

# `volicord.prepare_write` reference

## What this document owns

This document owns baseline method behavior for `volicord.prepare_write`:

- method-specific required inputs, invocation requirements, state version behavior, result branches, and `dry_run` behavior
- `PrepareWriteResult` decision behavior
- method-specific handling for issuing one open write ticket authority record
- method-specific `WriteDecisionReason.code` production behavior
- prepare-write examples

## What this document does not own

This document does not own:

- common request envelope, response branch, dry-run, or rejected-response schema bodies
- nested state, judgment, value-set, or error schema definitions
- Core meaning of write tickets, ordinary write approval, sensitive-action approval, final acceptance, residual-risk acceptance, or user-owned judgment
- storage DDL, storage record layouts, exact storage effects, artifact lifecycle, or security guarantees
- public error code meaning, public error precedence, or shared response-branch routing

## Purpose

`volicord.prepare_write` checks one proposed product-file write against:

- current Task
- currently applied Change Unit
- current scope
- current Change Unit effect contract, when one is recorded
- baseline
- required separate sensitive-action approval
- verified invocation context

When the check is allowed, the method issues one open write ticket. The ticket is a Volicord authority record for authorized write intent within the current Task and Change Unit. It is not filesystem enforcement, OS permission, shell permission, or proof that a write occurred. When the check is not allowed, the method denies or defers the ticket path.

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
- `intended_paths` entries are `Product Repository` API product paths. Product Repository path normalization is owned by [Runtime Boundaries](../runtime-boundaries.md#product-repository-api-path-normalization); this method uses normalized repo-relative paths when forming and comparing the path-level `WriteTicketScope` and compatibility storage scope.
- `sensitive_categories` entries are opaque sensitive-category classification strings unless this method or a profile owner publishes a narrower local list.

## Access requirements

Requires:

- verified invocation context with `operation_category=agent_workflow`
- compatible current scope
- compatible current Change Unit effect contract for product-file writes, when one is recorded
- compatible baseline
- required user-owned judgments
- any separate accepted sensitive-action approval (`sensitive_approval`)
- compatible `actor_source` for the agent workflow invocation

A separate sensitive-action approval satisfies this method only when the judgment is current, resolved with `resolved_by_actor_source=local_user` and compatible User Channel provenance, selected an option with `resolution_outcome=accepted`, and its `JudgmentBasis` remains compatible with the current `scope_revision`, current Change Unit, intended operation, normalized `intended_paths`, sensitive categories, and `baseline_ref`. A judgment cannot satisfy sensitive-action approval if it has invalid basis state or is stale, superseded, expired, rejected, deferred, missing required resolution authority, or incompatible. Callers do not submit revision fields to make an approval compatible.

## State version behavior

| Result | State-version effect | Write-ticket effect |
|---|---|---|
| Committed `decision=allowed` | Increments `project_state.state_version` exactly once. | Issues one open write ticket. |
| Committed non-allow decision | Increments `project_state.state_version` exactly once. | Issues no write ticket. |
| Pre-commit rejection or dry run | Increments nothing. | Creates nothing. |

## Write ticket lifetime and ID allocation

Newly issued write tickets have a default lifetime of 15 minutes. `expires_at` is an enforced Volicord compatibility condition, not display-only metadata and not an OS-level write deadline. The effective expiration is the earlier of stored `expires_at` and `created_at + 15 minutes`; this same effective rule limits historical rows with far-future expiration timestamps. Expiration is calculated using parsed UTC timestamps, not lexical string comparison.

A newly allowed committed call receives its durable `write_ticket_id` only when the allowed mutation is committed. Blocked, approval-required, decision-required, rejected, and `dry_run` paths do not allocate a durable write ticket ID.

## Method result fields

`PrepareWriteResult` is the method-specific result branch for committed write-preparation decisions. It carries `base: ToolResultBase` and these method-owned top-level fields:

| Field | Result-field meaning |
|---|---|
| `base` | Common result metadata. The `ToolResultBase` shape, including `disclosure` and `events`, is owned by [API Schema Core](schema-core.md#common-response). Committed `PrepareWriteResult` branches use `base.response_kind=result`, `base.effect_kind=core_committed`, and `base.disclosure.guarantee_class=authority_record`. `base.events[].event_kind`, when present, is an opaque illustrative classification string. |
| `decision` | The method decision for this write-preparation attempt. Supported values are owned by [API Value Sets](schema-value-sets.md#method-local-values). |
| `state` | Current `StateSummary` when this result includes a state snapshot. Nested state fields, including `write_ticket_summary`, are owned by [API State Schemas](schema-state.md). |
| `write_ticket_id` | `WriteTicketId | null` for the issued write ticket in an allowed decision result. A new allowed commit allocates it; idempotent replay returns the stored original result without changing this field. It is `null` for non-allow committed decisions. |
| `write_ticket_ref` | `StateRecordRef | null` with `record_kind=write_ticket` for the issued write ticket. It is `null` for non-allow committed decisions. |
| `write_ticket` | `WriteTicket | null` for the issued write-ticket authority record. It is `null` for non-allow committed decisions. |
| `write_ticket_effect` | Method result effect for the write-ticket path. `issued` means this committed result created the open ticket. `none` means no ticket was issued. Supported values are owned by [API Value Sets](schema-value-sets.md#method-local-values). |
| `allowed_path_patterns` | Normalized Product Repository path patterns captured as allowed by the ticket decision. In an allowed result, this is the ticket's allowed path pattern list. |
| `denied_path_patterns` | Normalized Product Repository path patterns captured as denied by the ticket decision, or `[]` when no path-level denial applies. |
| `control_surface` | `ControlSurfaceSummary | null` describing the current Volicord control surface used for disclosure. `os_enforced=false` means the ticket is not OS-level enforcement. |
| `active_user_judgment_refs` | `StateRecordRef[]` for current accepted user-owned judgments applied to the write-preparation decision, including matching `sensitive_approval` judgments when present. |
| `write_decision_reasons` | `WriteDecisionReason[]` explaining non-allow decisions. The shape is owned by [API State Schemas](schema-state.md#current-position-display-shapes). |
| `user_judgment_candidate` | `UserJudgmentCandidate | null` when the method proposes a focused user-owned judgment instead of issuing a write ticket; otherwise `null`. The shape is owned by [API Judgment Schemas](schema-judgment.md#userjudgmentcandidate). |
| `guarantee_display` | `GuaranteeDisplay | null` for the method's compatibility display. The display shape is owned by [API State Schemas](schema-state.md#close-readiness-and-validation-shapes); security guarantee meaning is owned by [Security](../security.md). |

Nested `StateRecordRef`, `StateSummary`, `WriteTicket`, `ControlSurfaceSummary`, `WriteTicketSummary`, `WriteDecisionReason`, `UserJudgmentCandidate`, and `GuaranteeDisplay` field bodies stay with the schema owners linked above.

## Success result

Returns `PrepareWriteResult` with:

- `base.response_kind=result`
- `base.effect_kind=core_committed`

For `decision=allowed`:

- `write_ticket_id`, `write_ticket_ref`, and `write_ticket` are non-null
- `write_ticket_ref.record_kind` is `write_ticket`
- `write_ticket.state` is `open`
- `write_ticket_effect` is `issued` for a new committed `decision=allowed` response
- `write_ticket.path_patterns.allowed` and top-level `allowed_path_patterns` contain the normalized repo-relative `intended_paths` allowed for this ticket
- `write_ticket.path_patterns.denied` and top-level `denied_path_patterns` are `[]` for an allowed result
- `write_ticket.observed_paths` is `[]` until a detective-profile hook, watcher, or later owner-defined observation path connects observations to the ticket
- `control_surface` and `write_ticket.control_surface` disclose the current Volicord control surface, including `os_enforced=false` in the baseline non-enforcement model
- idempotent replay returns the stored original committed `PrepareWriteResult` exactly; it does not recompute or reclassify `write_ticket_effect`, `base.state_version`, `base.events`, or any other response field, and it does not create another write ticket or repeat the storage effect
- the write ticket is scoped to `WriteTicketScope` using normalized repo-relative `intended_paths`
- `active_user_judgment_refs` may cite current accepted user-owned judgments that satisfy write preconditions, including a separate `sensitive_approval`

## Blocked result

Committed blocked decisions are `PrepareWriteResult` values with one of these decision values:

- `decision=blocked`
- `decision=approval_required`
- `decision=decision_required`

Result data:

- `write_ticket_id`, `write_ticket_ref`, and `write_ticket` are `null`.
- `write_ticket_effect` is `none`.
- `write_decision_reasons` must be non-empty.
- A valid committed `dry_run=false` non-allow result appends one `authority_events` row containing the structured `write_decision_reasons`, creates a replay row when an idempotency key is present, and increments `project_state.state_version` exactly once.
- It issues no write ticket, creates no separate public history method, and does not create a product-file write authority record.
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
| `effect_contract_forbids_product_file_write` | `effect_contract` | The current Change Unit effect contract explicitly forbids product-file writes. |
| `effect_contract_effect_not_allowed` | `effect_contract` | The current Change Unit effect contract has a non-empty allowed-effect list that does not include `product_file_write`. |
| `effect_contract_path_not_allowed` | `effect_contract` | One or more `intended_paths` are outside the current Change Unit effect contract `allowed_paths`. |
| `product_write_flag_mismatch` | `write_compatibility` | `product_file_write_intended` does not match the intended operation or paths. |
| `no_current_change_unit` | `scope` | No current Change Unit can be resolved for the write-preparation decision. |

Non-claims:

- These codes are method-local `WriteDecisionReason.code` values. They are not public `ErrorCode` values, not `CloseReadinessBlocker.code` values, and not global value-set entries.
- `STATE_VERSION_CONFLICT` is a rejected-response `ErrorCode`; it must not be represented as a method-local write decision reason.
- `write_decision_reasons` are not `CloseReadinessBlocker` values.
- `write_decision_reasons` do not evaluate close readiness.
- Effect contract decision reasons do not replace sensitive-action approval, user-owned judgment, evidence, final acceptance, close readiness, residual-risk acceptance, or the separate write ticket that this method creates only on `decision=allowed`.
- No write ticket is issued.
- The result disclosure is not OS sandboxing, network isolation, malware defense, full write prevention, correctness proof, test sufficiency proof, human review replacement, or actor attribution proof.

## Rejected result

Returns `ToolRejectedResponse` for failures before decision evaluation or commit, including:

- stale `expected_state_version`
- idempotency request-hash conflict
- request validation failure
- missing current Task or currently applied Change Unit
- actor-source or operation-category mismatch
- Core unavailability
- stale baseline
- invalid requested guarantee
- unsupported invocation context

Non-claim: `STATE_VERSION_CONFLICT` is always a rejected response error, never a method-local write decision reason.

Public error code meaning, precedence, and rejected-response routing are owned by the error documents linked below.

## Dry-run behavior

For `dry_run=true`, a valid preview:

- returns `ToolDryRunResponse`
- issues no committed write ticket
- may describe a planned `write_ticket` effect such as `would_issue` in the dry-run summary when the preview would otherwise be allowed
- persists no write-decision state

## Storage effect

On commit, the method may persist a write ticket or write-decision state according to the method result. Exact storage effects, including the physical table backing the ticket record, are owned by the storage documents linked below.

The examples are intentionally compact and method-local. Representative responses show fields needed for the relevant `PrepareWriteResult` branch; nested schema bodies are illustrated only where they clarify the method result.

## Minimal valid request

This example uses `account_preference_update` as a sample `sensitive_categories` string. It does not define the sensitive-category value set.

```yaml
method: volicord.prepare_write
params:
  envelope:
    project_id: proj_pref_001
    task_id: task_pref_001
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

`uj_sensitive_pref_001` represents an existing current `judgment_kind=sensitive_approval` resolved by the user with `resolution_outcome=accepted` and a `SensitiveActionScope` that matches the profile preference update. It is not ordinary write approval, final acceptance, residual-risk acceptance, or a write ticket.

In this example, the request carries `expected_state_version: 19`; the allowed commit advances the project to `state_version: 20` and issues an open write ticket with `basis_state_version: 20`.

```yaml
base:
  response_kind: result
  effect_kind: core_committed
  dry_run: false
  state_version: 20
  events:
    - event_id: evt_pref_001
      event_kind: write_ticket_issued
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
  write_ticket_summary:
    status: active
    write_ticket_ref:
      record_kind: write_ticket
      record_id: wt_pref_001
      project_id: proj_pref_001
      task_id: task_pref_001
      state_version: 20
    basis_state_version: 20
    intended_paths:
      - src/preferences/profile-save.ts
      - src/preferences/profile-save.test.ts
    guarantee_display:
      level: cooperative
      basis: "Write ticket is a Volicord authority record, not OS permission."
      capability_refs: []
  evidence_summary: null
  close_state: null
  close_blockers: []
  guarantee_display:
    level: cooperative
    basis: "Write ticket is a Volicord authority record, not OS permission."
    capability_refs: []
write_ticket_id: wt_pref_001
write_ticket_ref:
  record_kind: write_ticket
  record_id: wt_pref_001
  project_id: proj_pref_001
  task_id: task_pref_001
  state_version: 20
write_ticket:
  write_ticket_id: wt_pref_001
  write_ticket_ref:
    record_kind: write_ticket
    record_id: wt_pref_001
    project_id: proj_pref_001
    task_id: task_pref_001
    state_version: 20
  state: open
  scope:
    task_id: task_pref_001
    change_unit_id: cu_pref_001
    intended_operation: "update profile preference save flow"
    product_file_write_intended: true
    sensitive_categories:
      - account_preference_update
    baseline_ref: baseline_pref_001
  path_patterns:
    allowed:
      - src/preferences/profile-save.ts
      - src/preferences/profile-save.test.ts
    denied: []
  observed_paths: []
  basis_state_version: 20
  expires_at: "<future-expiration-timestamp>"
  control_surface:
    selected_profile: record
    host_hooks_active: false
    session_watcher_active: false
    cooperative_pre_tool_warning_available: false
    cooperative_pre_tool_denial_available: false
    unrecorded_changes_detectable: false
    actor_identity_provable: false
    os_enforced: false
  guarantee_display:
    level: cooperative
    basis: "Write ticket is a Volicord authority record, not OS permission."
    capability_refs: []
write_ticket_effect: issued
allowed_path_patterns:
  - src/preferences/profile-save.ts
  - src/preferences/profile-save.test.ts
denied_path_patterns: []
control_surface:
  selected_profile: record
  host_hooks_active: false
  session_watcher_active: false
  cooperative_pre_tool_warning_available: false
  cooperative_pre_tool_denial_available: false
  unrecorded_changes_detectable: false
  actor_identity_provable: false
  os_enforced: false
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
  basis: "Write ticket is a Volicord authority record, not OS permission."
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
write_ticket_id: null
write_ticket_ref: null
write_ticket: null
write_ticket_effect: none
allowed_path_patterns:
  - src/preferences/profile-save.ts
  - src/preferences/profile-save.test.ts
denied_path_patterns: []
control_surface:
  selected_profile: record
  host_hooks_active: false
  session_watcher_active: false
  cooperative_pre_tool_warning_available: false
  cooperative_pre_tool_denial_available: false
  unrecorded_changes_detectable: false
  actor_identity_provable: false
  os_enforced: false
write_decision_reasons:
  - category: sensitive_approval
    code: sensitive_approval_missing
    message: "Profile preference updates require separate sensitive-action approval before write ticket issuance."
    related_refs: []
active_user_judgment_refs: []
user_judgment_candidate: null
guarantee_display:
  level: cooperative
  basis: "Write ticket is a Volicord authority record, not OS permission."
  capability_refs: []
```

## Owner links

- Request envelope, common result branches, and dry-run summaries: [API Schema Core](schema-core.md).
- `WriteTicket`, compatibility `WriteTicketSummary`, state summaries, and refs: [API State Schemas](schema-state.md).
- `SensitiveActionScope` and user-owned approval shapes: [API Judgment Schemas](schema-judgment.md).
- Write ticket, write approval, sensitive-action approval, final-acceptance, and residual-risk boundaries: [Core Model](../core-model.md).
- Product Repository path normalization: [Runtime Boundaries](../runtime-boundaries.md#product-repository-api-path-normalization).
- Supported values and operation categories: [API Value Sets](schema-value-sets.md#operation-category-values).
- Public errors, `STATE_VERSION_CONFLICT`, branch routing, and blocked/dry-run behavior: [API error codes](error-codes.md), [API error precedence](error-precedence.md), and [API error routing](error-routing.md).
- Persistence effects and state clocks: [Storage Effects](../storage-effects.md) and [Storage Versioning](../storage-versioning.md).
