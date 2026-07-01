<a id="volicordreconcile_changes"></a>

# `volicord.reconcile_changes` reference

## What this document owns

This document owns baseline method behavior for `volicord.reconcile_changes`:

- method-specific required inputs, access requirements, state version behavior, result branches, and `dry_run` behavior
- listing unresolved unrecorded Product Repository change findings for the current project and Task
- resolving findings that Core can verify deterministically
- creating pending user-owned judgments for findings that require user acceptance
- rejecting agent-only dismissal of unresolved Product Repository change findings

## What this document does not own

This document does not own:

- common request envelope, response branch, dry-run, or rejected-response schema bodies
- `UserJudgment`, `StateRecordRef`, `CloseReadinessBlocker`, `GuardHealthSummary`, or `NextActionSummary` field definitions
- storage table layout, SQLite constraints, public error code meaning, public error precedence, or shared response-branch routing
- proof of correctness, test sufficiency, review completion, final acceptance, residual-risk acceptance, or security guarantees

## Purpose

`volicord.reconcile_changes` is the public recovery path for guarded and session-watch-created unrecorded Product Repository change findings.

The method lists unresolved findings for the selected Task, resolves findings that Core can verify from stored Core, guard, expected-write, or session-watch records, and creates ordinary pending `UserJudgment` rows when a remaining finding requires a user-owned acceptance decision. It must not silently dismiss a bypass finding. It must not let an Agent Connection mark an unrecorded Product Repository change accepted without a compatible resolved User Channel judgment.

Resolving an unrecorded-change finding removes that finding from the unresolved guard-health count and from the `unresolved_unrecorded_changes` close blocker calculation. It does not prove that the changed product files are correct, reviewed, tested, accepted for close, or acceptable residual risk.

## Required inputs

- A valid `ToolEnvelope`; committed non-dry-run requests that mutate state require non-null `idempotency_key` and current `expected_state_version`.
- `task_id` for the Task whose unresolved findings are being reconciled.
- Optional `resolution_requests` entries when the caller wants to point Core at a resolved user judgment for `accepted_by_user`.

Core also scans the current project and Task for unresolved findings. Callers do not provide observed paths, detection facts, actor provenance, deterministic proof, or close-blocker state.

## Request schema

This method owns the top-level `params` request shape below. `envelope` is the shared [`ToolEnvelope`](schema-core.md#tool-envelope); this block does not redefine `ToolEnvelope` fields.

```yaml
ReconcileChangesRequest:
  envelope: ToolEnvelope
  task_id: string
  resolution_requests: UnrecordedChangeResolutionRequest[]

UnrecordedChangeResolutionRequest:
  unrecorded_change_id: string
  basis: string
  user_judgment_id: string | null
```

Request field notes:

- `resolution_requests` defaults to `[]`.
- `basis=accepted_by_user` requires `user_judgment_id` for an existing resolved, current, same-Task `product_decision` judgment linked to the unrecorded-change ref, recorded through a compatible User Channel with `actor_source=local_user`, `machine_action=accept`, and `resolution_outcome=accepted`.
- Caller-supplied `reverted`, `covered_by_write_readiness`, `recorded_as_expected_write`, `not_product_change`, `superseded_by_new_observation`, or `invalid_observation` requests reject as agent-supplied system resolution bases. Core may still apply those bases itself when it can verify them deterministically.

Nested owner links:

- `UnrecordedChangeFinding` and `UnrecordedChangeResolutionSummary` shapes are owned by [API State Schemas](schema-state.md#unrecorded-change-reconciliation-shapes).
- Resolution basis values are owned by [API Value Sets](schema-value-sets.md#unrecorded-change-resolution-basis-values).
- User-owned judgment shapes are owned by [API Judgment Schemas](schema-judgment.md).

## Access requirements

The method requires:

- verified invocation context with `operation_category=agent_workflow` for Agent Connection workflow calls or `operation_category=local_recovery` for local-user recovery calls
- a compatible same-project Task selected by `task_id`
- a workflow-capable Agent Connection when called through MCP

Local administrative recovery commands may call the same Core method with `actor_source=local_user` and `operation_category=local_recovery`. That CLI path is not an MCP Agent Connection path and does not let the CLI impersonate a user judgment. User-owned acceptance still requires a compatible resolved User Channel judgment before `accepted_by_user` can resolve the finding.

## State version behavior

A committed non-dry-run result that has planned storage effects:

- increments `project_state.state_version` exactly once
- resolves one or more `unrecorded_changes` rows and stores resolution basis, capture basis, actor source, timestamp, and optional linked user-judgment ref
- and/or creates pending `user_judgments` rows for remaining findings that require user acceptance
- appends one task event and creates a replay row when an idempotency key is present
- updates close-readiness projections so resolved findings no longer count as unresolved

A valid call with no storage mutations returns a read-only result and does not create a replay row, event, or state-version increment.

At a session-bound method boundary, the runtime may run a bounded session-watch check before reconciliation planning. That diagnostic check can create a new unresolved unrecorded-change finding when an unexpected Product Repository snapshot change is not covered by expected-write correlation.

Dry run previews planned resolutions or pending judgments without creating refs, events, replay rows, user judgments, or resolution rows. Rejected attempts create no effects.

## Success result

Returns `ReconcileChangesResult` with:

- `base.response_kind=result`
- `base.effect_kind=core_committed` when findings are resolved or judgments are created
- `base.effect_kind=read_only` when no storage mutation is needed
- `task_ref`
- `unresolved_changes`
- `resolved_changes`
- `pending_user_judgment_refs`
- `rejected_resolution_requests`
- current `state`
- projected `close_blockers`
- projected `guard_health`
- `next_actions`

## Method result fields

| Field | Result-field meaning |
|---|---|
| `base` | Common result metadata. The `ToolResultBase` shape is owned by [API Schema Core](schema-core.md#common-response). |
| `task_ref` | `StateRecordRef` for the reconciled Task. |
| `unresolved_changes` | Remaining unresolved findings after applying deterministic and accepted-user resolutions selected by this call. |
| `resolved_changes` | Findings that this call resolved, including basis, actor source, capture basis, timestamp, and optional linked user judgment. |
| `pending_user_judgment_refs` | Pending `UserJudgment` refs relevant to the Task after this call, including judgments created for unresolved findings. |
| `rejected_resolution_requests` | Caller-supplied resolution requests that Core refused. These are structured rejections inside a successful method result, not public `ToolRejectedResponse` errors. |
| `state` | Current `StateSummary` after the reconciliation projection or commit. |
| `close_blockers` | Projected close blockers after planned reconciliation effects. |
| `guard_health` | Projected guard-health summary when guard health is available for the verified connection. |
| `next_actions` | Next safe steps, such as recording the created user-owned judgment or rerunning reconciliation. |

## Resolution behavior

Core-owned deterministic bases:

- `invalid_observation`: stored observation data is invalid for interpretation as Product Repository paths.
- `not_product_change`: stored observation data contains no Product Repository path to reconcile.
- `recorded_as_expected_write`: a recorded Run for the same Task already covers the observed Product Repository paths, or expected-write correlation for the same Task covers watcher-observed Product Repository paths.
- `covered_by_write_readiness`: a consumed compatible `Write Check` for the same Task covers the observed Product Repository paths.
- `reverted`: a watcher-created finding is linked to a session-watch observation and the current Product Repository snapshot matches the stored watch baseline again.

User-owned basis:

- `accepted_by_user`: a compatible resolved `product_decision` judgment linked to the finding records that the local user accepts the observed change as intentional for the Task.

Reserved or future owner-defined bases such as `superseded_by_new_observation` and any other listed basis may be stored only when their owner-defined verification is implemented. This method must not implement a filesystem-reverting or filesystem-probing basis unless that verification is safe and owner-defined.

For findings that still require acceptance, Core creates pending `UserJudgment` rows rather than accepting them. Existing User Channel paths can answer those judgments, including MCP elicitation flows where the initialized client supports them, guarded prompt-capture command handling when prompt-capture availability is `configured`, `observed`, or `active`, loopback local web consent when the adapter can safely expose it, and local `volicord user` commands for CLI recovery. After the user-owned judgment is resolved, `volicord.reconcile_changes` can resolve the linked finding with `accepted_by_user`.

## Rejected result

Returns `ToolRejectedResponse` for pre-commit failures such as:

- invalid request shape
- missing or incompatible Task identity
- actor-source or operation-category mismatch
- unsupported invocation context
- stale `expected_state_version`
- idempotency request-hash conflict
- unreadable owner state

Agent-only dismissal attempts for individual findings do not normally reject the whole method call. They appear in `rejected_resolution_requests` and leave the finding unresolved.

## Dry-run behavior

For `dry_run=true`, a valid preview returns `ToolDryRunResponse` with planned effects. It does not resolve findings, create pending judgments, append events, create replay rows, or increment `project_state.state_version`.

## Storage effect

On commit, the method may persist unrecorded-change resolution rows and pending user judgments. Exact storage effects are owned by [Storage Effects](../storage-effects.md#volicordreconcile_changes).

## Related owners

- Exact public method routing: [API Methods](methods.md).
- Value sets: [API Value Sets](schema-value-sets.md#unrecorded-change-resolution-basis-values).
- State-shaped response fields: [API State Schemas](schema-state.md#unrecorded-change-reconciliation-shapes).
- User judgment authority: [`volicord.record_user_judgment`](method-record-user-judgment.md).
- Close blocker production: [`volicord.close_task`](method-close-task.md).
- Storage effects: [Storage Effects](../storage-effects.md#volicordreconcile_changes).
