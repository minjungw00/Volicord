# API value sets

This document owns supported API value sets and enum-like public values for the baseline scope. Naming a reserved or out-of-scope value does not widen baseline scope.

## Owns / does not own

This document owns:

- supported public method-name values
- API `response_kind` and `effect_kind` values
- supported `access_class` values
- record/reference discriminator values used by shared state references
- supported lifecycle, close-state, source-kind, judgment-kind, presentation, required-for, artifact redaction, artifact availability display, `ValidatorResult.status`, `ValidatorResult.severity`, guarantee-display, and similar API value sets
- the boundary for supported public `ValidatorResult.validator_id` values
- profile-gated or reserved value boundaries where they affect supported schema interpretation
- the rule that rendered labels are not canonical schema values

This document does not own:

- public `ErrorCode` values or precedence; see [API error codes](error-codes.md) and [API error precedence](error-precedence.md)
- close-readiness blocker routing; see [API blocker routing](blocker-routing.md)
- field shapes that use these values; see [API Schema Core](schema-core.md), [API State Schemas](schema-state.md), [API Artifact Schemas](schema-artifacts.md), and [API Judgment Schemas](schema-judgment.md)
- method behavior; see the [API Methods](methods.md) and method owner documents
- security guarantee meaning; see [Security](../security.md)
- out-of-scope capability promotion; see [Scope Reference](../scope.md)

## Boundary

Only values listed as supported in this document are supported API values.

- Profile-gated values must name the profile or capability gate at the point of use.
- Values outside the supported lists are not baseline API values unless [Scope](../scope.md) and the affected semantic owner define the supported behavior.
- Naming a value outside a supported list does not widen baseline scope.
- Rendered labels are display text. They do not replace the canonical values listed in this document.

<a id="method-name-values"></a>
## Method name values

The supported public method-name set is:

```text
harness.intake
harness.update_scope
harness.status
harness.prepare_write
harness.stage_artifact
harness.record_run
harness.request_user_judgment
harness.record_user_judgment
harness.close_task
```

Method behavior is owned by method owner documents routed from [API Methods](methods.md). Method names are not Task lifecycle values.

<a id="response-and-effect-values"></a>
## Response and effect values

`ToolResultBase.response_kind` uses:

```text
result
rejected
dry_run
```

`ToolResultBase.effect_kind` uses:

```text
read_only
core_committed
staging_created
no_effect
```

`response_kind` and `effect_kind` are branch metadata values. Common branch shape is owned by [API Schema Core](schema-core.md#common-response), and method-specific state effects are owned by method owner documents. Public error semantics for rejected branches are owned by [API error codes](error-codes.md) and [API error routing](error-routing.md).

<a id="access-class-values"></a>
## Access class values

`VerifiedSurfaceContext.access_class` uses exactly one request-level value per public API request:

| Value | Semantic owner |
|---|---|
| `read_status` | Read-only status and close-check reads. |
| `core_mutation` | Core state mutation not otherwise specialized. |
| `write_authorization` | `harness.prepare_write`. |
| `run_recording` | `harness.record_run`. |
| `artifact_registration` | `harness.stage_artifact`. |
| `artifact_read` | Artifact body reads when an artifact owner defines support. |

Access classes are Harness API compatibility classes, not OS permission classes. Method access requirements stay with method owner documents routed from [API Methods](methods.md); local surface verification behavior stays with [Agent Integration](../agent-integration.md) and [Security](../security.md).

<a id="record-and-reference-values"></a>
## Record and reference values

`StateRecordRef.record_kind` uses:

```text
project_state
task
change_unit
write_authorization
user_judgment
run
evidence_summary
artifact
blocker
task_event
local_surface_registration
```

These values identify persisted Core or local-surface record kinds in API references. They do not replace storage table names, DDL, or method-specific ownership rules.

<a id="task-lifecycle-values"></a>
## Task lifecycle values

`StateSummary.mode` and persisted resolved Task mode use:

```text
advisor
direct
work
```

`requested_mode` for `harness.intake` also accepts `auto` as input only. `auto` must resolve to `advisor`, `direct`, or `work` before persisted or displayed Task state.

`Task.lifecycle_phase` uses:

```text
shaping
ready
executing
waiting_user
blocked
completed
cancelled
superseded
```

`CloseTaskResult.close_state` uses:

```text
ready
blocked
closed
cancelled
superseded
```

`StatusResult.close_state` also permits `none` when no active close state is available.

`Task.close_reason` uses:

```text
none
completed_self_checked
completed_with_risk_accepted
cancelled
superseded
```

`Task.result` uses:

```text
none
advice_only
completed
cancelled
superseded
```

Run failures, violations, blocked closes, and evidence gaps are not terminal `Task.result` values.

## Method-local values

`resume_policy` for `harness.intake` uses:

```text
resume_active
create_new
supersede_active
reject_if_active
```

`harness.close_task.intent` uses:

```text
check
complete
cancel
supersede
```

`PrepareWriteResult.decision` uses:

```text
allowed
blocked
approval_required
decision_required
```

`PrepareWriteResult.authorization_effect` uses:

```text
none
would_create
created
returned
```

`RecordRunRequest.kind` and `RunSummary.kind` use:

```text
shaping_update
implementation
direct
```

<a id="state-and-blocker-values"></a>
## State and blocker values

`PlannedBlocker.source_kind` uses:

```text
write_decision
close_readiness
```

`CloseReadinessBlocker.category` uses:

```text
task
open_run
scope
user_judgment
sensitive_approval
write_compatibility
baseline
surface_capability
evidence
artifact_availability
final_acceptance
residual_risk_visibility
residual_risk_acceptance
recovery
```

`EvidenceSummary.status` uses:

```text
unknown
insufficient
sufficient
blocked
```

`EvidenceCoverageItem.coverage_state` uses:

```text
unsupported
partial
supported
not_applicable
stale
blocked
```

`ValidatorResult.status` uses:

```text
passed
warning
failed
blocked
```

`ValidatorResult.severity` uses:

```text
info
warning
error
blocking
```

This baseline value-set owner does not publish a supported stable `ValidatorResult.validator_id` set. A `validator_id` string is a reporting label unless an affected owner publishes the exact stable value here and defines its semantic meaning.

`GuaranteeDisplay.level` uses baseline values:

```text
cooperative
detective
```

<a id="artifact-values"></a>
## Artifact values

`ArtifactInput.source_kind` uses:

```text
staged_artifact
existing_artifact
```

Value meanings:
- `staged_artifact` selects a compatible transient staged handle through artifact-owner behavior.
- `existing_artifact` selects an already persistent same-project artifact without registering new bytes.

The selected source value determines which `ArtifactInput` source field applies. The exact shape invariant is owned by [API Artifact Schemas](schema-artifacts.md#artifactinput).

Values outside this list are not supported source values. New source behavior needs a supported value here and an affected semantic owner.

`redaction_state` uses:

```text
none
redacted
secret_omitted
blocked
```

Artifact availability display values use:

```text
available
unavailable
missing
integrity_failed
blocked
unusable
```

Artifact storage lifecycle and body-read eligibility are owned by [Artifact Storage](../storage-artifacts.md).

<a id="judgment-values"></a>
## Judgment values

`judgment_kind` uses:

```text
product_decision
technical_decision
scope_decision
sensitive_approval
final_acceptance
residual_risk_acceptance
cancellation
```

`presentation` uses:

```text
short
```

`required_for` uses:

```text
next_action
write
run
close
acceptance
risk
```

`UserJudgment.status` uses:

```text
pending
resolved
rejected
deferred
blocked
stale
superseded
incompatible
```

`UserJudgmentOption.option_id` is scoped to the judgment and is not a global value set. Rendered option labels are display text only.

## Error detail helper values

`ToolError.details.authorization_reason` and `ToolError.details.artifact_input_error.reason` helper values are owned by [API error details](error-details.md#error-detail-helper-values). This value-set document does not define machine-readable error detail semantics.

## Profile-gated and reserved values

Reserved or profile-gated names are not default baseline values. This document does not publish unsupported value names as part of the supported value sets.

Boundary:
- A name outside a supported list is not available as baseline behavior by appearing in a note, example, route page, or rendered label.
- A reserved or profile-gated value needs the [Scope](../scope.md) boundary and affected semantic owner before any behavior can be described as supported.

## Related owners

- [Scope](../scope.md) for whether a value belongs in the baseline scope.
- [API error codes](error-codes.md) for public error code meanings.
- [API error precedence](error-precedence.md) for public error precedence.
- [API blocker routing](blocker-routing.md) for close-readiness blocker routing.
- [API error details](error-details.md) for machine-readable error detail helper values.
- [API Schema Core](schema-core.md), [API State Schemas](schema-state.md), [API Artifact Schemas](schema-artifacts.md), and [API Judgment Schemas](schema-judgment.md) for fields that use these values.
- [API Methods](methods.md) and method owner documents for method behavior using these values.
- [Scope Reference](../scope.md) for reserved and profile-gated value boundaries.
