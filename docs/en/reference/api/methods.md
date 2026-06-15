# API methods

Use this method-family router to find the owner document for public Harness API method behavior. For exact machine-readable owner routing, use [`docs/doc-index.yaml`](../../../doc-index.yaml).

This page does not define method behavior, request or response bodies, shared schemas, storage effects, error semantics, security guarantees, or Core authority semantics.

<a id="method-owner-routing-table"></a>

## Method Owners

<a id="harnessintake"></a>
<a id="harnessupdate_scope"></a>
<a id="harnessstatus"></a>
<a id="harnessprepare_write"></a>
<a id="harnessstage_artifact"></a>
<a id="harnessrecord_run"></a>
<a id="harnessrequest_user_judgment"></a>
<a id="harnessrecord_user_judgment"></a>
<a id="harnessclose_task"></a>

| Method | Owner |
|---|---|
| `harness.intake` | [Intake method](method-intake.md) |
| `harness.update_scope` | [Update-scope method](method-update-scope.md) |
| `harness.status` | [Status method](method-status.md) |
| `harness.prepare_write` | [Prepare-write method](method-prepare-write.md) |
| `harness.stage_artifact` | [Stage-artifact method](method-stage-artifact.md) |
| `harness.record_run` | [Record-run method](method-record-run.md) |
| `harness.request_user_judgment` | [User-judgment method owner](method-user-judgment.md#harnessrequest_user_judgment) |
| `harness.record_user_judgment` | [User-judgment method owner](method-user-judgment.md#harnessrecord_user_judgment) |
| `harness.close_task` | [Close-task method](method-close-task.md) |

## Nearby Routes

- Shared envelopes and response branch shapes: [API Schema Core](schema-core.md).
- Method-independent API value sets: [API Value Sets](schema-value-sets.md).
- API error families: [API Errors](errors.md).
- Storage effects by method or branch: [Storage Effects](../storage-effects.md).
- Product and Core concepts used by methods: [Core Model](../core-model.md).
