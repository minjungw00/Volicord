# API methods

Use this page to find the Reference owner for each public Volicord API method. The linked pages define the exact method contracts.

This page does not define method behavior, request or response bodies, shared schemas, storage effects, error semantics, security guarantees, or Core authority semantics.

<a id="surface-stability"></a>
## Surface Stability

The labels below use the vocabulary in [Documentation Policy](../../maintain/documentation-policy.md#surface-stability-labels).

| Surface | Stability | Notes |
|---|---|---|
| Supported public method names in the table below | `stable` | These names make up the supported public API method set. |
| Linked method owner documents | `stable` | Each linked owner defines the method behavior, request and response shape, and effects it owns unless that owner labels a narrower nested surface differently. |

<a id="method-owner-routing-table"></a>

## Method Owners

<a id="volicordintake"></a>
<a id="volicordupdate_scope"></a>
<a id="volicordrecord_shaping"></a>
<a id="volicordadvance_task"></a>
<a id="volicordstatus"></a>
<a id="volicordget_operation_result"></a>
<a id="volicordprepare_evidence_capture"></a>
<a id="volicordprepare_write"></a>
<a id="volicordstage_artifact"></a>
<a id="volicordrecord_run"></a>
<a id="volicordrequest_user_action"></a>
<a id="volicordresolve_user_action"></a>
<a id="volicordreconcile_changes"></a>
<a id="volicordcheck_close"></a>
<a id="volicordclose_task"></a>

| Method | Owner |
|---|---|
| `volicord.intake` | [Intake method](method-intake.md) |
| `volicord.update_scope` | [Update-scope method](method-update-scope.md) |
| `volicord.record_shaping` | [Record-shaping method](method-record-shaping.md) |
| `volicord.advance_task` | [Advance-task method](method-advance-task.md) |
| `volicord.status` | [Status method](method-status.md) |
| `volicord.get_operation_result` | [Get-operation-result method](method-get-operation-result.md#volicordget_operation_result) |
| `volicord.prepare_evidence_capture` | [Prepare-evidence-capture method](method-prepare-evidence-capture.md#volicordprepare_evidence_capture) |
| `volicord.prepare_write` | [Prepare-write method](method-prepare-write.md) |
| `volicord.stage_artifact` | [Stage-artifact method](method-stage-artifact.md) |
| `volicord.record_run` | [Record-run method](method-record-run.md) |
| `volicord.request_user_action` | [Request-user-action method](method-request-user-action.md#volicordrequest_user_action) |
| `volicord.resolve_user_action` | [Resolve-user-action method](method-resolve-user-action.md#volicordresolve_user_action) |
| `volicord.reconcile_changes` | [Reconcile-changes method](method-reconcile-changes.md#volicordreconcile_changes) |
| `volicord.check_close` | [Close method](method-close-task.md#volicordcheck_close) |
| `volicord.close_task` | [Close method](method-close-task.md#volicordclose_task) |

## Nearby Routes

- Shared envelopes and response branch shapes: [API Schema Core](schema-core.md).
- Method-independent API value sets: [API Value Sets](schema-value-sets.md).
- API error families: [API Errors](errors.md).
- Storage effects by method or branch: [Storage Effects](../storage-effects.md).
- Product and Core concepts used by methods: [Core Model](../core-model.md).
