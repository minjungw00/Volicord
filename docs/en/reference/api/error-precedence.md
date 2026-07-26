# API error precedence

This document owns primary public-error selection when more than one public error candidate exists. It also owns the canonical error/blocker decision flow and public stale-state and idempotency conflict behavior for `STATE_VERSION_CONFLICT`.

Use it to choose the primary public code for an error-bearing branch. Use adjacent owners for code meanings, branch routing, schemas, storage, and display wording.

## Owner boundaries

Owned here:

- The canonical decision flow that distinguishes transport or adapter failures, Core rejected responses, `dry_run` previews, method-owned blocked results, and committed blocker-shaped results.
- The primary `errors[0]` selection order for error-bearing branches.
- The result-side and blocker-code path boundary for `STATE_VERSION_CONFLICT`.
- Public stale `expected_state_version`, idempotency request-hash conflict, and state-bound write-ticket invalidation behavior.

Adjacent owners:

- MCP JSON-RPC errors, `CallToolResult.isError`, and `tools/call` wrapping; see [MCP transport](../mcp-transport.md).
- Agent Connection project selection, mode, and current connection context; see [Agent Connection](../agent-connection.md).
- Public code meanings outside precedence selection; see [API error codes](error-codes.md).
- API response branch routing; see [API error routing](error-routing.md).
- Close-readiness blocker/API response boundary; see [API blocker routing](blocker-routing.md).
- Method-specific behavior; see [`volicord.close_task`](method-close-task.md) and other method owners.
- Machine-readable conflict detail fields; see [API error details](error-details.md#state-conflict-detail-fields).
- Committed result storage effects; see [Storage Effects](../storage-effects.md).
- Storage replay rows and state clocks; see [Storage Versioning](../storage-versioning.md).
- Display wording only; see [Template Bodies](../template-bodies.md).
- Product-wide category selection boundaries; see the
  [Failure Model](../failure-model.md).

<a id="canonical-error-blocker-decision-flow"></a>

## Canonical error/blocker decision flow

Exact identifiers used in this section: `Task`.

Use this flow before applying [primary error-code precedence](#primary-error-code-precedence). It chooses the response family first. The precedence table below applies only after a call has become a Volicord rejected response with `ToolRejectedResponse.errors[]`.

| Order | Boundary | Execution point | Public shape | Routing rule |
|---:|---|---|---|---|
| 1 | Transport, JSON-RPC, or adapter message shape fails before a public Volicord request exists. | Transport or adapter layer, before Core execution. | JSON-RPC `error`, process exit diagnostic, or transport-owned failure shape. No Volicord `ErrorCode` is selected. | Route to [MCP transport](../mcp-transport.md) or the owning transport or adapter. Do not translate this into `ToolRejectedResponse.errors[]` after the request failed before Core. |
| 2 | A known MCP tool call is rejected by the MCP adapter before Core execution. This includes Agent Connection mode rejection, project-selection failure, project allowlist failure, caller-owned invocation fields, or known-tool argument decoding/schema rejection. | MCP adapter layer, before Core execution. | MCP `CallToolResult` with `isError: true` and text content, unless the condition is a JSON-RPC protocol or parameter error owned by [MCP transport](../mcp-transport.md). | This is not a Volicord method result and has no `ToolRejectedResponse.errors[]`. Fix the MCP call, connection mode, or project selector before retrying. |
| 3 | A typed Volicord request reaches Core and request validation, exact-contract selection, persisted owner-data validation, Core preflight, invocation compatibility, task lookup, freshness, or idempotency fails before the method-owned result branch. | Inside Core, before committed method execution. | `ToolRejectedResponse.errors[]` with required `FailureCategory` and public `ErrorCode` values. | Use [API error routing](error-routing.md) for the rejected branch and this document's precedence table for `errors[0]`. No committed operation proceeds. Known corruption keeps its distinct category and code. |
| 4 | A valid `dry_run` request reaches a preview branch after preflight. | Inside Core, after validation and preflight, before commit. | `ToolDryRunResponse` with `DryRunSummary.would_errors[]` or `DryRunSummary.would_blockers[]` when the method defines them. | Route preview branch behavior to [API error routing](error-routing.md#dry-run-behavior). Preview blockers are `PlannedBlocker`, not stored `CloseReadinessBlocker` objects. |
| 5 | A valid method evaluation reaches `NotAllowed` and returns a method-defined non-allow result without selecting a committed effect. | Inside Core, after method-owned policy evaluation. | Method-specific `MethodResult` fields, such as `CloseTaskResult(close_state=blocked)` or method-owned decision fields. No `errors[]` branch is present. | When the method owner defines this result, route it through [API error routing](error-routing.md#blocked-result-behavior), close-readiness blocker boundaries through [API blocker routing](blocker-routing.md), and method details through the method owner. This is not structural `Rejected`. |
| 6 | A valid method evaluation reaches `NotAllowed` and selects a committed blocker-shaped or non-allow result. | Inside Core, on a method-owned committed branch. | Method-specific `MethodResult` with committed effects only when allowed by the method and storage-effect owners, such as committed `PrepareWriteResult` non-allow decisions. | This can be durable state rather than a failed transport call. Route exact storage effects to [Storage Effects](../storage-effects.md) and exact result fields to the method owner. Public error precedence does not apply because there is no `ToolRejectedResponse.errors[]` branch. The category alone never authorizes the commit. |

For MCP `tools/call`, successful MCP transport carries a Volicord response as
a non-error tool result, including a Volicord domain-level
`ToolRejectedResponse`. A caller reads the authoritative object from
`toolResult` for `2024-10-07`, parses the first `content` text item for
`2024-11-05` and `2025-03-26`, or reads `structuredContent` for `2025-06-18`
and `2025-11-25`. The latter four revisions use `isError: false` for this
successful transport branch. See [MCP transport](../mcp-transport.md) for the
revision carriers.

<a id="primary-error-code-precedence"></a>

## Error precedence

When an error-bearing branch has non-empty `errors`, first select each
`ToolError.category` from the [Failure Model](../failure-model.md), then select
`errors[0]` by this order unless a method owner defines a stricter
method-specific order. Known `Corrupt` data is not reclassified as
`Unavailable`. A continued core operation with a `Degraded` diagnostic
does not enter this table. Public code meanings stay in
[API error codes](error-codes.md).

| Precedence | Primary `ErrorCode` | Meaning owner |
|---:|---|---|
| <a id="precedence-validation-failed"></a>1 | `VALIDATION_FAILED` | [`VALIDATION_FAILED`](error-codes.md#errorcode-validation-failed) |
| <a id="precedence-persisted-data-corrupt"></a>2 | `PERSISTED_DATA_CORRUPT` | [`PERSISTED_DATA_CORRUPT`](error-codes.md#errorcode-persisted-data-corrupt) |
| 3 | `STATE_VERSION_CONFLICT` | [`STATE_VERSION_CONFLICT`](error-codes.md#errorcode-state-version-conflict) |
| <a id="precedence-mcp-unavailable"></a>4 | `MCP_UNAVAILABLE` | [`MCP_UNAVAILABLE`](error-codes.md#errorcode-mcp-unavailable) |
| <a id="precedence-invocation-context-mismatch"></a>5 | `INVOCATION_CONTEXT_MISMATCH` | [`INVOCATION_CONTEXT_MISMATCH`](error-codes.md#errorcode-invocation-context-mismatch) |
| <a id="precedence-no-active-task"></a>6 | `NO_ACTIVE_TASK` | [`NO_ACTIVE_TASK`](error-codes.md#errorcode-no-active-task) |
| <a id="precedence-no-active-change-unit"></a>7 | `NO_ACTIVE_CHANGE_UNIT` | [`NO_ACTIVE_CHANGE_UNIT`](error-codes.md#errorcode-no-active-change-unit) |
| <a id="precedence-baseline-stale"></a>8 | `BASELINE_STALE` | [`BASELINE_STALE`](error-codes.md#errorcode-baseline-stale) |
| <a id="precedence-scope-required"></a>9 | `SCOPE_REQUIRED` | [`SCOPE_REQUIRED`](error-codes.md#errorcode-scope-required) |
| <a id="precedence-scope-violation"></a>10 | `SCOPE_VIOLATION` | [`SCOPE_VIOLATION`](error-codes.md#errorcode-scope-violation) |
| <a id="precedence-write-ticket-required"></a>11 | `WRITE_TICKET_REQUIRED` | [`WRITE_TICKET_REQUIRED`](error-codes.md#errorcode-write-ticket-required) |
| <a id="precedence-write-ticket-invalid"></a>12 | `WRITE_TICKET_INVALID` | [`WRITE_TICKET_INVALID`](error-codes.md#errorcode-write-ticket-invalid) |
| <a id="precedence-approval-denied"></a>13 | `APPROVAL_DENIED` | [`APPROVAL_DENIED`](error-codes.md#errorcode-approval-denied) |
| <a id="precedence-approval-expired"></a>14 | `APPROVAL_EXPIRED` | [`APPROVAL_EXPIRED`](error-codes.md#errorcode-approval-expired) |
| <a id="precedence-approval-required"></a>15 | `APPROVAL_REQUIRED` | [`APPROVAL_REQUIRED`](error-codes.md#errorcode-approval-required) |
| <a id="precedence-decision-unresolved"></a>16 | `DECISION_UNRESOLVED` | [`DECISION_UNRESOLVED`](error-codes.md#errorcode-decision-unresolved) |
| <a id="precedence-autonomy-boundary-exceeded"></a>17 | `AUTONOMY_BOUNDARY_EXCEEDED` | [`AUTONOMY_BOUNDARY_EXCEEDED`](error-codes.md#errorcode-autonomy-boundary-exceeded) |
| <a id="precedence-decision-required"></a>18 | `DECISION_REQUIRED` | [`DECISION_REQUIRED`](error-codes.md#errorcode-decision-required) |
| <a id="precedence-capability-insufficient"></a>19 | `CAPABILITY_INSUFFICIENT` | [`CAPABILITY_INSUFFICIENT`](error-codes.md#errorcode-capability-insufficient) |
| <a id="precedence-evidence-insufficient"></a>20 | `EVIDENCE_INSUFFICIENT` | [`EVIDENCE_INSUFFICIENT`](error-codes.md#errorcode-evidence-insufficient) |
| <a id="precedence-residual-risk-not-visible"></a>21 | `RESIDUAL_RISK_NOT_VISIBLE` | [`RESIDUAL_RISK_NOT_VISIBLE`](error-codes.md#errorcode-residual-risk-not-visible) |
| <a id="precedence-acceptance-required"></a>22 | `ACCEPTANCE_REQUIRED` | [`ACCEPTANCE_REQUIRED`](error-codes.md#errorcode-acceptance-required) |
| <a id="precedence-projection-stale"></a>23 | `PROJECTION_STALE` | [`PROJECTION_STALE`](error-codes.md#errorcode-projection-stale) |
| <a id="precedence-artifact-missing"></a>24 | `ARTIFACT_MISSING` | [`ARTIFACT_MISSING`](error-codes.md#errorcode-artifact-missing) |
| <a id="precedence-validator-failed"></a>25 | `VALIDATOR_FAILED` | [`VALIDATOR_FAILED`](error-codes.md#errorcode-validator-failed) |
| <a id="precedence-operation-result-unavailable"></a>26 | `OPERATION_RESULT_UNAVAILABLE` | [`OPERATION_RESULT_UNAVAILABLE`](error-codes.md#errorcode-operation-result-unavailable) |

For `volicord.get_operation_result`, an access-context mismatch selects
`INVOCATION_CONTEXT_MISMATCH` before the method-local unavailable-result branch.
The otherwise applicable global order is unchanged.

<a id="state-version-conflict-precedence-exclusion"></a>
### `STATE_VERSION_CONFLICT` selection boundary

Selection condition:
- A rejected response selects `STATE_VERSION_CONFLICT` when a stale `expected_state_version` or idempotency request-hash conflict prevents the method from proceeding. Ticket `basis_state_version` is audit-only and is excluded.

Selection boundary:
- Represent these conflicts through `ToolRejectedResponse.errors[]`; they do not produce a `MethodResult` or `CloseTaskResult(close_state=blocked)` branch. Do not model `STATE_VERSION_CONFLICT` as a result-side decision, blocker code, close-readiness blocker code, or planned blocker code, including `WriteDecisionReason.code`, `CloseReadinessBlocker.code`, or `PlannedBlocker.code`.

Related owner:
- Machine-readable fields for these conflicts belong to [API error details](error-details.md#state-conflict-detail-fields).

<a id="idempotency"></a>
<a id="state-conflict-behavior"></a>

## State version conflict

| Conflict case | Detail section |
|---|---|
| stale `expected_state_version` | [Stale `expected_state_version`](#state-conflict-expected-state-version) |
| idempotency request-hash conflict | [Idempotency request-hash conflict](#state-conflict-idempotency-hash) |

For precedence, these conflict cases select `STATE_VERSION_CONFLICT` as a project-wide pre-commit freshness or idempotency conflict. State-bound ticket invalidation instead selects `WRITE_TICKET_INVALID` on a ticket-consuming attempt.

Conflict routing boundary:

| Boundary | This document's rule | Neighbor owner |
|---|---|---|
| Conflict selection | Select `STATE_VERSION_CONFLICT` for the conflict cases below. | Public code meanings: [API error codes](error-codes.md). |
| Response path | Use `ToolRejectedResponse.errors[]` for these conflicts. | Response branch routing: [API error routing](error-routing.md). |
| Result, blocker, and close-readiness boundary paths | Do not use `STATE_VERSION_CONFLICT` as a blocker code, `dry_run` preview, `MethodResult.decision`, `WriteDecisionReason.code`, `CloseReadinessBlocker.code`, or `PlannedBlocker.code`. | Boundary routing: [API blocker routing](blocker-routing.md). Method behavior: [`volicord.close_task`](method-close-task.md). |
| Detail fields | Use the state-conflict detail-field family for these conflicts. | Machine-readable fields: [API error details](error-details.md#state-conflict-detail-fields). |

<a id="state-conflict-expected-state-version"></a>
### Stale `expected_state_version`

Condition:
- `ToolEnvelope.expected_state_version` is older than `project_state.state_version`.

Public code:
- `STATE_VERSION_CONFLICT`

Response path:
- `ToolRejectedResponse.errors[]`

Detail fields:
- Use [State conflict detail fields](error-details.md#state-conflict-detail-fields).
<a id="state-conflict-write-ticket-basis"></a>
### Write-ticket audit basis exclusion

Condition:
- `WriteTicket.basis_state_version` does not equal the current global state version, but the ticket's explicit validity coordinates still match.

Public code:
- None for that mismatch alone.

Boundary:
- The audit-basis mismatch neither invalidates nor consumes the ticket. Validate Task, Change Unit, scope revision, baseline, workspace, approval basis, task state, consumption, explicit revocation, and optional idle timeout instead.

No error detail object is produced for this non-error condition.

### State-bound invalid write ticket

Condition:
- Before consumption, the ticket has a recorded invalidation reason, is consumed or revoked, mismatches its current validity coordinates, or crosses a configured optional idle timeout.

Public code:
- `WRITE_TICKET_INVALID`

Response path:
- `ToolRejectedResponse.errors[]`

Precedence boundary:
- A stale request-envelope `expected_state_version` still selects `STATE_VERSION_CONFLICT` first. Ticket `basis_state_version` never does.
- Invalid ticket use is not modeled as a result-side decision or planned blocker. Close-readiness may separately expose its stable method-owned unresolved-ticket blocker.

Detail fields:
- Use the stable state-bound or attempt-mismatch `write_ticket_reason` owned by [API error details](error-details.md#write-ticket-reason).

<a id="state-conflict-idempotency-hash"></a>
### Idempotency request-hash conflict

Condition:
- The same `idempotency_key` is reused with a different request hash.

Public code:
- `STATE_VERSION_CONFLICT`

Response path:
- `ToolRejectedResponse.errors[]`

Detail fields:
- Use [State conflict detail fields](error-details.md#state-conflict-detail-fields).
