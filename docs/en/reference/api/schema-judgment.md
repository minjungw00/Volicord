# API judgment schemas

This document owns the choice-judgment payloads nested inside the common
user-action schemas. It does not own a separate durable judgment lifecycle.
Request identity, effective status, basis, adapter-neutral resolution form, expiry, channel paths,
and immutable resolution identity belong to [API User Action
Schemas](schema-user-action.md).

## Boundary

The seven `judgment_kind` values are `product_decision`,
`technical_decision`, `scope_decision`, `sensitive_approval`,
`final_acceptance`, `residual_risk_acceptance`, and `cancellation`. They appear
only inside `action_type=choice`. `evidence_observation` is the other user-action
family and is not a judgment.

## Choice request payload

```schema
UserActionDraft:
  action_type: choice
  judgment_kind: string
  presentation: short
  question: string
  options: UserActionOptionInput[] | null
  context: UserActionContext
  affected_refs: StateRecordRef[]
  sensitive_action_scope: SensitiveActionScope | null

UserActionOptionInput:
  option_id: string
  label: string
  description: string
  consequence: string
  is_default: boolean

UserActionOption:
  option_id: string
  label: string
  description: string
  consequence: string
  machine_action: accept | reject | defer
  resolution_outcome: accepted | rejected | deferred
  is_default: boolean

UserActionContext:
  summary: string
  related_refs: StateRecordRef[]
  artifact_refs: ArtifactRef[]
  visible_risks: AcceptedRiskInput[]
  constraints: string[]
```

Caller-authored options are accepted only for `product_decision` and
`technical_decision` and contain no machine action or outcome. For
authority-bearing kinds Core creates the options and mapping. `accept` maps only
to `accepted`, `reject` only to `rejected`, and `defer` only to `deferred`.
Labels and free text cannot invert that mapping or grant authority.

The common choice `UserActionBasis` carries current close-basis revision,
result refs, residual-risk IDs, and sensitive-action scope. Those coordinates
are Core-derived and are not resolution input.

## Choice resolution payload

```schema
UserActionResolutionInput:
  resolution_type: choice
  selected_option_id: string
  note: string | null

UserActionResolutionBody:
  resolution_type: choice
  selected_option_id: string
  machine_action: accept | reject | defer
  resolution_outcome: accepted | rejected | deferred
  note: string | null
  accepted_risk_ids: string[]
```

The user submits only a stored option ID and an optional note of at most 1,000
Unicode scalar values. Core copies the machine action and outcome from the
stored option and derives the current accepted residual-risk IDs from the
stored request and compatible basis. `judgment_kind` and durable `action_kind`
come from the request. Sensitive scope and other authority coordinates remain
in the request basis rather than being duplicated in the resolution.

The caller cannot submit or override machine action, outcome, risk objects,
accepted-risk IDs, an answer branch, sensitive scope, or rationale. Core does
not invent an uncaptured rationale or synthetic user answer.

An accepted choice satisfies its kind-specific requirement only when the basis
remains current and the immutable resolution has compatible `local_user` User
Channel provenance. Rejected and deferred choices remain durable user choices
but do not approve, accept, authorize, waive, or close anything.

## `SensitiveActionScope`

```schema
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

This is bounded sensitive-action context, not a write ticket, OS permission,
security boundary, final acceptance, or evidence.

<a id="acceptedriskinput"></a>
## `AcceptedRiskInput`

```schema
AcceptedRiskInput:
  risk_id: string
  summary: string
  consequence: string
  related_refs: StateRecordRef[]
  accepted_for_close: boolean
```

Visible risks belong to the request context and basis. The choice resolution
stores only the exact current IDs Core derived; it does not duplicate these
objects. Residual-risk acceptance does not prove that no risk remains.

## Related owners

- [API User Action Schemas](schema-user-action.md).
- [`volicord.request_user_action`](method-request-user-action.md).
- [`volicord.resolve_user_action`](method-resolve-user-action.md).
- [Core Model](../core-model.md) for judgment and non-substitution meaning.
