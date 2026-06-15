# API state schemas

This document owns API state-shaped schemas for the baseline scope. It defines public response shapes for `StateSummary`, `StateRecordRef`, lifecycle state as API data, state-related snapshots, `ShapingReadiness`, and display shapes such as `NextActionSummary`, `WriteAuthoritySummary`, `EvidenceSummary`, `CloseReadinessBlocker`, `ValidatorResult`, and `GuaranteeDisplay`.

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

Owner links:
- Response branch selection: [Common response branches](schema-core.md#common-response)
- Method behavior and effects: [API Methods](methods.md) and method owner documents

## State references

Meaning:
- `StateRecordRef` is the common public reference shape for Core-owned records that appear in API responses.

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
- storage table names and DDL: [Storage Records](../storage-records.md)

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

Does not imply:
- `StateSummary` field presence does not define whether a method committed.

Owner links:
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

Owner links:
- Method behavior and durable effects: method owner documents routed from [API Methods](methods.md) and [Storage Effects](../storage-effects.md)

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
  authorized_attempt_scope: object
  basis_state_version: integer
  expires_at: string | null

WriteDecisionReason:
  category: string
  code: string
  message: string
  related_refs: StateRecordRef[]
```

Meaning:
- `NextActionSummary` is the canonical next-action display shape.
- `owner_method`, when present, names the supported public method that owns the next step.
- `label` and `blocking_question` are display text; `required_refs` names records needed for the next step.
- `WriteDecisionReason` is used by `PrepareWriteResult.write_decision_reasons`.

It is distinct from `CloseReadinessBlocker`.

Owner links:
- `owner_method` values: [method name values](schema-value-sets.md#method-name-values)
- Supported categories and reason values: [state and blocker values](schema-value-sets.md#state-and-blocker-values)
- Public error code meaning: [API error codes](error-codes.md)

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

Owner links:
- `ArtifactRef`: [API Artifact Schemas](schema-artifacts.md)
- Evidence sufficiency meaning: [Core Model evidence and run authority](../core-model.md#9-evidence-and-run-authority)
- Method behavior: method owner documents routed from [API Methods](methods.md)

<a id="close-readiness-and-validation-shapes"></a>
## Close readiness and validation shapes

```yaml
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
- `CloseReadinessBlocker` is a data shape for close-readiness findings.

This shape does not define close-readiness meaning, response routing, or persistence behavior.

Owner links:
- Close-readiness meaning and non-substitution rules: [Core Model close readiness](../core-model.md#close_task)
- Response branch behavior, close-readiness evaluation order, and committed blocked outcomes: [`harness.close_task`](method-close-task.md)
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
