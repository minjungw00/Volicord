# API value sets

This document owns supported API value sets and enum-like public values for the baseline scope. Naming a reserved or out-of-scope value does not widen baseline scope.

## Owns / does not own

This document owns:

- supported public method-name values
- supported actor-kind values
- supported next-action values
- API `response_kind` and `effect_kind` values
- supported `access_class` values
- record/reference discriminator values used by shared state references
- supported lifecycle, close-state, source-kind, write-decision category, judgment-kind, presentation, required-for, judgment resolution outcome, artifact redaction, artifact integrity, artifact availability display, `ValidatorResult.status`, `ValidatorResult.severity`, guarantee-display, and similar API value sets
- supported `change_unit.operation` values
- the boundary for supported public `ValidatorResult.validator_id` values
- the value-set boundary for method-scoped reason codes and opaque classification strings
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
- API examples must use supported enum-like values from this document unless the schema owner explicitly defines the field as a free-form display string, an opaque identifier, or an opaque classification string.
- A string-like field is controlled by this document only when the schema owner routes that field to a value set here. Opaque identifiers, opaque classification strings, and free-form display strings stay with their schema or method owner.
- A method example may show an opaque reason code or classification string without making that string a supported global value.

<a id="method-name-values"></a>
## Method name values

The supported public method-name set is:

```text
volicord.intake
volicord.update_scope
volicord.status
volicord.prepare_write
volicord.stage_artifact
volicord.record_run
volicord.request_user_judgment
volicord.record_user_judgment
volicord.close_task
```

Method behavior is owned by method owner documents routed from [API Methods](methods.md). Method names are not Task lifecycle values.

<a id="actor-values"></a>
## Actor values

`ToolEnvelope.actor_kind` and `UserJudgmentResolution.resolved_by_actor_kind` use the same controlled value set:

| Value | Used by | Owner route |
|---|---|---|
| `agent` | Request envelopes and judgment resolution shapes. | Shape owner: [API Schema Core](schema-core.md#tool-envelope); resolution shape owner: [API Judgment Schemas](schema-judgment.md). |
| `user` | Request envelopes and judgment resolution shapes. | Shape owner: [API Schema Core](schema-core.md#tool-envelope); resolution shape owner: [API Judgment Schemas](schema-judgment.md). |

These values classify the API actor named by the request or resolution shape. They do not by themselves create user-owned judgment, approval, scope-decision authority, final acceptance, residual-risk acceptance, or `Write Authorization`. `actor_kind=user` is attribution, not proof; authority-bearing resolution also requires compatible internal `VerifiedActorContext` provenance from [Agent Integration](../agent-integration.md).

<a id="next-action-values"></a>
## Next-action values

`NextActionSummary.action_kind` is a controlled action-category value. It uses only these supported values:

| `action_kind` value | `owner_method` when one method owns the next step |
|---|---|
| `update_scope` | `volicord.update_scope` |
| `prepare_write` | `volicord.prepare_write` |
| `stage_artifact` | `volicord.stage_artifact` |
| `record_run` | `volicord.record_run` |
| `request_user_judgment` | `volicord.request_user_judgment` |
| `record_user_judgment` | `volicord.record_user_judgment` |
| `close_task` | `volicord.close_task` |

`action_kind` is not a method-name value. `NextActionSummary.owner_method` uses the [method-name value set](#method-name-values) when one supported public method owns the next step, and it is `null` when no single owner method applies. Method behavior for the next step stays with the method owner document routed from [API Methods](methods.md). The full `NextActionSummary` shape is owned by [API State Schemas](schema-state.md#current-position-display-shapes).

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

`response_kind` and `effect_kind` are branch metadata values. Common branch shape is owned by [API Schema Core](schema-core.md#common-response), method-specific effects are owned by method owner documents, and public error semantics for rejected branches are owned by [API error codes](error-codes.md) and [API error routing](error-routing.md).

<a id="opaque-and-method-scoped-string-fields"></a>
## Opaque and method-scoped string fields

The fields below are intentionally not global closed value sets:

| Field | Classification | Owner route |
|---|---|---|
| `EventRef.event_kind` | Opaque event classification string. Method examples may show event-kind strings, but this document does not publish an exhaustive public event-kind value set. | Shape owner: [API Schema Core](schema-core.md#shared-support-shapes). Event-producing behavior: method owner documents. |
| `WriteDecisionReason.code` | Method-scoped opaque reason code. Method owners may show example codes without creating a global exhaustive code list. | Shape owner: [API State Schemas](schema-state.md#current-position-display-shapes). Production and local meaning: [`volicord.prepare_write`](method-prepare-write.md) and other affected method owners. |

Public `ErrorCode` values are separate and are owned by [API error codes](error-codes.md).

<a id="access-class-values"></a>
## Access class values

`VerifiedSurfaceContext.access_class` uses exactly one request-level value per public API request:

| Value | Vocabulary note |
|---|---|
| `read_status` | Status and close-check read access-class value. |
| `core_mutation` | Core-mutation access-class value. |
| `write_authorization` | Access-class value associated with `volicord.prepare_write`. |
| `run_recording` | Access-class value associated with `volicord.record_run`. |
| `artifact_registration` | Access-class value associated with `volicord.stage_artifact`. |
| `artifact_read` | Artifact-read access-class value; artifact body-read support is owned by [Artifact Storage](../storage-artifacts.md). |

Access classes are Volicord API compatibility classes, not OS permission classes. Method access requirements stay with method owner documents routed from [API Methods](methods.md); local surface verification behavior stays with [Agent Integration](../agent-integration.md) and [Security](../security.md).

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

These values identify API reference kinds. They do not replace storage table names, DDL, Core authority meaning, or method-specific ownership rules.

<a id="task-lifecycle-values"></a>
## Task lifecycle values

`StateSummary.mode` and resolved `Task.mode` fields use:

```text
advisor
direct
work
```

`requested_mode` for `volicord.intake` also accepts `auto` as an input-only value. Output `Task.mode` fields use `advisor`, `direct`, or `work`; intake resolution behavior is owned by [Intake method](method-intake.md).

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

`StatusResult.close_state` also permits `none` when no current close state is available.

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

<a id="method-local-values"></a>
## Method-local values

`resume_policy` for `volicord.intake` uses:

```text
resume_active
create_new
supersede_active
reject_if_active
```

`change_unit.operation` uses:

```text
keep_current
create_current
replace_current
```

Value meanings:
- `keep_current` updates scope-related Task fields without changing the current Change Unit.
- `create_current` creates the current Change Unit when there is no suitable current Change Unit.
- `replace_current` replaces the current Change Unit with a new work boundary.

Method behavior for each operation is owned by [`volicord.update_scope`](method-update-scope.md). The supported value set stays here so API examples and schema readers have one canonical value owner.

`volicord.close_task.intent` uses:

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
```

`WriteAuthoritySummary.status` and `WriteAuthorizationSummary.status` use:

```text
active
consumed
expired
stale
revoked
```

`RecordRunRequest.kind` and `RunSummary.kind` use:

```text
shaping_update
implementation
direct
```

<a id="state-and-blocker-values"></a>
## State and blocker values

The `CloseReadinessBlocker` object shape is owned by [API State Schemas](schema-state.md#close-readiness-and-validation-shapes). This section owns the supported `CloseReadinessBlocker.category` values and neighboring state/blocker values.

`PlannedBlocker.source_kind` uses:

```text
write_decision
close_readiness
```

`WriteDecisionReason.category` is a controlled category value. It uses only these supported values:

| Value | Category family |
|---|---|
| `scope` | Scope compatibility or scope-boundary reason. |
| `user_judgment` | Required user-owned judgment reason. |
| `sensitive_approval` | Required separate sensitive-action approval reason. |
| `write_compatibility` | Write-compatibility reason. |
| `baseline` | Baseline compatibility reason. |
| `surface_capability` | Verified surface capability reason. |

These categories classify `volicord.prepare_write` decision reasons. They are not `CloseReadinessBlocker` objects and do not evaluate close readiness. Method-specific decision behavior and reason production stay with [`volicord.prepare_write`](method-prepare-write.md).

This value set controls `category` only. `WriteDecisionReason.code` is not a global closed enum. It is a method-scoped opaque reason code; method owners may show example codes without adding them to a global supported list. `message` is a free-form display string, and `related_refs` uses `StateRecordRef`.

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

`cooperative` is the baseline fallback. `detective` may be displayed only when the security owner supports the claim and the project enforcement profile, verified bound surface registration, enabled enforcement mechanism, and observed-scope facts support it. Capability declarations alone cannot raise the displayed guarantee.

<a id="artifact-values"></a>
## Artifact values

`ArtifactInput.source_kind` uses:

```text
staged_artifact
existing_artifact
```

Value meanings:
- `staged_artifact` pairs with `ArtifactInput.staged_artifact_handle`.
- `existing_artifact` pairs with `ArtifactInput.existing_artifact_ref`.

The selected source value determines which `ArtifactInput` source field applies. The exact shape invariant is owned by [API Artifact Schemas](schema-artifacts.md#artifactinput).

Values outside this list are not supported source values. New source vocabulary needs a supported value here and an affected semantic owner before behavior can be described as supported.

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

`ArtifactIntegrityStatus` uses:

```text
verified
corrupt
```

`verified` means persisted artifact facts are complete enough for integrity-aware use and current-byte verification may be performed before authority use. `corrupt` means stored bytes or metadata are known not to match persisted integrity facts, or the stored verified-fact relationship is invalid. Artifact evidence and close use require the current-byte checks owned by [Artifact Storage](../storage-artifacts.md). Missing, unreadable, unavailable, or unusable backing bytes are represented by artifact availability values, not by artifact integrity values.

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

`required_for` uses operation-target values:

```text
scope_update
prepare_write
record_run
close_complete
close_cancel
close_supersede
informational
```

`UserJudgment.status` uses:

```text
pending
resolved
stale
superseded
expired
```

Status values describe the judgment lifecycle. `resolved` means an answer was recorded; it does not by itself mean approval, acceptance, or authorization.

`JudgmentResolutionOutcome` uses:

```text
accepted
rejected
deferred
```

`JudgmentBasis.compatibility_status` uses:

```text
current
stale
superseded
```

Meaning:
- `current` means the basis currently matches the requirement it may satisfy.
- `stale` means the stored basis no longer matches current state; a resolved row may remain for audit but is ineligible for current requirements.
- `superseded` means a pending judgment has been replaced by a newer question or basis and cannot be answered successfully.

Authority option action values:
- `accept` maps to `accepted`.
- `reject` maps to `rejected`.
- `defer` maps to `deferred` only where the method or semantic owner permits deferral.

Resolution outcome meaning:
- `accepted` is the only outcome that can satisfy an authority-bearing judgment requirement when the judgment kind, basis, verified actor provenance, selected option, and `machine_action=accept` are otherwise compatible.
- `rejected` and `deferred` are durable user decisions but do not approve, accept, authorize, waive, or close anything.
- `blocked` is used by unrelated blocked-result and blocker value sets elsewhere in the product, but it is not a `JudgmentResolutionOutcome` value and cannot be persisted as a selected-option resolution.
- Absence of a machine-readable outcome must never be interpreted as `accepted`.

Pending-judgment relevance:
- A pending judgment blocks an operation only when its current `required_for` target includes that operation, its `judgment_kind` is relevant to that operation, and its Task, Change Unit, affected refs, and basis are compatible.
- For sensitive approval, the pending question is relevant only when its sensitive-action scope overlaps the current sensitive action requirement.
- `informational` judgments are audit or display context and do not block write, run, or close operations by themselves.

`UserJudgmentOption.option_id` is scoped to the judgment and is not a global value set. Rendered option labels are display text only. Current public `UserJudgmentOption.machine_action` uses the authority option action values above. `UserJudgmentOption.resolution_outcome` uses `JudgmentResolutionOutcome`; option labels and explanatory text must not invert the machine-readable action or outcome.

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
