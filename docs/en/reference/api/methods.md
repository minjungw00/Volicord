# API methods

This reference owns the active public API method list and routes each method to its behavior owner. It does not define method behavior, shared schema bodies, storage effects, public error semantics, or example consistency rules.

<a id="baseline-scope-method-behavior"></a>

## Supported-method boundary

Only methods listed below are supported public API methods routed by this document. A method name not listed here is outside the supported public method family.

Method-specific behavior belongs to the method owner documents. Out-of-scope API or schema capabilities remain outside this method router unless [Scope](../scope.md) and the affected owners define them as active.

<a id="method-owner-routing-table"></a>

## Current API method list

This table is both the supported public method list and the first-hop route for method behavior questions.

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
