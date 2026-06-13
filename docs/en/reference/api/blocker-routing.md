# API blocker routing

This document owns the routing boundary between close-readiness blockers and API response branches.

It explains when a close-relevant finding is represented as `CloseReadinessBlocker[]`, when the API stays on a rejected or preview branch, and which owner defines the neighboring contract. It does not define `harness.close_task` method behavior, `CloseReadinessBlocker` shape, blocker category values, Core close-readiness authority, storage effects, public `ErrorCode` meanings, response-branch selection, or display wording.

## Owner boundaries

| Concern | Owner |
|---|---|
| Blocker/API response routing boundary | This document |
| `harness.close_task` request behavior, evaluation order, result branches, and committed blocked outcomes | [`harness.close_task`](method-close-task.md) |
| `CloseReadinessBlocker` fields and nested shape | [API State Schemas](schema-state.md) |
| Exact `CloseReadinessBlocker.category` values and other enum-like API vocabulary | [API Value Sets](schema-value-sets.md#state-and-blocker-values) |
| Core close-readiness authority, final acceptance, residual-risk acceptance, and non-substitution rules | [Core Model close readiness](../core-model.md#close_task) |
| Rejected-response, blocked-result, and `dry_run` response branch selection | [API error routing](error-routing.md) |
| Public `ErrorCode` meanings and precedence | [API error codes](error-codes.md) and [API error precedence](error-precedence.md) |
| Display labels and rendered wording | [Template Bodies](../template-bodies.md) |

## API error and blocker boundary

| Situation | Route | Boundary |
|---|---|---|
| Failure before a valid close-readiness evaluation | `ToolRejectedResponse.errors[]` with `ToolError.code: ErrorCode` | The request did not reach a valid close-readiness result. It does not return `CloseReadinessBlocker[]`. |
| Valid close-readiness evaluation finds a close blocker | `CloseReadinessBlocker[]` in the method result or read-only state result | The data explains why close is blocked. Schema shape and exact category values stay with the schema and value-set owners. |
| Valid `dry_run` preview predicts blocker-like outcomes | `DryRunSummary.would_blockers: PlannedBlocker[]` | Preview blockers are not stored `CloseReadinessBlocker` objects and do not create close-readiness state. |
| Response branch selection is the question | [API error routing](error-routing.md) | This page applies after the branch boundary is identified; it does not choose every response branch. |

## Category routing boundary

`CloseReadinessBlocker.category` identifies the owner family responsible for a close-readiness blocker. Exact category values belong to [API Value Sets](schema-value-sets.md#state-and-blocker-values); this page only routes category-bearing blocker data to the appropriate owner concern.

| Owner concern | Routing use | Boundary |
|---|---|---|
| Core state, terminal transition, baseline, recovery, and write compatibility | A category-bearing blocker can point readers to Core or method-owned state requirements. | Core meaning stays with [Core Model](../core-model.md); method behavior stays with [`harness.close_task`](method-close-task.md). |
| Scope, user-owned judgment, sensitive approval, and surface capability | A category-bearing blocker can show that close depends on a user, scope, approval, or surface-capability owner. | The blocker does not record the user decision, approval, scope change, or capability declaration. |
| Evidence and artifact basis | A category-bearing blocker can show that close depends on evidence sufficiency or persistent artifact availability. | Evidence and artifact semantics stay with their owners; the route does not prove sufficiency or availability. |
| Final acceptance and residual risk | A category-bearing blocker can show that close depends on final acceptance, residual-risk visibility, or residual-risk acceptance. | The blocker does not create acceptance or risk acceptance. |

## Public code boundary

Public `ErrorCode` values are public API identifiers, not blocker codes. A public error-code family may be cited as related to a close-readiness blocker only when the condition is found during a valid close-readiness evaluation and the applicable owner defines a supported blocker category or blocker code for that condition.

The public value is not copied into `CloseReadinessBlocker.code` unless the schema or method owner explicitly allows that exact use.

| Public-code relationship | Blocker-side route | Boundary |
|---|---|---|
| Evidence, artifact, acceptance, user-judgment, approval, scope, autonomy-boundary, baseline, or capability families | Route through the owner-defined `CloseReadinessBlocker.category` and `CloseReadinessBlocker.code`. | Public code meanings stay with [API error codes](error-codes.md); exact blocker values stay with [API State Schemas](schema-state.md) and [API Value Sets](schema-value-sets.md). |
| Readable-view freshness families | May be named as related diagnostics when the owner allows it. | A freshness diagnostic by itself is not a close-readiness blocker. |
| State-version or idempotency conflict families | No close-readiness blocker representation. | These failures are rejected before close-readiness evaluation and stay with [API error precedence](error-precedence.md). |

<a id="harnessclose_task-close-blockers"></a>
## `harness.close_task` method route

Method-specific close behavior belongs to [`harness.close_task`](method-close-task.md). Route preflight rejection, `intent=check`, `intent=complete`, terminal mutation, invalid terminal transition, state-version behavior, and committed blocked outcomes to that method owner.

This document only defines the boundary between the blocker data returned by that method and the neighboring API error, schema, value-set, Core, storage, and display owners.

## Authority boundary

Blocker routing classifies close-readiness blocker data. It does not create or replace:

- final acceptance or residual-risk acceptance
- user-owned judgment, sensitive-action approval, or `Write Authorization`
- evidence sufficiency or artifact availability
- close completion or terminal `Task` state
- blocker persistence or state-version increments
- rendered display wording
