# API value sets

This document owns supported API value sets and enum-like public values for the baseline scope. Naming a reserved or out-of-scope value does not widen baseline scope.

## Owns / does not own

This document owns:

- supported public method-name values
- supported actor-source values
- supported next-action values
- API `response_kind` and `effect_kind` values
- supported `FailureCategory` identifiers
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
- Values outside the supported lists are not baseline API values.
- Naming a value outside a supported list does not widen baseline scope.
- Rendered labels are display text. They do not replace the canonical values listed in this document.
- API examples must use supported enum-like values from this document unless the schema owner explicitly defines the field as a free-form display string, an opaque identifier, or an opaque classification string.
- A string-like field is controlled by this document only when the schema owner routes that field to a value set here. Opaque identifiers, opaque classification strings, and free-form display strings stay with their schema or method owner.
- A method example may show an opaque reason code or classification string without making that string a supported global value.

## Find a value set

| Value family | Start here |
|---|---|
| Methods, actor provenance, next actions, response branches, failure categories, and operation categories | [Method name values](#method-name-values), [Actor source values](#actor-source-values), [Next-action values](#next-action-values), [Response and effect values](#response-and-effect-values), [Failure category values](#failure-category-values), and [Operation category values](#operation-category-values) |
| Record references, project continuity, and `Task` lifecycle | [Record and reference values](#record-and-reference-values), [Project continuity values](#project-continuity-values), and [Task lifecycle values](#task-lifecycle-values) |
| Method-specific request and result values | [Method-local values](#method-local-values) |
| Observation health, evidence state, and blocker categories | [State and blocker values](#state-and-blocker-values) |
| Evidence provenance and assurance | [Evidence observation values](#evidence-observation-values) |
| Artifact and judgment values | [Artifact values](#artifact-values) and [Judgment values](#judgment-values) |
| Error-detail helpers and values outside the baseline | [Error detail helper values](#error-detail-helper-values) and [Profile-gated and reserved values](#profile-gated-and-reserved-values) |

<a id="method-name-values"></a>
## Method name values

The supported public method-name set is:

```text
volicord.intake
volicord.update_scope
volicord.status
volicord.get_operation_result
volicord.check_close
volicord.prepare_write
volicord.prepare_evidence_capture
volicord.stage_artifact
volicord.record_run
volicord.request_user_action
volicord.resolve_user_action
volicord.reconcile_changes
volicord.close_task
```

Method behavior is owned by method owner documents routed from [API Methods](methods.md). Method names are not `Task` lifecycle values.

<a id="actor-source-values"></a>
<a id="actor-values"></a>
## Actor source values

Actor provenance fields such as `EvidenceObservation.observed_by_actor_source`, `EvidenceObservationInput.observed_by_actor_source`, and `UserActionResolution.resolved_by_actor_source` use the `ActorSource` value set:

| Value | Used by | Owner route |
|---|---|---|
| `local_user` | User Channel invocation provenance for every user-action resolution. | Invocation meaning: [Agent Connection](../agent-connection.md); resolution shape owner: [API User Action Schemas](schema-user-action.md). |
| `agent_connection:<connection_id>` | Agent Connection invocation provenance and agent-created or agent-observed state. | Invocation meaning: [Agent Connection](../agent-connection.md); nested shape owners define where the value appears. |
| `system` | Internal system provenance where an owner explicitly allows it. | Method and storage owners define where the value appears. |

These values classify derived invocation or persisted actor provenance. They do not by themselves create user-owned judgment, evidence relevance, approval, scope-decision authority, final acceptance, residual-risk acceptance, or write ticket. Every user-action resolution requires `resolved_by_actor_source=local_user` with compatible User Channel provenance as defined by [Agent Connection](../agent-connection.md) and the method owner.

<a id="next-action-values"></a>
## Next-action values

`NextActionSummary.action_kind` is a controlled action-category value. It uses only these supported values and owner-supported invocation categories:

| `action_kind` value | `owner_method` when one method owns the next step | `allowed_operation_categories` |
|---|---|---|
| `update_scope` | `volicord.update_scope` | `agent_workflow` |
| `prepare_write` | `volicord.prepare_write` | `agent_workflow` |
| `stage_artifact` | `volicord.stage_artifact` | `agent_workflow` |
| `record_run` | `volicord.record_run` | `agent_workflow` |
| `request_user_action` | `volicord.request_user_action` | `agent_workflow` |
| `resolve_user_action` | `volicord.resolve_user_action` | `user_only` |
| `reconcile_changes` | `volicord.reconcile_changes` | `agent_workflow`, `local_recovery` |
| `close_task` | `volicord.close_task` | `agent_workflow` |

`action_kind` is not a method-name value. `NextActionSummary.owner_method` uses the [method-name value set](#method-name-values) when one supported public method owns the next step, and it is `null` when no single owner method applies. An action with `owner_method=null` uses `allowed_operation_categories=[]`; its label and the containing owner identify any required host, terminal, filesystem, or setup work. The operation-category list describes owner-supported invocation paths, not current connection availability or granted authority. Method behavior for the next step stays with the method owner document routed from [API Methods](methods.md). The full `NextActionSummary` shape is owned by [API State Schemas](schema-state.md#current-position-display-shapes).

<a id="response-and-effect-values"></a>
## Response and effect values

The closed `response_kind` value set is:

```text
result
rejected
dry_run
```

The result-branch `effect_kind` value set is:

```text
read_only
core_committed
staging_created
no_effect
```

`response_kind` and `effect_kind` are branch metadata values. Their exact
singleton assignments, result `dry_run` compatibility, and closed generated
schemas are owned by [API Schema Core](schema-core.md#common-response).
Method-specific effects are owned by method owner documents, and public error
semantics for rejected branches are owned by [API error codes](error-codes.md) and
[API error routing](error-routing.md).

<a id="failure-category-values"></a>
## Failure category values

`FailureCategory` uses exactly these machine-readable identifiers:

```text
rejected
not_allowed
unavailable
degraded
corrupt
```

These identifiers correspond exactly to `Rejected`, `NotAllowed`,
`Unavailable`, `Degraded`, and `Corrupt`. Their semantic
boundaries are owned by the [Failure Model](../failure-model.md).

`ToolError.category` is a required controlled field with this value set, and
its value is fixed by `ToolError.code` according to the generated catalog in
[API error codes](error-codes.md#error-taxonomy). A category does not replace
the code, a domain-specific
`ToolError.details.reason`, or response-branch selection. API branch routing,
including continued-operation `degraded` diagnostics and method-owned
`not_allowed` results, is owned by [API error routing](error-routing.md).

<a id="opaque-and-method-scoped-string-fields"></a>
## Opaque and method-scoped string fields

The fields below are intentionally not global closed value sets:

| Field | Classification | Owner route |
|---|---|---|
| `EventRef.event_kind` | Opaque `event_kind` classification string. Method examples may show event-kind strings, but this document does not publish an exhaustive public event-kind value set. | Shape owner: [API Schema Core](schema-core.md#shared-support-shapes). Event-producing behavior: method owner documents. |
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
shaping_checkpoint
shaping_gap
write_ticket
user_action_request
user_action_resolution
run
evidence_summary
evidence_observation
evidence_capture_intent
evidence_producer
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

These values classify durable project-level context. They do not by themselves create current `Task` authority, satisfy pending user actions, prove evidence, grant write-ticket authority, satisfy close readiness, or accept residual risk for a future close basis.

<a id="task-lifecycle-values"></a>
## Task lifecycle values

`StateSummary.mode` and resolved `Task.mode` fields use:

```text
advisor
direct
work
```

`requested_mode` for `volicord.intake` also accepts `auto` as an input-only value. Output `Task.mode` fields use `advisor`, `direct`, or `work`; intake resolution behavior is owned by [Intake method](method-intake.md).

`requested_control_level` uses:

```text
auto
observe
light
tracked
sensitive
```

`effective_control_level` uses the same set without `auto`. Control order is
`observe < light < tracked < sensitive`; Core and project policy may raise a
Task but never lower its effective control.

Mode and `work_phase` jointly select progression and execution authority:

| `Task.mode` | `work_phase` | Supported authority | Successful `intent=complete` result |
|---|---|---|---|
| `advisor` | `shaping` | `volicord.record_shaping` | `advice_only` |
| `direct` | `implementation` | `direct` | `completed` |
| `work` | `shaping` | `volicord.record_shaping`, then `volicord.advance_task` | `completed` |
| `work` | `implementation` | `implementation` | `completed` |

`RunKind` contains only `direct` and `implementation`. Advisor Tasks use shaping checkpoints and do not use `prepare_write` or write tickets.

`StateSummary.work_phase` and `TaskFlowItem.work_phase` use:

```text
shaping
implementation
```

This phase preserves one Task's longer-lived outcome while separating analysis
from write-capable execution. It is independent of `lifecycle_phase`.

`ShapingCheckpoint.readiness` uses:

```text
blocked
ready
superseded
```

`blocked` means the current checkpoint is structurally incomplete or has a
`current` gap. `ready` means its baseline and implementation boundary exist,
non-user shaping gaps are closed, and no gap is `current`; resolved user-owned
gaps may still await their application owner. `superseded` is a non-current
predecessor replaced through the explicit checkpoint operation. Readiness does
not apply a decision, advance a Task, finalize advice, or establish a close
basis.

`ShapingGapInput.gap_kind` and `ShapingCheckpointGap.gap_kind` use exactly:

```text
goal_missing
scope_boundary_missing
non_goals_missing
acceptance_criteria_missing
autonomy_boundary_missing
implementation_boundary_missing
baseline_missing
user_product_decision_required
user_technical_decision_required
user_scope_decision_required
sensitive_approval_required
```

The final four kinds are user-owned and require one compatible linked
UserAction draft; all other kinds forbid one.

`ShapingCheckpointGap.status` uses:

```text
current
resolved
applied
```

`current` means the decision or shaping work is unresolved. `resolved` means
exact User Channel authority exists but its semantic application owner has not
applied it. `applied` means that owner consumed and bound the exact resolution
to current state. Resolution alone never selects `applied`.

User-owned gaps use this mode-aware closed application-owner mapping:

```text
work + user_product_decision_required -> volicord.advance_task
work + user_technical_decision_required -> volicord.advance_task
advisor|work + user_scope_decision_required -> volicord.update_scope
work + sensitive_approval_required -> volicord.advance_task
advisor + user_product_decision_required -> volicord.record_shaping
advisor + user_technical_decision_required -> volicord.record_shaping
advisor + sensitive_approval_required -> volicord.record_shaping
```

The only `ShapingDecisionApplicationOwner` values are
`volicord.update_scope`, `volicord.record_shaping`, and
`volicord.advance_task`.

`WorkflowProjection.kind` uses exactly:

```text
no_active_task
shaping_required
awaiting_user_action
ready_to_apply_decisions
ready_for_change_unit
ready_to_finalize_advice
ready_for_implementation
implementation
close_review
terminal
```

`WorkflowProjection.blocking_reason`, when non-null, uses exactly:

```text
no_current_checkpoint
shaping_gaps_current
user_action_pending
resolved_decisions_not_applied
change_unit_required
advisor_finalization_required
explicit_advance_required
recovery_constraint
inconsistent_authority_state
```

These values describe current progression only. Close-readiness blockers keep
their own local categories and remediation and do not select a workflow kind
or required action.

`StateSummary.acceptance_policy` uses:

```text
required
not_required
policy_dependent
```

The policy is selected and reasoned from effective control plus the current
project policy. `observe` uses `not_required`; `tracked` and `sensitive` use
`required`; `light` uses `policy_dependent` and may resolve to `not_required`
only under an explicit current project rule and its complete close conditions.
It is not an agent-selected waiver.

`TaskLineageSummary.relation` uses:

```text
continues
derived_from
split_from
replaces
implements_advice_from
```

`CarryForwardDisposition.kind` uses:

```text
scope
non_goals
user_decisions
source_refs
context_refs
known_limitations
unresolved_obligations
residual_risks
baseline
```

Its `status` uses `applied` or `reference_only`. Applied material is validated
again as new-Task input. Reference-only context does not revive predecessor
scope, judgment, evidence, acceptance, risk acceptance, or write authority.

`WorkspaceContext.vcs` currently uses only `git`. `branch_ref=null` represents
detached HEAD. `AuthorityReceipt.next_actor` uses `agent`, `user`, or `none`.

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

Lifecycle meaning for user-owned action waiting:

- `waiting_user` means the current non-terminal Task has at least one pending `UserActionRequest` with current compatible basis state and a non-informational `required_for` target that still requires a user resolution. This includes choice judgments such as `product_decision` and `technical_decision` and also evidence observations; it is not limited to judgments that use Core-created authority options.
- Informational requests and pending requests with stale or superseded basis state do not create or preserve `waiting_user`.
- Resolving the last such pending user action restores `ready` when a current Change Unit exists and `shaping` otherwise. Resolving one of several such actions preserves `waiting_user`.
- `completed`, `cancelled`, and `superseded` are terminal and are never replaced by user-action lifecycle maintenance.

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

MCP mutation argument `detail` uses:

```text
summary
workflow
full
```

`summary` is the default wrapper over a fresh authority receipt and compact
method result, `workflow` adds normalized next actions, and `full` pairs the
fresh receipt with the exact bounded method result. The transport still uses
its compatibility text member, but the text is a bounded summary rather than a
duplicate full JSON document.

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
- `keep_current` updates scope-related `Task` fields without changing the current Change Unit.
- `create_current` creates the current Change Unit when there is no suitable current Change Unit.
- `replace_current` replaces the current Change Unit with a new work boundary.

Method behavior for each `operation` is owned by [`volicord.update_scope`](method-update-scope.md). The supported value set stays here so API examples and schema readers have one canonical value owner.

`ChangeUnitEffectContract.allowed_effects` and `ChangeUnitEffectContract.forbidden_effects` use:

```text
product_file_write
artifact_registration
run_recording
user_action_request
evidence_update
sensitive_action
external_network
secret_access
```

These values classify effects as Core state. They do not by themselves create a runtime sandbox, command interception, network blocking, secret isolation, a user-action request or resolution, sensitive-action approval, evidence, write ticket, final acceptance, close readiness, or residual-risk acceptance.

`volicord.check_close` has no `intent` field. `volicord.close_task.intent` uses:

```text
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

`PrepareWriteResult.write_ticket_effect` uses:

```text
none
would_issue
issued
reused
```

`issued` means a committed allowed result created one ticket. `reused` means it
returned one already-active compatible ticket without creating another.
`would_issue` is preview-only and creates no ticket.

`WriteTicket.state` uses:

```text
open
observed
reconciled
closed
invalidated
revoked
```

These states describe Volicord ticket authority and observation lifecycle. They do not imply filesystem ACLs, OS-level enforcement, shell permission, command approval, or proof that a write occurred.

`WriteTicketStateSummary.status` uses:

```text
active
consumed
invalidated
revoked
```

`WriteTicket.invalidation_reason` uses:

```text
scope_revision_changed
change_unit_changed
baseline_changed
workspace_changed
approval_basis_changed
idle_timeout
task_closed
explicit_revoke
```

These reasons are state-bound. `basis_state_version` mismatch and unrelated
state changes are deliberately absent.

A change to the normalized write-authority fingerprint durably invalidates
affected active tickets with `status=invalidated` and
`invalidation_reason=explicit_revoke`; a normalized-equivalent policy apply
does not. Historical consumed tickets remain `consumed` and inspectable.
`policy_authority_mismatch` is a `WRITE_TICKET_INVALID` error-detail reason,
while `policy_authority_stale` and `write_ticket_policy_changed` are Guard
diagnostic values. None of those three values belongs to this invalidation
reason set.

`RecordRunRequest.kind` and `RunSummary.kind` use:

```text
implementation
direct
```

The Task-mode compatibility matrix above is exhaustive. These values have no compatibility aliases, and an incompatible mode/kind pair is not recorded.

<a id="state-and-blocker-values"></a>
## State and blocker values

The `CloseReadinessBlocker` object shape is owned by [API State Schemas](schema-state.md#close-readiness-and-validation-shapes). This section owns the supported `CloseReadinessBlocker.category` values and neighboring state/blocker values.

`PlannedBlocker.source_kind` uses:

```text
write_decision
close_readiness
```

`IntegrationProfile` uses exactly `record`. It selects the managed Codex
workflow configuration and is not a Task risk grade or a guarantee label.

Guard prompt-related data, when present, is an observation only. It cannot
resolve a UserAction, create user authority, or substitute for the CLI inbox.

`UnrecordedChangeFinding.status` uses:

```text
unresolved
resolved
```

The separate `ObservationConfidence` value used by `MutationAssessment` and
workflow diagnostics uses `confirmed`, `structured`, `heuristic`, or `unknown`.
`ObservedEffectKind` uses `read_only`, `product_file_write`,
`non_product_write`, `external_effect`, or `unknown`. These values do not
classify Repository Observation states or Unrecorded Changes.
[Repository Observation](../repository-observation.md) uses `open`, `complete`,
and `unavailable`; an Unrecorded Change exists only for a complete non-empty
unmatched delta.

<a id="unrecorded-change-resolution-basis-values"></a>
`UnrecordedChangeResolutionSummary.resolution_basis` and stored unrecorded-change resolution metadata use:

```text
reverted
covered_by_write_ticket
recorded_as_expected_write
accepted_by_user
```

These values classify why an unrecorded Product Repository change finding is resolved. They do not prove correctness, evidence sufficiency, review completion, final acceptance, residual-risk acceptance, or security. Caller use is method-gated by [`volicord.reconcile_changes`](method-reconcile-changes.md); naming a basis does not authorize an agent-only dismissal.

`WriteDecisionReason.category` is a controlled category value. It uses only these supported values:

| Value | Category family |
|---|---|
| `scope` | Scope compatibility or scope-boundary reason. |
| `workspace` | Product Repository workspace or changed-path compatibility reason. |
| `user_action` | Required user-owned action reason. |
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
user_action
pending_user_action
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

`StageArtifactResult.evidence_state` and `EvidenceSummary.evidence_state` use the evidence attachment display values below when that field is present:

```text
prepared
attached
accepted_for_close
```

These values are user-facing presentation states. `accepted_for_close` means evidence is available to the current close-readiness calculation; it is not a correctness proof, test-sufficiency proof, QA result, final acceptance, or residual-risk acceptance.

<a id="evidence-gate-values"></a>
### Evidence gate values

`EvidenceGateSummary.state` and a selected `SummaryCard.evidence` use exactly:

```text
not_required
optional_none
required_missing
partial
sufficient
stale
blocked
```

| Value | Meaning |
|---|---|
| `not_required` | No active criterion has `required` or `optional` evidence; zero criteria and an all-`not_required` set use this value. |
| `optional_none` | At least one active criterion is `optional`, none is `required`, and no optional criterion has recorded evidence support. |
| `required_missing` | At least one active criterion is `required` and none of the required criteria has recorded evidence support. |
| `partial` | Some required evidence support exists but the required set is not sufficient, or optional-only recorded evidence is not all `supported`. |
| `sufficient` | Every required criterion is exactly `supported` with no evidence-claim, provenance, or artifact-availability blocker; when there are only optional criteria, every optional item that has recorded support is `supported`. |
| `stale` | Required evidence or its provenance is stale against the current close basis and no higher-precedence evidence condition is blocked. |
| `blocked` | A required criterion is contradicted, or a non-stale evidence or provenance condition blocks the evidence gate, or an unavailable artifact blocker names an artifact that supports a required criterion. |

Core computes this derived projection once from active criterion requirements and coverage, canonical evidence observation freshness and provenance, canonical availability of required-criterion supporting artifacts, and evidence-related close blockers. `blocked` takes precedence over `stale`; then required coverage selects `sufficient`, `partial`, or `required_missing`. `optional` and `not_required` criteria never create close blockers and never lower a sufficient required gate. Non-evidence close blockers, including unavailable close-basis result artifacts that do not support a required criterion, do not change the evidence gate. The projection is copied into status and close results, `StateSummary.evidence_gate`, and `SummaryCard.evidence`; attachment display states are not another gate calculation.

`AcceptanceCriterion.evidence_requirement`, intake criterion input, and
update-scope criterion replacement input use:

```text
required
optional
not_required
```

Only `required` current criteria can create evidence close blockers.

`EvidenceTarget.target_kind` uses:

```text
acceptance_criterion
supplemental_claim
```

`EvidenceCoverageUpdate.coverage_state` uses:

```text
unsupported
partial
supported
contradicted
not_applicable
```

Committed `EvidenceCoverageItem.coverage_state` uses the same values and may
also use:

```text
stale
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

On `EvidenceUpdateProvenance` and `EvidenceObservationInput`, these values are
requested provenance classifications. On committed `EvidenceObservation`, they
are Core-derived classifications. A valid request pair does not grant its own
assurance.

Source-kind meanings:
- `agent_report` records a report made by an agent actor context. It is not an external tool result by itself.
- `connection_observation` names an observation backed by a target-scoped
  registered-connection producer finalized from a current capture intent and
  complete receipt. An unanchored direct `record_run` input downgrades this
  requested value to `agent_report`.
- `external_tool` requires an authority-owned verified tool or command producer
  finalized from a current capture intent and complete receipt and bound to the
  exact output artifact. Direct unanchored requests still downgrade; verified
  artifact bytes alone are insufficient.
- `user_observation` names an observation backed by a current target-bound
  `evidence_observation` `UserActionResolution` from
  `volicord.resolve_user_action`. Direct
  unanchored selection downgrades, and the observation is never final acceptance
  or another authority-bearing judgment.
- `reused_evidence` records Core-validated reuse of a prior strong observation. Direct caller selection is downgraded, and validated reuse is not a new observation by itself.
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
- `registered_connection_observed` requires a target-scoped verified
  connection-observation anchor; it is not derived from the current Agent
  Connection invocation alone and does not imply supported relevance.
- `external_tool_result` requires the authority-owned producer record, exact
  canonical output binding, and current bytes. It classifies producer
  provenance only and does not imply supported relevance.
- `user_observed` requires a current target-scoped User Channel observation,
  exact outputs, and an exact stored `relevance_status` of `supported` or
  `contradicted`. It classifies local-user producer provenance and does not
  turn negative relevance into support. Only `supported` may satisfy evidence
  coverage or sufficiency or qualify for validated reuse that establishes
  `supported`.
- `unverified` records absence of verified observation assurance.

Core downgrades a requested strong pair without its required anchor to
`agent_report` / `cooperative_report`. For `reused_evidence`, Core revalidates
the original identity, target, `Task` and Change Unit, source Run, scope revision,
baseline, inherited assurance, exact outputs, producer anchor, and relevance
assessment at every recursive hop. These values do not
grant user authority, satisfy final acceptance or residual-risk acceptance,
prove product correctness, or change `GuaranteeDisplay.level`.

`EvidenceProducerAnchor.producer_kind` uses:

```text
unverified_caller
user_channel_observation
registered_connection_observation
verified_tool_invocation
verified_command_execution
reused_evidence
```

`registered_connection_observation`, `verified_tool_invocation`, and
`verified_command_execution` are available only through the current
`EvidenceCaptureIntent` / complete `EvidenceCaptureReceipt` / `record_run`
finalization path. `user_channel_observation` and recursively validated
`reused_evidence` retain their existing authority-owned paths. Caller input,
raw guard payloads, and artifact bytes alone cannot create any of these anchors.

`ConnectionObservationSourceSelector.source_kind` uses exactly:

```text
guard_event
```

`ConnectionObservationGuardEventKind` uses:

```text
pre_tool
post_tool
prompt_capture
```

`prompt_capture` identifies only an observed Guard event. It is not a UserAction
resolution channel, user answer, or verification basis.

`EvidenceRelevanceAssessment.status` and
the evidence-observation resolution body use `unassessed`, `supported`, and
`contradicted`; the User Channel resolution accepts only `supported` or
`contradicted`. `unassessed` means no independent authority has established
whether the observation supports its target; a complete matching registered
capture uses this status. `supported` requires a separate owner-defined
relevance authority, and validated reuse can only retain it from an already
supported authority chain. `contradicted` preserves a complete capture failure
or mismatch and an owner-defined negative relevance assessment. A current User
Channel observation with `contradicted` relevance retains its
`user_observation` / `user_observed` producer provenance, but it cannot satisfy
supported coverage or the supported-reuse gate. Strong evidence requires a
separate current `supported` assessment.

<a id="source-ref-values"></a>
### Source reference values

`SourceRef.source_kind` uses:

```text
repository_file
git_commit
git_diff
command
external_uri
user_context
```

These values select one structurally distinct, non-authoritative source body.
They classify context or provenance only and never select an evidence assurance
level, user authority, scope, or close authority.

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

This baseline value-set owner does not publish a supported stable `ValidatorResult.validator_id` set. A `validator_id` string is a reporting label, not a stable controlled value.

`GuaranteeDisplay.level` uses exactly:

```text
cooperative
```

`GuaranteeDisclosure.guarantee_class` uses:

```text
authority_record
user_action_resolution
```

`authority_record` reports Core authority state within the method contract.
`user_action_resolution` reports an immutable local-user resolution received
through the CLI User Channel.

`GuaranteeDisclosure.non_guarantees` uses:

```text
NotOsSandbox
NotNetworkIsolation
NotMalwareDefense
NotTamperProofAuditLog
NotCorrectnessProof
NotTestSufficiencyProof
NotHumanReviewReplacement
NotFullWritePrevention
NotFullFilesystemMonitoring
NotActorAttributionProof
NotIntentProof
```

These values are stable non-claims. They state that a result must not be interpreted as OS sandboxing, network isolation, malware defense, tamper-proof audit logging, product correctness proof, test sufficiency proof, human-review replacement, full write prevention, full filesystem monitoring, actor attribution proof, or intent proof.

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
## User-action and judgment values

`UserActionRequest.action_kind` uses exactly:

```text
product_decision
technical_decision
scope_decision
sensitive_approval
final_acceptance
residual_risk_acceptance
cancellation
evidence_observation
```

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
advance_task
prepare_write
record_run
close_complete
close_cancel
close_supersede
informational
```

`UserActionRequest.status` uses:

```text
pending
resolved
stale
superseded
expired
```

One Core evaluator derives these effective values from immutable resolution presence, basis compatibility, and current time. `resolved` means a resolution was recorded; it does not by itself mean approval, acceptance, authorization, or supporting evidence.

`JudgmentResolutionOutcome` uses:

```text
accepted
rejected
deferred
```

`UserActionBasis.coordinates.compatibility_status` uses:

```text
current
stale
superseded
```

Meaning:
- `current` means the basis currently matches the requirement it may satisfy.
- `stale` means the stored basis no longer matches current state; a resolved row may remain for audit but is ineligible for current requirements.
- `superseded` means an unanswered action request has been replaced by a newer request or basis and cannot be resolved successfully.

Authority option action values:
- `accept` maps to `accepted`.
- `reject` maps to `rejected`.
- `defer` maps to `deferred` only where the method or semantic owner permits deferral.

Resolution outcome meaning:
- `accepted` is the only outcome that can satisfy an authority-bearing judgment requirement when the judgment kind, basis, verified actor provenance, selected option, and `machine_action=accept` are otherwise compatible.
- `rejected` and `deferred` are durable user decisions but do not approve, accept, authorize, waive, or close anything.
- `blocked` is used by unrelated blocked-result and blocker value sets elsewhere in the product, but it is not a `JudgmentResolutionOutcome` value and cannot be persisted as a selected-option resolution.
- Absence of a machine-readable outcome must never be interpreted as `accepted`.

Pending-action relevance:
- A pending choice action blocks an operation only when its current `required_for` target includes that operation, its `judgment_kind` is relevant to that operation, and its `Task`, Change Unit, affected refs, and basis are compatible.
- For sensitive approval, the pending question is relevant only when its sensitive-action scope overlaps the current sensitive action requirement.
- `informational` actions are audit or display context and do not block write, run, or close operations by themselves.

`UserActionOption.option_id` is scoped to the request and is not a global value set. Rendered option labels are display text only. Current public `UserActionOption.machine_action` uses the authority option action values above. `UserActionOption.resolution_outcome` uses `JudgmentResolutionOutcome`; option labels and explanatory text must not invert the machine-readable action or outcome.

## Error detail helper values

`ToolError.details.write_ticket_reason` and `ToolError.details.artifact_input_error.reason` helper values are owned by [API error details](error-details.md#error-detail-helper-values). This value-set document does not define machine-readable error detail semantics.

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
