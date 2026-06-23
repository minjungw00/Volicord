# API state schemas

This document owns API state-shaped schemas for the baseline scope. It defines public response shapes for `StateSummary`, `StateRecordRef`, lifecycle state as API data, state-related snapshots, `ShapingReadiness`, and display shapes such as `NextActionSummary`, `WriteAuthoritySummary`, `WriteAuthorizationSummary`, `AuthorizedAttemptScope`, `EvidenceSummary`, `CurrentCloseBasis`, `ResidualRisk`, `RiskAcceptanceCoverage`, `CloseReadinessBlocker`, `ValidatorResult`, and `GuaranteeDisplay`.

## Owner boundary

This document owns state-shaped API fields, nesting, references, summaries, snapshots, display shapes, and the boundary between field presence and response effects. Neighboring contracts remain with these owners:

| Neighboring contract | Owner |
|---|---|
| Common envelopes and response branches | [API Schema Core](schema-core.md) |
| Supported enum-like values | [API Value Sets](schema-value-sets.md) |
| Method behavior | [API Methods](methods.md) and method owner documents |
| Public error semantics | [API error codes](error-codes.md) and [API error routing](error-routing.md) |
| Core lifecycle and close-readiness product meaning | [Core Model](../core-model.md) |
| Storage records and persistence effects | [Storage Records](../storage-records.md) and [Storage Effects](../storage-effects.md) |

## Boundary

State schemas describe API data shapes only. A state-shaped field does not choose a response branch or create persistence, Core transitions, replay rows, `task_events`, artifact effects, `Write Authorization` effects, or a `state_version` increment.

State projections must be truthful about computed state:
- A `null` or omitted field means the method did not select a value, the value is unavailable, or the owning schema explicitly allows absence. It must not be replaced with an empty value that implies "computed and none."
- Empty arrays such as `close_blockers: []` or `risk_acceptance_coverage: []` mean the relevant computation ran and found no entries.
- Mutation results and `volicord.status` projections must describe the same current state where their schemas overlap.
- Computed blockers use the same close-readiness calculation as the shared close-readiness engine; method owners decide only whether a branch persists an effect.

Owner links:
- Response branch selection: [Common response branches](schema-core.md#common-response)
- Method behavior and effects: [API Methods](methods.md) and method owner documents

<a id="state-references"></a>
## State references

Meaning:
- `StateRecordRef` is the common public reference shape for Core-owned records that appear in API responses.
- `record_kind` is a controlled value string.
- `record_id`, `project_id`, and `task_id` are opaque identifiers.

It is a public reference, not an embedded storage row.

```yaml
StateRecordRef:
  record_kind: string
  record_id: string
  project_id: string
  task_id: string | null
  state_version: integer | null
```

Owner links:
- `record_kind` values: [record and reference values](schema-value-sets.md#record-and-reference-values)
- storage record families and values: [Storage Records](../storage-records.md)
- storage table names and DDL: [Storage DDL](../storage-ddl.md)

## `StateSummary`

`StateSummary` is the compact current-position state returned by supported methods that need to show the current Task path.

```yaml
StateSummary:
  project_id: string
  state_version: integer
  task_ref: StateRecordRef | null
  mode: string | null
  lifecycle: TaskLifecycleState | null
  goal_summary: string | null
  scope_summary: string | null
  non_goals: string[]
  acceptance_criteria: string[]
  autonomy_boundary: string | null
  active_change_unit_ref: StateRecordRef | null
  baseline_ref: string | null
  shaping_readiness: ShapingReadiness | null
  pending_user_judgment_refs: StateRecordRef[]
  blocker_refs: StateRecordRef[]
  write_authority_summary: WriteAuthoritySummary | null
  evidence_summary: EvidenceSummary | null
  close_state: string | null
  close_blockers: CloseReadinessBlocker[]
  guarantee_display: GuaranteeDisplay | null
```

Meaning:
- `StateSummary` is a compact response shape for state references, summaries, and close-readiness fields.
- Method include flags may select only part of this shape. When a method owner says a projection is not selected, include-controlled fields such as `evidence_summary`, `close_state`, `close_blockers`, or `guarantee_display` are omitted instead of being returned as null or empty. A returned empty array means the projection was computed and found empty.
- `mode` and `close_state` are controlled value strings when present.
- `goal_summary`, `scope_summary`, `non_goals`, `acceptance_criteria`, and `autonomy_boundary` are free-form display strings.
- `baseline_ref` is an opaque baseline identifier.
- `pending_user_judgment_refs` lists current pending judgments relevant to the response view. A pending judgment is operation-blocking only when its `required_for` target, judgment kind, Task, Change Unit, affected refs, and basis are compatible with that operation.

Does not imply:
- `StateSummary` field presence does not define whether a method committed.

Owner links:
- `mode` and `close_state` values: [task lifecycle values](schema-value-sets.md#task-lifecycle-values)
- Commit decision branch: [Common response branches](schema-core.md#common-response)
- Method-specific commit behavior: method owner documents routed from [API Methods](methods.md)

## Task lifecycle state

`TaskLifecycleState` is the API shape for Task lifecycle fields that may appear inside `StateSummary` or close results.

```yaml
TaskLifecycleState:
  lifecycle_phase: string
  close_reason: string
  result: string
  closed_at: string | null
```

Owner links:
- Supported values for `lifecycle_phase`, `close_reason`, and `result`: [task lifecycle values](schema-value-sets.md#task-lifecycle-values)
- Product meaning of lifecycle areas: [Core Model task lifecycle](../core-model.md#6-task-lifecycle)

## `ShapingReadiness`

Meaning:
- `ShapingReadiness` is an API view shape over Task, Change Unit, pending judgment, evidence summary, blocker, and next-action fields.
- Its boolean fields and `gaps` array expose readiness-shaped data for the current state.

```yaml
ShapingReadiness:
  goal_summary_known: boolean
  scope_boundary_known: boolean
  non_goals_known: boolean
  affected_area_or_paths_known: boolean
  acceptance_criteria_known: boolean
  autonomy_boundary_known: boolean
  first_change_unit_known: boolean
  user_owned_blocker_kind: string | null
  next_safe_action: NextActionSummary | null
  gaps: ShapingGap[]

ShapingGap:
  gap_kind: string
  message: string
  blocker_ref: StateRecordRef | null
  user_judgment_candidate_ref: StateRecordRef | null
```

Meaning:
- `ShapingGap` can reference a blocker or user-judgment candidate by shape.
- `user_owned_blocker_kind` and `ShapingGap.gap_kind` are opaque readiness classification strings. They are not exhaustive public value sets unless an affected owner publishes narrower values.
- `ShapingGap.message` is a free-form display string.

Owner links:
- Method behavior and durable effects: method owner documents routed from [API Methods](methods.md) and [Storage Effects](../storage-effects.md)

<a id="current-position-display-shapes"></a>
## Current-position display shapes

```yaml
NextActionSummary:
  action_kind: string
  owner_method: string | null
  label: string
  blocking_question: string | null
  required_refs: StateRecordRef[]

WriteAuthoritySummary:
  status: string
  write_authorization_ref: StateRecordRef | null
  basis_state_version: integer | null
  intended_paths: string[]
  guarantee_display: GuaranteeDisplay | null

WriteAuthorizationSummary:
  write_authorization_ref: StateRecordRef
  status: string
  authorized_attempt_scope: AuthorizedAttemptScope
  basis_state_version: integer
  expires_at: string | null

AuthorizedAttemptScope:
  task_id: string
  change_unit_id: string
  intended_operation: string
  intended_paths: string[]
  product_file_write_intended: boolean
  sensitive_categories: string[]
  baseline_ref: string | null

WriteDecisionReason:
  category: string
  code: string
  message: string
  related_refs: StateRecordRef[]
```

Meaning:
- `NextActionSummary` is the canonical next-action display shape. Its valid fields are `action_kind`, `owner_method`, `label`, `blocking_question`, and `required_refs`.
- A `next_actions` entry that uses stale `action` or `reason` fields is not a valid `NextActionSummary`.
- `WriteAuthoritySummary.status` and `WriteAuthorizationSummary.status` are controlled value strings.
- `AuthorizedAttemptScope` is the one-attempt boundary captured by a `Write Authorization`.
- `AuthorizedAttemptScope` is not ordinary write approval, sensitive-action approval, final acceptance, residual-risk acceptance, or broad user approval.
- `WriteDecisionReason` is used by `PrepareWriteResult.write_decision_reasons`.

`NextActionSummary` field classifications:

| Field | Classification | Rule |
|---|---|---|
| `action_kind` | Controlled action category. | Uses the [next-action values](schema-value-sets.md#next-action-values). It is not a method-name value. |
| `owner_method` | Method-name value or `null`. | Names the API method that owns the next action when one supported public method applies. Use `null` when no single owner method applies. |
| `label` | Free-form display string. | Human- and agent-facing display text, not a canonical value. |
| `blocking_question` | Free-form display string or `null`. | The question to resolve before the action can proceed, or `null` when no blocking question is needed. |
| `required_refs` | `StateRecordRef[]`. | Records required for the next action. Use `[]` when there are no required refs. |

`AuthorizedAttemptScope` field classifications:

| Field | Classification | Rule |
|---|---|---|
| `task_id` | Opaque identifier. | Identifies the Task for the captured attempt boundary. |
| `change_unit_id` | Opaque identifier. | Identifies the Change Unit for the captured attempt boundary. |
| `intended_operation` | Free-form intent string. | Describes the intended operation without creating a controlled value set. |
| `intended_paths` | Normalized Product Repository path strings. | Product Repository relative paths after API-level path normalization. |
| `product_file_write_intended` | Boolean. | Indicates whether the captured attempt intended a product-file write. |
| `sensitive_categories` | Opaque sensitive-category classification strings. | Not an exhaustive public enum unless an affected method or profile owner publishes a narrower local list. |
| `baseline_ref` | Opaque baseline identifier or `null`. | Names the baseline identifier captured for the attempt boundary when present. |

`WriteDecisionReason` field classifications:

| Field | Classification | Rule |
|---|---|---|
| `category` | Controlled category value. | Uses the `WriteDecisionReason.category` values owned by [API Value Sets](schema-value-sets.md#state-and-blocker-values). |
| `code` | Method-scoped opaque reason code. | Not a global exhaustive enum. A method owner may define local codes, but example codes do not become global values. |
| `message` | Free-form display string. | Human- and agent-facing display text, not a canonical value. |
| `related_refs` | `StateRecordRef[]`. | Records related to the decision reason. Use `[]` when there are no related refs. |

`WriteDecisionReason` is distinct from `CloseReadinessBlocker`.

Owner links:
- `action_kind` values: [next-action values](schema-value-sets.md#next-action-values)
- `owner_method` values: [method name values](schema-value-sets.md#method-name-values)
- `WriteAuthoritySummary.status` and `WriteAuthorizationSummary.status` values: [method-local values](schema-value-sets.md#method-local-values)
- `WriteDecisionReason.category` values: [state and blocker values](schema-value-sets.md#state-and-blocker-values)
- `WriteDecisionReason.code` value-set boundary: [opaque and method-scoped string fields](schema-value-sets.md#opaque-and-method-scoped-string-fields)
- `WriteDecisionReason.code` production and local meaning: method owner documents, including [`volicord.prepare_write`](method-prepare-write.md)
- `Write Authorization` creation behavior: [`volicord.prepare_write`](method-prepare-write.md)
- `Write Authorization` product meaning and approval boundaries: [Core Model](../core-model.md)
- Public `ErrorCode` values are separate: [API error codes](error-codes.md)

## Evidence and run snapshot shapes

```yaml
EvidenceSummary:
  status: string
  completion_policy: CompletionPolicy
  coverage_items: EvidenceCoverageItem[]
  artifact_refs: ArtifactRef[]
  updated_by_run_ref: StateRecordRef | null

CompletionPolicy:
  evidence_required: boolean
  required_claims: string[]

EvidenceCoverageItem:
  claim: string
  required_for_close: boolean
  coverage_state: string
  supporting_refs: StateRecordRef[]
  supporting_artifact_refs: ArtifactRef[]
  gap_refs: StateRecordRef[]

RunSummary:
  run_ref: StateRecordRef
  kind: string
  summary: string
  observed_changes: ObservedChanges
  artifact_refs: ArtifactRef[]

ObservedChanges:
  changed_paths: string[]
  product_file_write_observed: boolean
  sensitive_categories: string[]
  baseline_ref: string | null
```

Meaning:
- `EvidenceSummary.status`, `EvidenceCoverageItem.coverage_state`, and `RunSummary.kind` are controlled value strings.
- `CompletionPolicy.required_claims`, `EvidenceCoverageItem.claim`, and `RunSummary.summary` are free-form claim or display strings.
- `ObservedChanges.changed_paths` are path strings.
- `ObservedChanges.sensitive_categories` are opaque sensitive-category classification strings unless an affected method or profile owner publishes a narrower local list.
- `ObservedChanges.baseline_ref` is an opaque baseline identifier.

Owner links:
- `ArtifactRef`: [API Artifact Schemas](schema-artifacts.md)
- evidence, coverage, and run-kind values: [state and blocker values](schema-value-sets.md#state-and-blocker-values) and [method-local values](schema-value-sets.md#method-local-values)
- Evidence sufficiency meaning: [Core Model evidence and run authority](../core-model.md#9-evidence-and-run-authority)
- Method behavior: method owner documents routed from [API Methods](methods.md)

<a id="close-readiness-and-validation-shapes"></a>
## Close readiness and validation shapes

```yaml
CurrentCloseBasis:
  close_basis_revision: integer
  scope_revision: integer
  task_id: string
  change_unit_id: string
  baseline_ref: string | null
  result_summary: string
  result_refs: StateRecordRef[]
  evidence_summary_ref: StateRecordRef | null
  residual_risks: ResidualRisk[]
  sensitive_categories: string[]
  sensitive_action_requirements: SensitiveActionRequirement[]
  recovery_constraints: string[]
  source_run_ref: StateRecordRef
  updated_at: string

SensitiveActionRequirement:
  action_kind: string
  normalized_paths: string[]
  sensitive_categories: string[]
  baseline_ref: string | null
  change_unit_id: string
  source_run_ref: StateRecordRef
  source_write_authorization_ref: StateRecordRef

ResidualRisk:
  risk_id: string
  summary: string
  consequence: string
  acceptance_required: boolean
  source_refs: StateRecordRef[]

RiskAcceptanceCoverage:
  risk_id: string
  accepted: boolean
  accepted_by_judgment_refs: StateRecordRef[]
  missing_reason: string | null

CloseReadinessBlocker:
  category: string
  code: string
  message: string
  related_refs: StateRecordRef[]
  next_actions: NextActionSummary[]

ValidatorResult:
  validator_id: string
  status: string
  severity: string | null
  message: string
  related_refs: StateRecordRef[]

GuaranteeDisplay:
  level: string
  basis: string
  capability_refs: StateRecordRef[]
```

Meaning:
- `CurrentCloseBasis` is the current result and residual-risk state used by close-readiness responses. It is not a terminal close summary.
- `close_basis_revision` and `scope_revision` are internal current-state coordinates surfaced for compatibility checks. They are not caller-selected authority.
- `ResidualRisk.risk_id` is an opaque Core-generated identifier. `ResidualRisk.summary` and `ResidualRisk.consequence` are display strings and do not authorize text matching.
- `result_refs`, `source_run_ref`, `source_refs`, `evidence_summary_ref`, and `accepted_by_judgment_refs` use `StateRecordRef`.
- `sensitive_categories` are opaque sensitive-category classification strings unless an affected method or profile owner publishes a narrower local list.
- `sensitive_action_requirements` are Core-derived close requirements from committed Runs and consumed `Write Authorization` records. Category-only caller input cannot establish or erase these requirements.
- `recovery_constraints` and `RiskAcceptanceCoverage.missing_reason` are free-form display strings.
- `RiskAcceptanceCoverage` reports whether the current residual-risk requirements are covered by compatible judgments.
- `CloseReadinessBlocker` is a data shape for close-readiness findings.
- `CloseReadinessBlocker.category` is a controlled value string.
- `CloseReadinessBlocker.code` is an owner-defined blocker code. It is not an exhaustive global public enum unless the blocker or method owner publishes a narrower local list.
- `CloseReadinessBlocker.message`, `ValidatorResult.message`, and `GuaranteeDisplay.basis` are free-form display strings.
- `ValidatorResult.validator_id` is a reporting label unless the value-set owner publishes a supported stable value.
- `ValidatorResult.status`, `ValidatorResult.severity`, and `GuaranteeDisplay.level` are controlled value strings.

These shapes do not define close-readiness meaning, response routing, or persistence behavior.

Close-basis reference rules:
- Caller-supplied close-assessment refs accepted into `CurrentCloseBasis.result_refs` or `ResidualRisk.source_refs` are limited to result/evidence record kinds `run`, `artifact`, `evidence_summary`, and `change_unit` unless an owner document explicitly adds another kind.
- `project_state`, `write_authorization`, `user_judgment`, `blocker`, `task_event`, `local_surface_registration`, and `task` are not caller-supplied result refs for a close basis unless an owner document explicitly adds them.
- Every accepted ref must exist, belong to the same project and Task, and be canonicalized by Core. Core never preserves caller-supplied `state_version` metadata as authority.
- Artifact refs used for close evidence must be linked to the Task and have `integrity_status=verified` plus current-byte verification at use time under [Artifact Storage](../storage-artifacts.md).
- Evidence refs must identify the current Task evidence summary. Run refs used as current close-basis result refs must identify a recorded current Run compatible with the current Task, current Change Unit, current scope revision, compatible baseline, and recorded status. Historical Runs are audit records unless a current Run explicitly reuses their verified artifacts or evidence and records that reuse.
- Core may add the current Run, current Change Unit, and current EvidenceSummary refs when constructing the canonical close basis.

Guarantee display rules:
- `GuaranteeDisplay` is derived from the project enforcement profile, verified bound surface registration, enabled enforcement mechanisms, and supported baseline scope.
- Capability declarations alone do not create guarantees, and cooperative-only deployments must not claim `detective`.
- `detective` requires supported enforcement or observation facts for the named surface and observed scope, not only text in a capability profile.
- `capability_refs` should identify the actual profile or surface facts when such refs are available.

Owner links:
- Close-readiness meaning and non-substitution rules: [Core Model close readiness](../core-model.md#close_task)
- Current close basis creation: [`volicord.record_run`](method-record-run.md)
- Judgment compatibility and accepted-risk input: [API Judgment Schemas](schema-judgment.md)
- Response branch behavior, close-readiness evaluation order, and committed blocked outcomes: [`volicord.close_task`](method-close-task.md)
- Close-readiness blocker/API response routing semantics: [API blocker routing](blocker-routing.md)
- Supported `CloseReadinessBlocker.category`, `ValidatorResult.status`, `ValidatorResult.severity`, and `GuaranteeDisplay.level` values: [API Value Sets](schema-value-sets.md#state-and-blocker-values)
- Security guarantee meaning: [Security](../security.md)

## Related owners

- [API Schema Core](schema-core.md) for `ToolEnvelope`, `ToolResultBase`, `ToolRejectedResponse`, and `ToolDryRunResponse`.
- [API Value Sets](schema-value-sets.md#state-and-blocker-values) for exact close-readiness blocker category values and neighboring state values.
- [API Methods](methods.md) and method owner documents for the methods that return these schemas.
- [API Artifact Schemas](schema-artifacts.md) for `ArtifactRef`.
- [API Judgment Schemas](schema-judgment.md) for `UserJudgmentCandidate`.
- [Storage Effects](../storage-effects.md) for persistence and state-effect consequences.
