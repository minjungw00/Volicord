# API blocker routing

This document owns the routing boundary between close-readiness blockers and API response branches. It is a boundary router, not the method behavior owner or the schema owner.

Use it only after the response branch boundary is known. It routes owner questions at the error/blocker boundary; it does not define `harness.close_task` method behavior, `CloseReadinessBlocker` shape, blocker category values, Core close-readiness authority, storage effects, public `ErrorCode` meanings, API error precedence, response-branch selection, or display wording.

## Owner boundaries

| Concern | Owner |
|---|---|
| Close-readiness blocker/API response routing boundary | This document, limited to boundary routing |
| `harness.close_task` request behavior, evaluation order, result branches, and committed blocked outcomes | [`harness.close_task`](method-close-task.md) |
| `CloseReadinessBlocker` fields and nested shape | [API State Schemas](schema-state.md) |
| Exact `CloseReadinessBlocker.category` values and other enum-like API vocabulary | [API Value Sets](schema-value-sets.md#state-and-blocker-values) |
| Core close-readiness authority, final acceptance, residual-risk acceptance, and non-substitution rules | [Core Model close readiness](../core-model.md#close_task) |
| API response branch routing for rejected responses, blocked results, and `dry_run` previews | [API error routing](error-routing.md) |
| Public `ErrorCode` meanings | [API error codes](error-codes.md) |
| API error precedence and conflict selection | [API error precedence](error-precedence.md) |
| Display labels and rendered wording | [Template Bodies](../template-bodies.md) |

## Common error/blocker boundary

- A public `ErrorCode` identifies an API error condition defined by [API error codes](error-codes.md). It is not automatically a `CloseReadinessBlocker.category` value or any other close-readiness blocker category.
- A rejected response error code stays on the API error side even when the same underlying condition can affect close readiness. It is not used as a blocker category merely because of that relationship.
- Close-readiness blockers use the `CloseReadinessBlocker` shape from [API State Schemas](schema-state.md) and the blocker category value set from [API Value Sets](schema-value-sets.md#state-and-blocker-values).
- Blocker routing applies after API response branch routing and does not replace [API error precedence](error-precedence.md).
- [API error codes](error-codes.md) defines public error code meanings; this document defines the boundary between those errors and close-readiness blocker routing.

## API error and blocker boundary

| Situation | Route | Boundary |
|---|---|---|
| Failure before a valid close-readiness evaluation | `ToolRejectedResponse.errors[]` with `ToolError.code: ErrorCode` | The request did not reach a valid close-readiness result. It does not return `CloseReadinessBlocker[]`. |
| Valid close-readiness evaluation finds a close-readiness blocker | `CloseReadinessBlocker[]` in the method result or read-only state result | The data explains why close is blocked. Schema shape and exact category values stay with the schema and value-set owners. |
| Valid `dry_run` preview predicts blocker-like outcomes | `DryRunSummary.would_blockers: PlannedBlocker[]` | Preview blockers are not stored `CloseReadinessBlocker` objects and do not create close-readiness state. |
| Response branch selection is the question | [API error routing](error-routing.md) | This page applies after the branch boundary is identified; it does not choose every response branch. |

## Category routing boundary

`CloseReadinessBlocker.category` identifies the owner family responsible for close-readiness blocker data after a method or state result has returned that data under its owner contract. Exact category values belong to [API Value Sets](schema-value-sets.md#state-and-blocker-values); this page only routes category-bearing blocker data to the appropriate owner concern. It is not a full blocker taxonomy, schema field table, or close-task evaluation order.

| Owner concern | Routing use | Boundary |
|---|---|---|
| Core state, terminal transition, baseline, recovery, and write compatibility | A category-bearing blocker can point readers to Core or method-owned state requirements. | Core meaning stays with [Core Model](../core-model.md); method behavior stays with [`harness.close_task`](method-close-task.md). |
| Scope, user-owned judgment, sensitive approval, and surface capability | A category-bearing blocker can show that close depends on a user, scope, approval, or surface-capability owner. | The blocker does not record the user decision, approval, scope change, or capability declaration. |
| Evidence and artifact basis | A category-bearing blocker can show that close depends on evidence sufficiency or persistent artifact availability. | Evidence and artifact semantics stay with their owners; the route does not prove sufficiency or availability. |
| Final acceptance and residual risk | A category-bearing blocker can show that close depends on final acceptance, residual-risk visibility, or residual-risk acceptance. | The blocker does not create acceptance or risk acceptance. |

## Public code boundary

Use this table only after applying the common boundary above. Public-code families can be related to close-readiness blockers only through owner-defined blocker data; public `ErrorCode` values are not copied into `CloseReadinessBlocker.code` unless the schema or method owner explicitly allows that exact use.

| Public-code relationship | Blocker-side route | Boundary |
|---|---|---|
| Evidence, artifact, acceptance, user-judgment, approval, scope, autonomy-boundary, baseline, or capability families | Route through the owner-defined `CloseReadinessBlocker.category` and `CloseReadinessBlocker.code`. | Public code meanings stay with [API error codes](error-codes.md); blocker shape stays with [API State Schemas](schema-state.md), category values stay with [API Value Sets](schema-value-sets.md#state-and-blocker-values), and method-specific blocker production stays with [`harness.close_task`](method-close-task.md). |
| Readable-view freshness families | May be named as related diagnostics when the owner allows it. | A freshness diagnostic by itself is not a close-readiness blocker. |
| State-version or idempotency conflict families | No close-readiness blocker representation. | These failures are rejected before close-readiness evaluation and stay with [API error precedence](error-precedence.md). |

## `harness.close_task` method route

Method-specific close behavior belongs to [`harness.close_task`](method-close-task.md). Route request validation, intent handling, terminal mutation, state-version behavior, and committed blocked outcomes to that method owner.

This document only defines the boundary between the blocker data returned by that method and the neighboring API error, schema, value-set, Core, storage, and display owners.

## Authority boundary

Blocker routing classifies close-readiness blocker data. It does not create or replace:

- final acceptance or residual-risk acceptance
- user-owned judgment, sensitive-action approval, or `Write Authorization`
- evidence sufficiency or artifact availability
- close completion or terminal `Task` state
- blocker persistence or state-version increments
- rendered display wording
