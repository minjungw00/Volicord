# API methods

Use this human-readable method-family router to find the owner document for public Volicord API method behavior. For the exact machine-readable owner route, use [`docs/doc-index.yaml`](../../../doc-index.yaml).

This page does not define method behavior, request or response bodies, shared schemas, storage effects, error semantics, security guarantees, or Core authority semantics.

<a id="method-owner-routing-table"></a>

## Method Owners

<a id="volicordintake"></a>
<a id="volicordupdate_scope"></a>
<a id="volicordstatus"></a>
<a id="volicordprepare_write"></a>
<a id="volicordstage_artifact"></a>
<a id="volicordrecord_run"></a>
<a id="volicordrequest_user_judgment"></a>
<a id="volicordrecord_user_judgment"></a>
<a id="volicordreconcile_changes"></a>
<a id="volicordclose_task"></a>

| Method | Owner |
|---|---|
| `volicord.intake` | [Intake method](method-intake.md) |
| `volicord.update_scope` | [Update-scope method](method-update-scope.md) |
| `volicord.status` | [Status method](method-status.md) |
| `volicord.prepare_write` | [Prepare-write method](method-prepare-write.md) |
| `volicord.stage_artifact` | [Stage-artifact method](method-stage-artifact.md) |
| `volicord.record_run` | [Record-run method](method-record-run.md) |
| `volicord.request_user_judgment` | [Request-user-judgment method](method-request-user-judgment.md#volicordrequest_user_judgment) |
| `volicord.record_user_judgment` | [Record-user-judgment method](method-record-user-judgment.md#volicordrecord_user_judgment) |
| `volicord.reconcile_changes` | [Reconcile-changes method](method-reconcile-changes.md#volicordreconcile_changes) |
| `volicord.close_task` | [Close-task method](method-close-task.md) |

## Nearby Routes

- Shared envelopes and response branch shapes: [API Schema Core](schema-core.md).
- Method-independent API value sets: [API Value Sets](schema-value-sets.md).
- API error families: [API Errors](errors.md).
- Storage effects by method or branch: [Storage Effects](../storage-effects.md).
- Product and Core concepts used by methods: [Core Model](../core-model.md).
