# API judgment schemas

This document owns API schemas for user-owned judgment in the current MVP. It is documentation reference material only and does not record user decisions by itself.

## Owns / Does not own

This document owns:

- `UserJudgment`
- `UserJudgmentCandidate`
- `UserJudgmentOption`
- `UserJudgmentContext`
- `UserJudgmentResolution`
- `RecordUserJudgmentPayload`
- `SensitiveActionScope`
- `AcceptedRiskInput`
- user-owned judgment schema semantics

This document does not own:

- the product meaning and non-substitution rules for user-owned judgment; see [Core Model](../core-model.md)
- method behavior for requesting or recording judgment; see [User-judgment methods](method-user-judgment.md)
- active judgment-kind values, status values, presentation values, and required-for values; see [API Value Sets](schema-value-sets.md)
- final acceptance or residual-risk close effects; see [Core Model](../core-model.md) and [Close-task method](method-close-task.md)
- public error semantics for missing, unresolved, denied, or expired judgment; see [API Errors](errors.md)

## Boundary

Judgment schemas preserve the structure of a user-owned choice. They do not let broad approval replace product decisions, technical decisions, scope decisions, sensitive-action approval, final acceptance, residual-risk acceptance, cancellation, later QA waiver, or later verification-risk acceptance.

`UserJudgmentCandidate` is not a pending judgment.

Condition: a pending `UserJudgment` exists only after `harness.request_user_judgment` commits.

Effect: a recorded answer resolves only the specific pending judgment and its `judgment_kind`.

Non-claims:
- It does not silently update active scope.
- It does not create evidence.
- It does not create Write Authorization.
- It does not accept residual risk.
- It does not close a Task.

## `UserJudgment`

```yaml
UserJudgment:
  judgment_id: string
  project_id: string
  task_id: string
  change_unit_id: string | null
  judgment_kind: string
  status: string
  presentation: string
  question: string
  options: UserJudgmentOption[]
  context: UserJudgmentContext
  affected_refs: StateRecordRef[]
  required_for: string
  resolution: UserJudgmentResolution | null
  expires_at: string | null
  created_at: string
  resolved_at: string | null
```

`judgment_kind`, `status`, `presentation`, and `required_for` values are owned by [judgment values](schema-value-sets.md#judgment-values). Product meaning is owned by [Core Model user-owned judgment](../core-model.md#4-user-owned-judgment).

## `UserJudgmentCandidate`

`UserJudgmentCandidate` is a proposed focused question returned by another method when the next safe path requires user-owned judgment. It is displayable, but it is not durable until `harness.request_user_judgment` commits it.

```yaml
UserJudgmentCandidate:
  judgment_kind: string
  presentation: string
  question: string
  options: UserJudgmentOption[]
  context: UserJudgmentContext
  affected_refs: StateRecordRef[]
  required_for: string
  expires_at: string | null
```

## Option and context shapes

```yaml
UserJudgmentOption:
  option_id: string
  label: string
  description: string
  consequence: string
  is_default: boolean

UserJudgmentContext:
  summary: string
  related_refs: StateRecordRef[]
  artifact_refs: ArtifactRef[]
  visible_risks: AcceptedRiskInput[]
  constraints: string[]
```

`option_id` is scoped to the judgment. Rendered labels are display text, not canonical schema values.

## Resolution and answer payload

```yaml
UserJudgmentResolution:
  selected_option_id: string
  answer: RecordUserJudgmentPayload
  note: string | null
  accepted_risks: AcceptedRiskInput[]
  resolved_by_actor_kind: string

RecordUserJudgmentPayload:
  product_decision: object | null
  technical_decision: object | null
  scope_decision: object | null
  sensitive_action_scope: SensitiveActionScope | null
  final_acceptance: object | null
  residual_risk_acceptance: object | null
  cancellation: object | null
```

`selected_option_id` and `note` are request-level and resolution-level fields. `RecordUserJudgmentPayload` must not repeat them. Exactly one decision-specific payload branch should be populated for the active `judgment_kind` unless a method owner explicitly allows a narrower structure.

## `SensitiveActionScope`

`SensitiveActionScope` describes the named sensitive step the user is asked to approve. It is not `AuthorizedAttemptScope`, not Write Authorization, and not security authority; see [Security](../security.md).

```yaml
SensitiveActionScope:
  action_kind: string
  description: string
  intended_paths: string[]
  sensitive_categories: string[]
  command_or_tool_summary: string | null
  network_or_host_summary: string | null
  secret_or_credential_summary: string | null
  capability_claim: string
  expires_at: string | null
```

Sensitive-action approval can be required before write compatibility, run recording, or close, but it does not replace the `harness.prepare_write` path for product-file writes.

## `AcceptedRiskInput`

`AcceptedRiskInput` names a visible residual risk the user may accept for the judgment being recorded.

```yaml
AcceptedRiskInput:
  risk_id: string | null
  summary: string
  consequence: string
  related_refs: StateRecordRef[]
  accepted_for_close: boolean
```

Accepted risk is scoped to the named visible risk and the requested judgment. It is not verification, evidence sufficiency, QA, final acceptance, or proof that the result has no risk.

## Related owners

- [Core Model](../core-model.md) for user-owned judgment meaning and non-substitution rules.
- [User-judgment methods](method-user-judgment.md) for `harness.request_user_judgment` and `harness.record_user_judgment`.
- [API Value Sets](schema-value-sets.md) for `judgment_kind`, `presentation`, `required_for`, status, and option display boundaries.
- [API State Schemas](schema-state.md) for `StateRecordRef`.
- [API Artifact Schemas](schema-artifacts.md) for `ArtifactRef`.
- [Scope Reference](../scope.md) for reserved judgment routes and active-boundary checks.
