# API value sets

This document owns supported API value sets and enum-like public values for the baseline scope. Naming a reserved or out-of-scope value does not widen baseline scope.

## Owns / does not own

This document owns:

- supported public method-name values
- supported actor-kind values
- supported next-action values
- API `response_kind` and `effect_kind` values
- supported operation-category values
- record/reference discriminator values used by shared state references
- supported lifecycle, close-state, evidence observation source and assurance, write-decision category, judgment-kind, presentation, required-for, judgment resolution outcome, artifact redaction, artifact integrity, artifact availability display, `ValidatorResult.status`, `ValidatorResult.severity`, guarantee-display, and similar API value sets
- supported `change_unit.operation` values
- the boundary for supported public `ValidatorResult.validator_id` values
- the value-set boundary for method-scoped reason codes and opaque classification strings
- mode-gated or reserved value boundaries where they affect supported schema interpretation
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

- Mode-gated values must name the connection mode, User Channel, admin-local, or owner-defined gate at the point of use.
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
volicord.reconcile_changes
volicord.close_task
```

Method behavior is owned by method owner documents routed from [API Methods](methods.md). Method names are not Task lifecycle values.

<a id="actor-source-values"></a>
<a id="actor-values"></a>
## Actor source values

Actor provenance fields such as `EvidenceObservation.observed_by_actor_source`, `EvidenceObservationInput.observed_by_actor_source`, and `UserJudgmentResolution.resolved_by_actor_source` use the `ActorSource` value set:

| Value | Used by | Owner route |
|---|---|---|
| `local_user` | User Channel invocation provenance and authority-bearing user-judgment resolution. | Invocation meaning: [Agent Connection](../agent-connection.md); resolution shape owner: [API Judgment Schemas](schema-judgment.md). |
| `agent_connection:<connection_id>` | Agent Connection invocation provenance and agent-created or agent-observed state. | Invocation meaning: [Agent Connection](../agent-connection.md); nested shape owners define where the value appears. |
| `system` | Internal system provenance where an owner explicitly allows it. | Method and storage owners define where the value appears. |

These values classify derived invocation or persisted actor provenance. They do not by themselves create user-owned judgment, approval, scope-decision authority, final acceptance, residual-risk acceptance, or `Write Check`. Authority-bearing user-judgment resolution requires `resolved_by_actor_source=local_user` with compatible User Channel provenance as defined by [Agent Connection](../agent-connection.md) and the method owner.

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
| `reconcile_changes` | `volicord.reconcile_changes` |
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

<a id="operation-category-values"></a>
## Operation category values

Method-owned API compatibility checks use exactly one request-level operation category per public API request:

| Value | Vocabulary note |
|---|---|
| `read` | Read-only API operation category. A `read_only` Agent Connection can dispatch this category. |
| `agent_workflow` | Agent workflow operation category. A `workflow` Agent Connection can dispatch this category and `read`. |
| `user_only` | User Channel operation category for authority-bearing user actions. Agent Connections do not dispatch this category. |
| `admin_local` | Local administrative operation category. Agent Connections do not dispatch this category. |
| `local_recovery` | Local user recovery operation category for method-owned recovery paths such as `volicord.reconcile_changes`. Agent Connections do not dispatch this category. |

Operation categories are Volicord API compatibility categories, not OS permission classes, filesystem ACLs, sandbox rules, network policy, or secret isolation. Method operation requirements stay with method owner documents routed from [API Methods](methods.md); Agent Connection invocation verification behavior stays with [Agent Connection](../agent-connection.md) and [Security](../security.md).

<a id="record-and-reference-values"></a>
## Record and reference values

`StateRecordRef.record_kind` uses:

```text
project_state
task
change_unit
write_check
user_judgment
run
evidence_summary
evidence_observation
artifact
blocker
task_event
agent_connection
project_continuity_record
unrecorded_change
```

These values identify API reference kinds. They do not replace storage table names, DDL, Core authority meaning, or method-specific ownership rules.

<a id="project-continuity-values"></a>
## Project continuity values

`ProjectContinuityRecord.kind` and `ProjectContinuitySummary.kind` use:

```text
decision
obligation
known_limit
accepted_risk
constraint
```

`ProjectContinuityRecord.status` and `ProjectContinuitySummary.status` use:

```text
active
superseded
closed
```

These values classify durable project-level context. They do not by themselves create current Task authority, satisfy pending user judgments, prove evidence, grant `Write Check`, satisfy close readiness, or accept residual risk for a future close basis.

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

`ChangeUnitEffectContract.allowed_effects` and `ChangeUnitEffectContract.forbidden_effects` use:

```text
product_file_write
artifact_registration
run_recording
user_judgment_request
evidence_update
sensitive_action
external_network
secret_access
```

These values classify effects as Core state. They do not by themselves create a runtime sandbox, command interception, network blocking, secret isolation, user judgment, sensitive-action approval, evidence, `Write Check`, final acceptance, close readiness, or residual-risk acceptance.

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

`PrepareWriteResult.write_check_effect` uses:

```text
none
would_create
created
```

`WriteCheckStateSummary.status` and `WriteCheckSummary.status` use:

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

`GuardHealthSummary.guard_mode` uses:

```text
mcp_only
guarded
managed
```

`managed` is a recorded guard mode only for installations backed by a verified
managed distribution contract, such as a host-supported plugin, managed
configuration bundle, or managed policy layer. A host that lacks such a verified
contract must report managed initialization as unsupported instead of recording
ordinary project-local guarded files as managed mode.

`GuardHealthSummary.guard_strength` uses:

```text
authority_record_only
detective_watch
host_hook_guarded
managed_guarded
```

`authority_record_only` means Volicord can record authority state but no active
session watcher or effective host hook guard is available for the selected
view. `detective_watch` means a session watcher is active and can detect bypass
Product Repository changes after its coverage start, but it cannot pre-block
writes or identify the actor. `host_hook_guarded` means the selected
project-local guarded host hooks have verified generated config,
cwd-independent and subdirectory-safe hook commands, native host output,
required lifecycle phases, Bash/shell and direct file-write matcher coverage,
matching policy hash, and current runtime guard observation.
`managed_guarded` means the host-hook guarded condition is met and the selected
managed distribution metadata is verified. These labels do not prove product
correctness, review completion, test sufficiency, OS enforcement, sandboxing,
security isolation, final acceptance, or residual-risk acceptance.

`GuardHealthSummary.hook_path_safety` uses:

```text
ok
not_recorded
metadata_missing
authority_mismatch
policy_hash_mismatch
host_output_mismatch
relative_path_unsafe
absolute_path_stale
placeholder_unsupported
dispatch_missing
wrapper_missing
wrapper_not_executable
```

`ok` means every required host hook command is recorded as cwd-independent and
subdirectory-safe and resolves to the expected managed wrapper path. Failure
values report the primary reason that condition is not met: relative commands
that depend on the session cwd, stale absolute project roots, unsupported
placeholders, missing dispatch or wrapper scripts, non-executable wrapper
scripts on supported Unix-like platforms, generated wrapper metadata mismatch,
or missing verification metadata.

`GuardHealthSummary.guard_installation_status` uses:

```text
absent
configured
reload_required
active
degraded
stale
broken
```

`GuardHealthSummary.guard_configuration_status` uses:

```text
absent
configured
reload_required
degraded
stale
broken
```

`GuardHealthSummary.guard_observation_status` uses:

```text
not_observed
observed
stale_observation
```

`GuardHealthSummary.effective_guard_status` uses:

```text
inactive
action_required
active
degraded
broken
```

`GuardHealthSummary.prompt_capture_status` uses:

```text
unavailable
unsupported_by_host
not_configured
reload_required
configured
observed
active
degraded
```

`GuardHealthSummary.session_watch_status` uses:

```text
disabled
active
degraded
unavailable
pending_project_selection
```

`GuardHealthSummary.session_watch_coverage_basis` uses:

```text
mcp_start
first_project_selection
method_boundary
```

These values report guard integration state for close-readiness and status projections. `guard_installation_status` is the stored lifecycle value, `guard_configuration_status` derives file and required-hook completeness, `guard_observation_status` derives whether the current installation has a matching hook observation, and `effective_guard_status` is the close-readiness health used for guarded or managed paths. `active` effective health requires guarded or managed mode, complete required hook configuration, a non-stale and non-broken installation, a current matching observation, and matching host and policy identity. `prompt_capture_status` is the prompt-capture availability state for user-owned judgment chat commands: `unsupported_by_host` means the host capability is absent, `not_configured` means the prompt-capture phase is not configured for the selected connection, `reload_required` means installed configuration or policy identity must be reloaded before use, `configured` means verification-code chat commands may be shown before a prompt-capture observation, `observed` means a matching guard hook has been observed, `active` means a matching prompt-capture hook observation is recorded, and `degraded` means prompt capture is blocked by degraded guard health. `session_watch_status` is detective watcher availability: `disabled` means no selected session-watch baseline is available, `active` means bounded snapshot comparison is available, `degraded` means watcher output is partial or needs operator attention, and `unavailable` means the watcher could not perform the selected snapshot check. These values do not prove product correctness, test sufficiency, OS enforcement, sandboxing, security isolation, final acceptance, or residual-risk acceptance. `mcp_only` remains cooperative except that unresolved watcher-created unrecorded-change findings block close while an active session watch is selected.

`pending_project_selection` means an MCP session has more than one available
project and has not yet selected a project explicitly enough to create a
session-watch baseline. `mcp_start` means watcher coverage starts before MCP
tool handling for a project-bound startup or HTTP session initialization.
`first_project_selection` means coverage starts when a multi-project session
first names an explicit `project_selector`. `method_boundary` means coverage
starts at the Core method-boundary fallback. `first_project_selection` and
`method_boundary` are partial coverage bases; Product Repository changes before
the recorded coverage start are outside watcher coverage.

`UnrecordedChangeFinding.status` uses:

```text
unresolved
resolved
```

<a id="unrecorded-change-resolution-basis-values"></a>
`UnrecordedChangeResolutionSummary.resolution_basis` and stored unrecorded-change resolution metadata use:

```text
reverted
covered_by_write_readiness
recorded_as_expected_write
accepted_by_user
not_product_change
superseded_by_new_observation
invalid_observation
```

These values classify why an unrecorded Product Repository change finding is resolved. They do not prove correctness, evidence sufficiency, review completion, final acceptance, residual-risk acceptance, or security. Caller use is method-gated by [`volicord.reconcile_changes`](method-reconcile-changes.md); naming a basis does not authorize an agent-only dismissal.

`WriteDecisionReason.category` is a controlled category value. It uses only these supported values:

| Value | Category family |
|---|---|
| `scope` | Scope compatibility or scope-boundary reason. |
| `user_judgment` | Required user-owned judgment reason. |
| `sensitive_approval` | Required separate sensitive-action approval reason. |
| `write_compatibility` | Write-compatibility reason. |
| `baseline` | Baseline compatibility reason. |
| `effect_contract` | Change Unit effect contract compatibility reason. |
| `connection_capability` | Agent Connection compatibility or mode-support reason. |

These categories classify `volicord.prepare_write` decision reasons. They are not `CloseReadinessBlocker` objects and do not evaluate close readiness. Method-specific decision behavior and reason production stay with [`volicord.prepare_write`](method-prepare-write.md).

This value set controls `category` only. `WriteDecisionReason.code` is not a global closed enum. It is a method-scoped opaque reason code; method owners may show example codes without adding them to a global supported list. `message` is a free-form display string, and `related_refs` uses `StateRecordRef`.

`CloseReadinessBlocker.category` uses:

```text
task
open_run
scope
user_judgment
pending_user_judgment
sensitive_approval
write_compatibility
baseline
connection_capability
evidence
evidence_claim
evidence_provenance
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

<a id="evidence-observation-values"></a>
## Evidence observation values

`EvidenceUpdateProvenance.source_kind`, `EvidenceObservation.source_kind`, and `EvidenceObservationInput.source_kind` use:

```text
agent_report
connection_observation
external_tool
user_observation
reused_evidence
unverified_claim
```

Source-kind meanings:
- `agent_report` records a report made by an agent actor context. It is not an external tool result by itself.
- `connection_observation` records an observation attributed to a registered Agent Connection. It is not proof by itself.
- `external_tool` records output or status from an external tool. It is not a product correctness proof by itself.
- `user_observation` records a user-attributed observation. It is not final acceptance or any other authority-bearing judgment by itself.
- `reused_evidence` records reuse of prior evidence or artifacts. It is not a new observation by itself.
- `unverified_claim` preserves a claim without verified observation. It is not sufficient evidence by itself.

`EvidenceUpdateProvenance.assurance_level`, `EvidenceObservation.assurance_level`, and `EvidenceObservationInput.assurance_level` use:

```text
cooperative_report
registered_connection_observed
external_tool_result
user_observed
unverified
```

Assurance-level meanings:
- `cooperative_report` is a cooperative report from the submitting actor context.
- `registered_connection_observed` records that a registered Agent Connection observed the claim within its recorded connection context.
- `external_tool_result` records that the observation is based on an external tool result.
- `user_observed` records user-attributed observation provenance.
- `unverified` records absence of verified observation assurance.

These values classify evidence observation provenance. They do not grant user authority, satisfy final acceptance or residual-risk acceptance, prove product correctness, or change `GuaranteeDisplay.level`.

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

`cooperative` is the baseline fallback. `detective` may be displayed only when the security owner supports the claim and project enforcement facts, verified Agent Connection or User Channel provenance, enabled enforcement mechanism, and observed-scope facts support it. Declared connection capability alone cannot raise the displayed guarantee.

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

`ToolError.details.write_check_reason` and `ToolError.details.artifact_input_error.reason` helper values are owned by [API error details](error-details.md#error-detail-helper-values). This value-set document does not define machine-readable error detail semantics.

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
