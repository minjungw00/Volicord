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

- common request envelope, response branch, `dry_run`, or rejected-response schema bodies
- nested state, judgment, value-set, or error schema definitions
- Core meaning of write tickets, ordinary write approval, sensitive-action approval, final acceptance, residual-risk acceptance, or user-owned judgment
- storage DDL, storage record layouts, exact storage effects, artifact lifecycle, or security guarantees
- public error code meaning, public error precedence, or shared response-branch routing

## Purpose

`volicord.prepare_write` checks one proposed product-file write, or one exact
non-product action for an effective `sensitive` Task, against:

- current Task
- currently applied Change Unit
- current scope
- current Change Unit effect contract, when one is recorded
- baseline
- current Task work phase and the Change Unit's recorded workspace context
- current normalized project-policy write authority
- required separate sensitive-action approval
- verified invocation context

When the check is allowed, the method first searches for a compatible active
ticket. It reuses that ticket when its Task, Change Unit, `scope_revision`,
baseline, workspace, approval basis, and non-null normalized project
write-authority binding are current, its allowed paths cover all newly intended
paths, its sensitive basis is equal or stronger, and it remains unconsumed.
Core constructs the canonical Write Ticket approval requirement and typed
current sensitive-approval set, then assesses the Store-valid persisted basis
once. Reuse accepts a current basis or a basis that is not required and rejects
a changed basis. Sensitive reuse additionally requires the exact normalized
`intended_operation`; a reworded operation cannot borrow an earlier approval
or ticket, while additional unrelated current approvals do not replace or
invalidate its persisted basis. Otherwise it issues one open ticket. A ticket is a Volicord
authority record for the bounded product-write or sensitive-action intent within
the current Task and Change Unit. A non-product sensitive ticket has
`product_file_write_intended=false`; it binds the named operation, an empty
product-path set, baseline, Change Unit, scope revision, and user-owned approval
basis, and is consumed only by the matching sensitive Run. It is not filesystem
enforcement, OS permission, shell permission, or proof that an effect occurred.
After the current Change Unit precondition has been satisfied, a policy check
that is not allowed denies or defers the ticket path without invalidating an
unrelated compatible ticket. A missing current Change Unit is not a policy
decision and does not enter this path.

Every request is reevaluated under the current policy. A policy change can make
the proposed write `sensitive` or otherwise require a new user-owned approval;
an approval or ticket issued under an incompatible earlier write authority does
not cross that boundary. Final acceptance is a post-work judgment, not the
required pre-write sensitive-action approval, and cannot retroactively
authorize a write.

`Task.mode=advisor` is read-only with respect to Product Repository file effects. `volicord.prepare_write` rejects that Task mode before decision evaluation, does not recommend this method as the generic next action for an advisor Task, and never issues an advisor write ticket. A `work` Task must also have `work_phase=implementation`; shaping remains read-only. This does not prevent a compatible shaping `record_run` call from committing Core Run or evidence state.

Security non-claims belong to [Security](../security.md).

## Required inputs

- A valid `ToolEnvelope`; committed `dry_run=false` requests require non-null `idempotency_key` and current `expected_state_version`.
- `task_id` and `change_unit_id`, or `null` only when owner resolution can unambiguously use the current `Task` and currently applied Change Unit.
- `intended_operation`, `intended_paths`, `product_file_write_intended`, `sensitive_categories`, and `baseline_ref`.

## Request schema

This method owns the top-level `params` request fields in the generated table
below. `envelope` is the shared [`ToolEnvelope`](schema-core.md#tool-envelope);
the table does not redefine `ToolEnvelope` fields. Requiredness and nullability
come directly from the semantic request descriptor.

<!-- BEGIN GENERATED: contract-structures api.method.prepare_write.request[params] -->
<!-- Generated by `cargo run -p xtask -- docs-sync`; do not edit this region. -->

### `PrepareWriteRequest` fields

| Field | Required | Nullable | Type |
|---|---|---|---|
| `baseline_ref` | yes | no | `string` |
| `change_unit_id` | yes | yes | `string` |
| `envelope` | yes | no | `ToolEnvelope` |
| `intended_operation` | yes | no | `string` |
| `intended_paths` | yes | no | `string[]` |
| `product_file_write_intended` | yes | no | `boolean` |
| `sensitive_categories` | yes | no | `string[]` |
| `task_id` | yes | yes | `string` |
<!-- END GENERATED: contract-structures api.method.prepare_write.request[params] -->



Field notes:
- `intended_paths` entries are `Product Repository` API product paths. Product Repository path normalization is owned by [Runtime Boundaries](../runtime-boundaries.md#product-repository-api-path-normalization); this method uses normalized repo-relative paths when forming and comparing the path-level `WriteTicketScope` and compatibility storage scope.
- `sensitive_categories` entries are opaque sensitive-category classification strings unless this method or a profile owner publishes a narrower local list.

## Evaluation order and current Change Unit precondition

Write preparation uses this order:

1. Validate the request shape.
2. Resolve the addressed or current Task.
3. Resolve the current Change Unit for that Task.
4. If no current Change Unit exists, return `ToolRejectedResponse` with
   `errors[].category=rejected`, `errors[].code=NO_ACTIVE_CHANGE_UNIT`, and the method-specific
   `errors[].details.reason=current_change_unit_required`.
5. Build the canonical resolved context with the concrete `TaskId` and
   `ChangeUnitId`.
6. Evaluate current policy and write compatibility.
7. Plan ticket issuance or reuse only when the decision is allowed.

The current Change Unit check is a structural precondition, not a policy
decision. It applies identically to `dry_run=false` and `dry_run=true`. This
rejection creates, reuses, or invalidates no `WriteTicket`; creates no
`WriteDecision` or authority event; creates no replay or invocation row; and
does not increment `project_state.state_version`. A bounded diagnostic may
record the method, reason `current_change_unit_required`, the project and resolved
Task identifiers, and the observation time. That diagnostic is not an
append-only rejection stream or Core authority state.

While the resolved Task still has no current Change Unit, repeated calls reject
deterministically with `NO_ACTIVE_CHANGE_UNIT` and reason
`current_change_unit_required`, and repeat none of those effects. Any
`WriteTicketAttemptScope` or `WriteTicketValidityBasis` formed
after this precondition carries the actual resolved `ChangeUnitId`; neither
structure uses a null, optional, or placeholder Change Unit.

## Access requirements

Requires:

- verified invocation context with `operation_category=agent_workflow`
- a current Change Unit resolved for the addressed Task; absence follows the
  structural rejection above before policy evaluation
- a current Task whose mode is `direct` or `work` and whose `work_phase` is `implementation`; `advisor` and shaping are incompatible with write preparation
- a current effective control level. `observe` is incompatible with product
  writes. An effective `sensitive` Task also requires an exact ticket-backed
  action basis when its Run has no product-file write. Before ticket selection
  Core applies any pending policy reevaluation without lowering the active
  Task; this includes same-level write-authority changes as well as stricter
  control or acceptance requirements.
- for `light`, every intended path must be covered by an allowed normalized
  repository-relative prefix and by no denied prefix. Prefixes match the exact
  path or descendants; wildcard/glob syntax, absolute paths, empty entries,
  `..`, and ambiguous forms are invalid. An empty allowed set permits no Light
  product-file write.
- compatible current scope
- compatible current Change Unit effect contract for product-file writes, when one is recorded
- a request baseline that exactly matches both the current Task baseline and
  the current Change Unit write basis
- an exact match between the verified current Git workspace context and the
  Change Unit context captured at its baseline, when the Product Repository is
  Git-backed
- required user-owned judgments
- any separate `accepted` sensitive-action approval (`sensitive_approval`)
- compatible `actor_source` for the agent workflow invocation

A separate sensitive-action approval satisfies this method only when the user action is current, resolved with `resolved_by_actor_source=local_user` and compatible User Channel provenance, selected an option with `resolution_outcome=accepted`, and its `UserActionBasis` remains compatible with the current `scope_revision`, current Change Unit, intended operation, normalized `intended_paths`, sensitive categories, and `baseline_ref`. A user action cannot satisfy sensitive-action approval if it has invalid basis state or is stale, superseded, expired, rejected, deferred, missing required resolution authority, or incompatible. Callers do not submit revision fields to make an approval compatible.

Ticket selection also requires a non-null
`WriteTicketValidityBasis.write_authority_fingerprint` equal to the current
normalized write-authority fingerprint. An active ticket with a missing
binding and one with a different binding both fail closed and
require reissuance under the current policy. During a committed `dry_run=false`
allowed or non-allow evaluation, Core durably invalidates each selected stale
active ticket with `invalidation_reason=explicit_revoke`; dry-run and
`ToolRejectedResponse` branches do not perform that invalidation mutation.

## State version behavior

| Result | State-version effect | Write-ticket effect |
|---|---|---|
| Committed `decision=allowed`, new ticket | Increments `project_state.state_version` exactly once. | Issues one open ticket; `write_ticket_effect=issued`. |
| Committed `decision=allowed`, compatible ticket found | Increments `project_state.state_version` exactly once. | Reuses the existing ticket; `write_ticket_effect=reused`; no ticket row is inserted. |
| Committed non-allow decision | Increments `project_state.state_version` exactly once. | Issues no write ticket; may invalidate selected stale active tickets with `explicit_revoke`. |
| Pre-commit rejection or `dry_run` | Increments nothing. | Creates or invalidates no ticket. |

The committed state-version increment is authority-event ordering. It does not
invalidate the issued or reused ticket and is not part of ticket validity.

## Write ticket validity, idle timeout, and ID allocation

There is no fixed lifetime and the default idle timeout is `null`. Project
policy may select an idle timeout; when it does, Core stores the derived
`idle_expires_at` and invalidates the ticket with reason `idle_timeout` at the
semantic UTC boundary. A sensitive approval can retain its own expiration;
approval expiry invalidates a dependent ticket as `approval_basis_changed`, not
as ordinary elapsed ticket time or persisted corruption. Persisted
approval-reference owner disagreement or duplicate full resolution identity is
corruption and does not enter semantic currentness evaluation.
The semantic assessment reports a changed basis when approval is newly
required, no current resolution exists, approval scope changed, or a persisted
basis resolution is no longer current. State summary, ticket reuse, Record Run
admission, and close readiness consume that assessment instead of independently
comparing approval references or resolution IDs.
`basis_state_version` records issuance order
for audit and references only; it is never a freshness condition.

A newly allowed committed call receives a durable `write_ticket_id` only when
the issuance mutation commits. A reuse result returns the existing ID and ref.
Blocked, approval-required, decision-required, rejected, and `dry_run` paths do
not allocate a durable ID.

Core semantic planning represents a prospective issuance as an identity-free
`PlannedWriteTicketDraft`; it does not inspect `dry_run`, allocate a durable
ID, attach response state versions, or construct the public result. A
`dry_run` request stops at the method boundary and projects only the preview.
For a committed issuance, the method supplies the durable ID and
approval-reference projection state version; the typed non-empty approval basis
constructs the state-versioned references while materializing one validated
`PlannedWriteTicket`. That materialized value is the single source for response
projection and the typed Store insertion input. Reuse instead projects the
Store-validated `StoredWriteTicket` that already exists.

## Method result fields

`PrepareWriteResult` is the method-specific result branch for committed write-preparation decisions. It carries `base: PrepareWriteResultBase`, whose only result effect is `core_committed`, and these method-owned top-level fields:

<!-- BEGIN GENERATED: contract-structures api.method.prepare_write.response[response_variants] api.method.prepare_write.response[result_body] api.method.prepare_write.response[result_metadata] api.method.prepare_write.response[rejection] api.method.prepare_write.response[dry_run] -->
<!-- Generated by `cargo run -p xtask -- docs-sync`; do not edit this region. -->

### `PrepareWriteResult` success fields

| Field | Required | Nullable | Type |
|---|---|---|---|
| `active_user_action_refs` | yes | no | `StateRecordRef[]` |
| `allowed_path_patterns` | yes | no | `string[]` |
| `base` | yes | no | `PrepareWriteResultBase` |
| `decision` | yes | no | `PrepareWriteDecision` |
| `denied_path_patterns` | yes | no | `string[]` |
| `guarantee_display` | no | yes | `GuaranteeDisplay` |
| `state` | no | yes | `StateSummary` |
| `user_action_draft` | no | yes | `UserActionDraft` |
| `write_decision_reasons` | yes | no | `WriteDecisionReason[]` |
| `write_ticket` | no | yes | `WriteTicket` |
| `write_ticket_effect` | yes | no | `WriteTicketEffect` |
| `write_ticket_id` | no | yes | `string` |
| `write_ticket_ref` | no | yes | `StateRecordRef` |

### `Result Metadata: core_committed` fields

Contract: `dry_run` is `false`; `events` contains at least one event (`minItems: 1`).

| Field | Required | Nullable | Type |
|---|---|---|---|
| `disclosure` | yes | no | `GuaranteeDisclosure` |
| `dry_run` | yes | no | `boolean enum(false)` |
| `effect_kind` | yes | no | `string enum("core_committed")` |
| `events` | yes | no | `NonEmptyEventRefs` |
| `response_kind` | yes | no | `string enum("result")` |
| `state_version` | yes | no | `integer` |

### `dry_run` request policy

- `volicord.prepare_write`: `dry_run=true` selects the `ToolDryRunResponse` preview branch, whose `base.dry_run` is `true`. `dry_run=false` or an omitted `dry_run` does not select a preview branch.


### Shared response structures

The response descriptor defines success, rejection, and preview as an exact `anyOf` branch union. The rejection branch uses the generated [`ToolRejectedResponse`](schema-core.md#common-response) structure. When method behavior selects a preview branch, it uses the generated [`ToolDryRunResponse`](schema-core.md#common-response) structure. Shared rejection and preview fields remain distinct from the success fields above.
<!-- END GENERATED: contract-structures api.method.prepare_write.response[response_variants] api.method.prepare_write.response[result_body] api.method.prepare_write.response[result_metadata] api.method.prepare_write.response[rejection] api.method.prepare_write.response[dry_run] -->

Nested `StateRecordRef`, `StateSummary`, `WriteTicket`, `WriteTicketStateSummary`, `WriteDecisionReason`, `UserActionDraft`, and `GuaranteeDisplay` field bodies stay with the schema owners linked above.

## Success result

Returns `PrepareWriteResult` with:

- `base.response_kind=result`
- `base.effect_kind=core_committed`

For `decision=allowed`:

- `write_ticket_id`, `write_ticket_ref`, and `write_ticket` are non-null
- `write_ticket_ref.record_kind` is `write_ticket`
- `write_ticket.state` is `open`
- `write_ticket_effect` is `issued` for new issuance or `reused` when an
  existing compatible active ticket is selected
- `write_ticket.path_patterns.allowed` and top-level `allowed_path_patterns` contain the normalized repo-relative `intended_paths` allowed for this ticket
- `write_ticket.path_patterns.denied` and top-level `denied_path_patterns` are `[]` for an allowed result
- `write_ticket.observed_paths` is `[]` when no observed path is part of the ticket
- top-level and ticket-local `guarantee_display` values disclose the cooperative authority-record boundary and do not claim OS-level enforcement
- idempotent replay returns the stored original committed `PrepareWriteResult` exactly; it does not recompute or reclassify `write_ticket_effect`, `base.state_version`, `base.events`, or any other response field, and it does not create another write ticket or repeat the storage effect
- replay eligibility requires the current verified invocation to retain the exact optional Git workspace context captured with the original replay row; a changed, newly absent, or newly present workspace context returns `INVOCATION_CONTEXT_MISMATCH` without exposing the stored allowed response or its write ticket
- the write ticket is scoped to `WriteTicketScope` using normalized repo-relative `intended_paths`
- `write_ticket.validity_basis` reports its `Task`, Change Unit,
  `scope_revision`, baseline, optional workspace digest, current normalized
  project write-authority fingerprint, and approval-basis refs;
  `basis_state_version` is audit-only
- Core reevaluates the requested paths against the current authoritative policy
  before issuance or reuse. Reuse requires an exact non-null
  `write_authority_fingerprint` match. A missing or mismatched binding is
  invalidated with `explicit_revoke` on any committed non-dry-run decision that
  selects it as stale, whether the current result is allowed or non-allow.
  Rejected and dry-run paths do not mutate it. The stale ticket cannot preserve
  an earlier Light classification across a current Sensitive decision or
  approval requirement.
- `active_user_action_refs` may cite current `accepted` user-owned judgments that satisfy write preconditions, including a separate `sensitive_approval`

## Blocked result

Committed blocked decisions are `PrepareWriteResult` values with one of these `decision` values:

- `decision=blocked`
- `decision=approval_required`
- `decision=decision_required`

Result data:

- `write_ticket_id`, `write_ticket_ref`, and `write_ticket` are `null`.
- `write_ticket_effect` is `none`.
- `write_decision_reasons` must be non-empty.
- A valid committed `dry_run=false` non-allow result appends one `authority_events` row containing the structured `write_decision_reasons`, creates a replay row when an idempotency key is present, and increments `project_state.state_version` exactly once.
- That non-allow commit and its unrelated state-version increment do not
  invalidate any otherwise compatible active ticket.
- It issues no write ticket, creates no separate public history method, and does not create a product-file write authority record.
- `volicord.status` is not required to expose historical non-allow decisions.
- Each entry is a `WriteDecisionReason`.
- `category` uses the controlled `WriteDecisionReason.category` value set.
- `code` uses this method's closed current code list below.
- `message` is a free-form display string.
- `related_refs` uses `StateRecordRef[]`; use `[]` when no related refs apply.

Method-local `WriteDecisionReason.code` list:

The production meanings below apply only when this method reaches a committed non-allow `PrepareWriteResult`. Pre-commit failures still return `ToolRejectedResponse` according to the error owners.

| Code | Category | Local production meaning |
|---|---|---|
| `scope_not_current` | `scope` | Current scope is not compatible with the addressed `Task`, Change Unit, or intended write basis. |
| `path_out_of_scope` | `scope` | One or more `intended_paths` are outside current scope. |
| `sensitive_approval_missing` | `sensitive_approval` | A required separate `sensitive_approval` user judgment is absent. |
| `user_action_unresolved` | `user_action` | A user-owned action required for the write preconditions remains unresolved. |
| `baseline_mismatch` | `baseline` | `baseline_ref` does not match the write-compatibility basis. |
| `workspace_context_mismatch` | `workspace` | The current Git common directory, worktree identity, branch or detached-HEAD state, HEAD SHA, or workspace fingerprint differs from the Change Unit baseline context. No ticket is issued until `update_scope` replaces the Change Unit with an explicit current baseline. |
| `effect_contract_forbids_product_file_write` | `effect_contract` | The current Change Unit effect contract explicitly forbids product-file writes. |
| `effect_contract_effect_not_allowed` | `effect_contract` | The current Change Unit effect contract has a non-empty allowed-effect list that does not include `product_file_write`. |
| `effect_contract_path_not_allowed` | `effect_contract` | One or more `intended_paths` are outside the current Change Unit effect contract `allowed_paths`. |
| `product_write_flag_mismatch` | `write_compatibility` | `product_file_write_intended` does not match the intended operation or paths. |

Non-claims:

- These codes are method-local `WriteDecisionReason.code` values. They are not public `ErrorCode` values, not `CloseReadinessBlocker.code` values, and not global value-set entries.
- `STATE_VERSION_CONFLICT` is a rejected-response `ErrorCode`; it must not be represented as a method-local write decision reason.
- `write_decision_reasons` are not `CloseReadinessBlocker` values.
- `write_decision_reasons` do not evaluate close readiness.
- Effect contract decision reasons do not replace sensitive-action approval, user-owned judgment, evidence, final acceptance, close readiness, residual-risk acceptance, or the issued-or-reused ticket selected only on `decision=allowed`.
- No write ticket is issued.
- The result disclosure is not OS sandboxing, network isolation, malware defense, full write prevention, correctness proof, test sufficiency proof, human review replacement, or actor attribution proof.

## Rejected result

Returns `ToolRejectedResponse` for failures before `decision` evaluation or commit, including:

- `Task.mode=advisor`
- stale `expected_state_version`
- idempotency request-hash conflict
- request validation failure
- missing current Task
- no current Change Unit after Task resolution; this uses public code
  `NO_ACTIVE_CHANGE_UNIT` with its canonical category and method-specific
  details reason `current_change_unit_required`, and occurs before policy
  evaluation
- actor-source or operation-category mismatch
- Core unavailability
- stale baseline
- invalid requested guarantee
- unsupported invocation context

Non-claim: `STATE_VERSION_CONFLICT` is always a rejected response error, never a method-local write decision reason.

Public error code meaning, precedence, and rejected-response routing are owned by the error documents linked below.

Advisor-mode rejection creates no write decision, write ticket, event, replay row, or state-version increment.

The `NO_ACTIVE_CHANGE_UNIT` branch with reason `current_change_unit_required`
creates no `WriteTicket`,
`WriteDecision`, authority event, replay row, invocation row, or state-version
effect. It has the same result for otherwise identical normal and `dry_run`
requests, and repeated calls do not turn the rejection into a committed or
replayed result.

## `dry_run` behavior

For `dry_run=true`, a valid preview:

- returns `ToolDryRunResponse`
- issues no committed write ticket
- may describe a planned `write_ticket` effect such as `would_issue` in the `dry_run` summary when the preview would otherwise be allowed
- may describe `would_reuse` as a planned effect when a compatible active
  ticket exists; this is preview text, not a committed `WriteTicketEffect`
- keeps a prospective issuance as an identity-free semantic draft rather than
  materializing a ticket record or Store insertion
- persists no write-decision state

A request without a current Change Unit is not a valid preview. It returns the
same `ToolRejectedResponse` with `NO_ACTIVE_CHANGE_UNIT` and reason
`current_change_unit_required` described above and produces no planned ticket
effect.

## Storage effect

On commit, the method may persist a write ticket or write-decision state according to the method result. Exact storage effects, including the physical table backing the ticket record, are owned by the storage documents linked below.

The examples are intentionally compact and method-local. Representative responses show fields needed for the relevant `PrepareWriteResult` branch; nested schema bodies are illustrated only where they clarify the method result.

## Minimal valid request

This example uses `account_preference_update` as a sample `sensitive_categories` string. It does not define the sensitive-category value set.

```yaml contract=api.method.prepare_write.request shape=complete_request
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

In this example, the request carries `expected_state_version: 19`; the allowed
commit advances the project to `state_version: 20` and issues an open ticket.
Its `basis_state_version: 20` records issuance order and is not its validity
basis.

The `write_authority_fingerprint` value below is an illustrative normalized
write-authority digest, distinct from the illustrative whole-policy
`policy_fingerprint`.

```schema
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
    produced_at_state_version: 20
  mode: work
  requested_control_level: auto
  effective_control_level: sensitive
  control_level_reason: "Current policy classifies this account preference update as sensitive."
  project_policy:
    policy_schema: volicord.workflow_policy
    policy_version: 4
    policy_fingerprint: sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb
    source: project_database
  work_phase: implementation
  acceptance_policy: required
  acceptance_policy_reason: "Sensitive work requires final acceptance."
  lineage: null
  lifecycle:
    lifecycle_phase: ready
    close_reason: none
    result: none
    closed_at: null
  scope_revision: 1
  goal_summary: "Update profile preference save flow."
  scope_summary: "Profile preference save flow update."
  non_goals:
    - "Changing account deletion."
  acceptance_criteria:
    - acceptance_criterion_id: criterion_profile_save_001
      statement: "Profile preferences save successfully with related tests."
      evidence_requirement: not_required
  autonomy_boundary: "Stay within the profile preference save flow."
  active_change_unit_ref:
    record_kind: change_unit
    record_id: cu_pref_001
    project_id: proj_pref_001
    task_id: task_pref_001
    produced_at_state_version: 20
  effect_contract: null
  baseline_ref: baseline_pref_001
  workspace_context: null
  shaping_readiness: null
  pending_user_action_summaries: []
  blocker_refs: []
  write_ticket_summary:
    status: active
    write_ticket_ref:
      record_kind: write_ticket
      record_id: wt_pref_001
      project_id: proj_pref_001
      task_id: task_pref_001
      produced_at_state_version: 20
    basis_state_version: 20
    validity_basis:
      task_id: task_pref_001
      change_unit_id: cu_pref_001
      scope_revision: 1
      baseline_ref: baseline_pref_001
      workspace_context_sha256: null
      write_authority_fingerprint: sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
      approval_basis_refs:
        - record_kind: user_action_resolution
          record_id: uj_sensitive_pref_001
          project_id: proj_pref_001
          task_id: task_pref_001
          produced_at_state_version: 19
    invalidation_reason: null
    idle_expires_at: null
    intended_paths:
      - src/preferences/profile-save.ts
      - src/preferences/profile-save.test.ts
    consumed_by_run_ref: null
    observation_refs: []
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
  produced_at_state_version: 20
write_ticket:
  write_ticket_id: wt_pref_001
  write_ticket_ref:
    record_kind: write_ticket
    record_id: wt_pref_001
    project_id: proj_pref_001
    task_id: task_pref_001
    produced_at_state_version: 20
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
  validity_basis:
    task_id: task_pref_001
    change_unit_id: cu_pref_001
    scope_revision: 1
    baseline_ref: baseline_pref_001
    workspace_context_sha256: null
    write_authority_fingerprint: sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
    approval_basis_refs:
      - record_kind: user_action_resolution
        record_id: uj_sensitive_pref_001
        project_id: proj_pref_001
        task_id: task_pref_001
        produced_at_state_version: 19
  invalidation_reason: null
  idle_expires_at: null
  guarantee_display:
    level: cooperative
    basis: "Write ticket is a Volicord authority record, not OS permission."
    capability_refs: []
write_ticket_effect: issued
allowed_path_patterns:
  - src/preferences/profile-save.ts
  - src/preferences/profile-save.test.ts
denied_path_patterns: []
active_user_action_refs:
  - record_kind: user_action_resolution
    record_id: uj_sensitive_pref_001
    project_id: proj_pref_001
    task_id: task_pref_001
    produced_at_state_version: 20
write_decision_reasons: []
user_action_draft: null
guarantee_display:
  level: cooperative
  basis: "Write ticket is a Volicord authority record, not OS permission."
  capability_refs: []
```

### Approval-required branch

This branch applies when the matching sensitive-action approval is missing.

The `code: sensitive_approval_missing` value below is one of this method's local reason codes. It is not a public `ErrorCode` value.

```schema
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
write_decision_reasons:
  - category: sensitive_approval
    code: sensitive_approval_missing
    message: "Profile preference updates require separate sensitive-action approval before write ticket issuance."
    related_refs: []
active_user_action_refs: []
user_action_draft: null
guarantee_display:
  level: cooperative
  basis: "Write ticket is a Volicord authority record, not OS permission."
  capability_refs: []
```

## Owner links

- Request envelope, common result branches, and `dry_run` summaries: [API Schema Core](schema-core.md).
- `WriteTicket`, `WriteTicketStateSummary`, state summaries, and refs: [API State Schemas](schema-state.md).
- `SensitiveActionScope` and user-owned approval shapes: [API Judgment Schemas](schema-judgment.md).
- Write ticket, write approval, sensitive-action approval, final-acceptance, and residual-risk boundaries: [Core Model](../core-model.md).
- Product Repository path normalization: [Runtime Boundaries](../runtime-boundaries.md#product-repository-api-path-normalization).
- Supported values and operation categories: [API Value Sets](schema-value-sets.md#operation-category-values).
- Public errors, `STATE_VERSION_CONFLICT`, branch routing, and blocked/`dry_run` behavior: [API error codes](error-codes.md), [API error precedence](error-precedence.md), and [API error routing](error-routing.md).
- Persistence effects and state clocks: [Storage Effects](../storage-effects.md) and [Storage Versioning](../storage-versioning.md).
